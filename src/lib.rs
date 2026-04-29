use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct CjError {
    message: String,
}

impl CjError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CjError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CjError {}

impl From<io::Error> for CjError {
    fn from(value: io::Error) -> Self {
        Self::new(value.to_string())
    }
}

type CjResult<T> = Result<T, CjError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskFile {
    env: EnvEntries,
    tasks: HashMap<String, Vec<TaskLine>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskLine {
    line_number: usize,
    indent: usize,
    text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct EnvEntries {
    overrides: HashMap<String, String>,
    fallbacks: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Top,
    Env,
    Task,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuoteMode {
    None,
    Shell,
}

#[derive(Debug, Clone)]
struct RuntimeEnv {
    vars: HashMap<String, String>,
    exports: HashMap<String, String>,
}

impl RuntimeEnv {
    fn new(initial: HashMap<String, String>) -> Self {
        Self {
            vars: initial.clone(),
            exports: initial,
        }
    }
}

pub fn run_cli(args: &[String]) -> CjResult<i32> {
    run_cli_from_cwd(args, &env::current_dir()?)
}

fn run_cli_from_cwd(args: &[String], cwd: &Path) -> CjResult<i32> {
    let (task_file, task_name) = resolve_invocation_from(args, cwd)?;
    let base_dir = task_file_base_dir(&task_file);
    let parsed = parse_task_file_path(&task_file)?;
    let mut env = RuntimeEnv::new(build_effective_env(base_dir, &parsed.env)?);

    run_task(base_dir, &parsed, &task_name, &mut env, &mut Vec::new())
}

fn resolve_invocation_from(args: &[String], cwd: &Path) -> CjResult<(PathBuf, String)> {
    match args.len() {
        1 => {
            let task_name = args[0].clone();
            validate_task_name(&task_name)
                .map_err(|err| CjError::new(format!("invalid task name '{task_name}': {err}")))?;
            Ok((discover_task_file(cwd)?, task_name))
        }
        2 => {
            let raw_target = PathBuf::from(&args[0]);
            let target = if raw_target.is_absolute() {
                raw_target
            } else {
                cwd.join(raw_target)
            };
            let task_name = args[1].clone();
            validate_task_name(&task_name)
                .map_err(|err| CjError::new(format!("invalid task name '{task_name}': {err}")))?;

            if target.is_dir() {
                Ok((discover_task_file(&target)?, task_name))
            } else if target.is_file() {
                if is_recognized_task_file(&target) {
                    Ok((target, task_name))
                } else {
                    Err(CjError::new(format!(
                        "task file must be named 'cjt' or 'cjtasks': {}",
                        target.display()
                    )))
                }
            } else if target.exists() {
                Err(CjError::new(format!(
                    "path is neither a recognized task file nor a directory: {}",
                    target.display()
                )))
            } else {
                Err(CjError::new(format!(
                    "path does not exist: {}",
                    target.display()
                )))
            }
        }
        _ => Err(CjError::new(
            "usage: cj <task> | cj <taskfile-or-directory> <task>",
        )),
    }
}

fn discover_task_file(dir: &Path) -> CjResult<PathBuf> {
    for name in ["cjt", "cjtasks"] {
        let path = dir.join(name);
        if path.is_file() {
            return Ok(path);
        }
    }

    Err(CjError::new(format!(
        "no cjt or cjtasks file found in {}",
        dir.display()
    )))
}

fn is_recognized_task_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "cjt" || name == "cjtasks")
}

fn task_file_base_dir(task_file: &Path) -> &Path {
    task_file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn parse_task_file_path(path: &Path) -> CjResult<TaskFile> {
    let source = fs::read_to_string(path)
        .map_err(|err| CjError::new(format!("failed to read {}: {err}", path.display())))?;
    parse_task_file(&source, path)
}

pub fn parse_task_file(source: &str, path: &Path) -> CjResult<TaskFile> {
    let mut env = EnvEntries::default();
    let mut tasks: HashMap<String, Vec<TaskLine>> = HashMap::new();
    let mut section = Section::Top;
    let mut current_task: Option<String> = None;
    let mut seen_env = false;

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if !line.starts_with(' ') {
            current_task = None;
            let key = parse_top_level_key(line, path, line_number)?;
            if key == "env" {
                if seen_env {
                    return Err(line_error(
                        path,
                        line_number,
                        "multiple env sections are not allowed",
                    ));
                }
                seen_env = true;
                section = Section::Env;
            } else {
                validate_task_name(&key).map_err(|err| {
                    line_error(
                        path,
                        line_number,
                        format!("invalid task name '{key}': {err}"),
                    )
                })?;
                if tasks.contains_key(&key) {
                    return Err(line_error(
                        path,
                        line_number,
                        format!("duplicate task '{key}'"),
                    ));
                }
                tasks.insert(key.clone(), Vec::new());
                current_task = Some(key);
                section = Section::Task;
            }
            continue;
        }

        let indent = line.chars().take_while(|ch| *ch == ' ').count();
        if indent < 2 || indent % 2 != 0 {
            return Err(line_error(
                path,
                line_number,
                "indented entries must use an even number of spaces, at least two",
            ));
        }

        match section {
            Section::Env => {
                if indent != 2 {
                    return Err(line_error(
                        path,
                        line_number,
                        "env entries must use exactly two leading spaces",
                    ));
                }
                parse_env_entry(&line[2..], &mut env, path, line_number)?;
            }
            Section::Task => {
                let task_name = current_task
                    .as_ref()
                    .ok_or_else(|| line_error(path, line_number, "command without a task"))?;
                let text = &line[indent..];
                if text.is_empty() {
                    continue;
                }
                validate_directive_syntax(text, path, line_number)?;
                tasks
                    .get_mut(task_name)
                    .expect("current task must exist")
                    .push(TaskLine {
                        line_number,
                        indent,
                        text: text.to_string(),
                    });
            }
            Section::Top => {
                return Err(line_error(
                    path,
                    line_number,
                    "indented entry is not under env or a task",
                ));
            }
        }
    }

    Ok(TaskFile { env, tasks })
}

fn parse_top_level_key(line: &str, path: &Path, line_number: usize) -> CjResult<String> {
    if !line.ends_with(':') || line[..line.len() - 1].contains(':') {
        return Err(line_error(
            path,
            line_number,
            "top-level entries must be a key followed by ':'",
        ));
    }

    let key = &line[..line.len() - 1];
    if key.trim() != key || key.is_empty() {
        return Err(line_error(path, line_number, "invalid top-level key"));
    }
    Ok(key.to_string())
}

fn validate_directive_syntax(text: &str, path: &Path, line_number: usize) -> CjResult<()> {
    if let Some(rest) = text.strip_prefix('@') {
        let (name, args) = split_directive(rest);
        let colon_block_directive = matches!(
            name,
            "if" | "if-exists"
                | "if-missing"
                | "if-set"
                | "if-unset"
                | "else"
                | "switch"
                | "case"
                | "default"
        ) && args.trim_end().ends_with(':');
        if name.ends_with(':') || colon_block_directive {
            return Err(line_error(
                path,
                line_number,
                "CJTasks directives do not use trailing ':'",
            ));
        }
    }
    Ok(())
}

fn parse_env_entry(
    entry: &str,
    env: &mut EnvEntries,
    path: &Path,
    line_number: usize,
) -> CjResult<()> {
    let Some(colon_index) = entry.find(':') else {
        return Err(line_error(path, line_number, "env entry must contain ':'"));
    };
    let raw_key = &entry[..colon_index];
    let fallback = raw_key.ends_with('?');
    let key = if fallback {
        &raw_key[..raw_key.len() - 1]
    } else {
        raw_key
    };

    validate_env_name(key).map_err(|err| {
        line_error(
            path,
            line_number,
            format!("invalid env name '{key}': {err}"),
        )
    })?;

    if env.overrides.contains_key(key) || env.fallbacks.contains_key(key) {
        return Err(line_error(
            path,
            line_number,
            format!("duplicate env entry '{key}'"),
        ));
    }

    let value = strip_matching_quotes(strip_one_leading_space(&entry[colon_index + 1..]));
    if fallback {
        env.fallbacks.insert(key.to_string(), value);
    } else {
        env.overrides.insert(key.to_string(), value);
    }
    Ok(())
}

fn validate_task_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("task name cannot be empty");
    }
    if name == "env" {
        return Err("'env' is reserved");
    }
    if name
        .chars()
        .all(|ch| ch == '-' || ch == '_' || ch.is_ascii_alphanumeric())
    {
        Ok(())
    } else {
        Err("task names must contain only ASCII letters, digits, hyphens, and underscores")
    }
}

