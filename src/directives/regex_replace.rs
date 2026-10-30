use std::fs;

use fancy_regex::Regex;

use crate::command_text::interpolate_argv;
use crate::runner::ExecutionContext;
use crate::{CjError, CjResult};

pub(super) fn execute_regex_replace_directive(
    ctx: &mut ExecutionContext<'_>,
    args: &str,
    line_number: usize,
) -> CjResult<()> {
    let argv = interpolate_argv(args, &ctx.env.vars)?;
    if argv.len() != 3 {
        return Err(CjError::new(format!(
            "line {line_number}: @regex-replace expects a file, a pattern, and a replacement"
        )));
    }

    let path = ctx.cwd.current().join(&argv[0]);
    let source = fs::read_to_string(&path).map_err(|error| {
        CjError::new(format!(
            "line {line_number}: @regex-replace could not read {}: {error}",
            path.display()
        ))
    })?;
    let regex = Regex::new(&argv[1]).map_err(|error| {
        CjError::new(format!(
            "line {line_number}: @regex-replace has an invalid pattern: {error}"
        ))
    })?;
    let replacement = normalize_capture_references(&argv[2]);
    let replaced = regex
        .try_replacen(&source, 0, replacement.as_str())
        .map_err(|error| {
            CjError::new(format!(
                "line {line_number}: @regex-replace failed while matching: {error}"
            ))
        })?;

    fs::write(&path, replaced.as_bytes()).map_err(|error| {
        CjError::new(format!(
            "line {line_number}: @regex-replace could not write {}: {error}",
            path.display()
        ))
    })
}

fn normalize_capture_references(replacement: &str) -> String {
    let mut normalized = String::with_capacity(replacement.len());
    let chars = replacement.chars().collect::<Vec<_>>();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '\\' && index + 1 < chars.len() && chars[index + 1] == '{' {
            normalized.push('{');
            index += 2;
            continue;
        }

        if chars[index] == '{' {
            let mut end = index + 1;
            while end < chars.len() && chars[end].is_ascii_digit() {
                end += 1;
            }
            if end > index + 1 && end < chars.len() && chars[end] == '}' {
                normalized.push_str("${");
                normalized.extend(chars[index + 1..end].iter());
                normalized.push('}');
                index = end + 1;
                continue;
            }
        }

        normalized.push(chars[index]);
        index += 1;
    }

    normalized
}
