use std::collections::HashSet;

use tower_lsp::lsp_types::*;

use super::{Analysis, LspSection, LspTaskContext, TaskDef};
use crate::command_text::{
    contains_variable_interpolation, split_words, unescape_variable_literals, variable_references,
};
use crate::directive_info::{directive, ArgumentRule};
use crate::directives::parse_variable_name_token;
use crate::task_file::{
    directive_syntax_error, parse_nested_task_header, parse_task_header, split_line_expressions,
    task_expression_syntax_error, validate_env_name, validate_task_name,
    validate_task_nesting_depth,
};
use crate::version::{validate_semver, version_env_key};

#[derive(Debug, Clone, Copy, Default)]
struct LeadingIndent {
    width: usize,
    bytes: usize,
    has_spaces: bool,
    has_tabs: bool,
}

pub(super) fn analyze(text: &str) -> Analysis {
    let mut analysis = Analysis::default();
    if diagnose_mixed_indentation(&mut analysis, text) {
        return analysis;
    }

    let mut section = LspSection::Top;
    let mut task_contexts: Vec<LspTaskContext> = Vec::new();
    let mut seen_env = false;
    let mut seen_task = false;
    let mut env_names = HashSet::new();
    let mut active_help_indent: Option<usize> = None;

    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index as u32;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim();
        let indent = leading_indent(line);

        if let Some(help_indent) = active_help_indent {
            if trimmed.is_empty() {
                continue;
            }
            if indent.bytes > 0 && indent.width > help_indent {
                diagnose_metadata_variables(
                    &mut analysis,
                    line_number,
                    indent.bytes,
                    line.len(),
                    "@help:",
                    &line[indent.bytes..],
                );
                continue;
            }
            active_help_indent = None;
        }

        if section == LspSection::Help {
            if trimmed.is_empty() {
                continue;
            }
            if indent.bytes > 0 {
                diagnose_metadata_variables(
                    &mut analysis,
                    line_number,
                    indent.bytes,
                    line.len(),
                    "@help:",
                    &line[indent.bytes..],
                );
                continue;
            }
            section = LspSection::Top;
        }

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if indent.bytes == 0 {
            task_contexts.clear();
            if analyze_version_header_lsp(
                &mut analysis,
                line_number,
                line,
                &mut env_names,
                seen_task,
            ) {
                section = LspSection::Top;
                continue;
            }
            if line == "@help:" {
                section = LspSection::Help;
                continue;
            }
            match parse_task_header(line) {
                Ok((key, _)) if key == "@env" => {
                    if seen_task {
                        push_diagnostic(
                            &mut analysis,
                            line_number,
                            0,
                            line.len(),
                            "@env: must appear before tasks",
                        );
                        section = LspSection::Top;
                    } else if seen_env {
                        push_diagnostic(
                            &mut analysis,
                            line_number,
                            0,
                            line.len(),
                            "multiple @env: sections are not allowed in the same block",
                        );
                        section = LspSection::Top;
                    } else {
                        seen_env = true;
                        section = LspSection::Env;
                    }
                }
                Ok((key, arguments)) => {
                    if add_task_lsp(
                        &mut analysis,
                        key.clone(),
                        arguments,
                        line_number,
                        0,
                        key.len(),
                        0,
                    ) {
                        seen_task = true;
                        task_contexts.push(LspTaskContext {
                            name: key,
                            header_indent: 0,
                        });
                    }
                    section = LspSection::Task;
                }
                Err(message) => {
                    push_diagnostic(&mut analysis, line_number, 0, line.len(), &message);
                    section = LspSection::Top;
                }
            }
            continue;
        }

        if indent.width < 2 || !indent.width.is_multiple_of(2) {
            push_diagnostic(
                &mut analysis,
                line_number,
                0,
                indent.bytes.max(1),
                "indented entries must use full indentation levels",
            );
            continue;
        }

        match section {
            LspSection::Env => {
                if indent.width != 2 {
                    push_diagnostic(
                        &mut analysis,
                        line_number,
                        0,
                        indent.bytes,
                        "env entries must use exactly one indentation level",
                    );
                    continue;
                }
                analyze_env_entry(
                    &mut analysis,
                    line_number,
                    indent.bytes,
                    &line[indent.bytes..],
                    &mut env_names,
                );
            }
            LspSection::Task => {
                while task_contexts.len() > 1
                    && indent.width <= task_contexts.last().expect("task context").header_indent
                {
                    task_contexts.pop();
                }
                let Some(context) = task_contexts.last() else {
                    push_diagnostic(
                        &mut analysis,
                        line_number,
                        0,
                        line.len(),
                        "command without a task",
                    );
                    continue;
                };
                let active_task = context.name.clone();
                let active_header_indent = context.header_indent;
                let text = &line[indent.bytes..];
                let logical_indent = indent.width.saturating_sub(active_header_indent);
                if logical_indent == 2 {
                    if let Some(header) = parse_nested_task_header(text) {
                        match header {
                            Ok((child_name, arguments)) => {
                                let nested_name = format!("{active_task}:{child_name}");
                                if add_task_lsp(
                                    &mut analysis,
                                    nested_name.clone(),
                                    arguments,
                                    line_number,
                                    indent.bytes,
                                    indent.bytes + child_name.len(),
                                    indent.width,
                                ) {
                                    task_contexts.push(LspTaskContext {
                                        name: nested_name,
                                        header_indent: indent.width,
                                    });
                                }
                            }
                            Err(message) => {
                                push_diagnostic(
                                    &mut analysis,
                                    line_number,
                                    indent.bytes,
                                    line.len(),
                                    &message,
                                );
                            }
                        }
                        continue;
                    }
                }
                for expression in split_line_expressions(text) {
                    if logical_indent == 2 {
                        record_description(&mut analysis, Some(&active_task), &expression);
                    }
                    if is_help_directive_lsp(&expression) {
                        active_help_indent = Some(indent.width);
                    }
                    analyze_task_expression(&mut analysis, line_number, indent.bytes, &expression);
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
            LspSection::Help => unreachable!("top-level help handled before section dispatch"),
        }
    }

    finalize_task_ranges(text, &mut analysis);
    analysis
}

fn diagnose_mixed_indentation(analysis: &mut Analysis, text: &str) -> bool {
    let mut saw_spaces = false;
    let mut saw_tabs = false;

    for (index, raw_line) in text.lines().enumerate() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let indent = leading_indent(line);
        if indent.has_spaces {
            saw_spaces = true;
        }
        if indent.has_tabs {
            saw_tabs = true;
        }
        if (indent.has_spaces && indent.has_tabs) || (saw_spaces && saw_tabs) {
            push_diagnostic(
                analysis,
                index as u32,
                0,
                indent.bytes.max(1),
                "taskfile uses both leading spaces and tabs for indentation; use one indentation style per file or run `cj --format` to normalize indentation to spaces",
            );
            return true;
        }
    }

    false
}

