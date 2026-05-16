use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

const DIRECTIVES: &[(&str, &str)] = &[
    ("shell", "Run command through /bin/sh -c on Unix."),
    ("task", "Run another task from same taskfile."),
    ("desc", "Describe task for listings and editor task views."),
    ("cd", "Change working directory for current scope."),
    ("back", "Undo one @cd within current scope."),
    ("echo", "Write text plus newline to stdout."),
    (
        "clean",
        "Remove file or directory relative to current working directory.",
    ),
    ("stop", "Write optional text, then stop with status 1."),
    (
        "set",
        "Set runtime variable, or capture block stdout with @set NAME:.",
    ),
    ("export", "Export variable to later child processes."),
    ("unset", "Remove runtime variable and export overlay."),
    (
        "return",
        "Write value and return derived status, or return block status.",
    ),
    ("success", "Return status 0."),
    ("fail", "Return status 1."),
    ("and", "Run block when previous expression returned 0."),
    (
        "or",
        "Run block when previous expression returned non-zero.",
    ),
    ("if", "Run block when condition is truthy."),
    ("else", "Else block for matching @if."),
    ("if-exists", "Run block when path exists."),
    ("if-missing", "Run block when path does not exist."),
    ("if-set", "Run block when variable is set."),
    ("if-unset", "Run block when variable is unset."),
    ("switch", "Switch on one value."),
    ("case", "Case inside @switch."),
    ("default", "Default case inside @switch."),
];

#[derive(Debug, Clone, Default)]
struct Document {
    text: String,
    analysis: Analysis,
}

#[derive(Debug, Clone, Default)]
struct Analysis {
    diagnostics: Vec<Diagnostic>,
    tasks: HashMap<String, TaskDef>,
    task_order: Vec<String>,
    variables: HashSet<String>,
}

#[derive(Debug, Clone)]
struct TaskDef {
    name: String,
    range: Range,
    description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LspSection {
    Top,
    Env,
    Task,
}

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<Mutex<HashMap<Url, Document>>>,
}

pub async fn run_stdio() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: Arc::new(Mutex::new(HashMap::new())),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_symbol_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["@".to_string(), "$".to_string()]),
                    ..CompletionOptions::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "cjtaskrunner-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "cjtaskrunner-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.update_document(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.update_document(params.text_document.uri, change.text)
                .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .lock()
            .await
            .remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    #[allow(deprecated)]
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> LspResult<Option<DocumentSymbolResponse>> {
        let documents = self.documents.lock().await;
        let Some(document) = documents.get(&params.text_document.uri) else {
            return Ok(None);
        };

        let symbols = document
            .analysis
            .task_order
            .iter()
            .filter_map(|name| document.analysis.tasks.get(name))
            .map(|task| DocumentSymbol {
                name: task.name.clone(),
                detail: task
                    .description
                    .clone()
                    .or_else(|| Some("task".to_string())),
                kind: SymbolKind::FUNCTION,
                tags: None,
                deprecated: None,
                range: task.range,
                selection_range: task.range,
                children: None,
            })
            .collect();

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let documents = self.documents.lock().await;
        let Some(document) = documents.get(&uri) else {
            return Ok(None);
        };

        let line = line_at(&document.text, position.line).unwrap_or_default();
        let prefix = &line[..byte_index_for_utf16_col(line, position.character)];
        let items = if prefix.trim_start().starts_with("@task ") {
            document
                .analysis
                .task_order
                .iter()
                .map(|name| CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some("CJTaskrunner task".to_string()),
                    ..CompletionItem::default()
                })
                .collect()
        } else if prefix.ends_with('$') || prefix.contains('$') {
            document
                .analysis
                .variables
                .iter()
                .map(|name| CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some("CJTaskrunner variable".to_string()),
                    ..CompletionItem::default()
                })
                .collect()
        } else {
            DIRECTIVES
                .iter()
                .map(|(name, detail)| CompletionItem {
                    label: format!("@{name}"),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some((*detail).to_string()),
                    insert_text: Some(format!("@{name}")),
                    ..CompletionItem::default()
                })
                .collect()
        };

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let documents = self.documents.lock().await;
        let Some(document) = documents.get(&uri) else {
            return Ok(None);
        };
        let Some(word) = word_at(&document.text, position) else {
            return Ok(None);
        };
        let directive = word.strip_prefix('@').unwrap_or(&word);
        let Some((name, detail)) = DIRECTIVES.iter().find(|(name, _)| *name == directive) else {
            return Ok(None);
        };

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("`@{name}`\n\n{detail}"),
            }),
            range: None,
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> LspResult<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let documents = self.documents.lock().await;
        let Some(document) = documents.get(&uri) else {
            return Ok(None);
        };
        let Some(task_name) = task_reference_at(&document.text, position) else {
            return Ok(None);
        };
        let Some(task) = document.analysis.tasks.get(&task_name) else {
            return Ok(None);
        };

        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: task.range,
        })))
    }
}