fn validate_env_name(name: &str) -> Result<(), &'static str> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err("env name cannot be empty");
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err("env names must start with a letter or underscore");
    }
    if chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
        Ok(())
    } else {
        Err("env names must contain only ASCII letters, digits, and underscores")
    }
}

fn strip_one_leading_space(value: &str) -> &str {
    value.strip_prefix(' ').unwrap_or(value)
}

fn strip_matching_quotes(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'\'' && last == b'\'') || (first == b'"' && last == b'"') {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

fn line_error(path: &Path, line_number: usize, message: impl Into<String>) -> CjError {
    CjError::new(format!(
        "{}:{line_number}: {}",
        path.display(),
        message.into()
    ))
}

fn build_effective_env(base_dir: &Path, entries: &EnvEntries) -> CjResult<HashMap<String, String>> {
    let mut effective: HashMap<String, String> = env::vars().collect();

    load_dot_env_absent_only(base_dir, &mut effective)?;

    for (key, value) in &entries.fallbacks {
        effective
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }

    for (key, value) in &entries.overrides {
        effective.insert(key.clone(), value.clone());
    }

    apply_python_venv(base_dir, &mut effective)?;

    Ok(effective)
}

fn load_dot_env_absent_only(
    base_dir: &Path,
    effective: &mut HashMap<String, String>,
) -> CjResult<()> {
    let path = base_dir.join(".env");
    if !path.exists() {
        return Ok(());
    }

    let source = fs::read_to_string(&path)
        .map_err(|err| CjError::new(format!("failed to read {}: {err}", path.display())))?;
    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some(eq_index) = line.find('=') else {
            return Err(line_error(
                &path,
                line_number,
                ".env entry must contain '='",
            ));
        };
        let key = &line[..eq_index];
        validate_env_name(key).map_err(|err| {
            line_error(
                &path,
                line_number,
                format!("invalid .env name '{key}': {err}"),
            )
        })?;
        let value = strip_matching_quotes(&line[eq_index + 1..]);
        effective.entry(key.to_string()).or_insert(value);
    }

    Ok(())
}