fn leading_indent(line: &str) -> LeadingIndent {
    let mut indent = LeadingIndent::default();
    for (index, ch) in line.char_indices() {
        match ch {
            ' ' => {
                indent.width += 1;
                indent.bytes = index + 1;
                indent.has_spaces = true;
            }
            '\t' => {
                indent.width += 2;
                indent.bytes = index + 1;
                indent.has_tabs = true;
            }
            _ => break,
        }
    }
    indent
}

fn add_task_lsp(
    analysis: &mut Analysis,
    key: String,
    arguments: Vec<String>,
    line_number: u32,
    start_col: usize,
    end_col: usize,
    header_indent: usize,
) -> bool {
    if let Err(err) = validate_task_name(&key) {
        push_diagnostic(
            analysis,
            line_number,
            start_col,
            end_col,
            &format!("invalid task name '{key}': {err}"),
        );
        return false;
    }
    if let Err(err) = validate_task_nesting_depth(&key) {
        push_diagnostic(
            analysis,
            line_number,
            start_col,
            end_col,
            &format!("invalid task name '{key}': {err}"),
        );
        return false;
    }
    if analysis.tasks.contains_key(&key) {
        push_diagnostic(
            analysis,
            line_number,
            start_col,
            end_col,
            &format!("duplicate task '{key}'"),
        );
        return false;
    }

    let selection_range = Range::new(
        Position::new(line_number, start_col as u32),
        Position::new(line_number, end_col as u32),
    );
    analysis.variables.extend(arguments.iter().cloned());
    analysis.task_order.push(key.clone());
    analysis.tasks.insert(
        key.clone(),
        TaskDef {
            name: key,
            arguments,
            range: selection_range,
            selection_range,
            description: None,
            header_indent,
        },
    );
    true
}