impl Backend {
    async fn update_document(&self, uri: Url, text: String) {
        let analysis = analyze(&text);
        self.client
            .publish_diagnostics(uri.clone(), analysis.diagnostics.clone(), None)
            .await;
        self.documents
            .lock()
            .await
            .insert(uri, Document { text, analysis });
    }
}

fn analyze(text: &str) -> Analysis {
    let mut analysis = Analysis::default();
    let mut section = LspSection::Top;
    let mut current_task: Option<String> = None;
    let mut seen_env = false;
    let mut env_names = HashSet::new();

    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index as u32;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if !line.starts_with(' ') {
            current_task = None;
            match parse_top_level_lsp(line) {
                Ok(key) if key == "env" => {
                    if seen_env {
                        push_diagnostic(
                            &mut analysis,
                            line_number,
                            0,
                            line.len(),
                            "multiple env sections are not allowed",
                        );
                    }
                    seen_env = true;
                    section = LspSection::Env;
                }
                Ok(key) => {
                    if let Err(err) = super::validate_task_name(&key) {
                        push_diagnostic(
                            &mut analysis,
                            line_number,
                            0,
                            line.len(),
                            &format!("invalid task name '{key}': {err}"),
                        );
                    } else if analysis.tasks.contains_key(&key) {
                        push_diagnostic(
                            &mut analysis,
                            line_number,
                            0,
                            line.len(),
                            &format!("duplicate task '{key}'"),
                        );
                    } else {
                        let range = Range::new(
                            Position::new(line_number, 0),
                            Position::new(line_number, utf16_len(&key)),
                        );
                        analysis.task_order.push(key.clone());
                        analysis.tasks.insert(
                            key.clone(),
                            TaskDef {
                                name: key.clone(),
                                range,
                                description: None,
                            },
                        );
                    }
                    current_task = Some(key);
                    section = LspSection::Task;
                }
                Err(message) => {
                    push_diagnostic(&mut analysis, line_number, 0, line.len(), message);
                    section = LspSection::Top;
                }
            }
            continue;
        }

        let indent = line.chars().take_while(|ch| *ch == ' ').count();
        if indent < 2 || indent % 2 != 0 {
            push_diagnostic(
                &mut analysis,
                line_number,
                0,
                indent.max(1),
                "indented entries must use an even number of spaces, at least two",
            );
            continue;
        }

        match section {
            LspSection::Env => {
                if indent != 2 {
                    push_diagnostic(
                        &mut analysis,
                        line_number,
                        0,
                        indent,
                        "env entries must use exactly two leading spaces",
                    );
                    continue;
                }
                analyze_env_entry(&mut analysis, line_number, &line[2..], &mut env_names);
            }
            LspSection::Task => {
                if current_task.is_none() {
                    push_diagnostic(
                        &mut analysis,
                        line_number,
                        0,
                        line.len(),
                        "command without a task",
                    );
                    continue;
                }
                let text = &line[indent..];
                for expression in super::split_line_expressions(text) {
                    if indent == 2 {
                        record_description(&mut analysis, current_task.as_deref(), &expression);
                    }
                    analyze_task_expression(&mut analysis, line_number, indent, &expression);
                }
            }
            LspSection::Top => {
                push_diagnostic(
                    &mut analysis,
                    line_number,
                    0,
                    line.len(),
                    "indented entry is not under env or a task",
                );
            }
        }
    }

    analysis
}

fn record_description(analysis: &mut Analysis, current_task: Option<&str>, expression: &str) {
    let Some(task_name) = current_task else {
        return;
    };
    let Some(args) = expression.strip_prefix("@desc") else {
        return;
    };
    if !args.is_empty() && !args.starts_with(char::is_whitespace) {
        return;
    }
    if let Some(task) = analysis.tasks.get_mut(task_name) {
        task.description = Some(args.trim().to_string());
    }
}

fn parse_top_level_lsp(line: &str) -> std::result::Result<String, &'static str> {
    if !line.ends_with(':') || line[..line.len() - 1].contains(':') {
        return Err("top-level entries must be a key followed by ':'");
    }
    let key = &line[..line.len() - 1];
    if key.trim() != key || key.is_empty() {
        return Err("invalid top-level key");
    }
    Ok(key.to_string())
}

