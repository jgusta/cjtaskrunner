#[allow(clippy::too_many_arguments)]
fn execute_directive(
    task_file: &TaskFile,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    directive: &str,
    effective_env: &mut RuntimeEnv,
    cwd: &mut CwdState,
    stack: &mut Vec<String>,
    output_mode: OutputMode,
) -> CjResult<i32> {
    let (name, args) = split_directive(directive);
    match name {
        "shell" => {
            let command = interpolate_shell_text(args, effective_env)?;
            *index += 1;
            run_shell_command(cwd.current(), &command, effective_env, output_mode)
                .map(|result| result.status)
        }
        "task" => {
            let argv = interpolate_argv(args, &effective_env.vars)?;
            if argv.len() != 1 {
                return Err(CjError::new(format!(
                    "line {}: @task expects exactly one task name",
                    lines[*index].line_number
                )));
            }
            *index += 1;
            run_task(task_file, &argv[0], effective_env, cwd, stack)
        }
        "cd" => {
            let argv = interpolate_argv(args, &effective_env.vars)?;
            if argv.len() != 1 {
                return Err(CjError::new(format!(
                    "line {}: @cd expects exactly one path",
                    lines[*index].line_number
                )));
            }
            let next = cwd.current().join(&argv[0]);
            if !next.is_dir() {
                return Err(CjError::new(format!(
                    "line {}: @cd path is not a directory: {}",
                    lines[*index].line_number,
                    next.display()
                )));
            }
            cwd.cd(next);
            *index += 1;
            Ok(0)
        }
        "back" => {
            if !args.trim().is_empty() {
                return Err(CjError::new(format!(
                    "line {}: @back does not take arguments",
                    lines[*index].line_number
                )));
            }
            cwd.back();
            *index += 1;
            Ok(0)
        }
        "desc" => {
            *index += 1;
            Ok(0)
        }
        "echo" => {
            let value = interpolate_text(args, &effective_env.vars, QuoteMode::None)?;
            write_output_line(&value, output_mode);
            *index += 1;
            Ok(0)
        }
        "clean" => {
            let argv = interpolate_argv(args, &effective_env.vars)?;
            if argv.len() != 1 {
                return Err(CjError::new(format!(
                    "line {}: @clean expects exactly one path",
                    lines[*index].line_number
                )));
            }
            let path = cwd.current().join(&argv[0]);
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else if path.exists() {
                fs::remove_file(&path)?;
            }
            *index += 1;
            Ok(0)
        }
        "stop" => {
            if !args.trim().is_empty() {
                let value = interpolate_text(args, &effective_env.vars, QuoteMode::None)?;
                write_output_line(&value, output_mode);
            }
            *index += 1;
            Ok(1)
        }
        "success" => {
            *index += 1;
            Ok(0)
        }
        "fail" => {
            *index += 1;
            Ok(1)
        }
        "return" => {
            let block_start = *index + 1;
            let block_end = find_block_end(lines, block_start, end, indent);
            if block_start < block_end {
                let status = execute_block(
                    task_file,
                    lines,
                    block_start,
                    block_end,
                    indent + 2,
                    effective_env,
                    cwd,
                    stack,
                    output_mode,
                )?;
                *index = block_end;
                Ok(status)
            } else {
                let status = return_value_status(args, effective_env)?;
                if !args.trim().is_empty() {
                    let value = interpolate_text(args, &effective_env.vars, QuoteMode::None)?;
                    write_output(&strip_matching_quotes(&value), output_mode);
                }
                *index += 1;
                Ok(status)
            }
        }
        "set" | "export" => {
            if name == "set" {
                let block_start = *index + 1;
                let block_end = find_block_end(lines, block_start, end, indent);
                if block_start < block_end && is_set_capture_args(args) {
                    let key = parse_set_capture_name(args, lines[*index].line_number)?;
                    let value = execute_block_capture(
                        task_file,
                        lines,
                        block_start,
                        block_end,
                        indent + 2,
                        effective_env,
                        cwd,
                        stack,
                    )?;
                    effective_env.vars.insert(key, value);
                    *index = block_end;
                    return Ok(0);
                }
                let (key, value) =
                    parse_env_mutation(args, effective_env, lines[*index].line_number)?;
                effective_env.vars.insert(key, value);
            } else {
                let (key, value) =
                    parse_export_mutation(args, effective_env, lines[*index].line_number)?;
                effective_env.vars.insert(key.clone(), value.clone());
                effective_env.exports.insert(key, value);
            }
            *index += 1;
            Ok(0)
        }
        "unset" => {
            let argv = split_words(args)?;
            if argv.len() != 1 {
                return Err(CjError::new(format!(
                    "line {}: @unset expects exactly one variable name",
                    lines[*index].line_number
                )));
            }
            validate_env_name(&argv[0]).map_err(|err| {
                CjError::new(format!(
                    "line {}: invalid env name '{}': {err}",
                    lines[*index].line_number, argv[0]
                ))
            })?;
            effective_env.vars.remove(&argv[0]);
            effective_env.exports.remove(&argv[0]);
            *index += 1;
            Ok(0)
        }
        "if" | "if-exists" | "if-missing" | "if-set" | "if-unset" => execute_if_directive(
            task_file,
            lines,
            index,
            end,
            indent,
            name,
            args,
            effective_env,
            cwd,
            stack,
            output_mode,
        ),
        "else" => Err(CjError::new(format!(
            "line {}: @else without matching @if",
            lines[*index].line_number
        ))),
        "switch" => execute_switch_directive(
            task_file,
            lines,
            index,
            end,
            indent,
            args,
            effective_env,
            cwd,
            stack,
            output_mode,
        ),
        "case" | "default" => Err(CjError::new(format!(
            "line {}: @{name} without matching @switch",
            lines[*index].line_number
        ))),
        "" => Err(CjError::new(format!(
            "line {}: empty directive",
            lines[*index].line_number
        ))),
        _ => Err(CjError::new(format!(
            "line {}: unknown directive @{name}",
            lines[*index].line_number
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_if_directive(
    task_file: &TaskFile,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    name: &str,
    args: &str,
    effective_env: &mut RuntimeEnv,
    cwd: &mut CwdState,
    stack: &mut Vec<String>,
    output_mode: OutputMode,
) -> CjResult<i32> {
    let condition = evaluate_condition(cwd.current(), name, args, effective_env)?;
    let then_start = *index + 1;
    let then_end = find_block_end(lines, then_start, end, indent);
    let mut else_range = None;

    if then_end < end && lines[then_end].indent == indent && lines[then_end].text == "@else" {
        let else_start = then_end + 1;
        let else_end = find_block_end(lines, else_start, end, indent);
        else_range = Some((else_start, else_end));
        *index = else_end;
    } else {
        *index = then_end;
    }

    if condition {
        execute_block(
            task_file,
            lines,
            then_start,
            then_end,
            indent + 2,
            effective_env,
            cwd,
            stack,
            output_mode,
        )
    } else if let Some((else_start, else_end)) = else_range {
        execute_block(
            task_file,
            lines,
            else_start,
            else_end,
            indent + 2,
            effective_env,
            cwd,
            stack,
            output_mode,
        )
    } else {
        Ok(0)
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_switch_directive(
    task_file: &TaskFile,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    args: &str,
    effective_env: &mut RuntimeEnv,
    cwd: &mut CwdState,
    stack: &mut Vec<String>,
    output_mode: OutputMode,
) -> CjResult<i32> {
    let values = interpolate_argv(args, &effective_env.vars)?;
    if values.len() != 1 {
        return Err(CjError::new(format!(
            "line {}: @switch expects exactly one value",
            lines[*index].line_number
        )));
    }
    let switch_value = &values[0];
    let switch_start = *index + 1;
    let switch_end = find_block_end(lines, switch_start, end, indent);
    let case_indent = indent + 2;
    let body_indent = indent + 4;
    let mut selected: Option<(usize, usize)> = None;
    let mut default: Option<(usize, usize)> = None;
    let mut cursor = switch_start;

    while cursor < switch_end {
        let line = &lines[cursor];
        if line.indent != case_indent {
            return Err(CjError::new(format!(
                "line {}: @switch body must contain @case or @default entries",
                line.line_number
            )));
        }
        let Some(rest) = line.text.strip_prefix('@') else {
            return Err(CjError::new(format!(
                "line {}: @switch body entries must use @case or @default",
                line.line_number
            )));
        };
        let (name, args) = split_directive(rest);
        if name != "case" && name != "default" {
            return Err(CjError::new(format!(
                "line {}: @switch body entries must use @case or @default",
                line.line_number
            )));
        }

        let body_start = cursor + 1;
        let body_end = find_case_body_end(lines, body_start, switch_end, case_indent);
        if name == "case" {
            let case_values = interpolate_argv(args, &effective_env.vars)?;
            if case_values.len() != 1 {
                return Err(CjError::new(format!(
                    "line {}: @case expects exactly one value",
                    line.line_number
                )));
            }
            if selected.is_none() && case_values[0] == *switch_value {
                selected = Some((body_start, body_end));
            }
        } else {
            if !args.trim().is_empty() {
                return Err(CjError::new(format!(
                    "line {}: @default does not take arguments",
                    line.line_number
                )));
            }
            default.get_or_insert((body_start, body_end));
        }
        cursor = body_end;
    }

    *index = switch_end;
    if let Some((start, end)) = selected.or(default) {
        execute_block(
            task_file,
            lines,
            start,
            end,
            body_indent,
            effective_env,
            cwd,
            stack,
            output_mode,
        )
    } else {
        Ok(0)
    }
}

fn find_block_end(lines: &[TaskLine], start: usize, end: usize, parent_indent: usize) -> usize {
    let mut cursor = start;
    while cursor < end && lines[cursor].indent > parent_indent {
        cursor += 1;
    }
    cursor
}

fn find_case_body_end(lines: &[TaskLine], start: usize, end: usize, case_indent: usize) -> usize {
    let mut cursor = start;
    while cursor < end && lines[cursor].indent > case_indent {
        cursor += 1;
    }
    cursor
}

fn evaluate_condition(
    base_dir: &Path,
    name: &str,
    args: &str,
    effective_env: &RuntimeEnv,
) -> CjResult<bool> {
    match name {
        "if" => {
            let argv = interpolate_argv(args, &effective_env.vars)?;
            match argv.as_slice() {
                [value] => Ok(is_truthy(value)),
                [left, op, right] if op == "==" => Ok(left == right),
                [left, op, right] if op == "!=" => Ok(left != right),
                _ => Err(CjError::new("@if expects a value or '<left> == <right>'")),
            }
        }
        "if-exists" | "if-missing" => {
            let argv = interpolate_argv(args, &effective_env.vars)?;
            if argv.len() != 1 {
                return Err(CjError::new(format!("@{name} expects exactly one path")));
            }
            let path = base_dir.join(&argv[0]);
            Ok(if name == "if-exists" {
                path.exists()
            } else {
                !path.exists()
            })
        }
        "if-set" | "if-unset" => {
            let argv = split_words(args)?;
            if argv.len() != 1 {
                return Err(CjError::new(format!(
                    "@{name} expects exactly one variable name"
                )));
            }
            let variable = parse_variable_name_token(&argv[0])?;
            let exists = effective_env.vars.contains_key(&variable);
            Ok(if name == "if-set" { exists } else { !exists })
        }
        _ => unreachable!("condition directive checked by caller"),
    }
}

fn is_truthy(value: &str) -> bool {
    !(value.is_empty() || value == "0" || value.eq_ignore_ascii_case("false"))
}

fn is_set_capture_args(args: &str) -> bool {
    split_words(args.trim_end_matches(':')).is_ok_and(|argv| argv.len() == 1)
}

fn parse_set_capture_name(args: &str, line_number: usize) -> CjResult<String> {
    let argv = split_words(args.trim_end_matches(':'))?;
    if argv.len() != 1 {
        return Err(CjError::new(format!(
            "line {line_number}: @set block expects exactly one variable name"
        )));
    }
    validate_env_name(&argv[0]).map_err(|err| {
        CjError::new(format!(
            "line {line_number}: invalid env name '{}': {err}",
            argv[0]
        ))
    })?;
    Ok(argv[0].clone())
}

fn return_value_status(args: &str, effective_env: &RuntimeEnv) -> CjResult<i32> {
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

fn write_output(value: &str, output_mode: OutputMode) {
    match output_mode {
        OutputMode::Inherit => print!("{value}"),
        OutputMode::Capture => {
            CAPTURED_OUTPUT.with(|captured| captured.borrow_mut().push_str(value))
        }
    }
}

fn write_output_line(value: &str, output_mode: OutputMode) {
    write_output(value, output_mode);
    write_output("\n", output_mode);
}

fn parse_env_mutation(
    args: &str,
    effective_env: &RuntimeEnv,
    line_number: usize,
) -> CjResult<(String, String)> {
    let (key, value) = args
        .trim_start()
        .split_once(char::is_whitespace)
        .ok_or_else(|| CjError::new(format!("line {line_number}: @set expects NAME and value")))?;
    validate_env_name(key).map_err(|err| {
        CjError::new(format!(
            "line {line_number}: invalid env name '{key}': {err}"
        ))
    })?;
    let value = interpolate_text(value.trim_start(), &effective_env.vars, QuoteMode::None)?;
    Ok((key.to_string(), value))
}

fn parse_export_mutation(
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
        validate_env_name(key).map_err(|err| {
            CjError::new(format!(
                "line {line_number}: invalid env name '{key}': {err}"
            ))
        })?;
        let value = interpolate_text(value.trim_start(), &effective_env.vars, QuoteMode::None)?;
        Ok((key.to_string(), value))
    } else {
        validate_env_name(trimmed).map_err(|err| {
            CjError::new(format!(
                "line {line_number}: invalid env name '{trimmed}': {err}"
            ))
        })?;
        let value = effective_env.vars.get(trimmed).cloned().ok_or_else(|| {
            CjError::new(format!(
                "line {line_number}: cannot export unset variable '{trimmed}'"
            ))
        })?;
        Ok((trimmed.to_string(), value))
    }
}

fn parse_variable_name_token(token: &str) -> CjResult<String> {
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

fn split_directive(directive: &str) -> (&str, &str) {
    let trimmed = directive.trim_start();
    match trimmed.find(char::is_whitespace) {
        Some(index) => (&trimmed[..index], trimmed[index..].trim_start()),
        None => (trimmed, ""),
    }
}