fn apply_python_venv(base_dir: &Path, effective: &mut HashMap<String, String>) -> CjResult<()> {
    let selected = if let Some(path) = non_empty_env(effective, "VIRTUAL_ENV") {
        Some(PathBuf::from(path))
    } else if let Some(path) = non_empty_env(effective, "CJ_VENV") {
        Some(PathBuf::from(path))
    } else {
        let local = base_dir.join(".venv");
        local.is_dir().then_some(local)
    };

    let Some(venv) = selected else {
        return Ok(());
    };

    let executable_dir = venv.join("bin");
    if !executable_dir.is_dir() {
        return Err(CjError::new(format!(
            "python virtualenv executable directory does not exist: {}",
            executable_dir.display()
        )));
    }

    let executable_dir = executable_dir.to_string_lossy().to_string();
    let path = match effective.get("PATH") {
        Some(existing) if !existing.is_empty() => format!("{executable_dir}:{existing}"),
        _ => executable_dir,
    };
    effective.insert("PATH".to_string(), path);
    effective.insert(
        "VIRTUAL_ENV".to_string(),
        venv.to_string_lossy().to_string(),
    );

    Ok(())
}

fn non_empty_env(effective: &HashMap<String, String>, key: &str) -> Option<String> {
    effective
        .get(key)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn run_task(
    base_dir: &Path,
    task_file: &TaskFile,
    task_name: &str,
    effective_env: &mut RuntimeEnv,
    stack: &mut Vec<String>,
) -> CjResult<i32> {
    validate_task_name(task_name)
        .map_err(|err| CjError::new(format!("invalid task name '{task_name}': {err}")))?;
    if let Some(index) = stack.iter().position(|active| active == task_name) {
        let mut cycle = stack[index..].to_vec();
        cycle.push(task_name.to_string());
        return Err(CjError::new(format!(
            "recursive @task cycle detected: {}",
            cycle.join(" -> ")
        )));
    }

    let lines = task_file
        .tasks
        .get(task_name)
        .ok_or_else(|| CjError::new(format!("task not found: {task_name}")))?;
    stack.push(task_name.to_string());
    let result = execute_block(
        base_dir,
        task_file,
        lines,
        0,
        lines.len(),
        2,
        effective_env,
        stack,
    );
    stack.pop();
    result
}

fn execute_block(
    base_dir: &Path,
    task_file: &TaskFile,
    lines: &[TaskLine],
    start: usize,
    end: usize,
    indent: usize,
    effective_env: &mut RuntimeEnv,
    stack: &mut Vec<String>,
) -> CjResult<i32> {
    let mut index = start;
    while index < end {
        let line = &lines[index];
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            return Err(CjError::new(format!(
                "line {}: unexpected indentation",
                line.line_number
            )));
        }

        if let Some(rest) = line.text.strip_prefix('@') {
            let status = execute_directive(
                base_dir,
                task_file,
                lines,
                &mut index,
                end,
                indent,
                rest,
                effective_env,
                stack,
            )?;
            if status != 0 {
                return Ok(status);
            }
        } else {
            let status = run_direct_command(base_dir, &line.text, effective_env)?;
            index += 1;
            if status != 0 {
                return Ok(status);
            }
        }
    }

    Ok(0)
}