fn finalize_task_ranges(text: &str, analysis: &mut Analysis) {
    let lines = text
        .lines()
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect::<Vec<_>>();
    let document_end = full_document_range(text).end;
    let task_names = analysis.task_order.clone();

    for name in task_names {
        let Some(task) = analysis.tasks.get(&name) else {
            continue;
        };
        let start_line = task.selection_range.start.line as usize;
        let header_indent = task.header_indent;
        let start = task.selection_range.start;
        let mut end = document_end;

        for (index, line) in lines.iter().enumerate().skip(start_line + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let indent = leading_indent(line);
            let boundary = if header_indent == 0 {
                indent.bytes == 0
            } else {
                indent.bytes == 0 || indent.width <= header_indent
            };
            if boundary {
                end = Position::new(index as u32, 0);
                break;
            }
        }

        if let Some(task) = analysis.tasks.get_mut(&name) {
            task.range = Range::new(start, end);
        }
    }
}

pub(super) fn document_symbols(analysis: &Analysis) -> Vec<DocumentSymbol> {
    document_symbols_for_parent(analysis, None)
}

#[allow(deprecated)]
fn document_symbols_for_parent(analysis: &Analysis, parent: Option<&str>) -> Vec<DocumentSymbol> {
    analysis
        .task_order
        .iter()
        .filter(|name| direct_symbol_child(analysis, name, parent))
        .filter_map(|name| analysis.tasks.get(name))
        .map(|task| {
            let children = document_symbols_for_parent(analysis, Some(&task.name));
            DocumentSymbol {
                name: task.name.clone(),
                detail: task.description.clone().or_else(|| {
                    Some(if task.arguments.is_empty() {
                        "task".to_string()
                    } else {
                        format!("({})", task.arguments.join(", "))
                    })
                }),
                kind: SymbolKind::FUNCTION,
                tags: None,
                deprecated: None,
                range: task.range,
                selection_range: task.selection_range,
                children: (!children.is_empty()).then_some(children),
            }
        })
        .collect()
}

fn direct_symbol_child(analysis: &Analysis, name: &str, parent: Option<&str>) -> bool {
    match (task_parent(name), parent) {
        (None, None) => true,
        (Some(task_parent), None) => !analysis.tasks.contains_key(task_parent),
        (Some(task_parent), Some(parent)) => task_parent == parent,
        (None, Some(_)) => false,
    }
}

fn task_parent(name: &str) -> Option<&str> {
    name.rsplit_once(':').map(|(parent, _)| parent)
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
    if contains_variable_interpolation(args) {
        return;
    }
    if let Some(task) = analysis.tasks.get_mut(task_name) {
        task.description = Some(unescape_variable_literals(args.trim()));
    }
}

fn analyze_version_header_lsp(
    analysis: &mut Analysis,
    line_number: u32,
    line: &str,
    env_names: &mut HashSet<String>,
    seen_task: bool,
) -> bool {
    let Some(rest) = line.strip_prefix('@') else {
        return false;
    };
    let (directive, args) = crate::directives::split_directive(rest);
    if directive != "version" {
        return false;
    }

    if seen_task {
        push_diagnostic(
            analysis,
            line_number,
            0,
            line.len(),
            "@version must appear before tasks",
        );
        return true;
    }

    let parts = args.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 2 {
        push_diagnostic(
            analysis,
            line_number,
            0,
            line.len(),
            &format!("@{directive} expects name and value"),
        );
        return true;
    }

    match version_env_key(parts[0]) {
        Ok(env_key) => {
            if !env_names.insert(env_key.clone()) {
                push_diagnostic(
                    analysis,
                    line_number,
                    0,
                    line.len(),
                    &format!("duplicate env entry '{env_key}'"),
                );
            }
            analysis.variables.insert(env_key);
        }
        Err(err) => push_diagnostic(
            analysis,
            line_number,
            0,
            line.len(),
            &format!("invalid version name '{}': {err}", parts[0]),
        ),
    }
    if let Err(err) = validate_semver(parts[1], line_number as usize) {
        push_diagnostic(analysis, line_number, 0, line.len(), &err.to_string());
    }
    true
}

