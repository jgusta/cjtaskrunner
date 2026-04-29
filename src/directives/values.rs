use crate::command_text::{interpolate_text, split_words};
use crate::runtime::{append_captured_output, OutputMode, QuoteMode, RuntimeEnv};
use crate::task_file::{strip_matching_quotes, validate_env_name};
use crate::{CjError, CjResult};

use super::conditions::is_truthy;

pub(super) fn is_set_capture_args(args: &str) -> bool {
    args.trim_end().ends_with(':')
        && split_words(args.trim_end_matches(':')).is_ok_and(|argv| argv.len() == 1)
}

pub(super) fn parse_set_capture_name_with_env(
    args: &str,
    effective_env: &RuntimeEnv,
    line_number: usize,
) -> CjResult<String> {
    let argv = split_words(args.trim_end_matches(':'))?;
    if argv.len() != 1 {
        return Err(CjError::new(format!(
            "line {line_number}: @set block expects exactly one variable name"
        )));
    }
    let key = interpolate_text(&argv[0], &effective_env.vars, QuoteMode::None)?;
    validate_env_name(&key).map_err(|err| {
        CjError::new(format!(
            "line {line_number}: invalid env name '{key}': {err}"
        ))
    })?;
    Ok(key)
}

pub(super) fn return_value_status(args: &str, effective_env: &RuntimeEnv) -> CjResult<i32> {
    let value = interpolate_text(args, &effective_env.vars, QuoteMode::None)?;
    let value = strip_matching_quotes(value.trim());
    if value.eq_ignore_ascii_case("true") {
        Ok(0)
    } else if value.eq_ignore_ascii_case("false") {
        Ok(1)
    } else if let Ok(code) = value.parse::<i32>() {
        Ok(code)
    } else if is_truthy(&value) {
        Ok(0)
    } else {
        Ok(1)
    }
}

pub(super) fn write_output(value: &str, output_mode: OutputMode) {
    match output_mode {
        OutputMode::Inherit => print!("{value}"),
        OutputMode::Capture => append_captured_output(value),
    }
}

pub(super) fn write_output_line(value: &str, output_mode: OutputMode) {
    write_output(value, output_mode);
    write_output("\n", output_mode);
}

pub(super) fn parse_env_mutation(
    args: &str,
    effective_env: &RuntimeEnv,
    line_number: usize,
) -> CjResult<(String, String)> {
    let (key, value) = args
        .trim_start()
        .split_once(char::is_whitespace)
        .ok_or_else(|| CjError::new(format!("line {line_number}: @set expects NAME and value")))?;
    let key = interpolate_text(key, &effective_env.vars, QuoteMode::None)?;
    validate_env_name(&key).map_err(|err| {
        CjError::new(format!(
            "line {line_number}: invalid env name '{key}': {err}"
        ))
    })?;
    let value = interpolate_text(value.trim_start(), &effective_env.vars, QuoteMode::None)?;
    Ok((key, value))
}

pub(super) fn parse_export_mutation(
    args: &str,
    effective_env: &RuntimeEnv,
    line_number: usize,
) -> CjResult<(String, String)> {
    let trimmed = args.trim_start();
    if trimmed.is_empty() {
        return Err(CjError::new(format!(
            "line {line_number}: @export expects NAME or NAME value"
        )));
    }
    if let Some((key, value)) = trimmed.split_once(char::is_whitespace) {
        let key = interpolate_text(key, &effective_env.vars, QuoteMode::None)?;
        validate_env_name(&key).map_err(|err| {
            CjError::new(format!(
                "line {line_number}: invalid env name '{key}': {err}"
            ))
        })?;
        let value = interpolate_text(value.trim_start(), &effective_env.vars, QuoteMode::None)?;
        Ok((key, value))
    } else {
        let key = interpolate_text(trimmed, &effective_env.vars, QuoteMode::None)?;
        validate_env_name(&key).map_err(|err| {
            CjError::new(format!(
                "line {line_number}: invalid env name '{key}': {err}"
            ))
        })?;
        let value = effective_env.vars.get(&key).cloned().ok_or_else(|| {
            CjError::new(format!(
                "line {line_number}: cannot export unset variable '{key}'"
            ))
        })?;
        Ok((key, value))
    }
}

pub(crate) fn parse_variable_name_token(token: &str) -> CjResult<String> {
    let name = if let Some(name) = token.strip_prefix("${").and_then(|v| v.strip_suffix('}')) {
        name
    } else if let Some(name) = token.strip_prefix('$') {
        name
    } else {
        token
    };
    validate_env_name(name)
        .map_err(|err| CjError::new(format!("invalid variable name '{token}': {err}")))?;
    Ok(name.to_string())
}
