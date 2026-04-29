use std::path::Path;

use crate::ansi::{paint, Style};
use crate::task_file::TaskFile;
use crate::{CjError, CjResult};

const HELP_MARKER: &str = "+";
const UNMARKED_TASK: &str = " ";
const DESCRIPTION_GAP: usize = 2;

pub(crate) fn format_task_listing(parsed: &TaskFile, task_file: &Path) -> String {
    let mut lines = task_listing_lines(parsed, task_file);
    append_summary_help_legend(&mut lines, parsed);
    lines.join("\n")
}

pub(crate) fn format_top_help(parsed: &TaskFile, task_file: &Path) -> String {
    let mut lines = Vec::new();
    if let Some(help) = &parsed.help {
        lines.extend(help.lines().map(ToOwned::to_owned));
        if lines.last().is_some_and(|line| !line.is_empty()) {
            lines.push(String::new());
        }
    }
    lines.extend(task_listing_lines(parsed, task_file));
    append_summary_help_legend(&mut lines, parsed);
    lines.join("\n")
}

pub(crate) fn format_task_help(
    parsed: &TaskFile,
    name: &str,
    task_file: &Path,
) -> CjResult<String> {
    let lines = task_help_lines(parsed, name, task_file)?;
    Ok(lines.join("\n"))
}

fn task_listing_lines(parsed: &TaskFile, task_file: &Path) -> Vec<String> {
    let mut lines = vec![paint(
        format!("Tasks in {}:", task_file.display()),
        Style::Section,
    )];
    let tasks = parsed
        .task_order
        .iter()
        .filter(|name| is_summary_visible_task(name))
        .map(String::as_str)
        .collect::<Vec<_>>();
    push_task_rows(&mut lines, parsed, &tasks);
    lines
}

fn is_summary_visible_task(name: &str) -> bool {
    !name.split(':').any(|part| part.starts_with('_'))
}

fn task_help_lines(parsed: &TaskFile, name: &str, task_file: &Path) -> CjResult<Vec<String>> {
    if !parsed.tasks.contains_key(name) {
        return Err(no_help_section(name, task_file));
    }

    let children = child_tasks(parsed, name);
    let description = parsed.descriptions.get(name);
    let body = parsed.task_help.get(name);
    if description.is_none() && body.is_none() && children.is_empty() {
        return Err(no_help_section(name, task_file));
    }

    let mut lines = Vec::new();
    lines.push(paint(task_signature(parsed, name), Style::Task));
    if let Some(description) = description {
        lines.push(format!("  {}", paint(description, Style::Description)));
    }
    if let Some(body) = body {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.extend(body.lines().map(ToOwned::to_owned));
    }
    if !children.is_empty() {
        if lines.last().is_some_and(|line| !line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(paint("Tasks:", Style::Section));
        push_task_rows(&mut lines, parsed, &children);
        append_help_legend_for_tasks(&mut lines, parsed, &children);
    }
    Ok(lines)
}

fn push_task_rows(lines: &mut Vec<String>, parsed: &TaskFile, tasks: &[&str]) {
    let label_width = tasks
        .iter()
        .map(|name| task_listing_label(parsed, name).len())
        .max()
        .unwrap_or(0);

    for name in tasks {
        push_task_row(lines, parsed, name, label_width);
    }
}

fn push_task_row(lines: &mut Vec<String>, parsed: &TaskFile, name: &str, label_width: usize) {
    let marker = if has_more_help(parsed, name) {
        HELP_MARKER
    } else {
        UNMARKED_TASK
    };
    let indent = "  ".repeat(task_depth(name) + 1);
    let signature = task_signature(parsed, name);
    let label = task_listing_label(parsed, name);
    if let Some(description) = parsed.descriptions.get(name) {
        lines.push(format!(
            "{}{}{}{}{}{}",
            marker,
            indent,
            paint(&signature, Style::Task),
            padding(&label, label_width),
            " ".repeat(DESCRIPTION_GAP),
            paint(description, Style::SummaryDescription)
        ));
    } else {
        lines.push(format!(
            "{}{}{}",
            marker,
            indent,
            paint(&signature, Style::Task)
        ));
    }
}

fn task_listing_label(parsed: &TaskFile, name: &str) -> String {
    format!(
        "{}{}",
        "  ".repeat(task_depth(name) + 1),
        task_signature(parsed, name)
    )
}

fn task_signature(parsed: &TaskFile, name: &str) -> String {
    match parsed.task_arguments.get(name) {
        Some(arguments) if !arguments.is_empty() => {
            let arguments = arguments
                .iter()
                .map(|argument| format!("${argument}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name} ({arguments})")
        }
        _ => name.to_string(),
    }
}

fn padding(value: &str, width: usize) -> String {
    " ".repeat(width.saturating_sub(value.len()))
}

fn task_depth(name: &str) -> usize {
    name.matches(':').count()
}

fn append_summary_help_legend(lines: &mut Vec<String>, parsed: &TaskFile) {
    let tasks = parsed
        .task_order
        .iter()
        .filter(|task| is_summary_visible_task(task))
        .map(String::as_str)
        .collect::<Vec<_>>();
    append_help_legend_for_tasks(lines, parsed, &tasks);
}

fn append_help_legend_for_tasks(lines: &mut Vec<String>, parsed: &TaskFile, tasks: &[&str]) {
    let has_marked_task = tasks.iter().any(|task| has_more_help(parsed, task));
    if !has_marked_task {
        return;
    }
    if lines.last().is_some_and(|line| !line.is_empty()) {
        lines.push(String::new());
    }
    lines.push(format!(
        "{} {}",
        paint(HELP_MARKER, Style::Task),
        paint(
            "commands with a + have help\nrun `cj help <task>` to view it",
            Style::SummaryDescription
        )
    ));
}

fn has_more_help(parsed: &TaskFile, name: &str) -> bool {
    parsed.task_help.contains_key(name)
}

fn child_tasks<'a>(parsed: &'a TaskFile, name: &str) -> Vec<&'a str> {
    let prefix = format!("{name}:");
    parsed
        .task_order
        .iter()
        .filter_map(|task| {
            let child = task.strip_prefix(&prefix)?;
            (!child.is_empty() && !child.contains(':')).then_some(task.as_str())
        })
        .collect()
}

fn no_help_section(name: &str, task_file: &Path) -> CjError {
    CjError::new(format!(
        "no help section '{name}' found in {}",
        task_file.display()
    ))
}
