use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::directive_info::{directive_description, DIRECTIVES};
use crate::formatter::format_taskfile_source;

mod analysis;
use analysis::{
    analyze, byte_index_for_utf16_col, document_symbols, full_document_range, line_at,
    task_reference_at, word_at,
};

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
    arguments: Vec<String>,
    range: Range,
    selection_range: Range,
    description: Option<String>,
    header_indent: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LspTaskContext {
    name: String,
    header_indent: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LspSection {
    Top,
    Env,
    Task,
    Help,
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
                document_formatting_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "cj lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "cj lsp initialized")
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

        let symbols = document_symbols(&document.analysis);

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
                .map(|directive| CompletionItem {
                    label: format!("@{}", directive.name),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(directive.description.to_string()),
                    insert_text: Some(format!("@{}", directive.name)),
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
        let Some(detail) = directive_description(directive) else {
            return Ok(None);
        };

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("`@{directive}`\n\n{detail}"),
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

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> LspResult<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let documents = self.documents.lock().await;
        let Some(document) = documents.get(&uri) else {
            return Ok(None);
        };
        let formatted = format_taskfile_source(&document.text);
        if formatted == document.text {
            return Ok(Some(Vec::new()));
        }
        Ok(Some(vec![TextEdit {
            range: full_document_range(&document.text),
            new_text: formatted,
        }]))
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

#[cfg(test)]
mod tests;