fn is_help_directive_lsp(expression: &str) -> bool {
    expression == "@help:"
}

pub(super) fn full_document_range(text: &str) -> Range {
    let mut line_count = 0;
    let mut last_line = "";
    for line in text.split('\n') {
        line_count += 1;
        last_line = line.strip_suffix('\r').unwrap_or(line);
    }
    Range::new(
        Position::new(0, 0),
        Position::new((line_count - 1) as u32, utf16_len(last_line)),
    )
}

fn analyze_env_entry(
    analysis: &mut Analysis,
    line_number: u32,
    indent_col: usize,
    entry: &str,
    env_names: &mut HashSet<String>,
) {
    let Some(colon_index) = entry.find(':') else {
        push_diagnostic(
            analysis,
            line_number,
            indent_col,
            entry.len() + indent_col,
            "env entry must contain ':'",
        );
        return;
    };
    let raw_key = &entry[..colon_index];
    let key = raw_key.strip_suffix('?').unwrap_or(raw_key);
    if let Err(err) = validate_env_name(key) {
        push_diagnostic(
            analysis,
            line_number,
            indent_col,
            colon_index + indent_col,
            &format!("invalid env name '{key}': {err}"),
        );
        return;
    }
    if !env_names.insert(key.to_string()) {
        push_diagnostic(
            analysis,
            line_number,
            indent_col,
            colon_index + indent_col,
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
    if let Some(message) = task_expression_syntax_error(expression) {
        push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            message,
        );
    }

    let Some(rest) = expression.strip_prefix('@') else {
        analysis.variables.extend(variable_references(expression));
        return;
    };
    let (name, args) = crate::directives::split_directive(rest);
    if name == "desc" {
        diagnose_metadata_variables(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            "@desc",
            args,
        );
    } else {
        analysis.variables.extend(variable_references(expression));
    }
    if name.is_empty() {
        push_diagnostic(analysis, line_number, indent, indent + 1, "empty directive");
        return;
    }
    let Some(info) = directive(name) else {
        push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + name.len() + 1,
            &format!("unknown directive @{name}"),
        );
        return;
    };

    if let Some(message) = directive_syntax_error(expression) {
        push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            message,
        );
        if name == "help" || name.ends_with(':') || args.trim_end().ends_with(':') {
            return;
        }
    }

    match info.arguments {
        ArgumentRule::None if !args.trim().is_empty() => push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            &format!("@{name} does not take arguments"),
        ),
        ArgumentRule::Exactly(expected) if arg_count(args) != Some(expected) => {
            let noun = if expected == 1 { "argument" } else { "arguments" };
            push_diagnostic(
                analysis,
                line_number,
                indent,
                indent + expression.len(),
                &format!("@{name} expects exactly {expected} {noun}"),
            );
        }
        ArgumentRule::AtLeast(minimum)
            if arg_count(args).is_none_or(|count| count < minimum) =>
        {
            push_diagnostic(
                analysis,
                line_number,
                indent,
                indent + expression.len(),
                &format!("@{name} expects at least {minimum} argument(s)"),
            );
        }
        ArgumentRule::Variable => match split_words(args) {
            Ok(argv) if argv.len() == 1 => {
                if let Ok(variable) = parse_variable_name_token(&argv[0]) {
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
        ArgumentRule::IfCondition => match split_words(args) {
            Ok(argv)
                if matches!(argv.as_slice(), [_])
                    || matches!(argv.as_slice(), [_, op, _] if op == "==" || op == "!=") =>
            {
            }
            _ => push_diagnostic(
                analysis,
                line_number,
                indent,
                indent + expression.len(),
                crate::directives::if_condition_error(),
            ),
        },
        ArgumentRule::IfInCondition => match split_words(args) {
            Ok(argv) if argv.len() >= 2 => {}
            _ => push_diagnostic(
                analysis,
                line_number,
                indent,
                indent + expression.len(),
                crate::directives::if_in_condition_error(),
            ),
        },
        ArgumentRule::VersionCondition => match split_words(args) {
            Ok(argv)
                if argv.len() == 2
                    && matches!(argv[1].as_str(), "prerelease" | "pre" | "release") => {}
            Ok(argv)
                if argv.len() == 3
                    && matches!(argv[1].as_str(), "==" | "!=" | "<" | "<=" | ">" | ">=") => {}
            _ => push_diagnostic(
                analysis,
                line_number,
                indent,
                indent + expression.len(),
                &format!(
                    "@{name} expects '<name> <op> <version>', '<name> prerelease', or '<name> release'"
                ),
            ),
        },
        ArgumentRule::BumpedCondition => match split_words(args) {
            Ok(argv) if argv.is_empty() || argv.len() == 1 => {}
            _ => push_diagnostic(
                analysis,
                line_number,
                indent,
                indent + expression.len(),
                &format!("@{name} expects no arguments or '<name>'"),
            ),
        },
        ArgumentRule::BumpKindCondition => match split_words(args) {
            Ok(argv) if argv.len() == 1 => {}
            _ => push_diagnostic(
                analysis,
                line_number,
                indent,
                indent + expression.len(),
                &format!("@{name} expects exactly one version name"),
            ),
        },
        ArgumentRule::Set => analyze_set_args(analysis, line_number, indent, expression, args),
        ArgumentRule::Export => {
            analyze_export_args(analysis, line_number, indent, expression, args)
        }
        ArgumentRule::VersionBump => {
            analyze_version_bump_args(analysis, line_number, indent, expression, args)
        }
        ArgumentRule::PreBump => {
            analyze_pre_bump_args(analysis, line_number, indent, expression, args)
        }
        _ => {}
    }

    if name == "version" {
        push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            "@version can only be used as a top-level header",
        );
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
    if let Err(err) = validate_env_name(name) {
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
    if let Err(err) = validate_env_name(name) {
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

fn analyze_version_bump_args(
    analysis: &mut Analysis,
    line_number: u32,
    indent: usize,
    expression: &str,
    args: &str,
) {
    let Ok(argv) = split_words(args) else {
        push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            "version bump directive expects exactly one version name",
        );
        return;
    };
    match argv.as_slice() {
        [name] => {
            record_static_bump_version(analysis, line_number, indent, expression, name);
        }
        _ => push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            "version bump directive expects exactly one version name",
        ),
    }
}

fn analyze_pre_bump_args(
    analysis: &mut Analysis,
    line_number: u32,
    indent: usize,
    expression: &str,
    args: &str,
) {
    let Ok(argv) = split_words(args) else {
        push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            "@pre expects <name> <prerelease>",
        );
        return;
    };
    match argv.as_slice() {
        [name, _] => {
            record_static_bump_version(analysis, line_number, indent, expression, name);
        }
        _ => push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            "@pre expects <name> <prerelease>",
        ),
    }
}