fn analyze_env_entry(
    analysis: &mut Analysis,
    line_number: u32,
    entry: &str,
    env_names: &mut HashSet<String>,
) {
    let Some(colon_index) = entry.find(':') else {
        push_diagnostic(
            analysis,
            line_number,
            2,
            entry.len() + 2,
            "env entry must contain ':'",
        );
        return;
    };
    let raw_key = &entry[..colon_index];
    let key = raw_key.strip_suffix('?').unwrap_or(raw_key);
    if let Err(err) = super::validate_env_name(key) {
        push_diagnostic(
            analysis,
            line_number,
            2,
            colon_index + 2,
            &format!("invalid env name '{key}': {err}"),
        );
        return;
    }
    if !env_names.insert(key.to_string()) {
        push_diagnostic(
            analysis,
            line_number,
            2,
            colon_index + 2,
            &format!("duplicate env entry '{key}'"),
        );
    }
    analysis.variables.insert(key.to_string());
}

fn analyze_task_expression(
    analysis: &mut Analysis,
    line_number: u32,
    indent: usize,
    expression: &str,
) {
    collect_variables(analysis, expression);

    let Some(rest) = expression.strip_prefix('@') else {
        return;
    };
    let (name, args) = super::split_directive(rest);
    if name.is_empty() {
        push_diagnostic(analysis, line_number, indent, indent + 1, "empty directive");
        return;
    }
    if !DIRECTIVES.iter().any(|(known, _)| *known == name) {
        push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + name.len() + 1,
            &format!("unknown directive @{name}"),
        );
        return;
    }

    if invalid_trailing_colon(name, args) {
        push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            "directive does not use trailing ':' here",
        );
    }

    match name {
        "task" => {
            if arg_count(args) != Some(1) {
                push_diagnostic(
                    analysis,
                    line_number,
                    indent,
                    indent + expression.len(),
                    "@task expects exactly one task name",
                );
            }
        }
        "cd" | "clean" | "if-exists" | "if-missing" | "switch" | "case" => {
            if arg_count(args) != Some(1) {
                push_diagnostic(
                    analysis,
                    line_number,
                    indent,
                    indent + expression.len(),
                    &format!("@{name} expects exactly one argument"),
                );
            }
        }
        "unset" | "if-set" | "if-unset" => match super::split_words(args) {
            Ok(argv) if argv.len() == 1 => {
                if let Ok(variable) = super::parse_variable_name_token(&argv[0]) {
                    analysis.variables.insert(variable);
                }
            }
            _ => push_diagnostic(
                analysis,
                line_number,
                indent,
                indent + expression.len(),
                &format!("@{name} expects exactly one variable name"),
            ),
        },
        "set" => analyze_set_args(analysis, line_number, indent, expression, args),
        "export" => analyze_export_args(analysis, line_number, indent, expression, args),
        "desc" => {}
        "default" | "success" | "fail" => {
            if !args.trim().is_empty() {
                push_diagnostic(
                    analysis,
                    line_number,
                    indent,
                    indent + expression.len(),
                    &format!("@{name} does not take arguments"),
                );
            }
        }
        "back" => {
            if !args.trim().is_empty() {
                push_diagnostic(
                    analysis,
                    line_number,
                    indent,
                    indent + expression.len(),
                    &format!("@{name} does not take arguments"),
                );
            }
        }
        "and" | "or" => {
            if !args.trim().is_empty() {
                push_diagnostic(
                    analysis,
                    line_number,
                    indent,
                    indent + expression.len(),
                    &format!("@{name} does not take arguments"),
                );
            }
        }
        _ => {}
    }
}

fn analyze_set_args(
    analysis: &mut Analysis,
    line_number: u32,
    indent: usize,
    expression: &str,
    args: &str,
) {
    if args.trim().is_empty() {
        push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            "@set expects NAME and value, or NAME: for capture",
        );
        return;
    }

    let capture = args.trim_end().ends_with(':');
    let name = if capture {
        args.trim_end().trim_end_matches(':').trim()
    } else {
        args.trim_start()
            .split_once(char::is_whitespace)
            .map(|(name, _)| name)
            .unwrap_or("")
    };
    if let Err(err) = super::validate_env_name(name) {
        push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            &format!("invalid env name '{name}': {err}"),
        );
    } else {
        analysis.variables.insert(name.to_string());
    }
}