#[allow(clippy::too_many_arguments)]
fn execute_directive(
    base_dir: &Path,
    task_file: &TaskFile,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    directive: &str,
    effective_env: &mut RuntimeEnv,
    stack: &mut Vec<String>,
) -> CjResult<i32> {
    let (name, args) = split_directive(directive);
    match name {
        "shell" => {
            let command = interpolate_shell_text(args, effective_env)?;
            *index += 1;
            run_shell_command(base_dir, &command, effective_env)
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
            run_task(base_dir, task_file, &argv[0], effective_env, stack)
        }
        "set" | "export" => {
            if name == "set" {
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
            base_dir,
            task_file,
            lines,
            index,
            end,
            indent,
            name,
            args,
            effective_env,
            stack,
        ),
        "else" => Err(CjError::new(format!(
            "line {}: @else without matching @if",
            lines[*index].line_number
        ))),
        "switch" => execute_switch_directive(
            base_dir,
            task_file,
            lines,
            index,
            end,
            indent,
            args,
            effective_env,
            stack,
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
    base_dir: &Path,
    task_file: &TaskFile,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    name: &str,
    args: &str,
    effective_env: &mut RuntimeEnv,
    stack: &mut Vec<String>,
) -> CjResult<i32> {
    let condition = evaluate_condition(base_dir, name, args, effective_env)?;
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
            base_dir,
            task_file,
            lines,
            then_start,
            then_end,
            indent + 2,
            effective_env,
            stack,
        )
    } else if let Some((else_start, else_end)) = else_range {
        execute_block(
            base_dir,
            task_file,
            lines,
            else_start,
            else_end,
            indent + 2,
            effective_env,
            stack,
        )
    } else {
        Ok(0)
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_switch_directive(
    base_dir: &Path,
    task_file: &TaskFile,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    args: &str,
    effective_env: &mut RuntimeEnv,
    stack: &mut Vec<String>,
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
            base_dir,
            task_file,
            lines,
            start,
            end,
            body_indent,
            effective_env,
            stack,
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

fn run_direct_command(base_dir: &Path, command: &str, effective_env: &RuntimeEnv) -> CjResult<i32> {
    let argv = interpolate_argv(command, &effective_env.vars)?;
    let Some(program) = argv.first() else {
        return Ok(0);
    };

    let status = Command::new(program)
        .args(&argv[1..])
        .current_dir(base_dir)
        .env_clear()
        .envs(&effective_env.exports)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|err| CjError::new(format!("failed to run command '{command}': {err}")))?;

    Ok(status.code().unwrap_or(1))
}

fn run_shell_command(base_dir: &Path, command: &str, effective_env: &RuntimeEnv) -> CjResult<i32> {
    let status = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .current_dir(base_dir)
        .env_clear()
        .envs(&effective_env.exports)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|err| CjError::new(format!("failed to run shell command '{command}': {err}")))?;

    Ok(status.code().unwrap_or(1))
}

fn interpolate_argv(
    command: &str,
    effective_env: &HashMap<String, String>,
) -> CjResult<Vec<String>> {
    split_words(command)?
        .into_iter()
        .map(|word| interpolate_text(&word, effective_env, QuoteMode::None))
        .collect()
}

fn interpolate_shell_text(command: &str, effective_env: &RuntimeEnv) -> CjResult<String> {
    interpolate_text(command, &effective_env.vars, QuoteMode::Shell)
}

fn interpolate_text(
    input: &str,
    effective_env: &HashMap<String, String>,
    quote_mode: QuoteMode,
) -> CjResult<String> {
    let mut output = String::new();
    let mut chars = input.char_indices().peekable();
    while let Some((_, ch)) = chars.next() {
        if ch == '\\' {
            if let Some((_, '$')) = chars.peek().copied() {
                chars.next();
                output.push('$');
            } else {
                output.push(ch);
            }
            continue;
        }
        if ch != '$' {
            output.push(ch);
            continue;
        }

        let Some((_, next)) = chars.peek().copied() else {
            output.push('$');
            continue;
        };
        if next == '{' {
            chars.next();
            let mut expression = String::new();
            let mut closed = false;
            for (_, expr_ch) in chars.by_ref() {
                if expr_ch == '}' {
                    closed = true;
                    break;
                }
                expression.push(expr_ch);
            }
            if !closed {
                return Err(CjError::new("unterminated variable interpolation"));
            }
            let value = expand_braced(&expression, effective_env)?;
            output.push_str(&quote_value(&value, quote_mode));
            continue;
        }
        if !is_env_start(next) {
            output.push('$');
            continue;
        }
        let mut name = String::new();
        while let Some((_, name_ch)) = chars.peek().copied() {
            if is_env_continue(name_ch) {
                chars.next();
                name.push(name_ch);
            } else {
                break;
            }
        }
        let value = effective_env.get(&name).cloned().unwrap_or_default();
        output.push_str(&quote_value(&value, quote_mode));
    }
    Ok(output)
}

fn expand_braced(expression: &str, effective_env: &HashMap<String, String>) -> CjResult<String> {
    if let Some((name, fallback)) = expression.split_once(":-") {
        validate_env_name(name).map_err(|err| {
            CjError::new(format!(
                "invalid variable interpolation '{expression}': {err}"
            ))
        })?;
        Ok(match effective_env.get(name) {
            Some(value) if !value.is_empty() => value.clone(),
            _ => fallback.to_string(),
        })
    } else {
        validate_env_name(expression).map_err(|err| {
            CjError::new(format!(
                "invalid variable interpolation '{expression}': {err}"
            ))
        })?;
        effective_env
            .get(expression)
            .cloned()
            .ok_or_else(|| CjError::new(format!("missing variable: {expression}")))
    }
}

fn quote_value(value: &str, quote_mode: QuoteMode) -> String {
    match quote_mode {
        QuoteMode::None => value.to_string(),
        QuoteMode::Shell => shlex::try_quote(value)
            .map(|quoted| quoted.into_owned())
            .unwrap_or_else(|_| "''".to_string()),
    }
}

fn is_env_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_env_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn split_words(command: &str) -> CjResult<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut quote: Option<char> = None;
    let mut in_word = false;

    while let Some(ch) = chars.next() {
        match quote {
            Some(active) if ch == active => {
                quote = None;
                in_word = true;
            }
            Some('\'') => {
                current.push(ch);
                in_word = true;
            }
            Some('"') if ch == '\\' => {
                if let Some(next) = chars.next() {
                    if next == '$' {
                        current.push('\\');
                    }
                    current.push(next);
                    in_word = true;
                } else {
                    current.push(ch);
                    in_word = true;
                }
            }
            Some(_) => {
                current.push(ch);
                in_word = true;
            }
            None if ch == '\'' || ch == '"' => {
                quote = Some(ch);
                in_word = true;
            }
            None if ch.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut current));
                    in_word = false;
                }
            }
            None if ch == '\\' => {
                if let Some(next) = chars.next() {
                    if next == '$' {
                        current.push('\\');
                    }
                    current.push(next);
                } else {
                    current.push(ch);
                }
                in_word = true;
            }
            None => {
                current.push(ch);
                in_word = true;
            }
        }
    }

    if let Some(active) = quote {
        return Err(CjError::new(format!("unterminated {active} quote")));
    }
    if in_word {
        words.push(current);
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(name: &str) -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        env::temp_dir().join(format!("cjtasks-{name}-{id}"))
    }

    fn minimal_env() -> RuntimeEnv {
        RuntimeEnv::new(HashMap::from([(
            "PATH".to_string(),
            env::var("PATH").unwrap_or_default(),
        )]))
    }

    #[test]
    fn parses_env_and_tasks() {
        let path = Path::new("cjt");
        let parsed = parse_task_file(
            r#"
# Project tasks
env:
  NODE_ENV: development
  PORT?: 5173
  EMPTY:

dev:
  echo # retained

test123:
  cargo test
"#,
            path,
        )
        .expect("parse");

        assert_eq!(parsed.env.overrides["NODE_ENV"], "development");
        assert_eq!(parsed.env.overrides["EMPTY"], "");
        assert_eq!(parsed.env.fallbacks["PORT"], "5173");
        assert_eq!(parsed.tasks["dev"][0].text, "echo # retained");
        assert_eq!(parsed.tasks["test123"][0].text, "cargo test");
    }

    #[test]
    fn rejects_duplicate_env_entries() {
        let err = parse_task_file(
            "env:\n  NAME: one\n  NAME?: two\nrun:\n  echo hi\n",
            Path::new("cjt"),
        )
        .expect_err("duplicate env should fail");

        assert!(err.to_string().contains("duplicate env entry 'NAME'"));
    }

    #[test]
    fn rejects_bad_indentation() {
        let err = parse_task_file("run:\n   echo hi\n", Path::new("cjt"))
            .expect_err("bad indentation should fail");

        assert!(err.to_string().contains("even number of spaces"));
    }

    #[test]
    fn rejects_trailing_colon_directives() {
        let err = parse_task_file("run:\n  @if true:\n    echo hi\n", Path::new("cjt"))
            .expect_err("directive colon should fail");

        assert!(err
            .to_string()
            .contains("directives do not use trailing ':'"));
    }

    #[test]
    fn discovers_cjt_before_cjtasks() {
        let dir = test_path("discover");
        fs::create_dir_all(&dir).expect("mkdir");
        File::create(dir.join("cjtasks")).expect("cjtasks");
        File::create(dir.join("cjt")).expect("cjt");

        let discovered = discover_task_file(&dir).expect("discover");
        assert_eq!(discovered.file_name().unwrap(), "cjt");

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn dot_env_and_task_env_merge_with_absent_only_fallbacks() {
        let dir = test_path("env");
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(
            dir.join(".env"),
            "CJTEST_FROM_DOT=dot\nCJTEST_EXISTING=dot\nCJTEST_QUOTED=\"quoted value\"\n",
        )
        .expect("write .env");

        env::set_var("CJTEST_EXISTING", "");
        env::set_var("CJTEST_FALLBACK_EMPTY", "");

        let mut entries = EnvEntries::default();
        entries
            .fallbacks
            .insert("CJTEST_FALLBACK_EMPTY".to_string(), "fallback".to_string());
        entries
            .fallbacks
            .insert("CJTEST_ONLY_FALLBACK".to_string(), "fallback".to_string());
        entries
            .overrides
            .insert("CJTEST_EXISTING".to_string(), "override".to_string());

        let merged = build_effective_env(&dir, &entries).expect("env");
        assert_eq!(merged["CJTEST_FROM_DOT"], "dot");
        assert_eq!(merged["CJTEST_EXISTING"], "override");
        assert_eq!(merged["CJTEST_FALLBACK_EMPTY"], "");
        assert_eq!(merged["CJTEST_ONLY_FALLBACK"], "fallback");
        assert_eq!(merged["CJTEST_QUOTED"], "quoted value");

        env::remove_var("CJTEST_EXISTING");
        env::remove_var("CJTEST_FALLBACK_EMPTY");
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn local_venv_prepends_bin_to_path() {
        let dir = test_path("venv");
        fs::create_dir_all(dir.join(".venv/bin")).expect("mkdir");

        let mut effective = HashMap::from([("PATH".to_string(), "/usr/bin".to_string())]);
        apply_python_venv(&dir, &mut effective).expect("venv");

        let expected_prefix = dir.join(".venv/bin").to_string_lossy().to_string();
        assert_eq!(
            effective["VIRTUAL_ENV"],
            dir.join(".venv").to_string_lossy()
        );
        assert!(effective["PATH"].starts_with(&format!("{expected_prefix}:")));

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn ordinary_commands_execute_directly_without_shell_splitting_interpolated_values() {
        let dir = test_path("direct");
        fs::create_dir_all(&dir).expect("mkdir");
        let parsed = parse_task_file(
            "run:\n  sh -c 'test \"$1\" = \"a b; echo injected\"' ignored $CJTEST_VALUE\n",
            Path::new("cjt"),
        )
        .expect("parse");
        let mut env = minimal_env();
        env.vars
            .insert("CJTEST_VALUE".to_string(), "a b; echo injected".to_string());

        let code = run_task(&dir, &parsed, "run", &mut env, &mut Vec::new()).expect("run");
        assert_eq!(code, 0);

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn shell_execution_is_explicit_and_quotes_interpolated_values() {
        let dir = test_path("shell");
        fs::create_dir_all(&dir).expect("mkdir");
        let parsed = parse_task_file(
            "run:\n  @shell printf '%s' $CJTEST_VALUE > out.txt\n",
            Path::new("cjt"),
        )
        .expect("parse");
        let mut env = minimal_env();
        env.vars
            .insert("CJTEST_VALUE".to_string(), "safe; echo bad".to_string());

        let code = run_task(&dir, &parsed, "run", &mut env, &mut Vec::new()).expect("run");
        assert_eq!(code, 0);
        assert_eq!(
            fs::read_to_string(dir.join("out.txt")).expect("out"),
            "safe; echo bad"
        );

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn task_composition_and_cycle_detection() {
        let parsed = parse_task_file(
            "first:\n  @task second\nsecond:\n  true\ncycle:\n  @task cycle\n",
            Path::new("cjt"),
        )
        .expect("parse");
        let dir = test_path("task");
        fs::create_dir_all(&dir).expect("mkdir");
        let mut env = minimal_env();
        assert_eq!(
            run_task(&dir, &parsed, "first", &mut env, &mut Vec::new()).expect("run"),
            0
        );

        let err = run_task(&dir, &parsed, "cycle", &mut env, &mut Vec::new())
            .expect_err("cycle should fail");
        assert!(err.to_string().contains("recursive @task cycle"));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn mutable_env_conditionals_and_switches() {
        let dir = test_path("controls");
        fs::create_dir_all(&dir).expect("mkdir");
        File::create(dir.join("exists.txt")).expect("file");
        let parsed = parse_task_file(
            r#"run:
  @set MODE prod
  @if $MODE == prod
    @shell printf yes > if.txt
  @else
    @shell printf no > if.txt
  @if-exists exists.txt
    @export FOUND 1
  @if-set FOUND
    @shell printf found > found.txt
  @switch $MODE
    @case dev
      @shell printf dev > switch.txt
    @case prod
      @shell printf prod > switch.txt
    @default
      @shell printf default > switch.txt
  @unset FOUND
  @if-unset FOUND
    @shell printf unset > unset.txt
"#,
            Path::new("cjt"),
        )
        .expect("parse");
        let mut env = minimal_env();

        let code = run_task(&dir, &parsed, "run", &mut env, &mut Vec::new()).expect("run");
        assert_eq!(code, 0);
        assert_eq!(fs::read_to_string(dir.join("if.txt")).expect("if"), "yes");
        assert_eq!(
            fs::read_to_string(dir.join("found.txt")).expect("found"),
            "found"
        );
        assert_eq!(
            fs::read_to_string(dir.join("switch.txt")).expect("switch"),
            "prod"
        );
        assert_eq!(
            fs::read_to_string(dir.join("unset.txt")).expect("unset"),
            "unset"
        );

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn set_is_internal_until_exported() {
        let dir = test_path("export");
        fs::create_dir_all(&dir).expect("mkdir");
        let parsed = parse_task_file(
            r#"run:
  @set SECRET hidden
  @shell printf "\${SECRET:-missing}" > before.txt
  @export SECRET
  @shell printf "\$SECRET" > after.txt
"#,
            Path::new("cjt"),
        )
        .expect("parse");
        let mut env = minimal_env();

        let code = run_task(&dir, &parsed, "run", &mut env, &mut Vec::new()).expect("run");
        assert_eq!(code, 0);
        assert_eq!(
            fs::read_to_string(dir.join("before.txt")).expect("before"),
            "missing"
        );
        assert_eq!(
            fs::read_to_string(dir.join("after.txt")).expect("after"),
            "hidden"
        );

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn resolves_single_arg_from_current_directory() {
        let dir = test_path("single-arg");
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("cjt"), "run:\n  true\n").expect("write cjt");

        let resolved = resolve_invocation_from(&["run".to_string()], &dir).expect("resolve");
        assert_eq!(resolved.0.file_name().unwrap(), "cjt");
        assert_eq!(resolved.1, "run");

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn resolves_two_arg_directory_and_direct_file() {
        let dir = test_path("two-arg");
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("cjtasks"), "run:\n  true\n").expect("write cjtasks");

        let from_dir = resolve_invocation_from(
            &[dir.to_string_lossy().to_string(), "run".to_string()],
            &dir,
        )
        .expect("resolve dir");
        assert_eq!(from_dir.0.file_name().unwrap(), "cjtasks");

        let from_file = resolve_invocation_from(
            &[
                dir.join("cjtasks").to_string_lossy().to_string(),
                "run".to_string(),
            ],
            &dir,
        )
        .expect("resolve file");
        assert_eq!(from_file.0, dir.join("cjtasks"));

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn bare_relative_task_file_runs_from_current_directory() {
        let dir = test_path("relative-file");
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("cjt"), "run:\n  @shell pwd > out.txt\n").expect("write cjt");

        let code = run_cli_from_cwd(&["cjt".to_string(), "run".to_string()], &dir).expect("run");
        assert_eq!(code, 0);
        let reported = fs::read_to_string(dir.join("out.txt")).expect("out");
        assert_eq!(
            fs::canonicalize(reported.trim()).expect("reported pwd"),
            fs::canonicalize(&dir).expect("dir")
        );

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn selected_venv_requires_bin_directory() {
        let dir = test_path("bad-venv");
        fs::create_dir_all(dir.join(".venv")).expect("mkdir");

        let mut effective = HashMap::new();
        let err = apply_python_venv(&dir, &mut effective).expect_err("missing bin");

        assert!(err.to_string().contains(".venv/bin"));
        fs::remove_dir_all(dir).expect("cleanup");
    }
}