fn record_static_bump_version(
    analysis: &mut Analysis,
    line_number: u32,
    indent: usize,
    expression: &str,
    name: &str,
) {
    if contains_variable_interpolation(name) {
        return;
    }
    match version_env_key(name) {
        Ok(env_key) => {
            analysis.variables.insert(env_key);
        }
        Err(err) => push_diagnostic(
            analysis,
            line_number,
            indent,
            indent + expression.len(),
            &format!("invalid version name '{name}': {err}"),
        ),
    }
}

fn arg_count(args: &str) -> Option<usize> {
    split_words(args).ok().map(|argv| argv.len())
}

fn diagnose_metadata_variables(
    analysis: &mut Analysis,
    line_number: u32,
    start_col: usize,
    end_col: usize,
    directive: &str,
    text: &str,
) {
    if contains_variable_interpolation(text) {
        push_diagnostic(
            analysis,
            line_number,
            start_col,
            end_col,
            &format!("{directive} text cannot contain variables"),
        );
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

pub(super) fn line_at(text: &str, line: u32) -> Option<&str> {
    text.lines().nth(line as usize)
}

pub(super) fn byte_index_for_utf16_col(line: &str, col: u32) -> usize {
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

pub(super) fn word_at(text: &str, position: Position) -> Option<String> {
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
    byte == b'@' || byte == b':' || byte == b'-' || byte == b'_' || byte.is_ascii_alphanumeric()
}

pub(super) fn task_reference_at(text: &str, position: Position) -> Option<String> {
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