fn analyze_export_args(
    analysis: &mut Analysis,
    line_number: u32,
    indent: usize,
    expression: &str,
    args: &str,
) {
    let Some(name) = args.split_whitespace().next() else {
        push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            "@export expects NAME or NAME value",
        );
        return;
    };
    if let Err(err) = super::validate_env_name(name) {
        push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            &format!("invalid env name '{name}': {err}"),
        );
    } else {
        analysis.variables.insert(name.to_string());
    }
}

fn invalid_trailing_colon(name: &str, args: &str) -> bool {
    if name == "set" {
        return false;
    }
    name.ends_with(':')
        || matches!(
            name,
            "if" | "if-exists"
                | "if-missing"
                | "if-set"
                | "if-unset"
                | "else"
                | "switch"
                | "case"
                | "default"
        ) && args.trim_end().ends_with(':')
}

fn arg_count(args: &str) -> Option<usize> {
    super::split_words(args).ok().map(|argv| argv.len())
}

fn collect_variables(analysis: &mut Analysis, input: &str) {
    let mut chars = input.char_indices().peekable();
    while let Some((_, ch)) = chars.next() {
        if ch != '$' {
            continue;
        }
        let Some((_, next)) = chars.peek().copied() else {
            continue;
        };
        if next == '{' {
            chars.next();
            let mut expression = String::new();
            for (_, expr_ch) in chars.by_ref() {
                if expr_ch == '}' {
                    break;
                }
                expression.push(expr_ch);
            }
            let name = expression
                .split_once(":-")
                .map_or(expression.as_str(), |v| v.0);
            if super::validate_env_name(name).is_ok() {
                analysis.variables.insert(name.to_string());
            }
            continue;
        }
        if !(next == '_' || next.is_ascii_alphabetic()) {
            continue;
        }
        let mut name = String::new();
        while let Some((_, name_ch)) = chars.peek().copied() {
            if name_ch == '_' || name_ch.is_ascii_alphanumeric() {
                chars.next();
                name.push(name_ch);
            } else {
                break;
            }
        }
        if !name.is_empty() {
            analysis.variables.insert(name);
        }
    }
}

fn push_diagnostic(
    analysis: &mut Analysis,
    line: u32,
    start_col: usize,
    end_col: usize,
    message: &str,
) {
    analysis.diagnostics.push(Diagnostic {
        range: Range::new(
            Position::new(line, start_col as u32),
            Position::new(line, end_col.max(start_col + 1) as u32),
        ),
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("cjtaskrunner".to_string()),
        message: message.to_string(),
        related_information: None,
        tags: None,
        data: None,
    });
}

fn line_at(text: &str, line: u32) -> Option<&str> {
    text.lines().nth(line as usize)
}

fn byte_index_for_utf16_col(line: &str, col: u32) -> usize {
    let mut units = 0;
    for (index, ch) in line.char_indices() {
        if units >= col {
            return index;
        }
        units += ch.len_utf16() as u32;
    }
    line.len()
}

fn utf16_len(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

fn word_at(text: &str, position: Position) -> Option<String> {
    let line = line_at(text, position.line)?;
    let index = byte_index_for_utf16_col(line, position.character);
    let bytes = line.as_bytes();
    let mut start = index.min(bytes.len());
    let mut end = start;
    while start > 0 && is_word_byte(bytes[start - 1]) {
        start -= 1;
    }
    while end < bytes.len() && is_word_byte(bytes[end]) {
        end += 1;
    }
    (start < end).then(|| line[start..end].to_string())
}

fn is_word_byte(byte: u8) -> bool {
    byte == b'@' || byte == b'-' || byte == b'_' || byte.is_ascii_alphanumeric()
}

fn task_reference_at(text: &str, position: Position) -> Option<String> {
    let line = line_at(text, position.line)?;
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("@task ")?;
    let task_name = rest.split_whitespace().next()?;
    let line_start = line.len() - trimmed.len();
    let task_start = line_start + trimmed.find(task_name)?;
    let task_end = task_start + task_name.len();
    let cursor = byte_index_for_utf16_col(line, position.character);
    (cursor >= task_start && cursor <= task_end).then(|| task_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsp_analyzes_multiple_diagnostics_and_symbols() {
        let analysis = analyze(
            r#"env:
  NAME: one
  NAME?: two
build:
  @task test
  @unknown
test:
  true
"#,
        );

        assert!(analysis.tasks.contains_key("build"));
        assert!(analysis.tasks.contains_key("test"));
        assert!(analysis.variables.contains("NAME"));
        assert!(analysis
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("duplicate env entry")));
        assert!(analysis
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("unknown directive")));
    }
}
