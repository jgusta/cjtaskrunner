use std::collections::HashMap;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, Once, OnceLock};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::runtime::{append_captured_output, CommandResult, OutputMode, QuoteMode, RuntimeEnv};
use crate::task_file::validate_env_name;
use crate::{CjError, CjResult};

static SIGNAL_HANDLER: Once = Once::new();
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static ACTIVE_CHILDREN: OnceLock<Mutex<Vec<ActiveChild>>> = OnceLock::new();

#[derive(Clone, Copy)]
struct ActiveChild {
    id: u32,
    isolated_process_group: bool,
}

pub(crate) fn run_direct_command(
    base_dir: &Path,
    command: &str,
    effective_env: &RuntimeEnv,
    output_mode: OutputMode,
) -> CjResult<CommandResult> {
    let argv = interpolate_argv(command, &effective_env.vars)?;
    let Some(program) = argv.first() else {
        return Ok(CommandResult::default());
    };

    let mut child = Command::new(program);
    let exported_values = effective_env.exported_values();
    child
        .args(&argv[1..])
        .current_dir(base_dir)
        .env_clear()
        .envs(&exported_values)
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit());

    let result = run_child(child, output_mode, true)
        .map_err(|err| CjError::new(format!("failed to run command '{command}': {err}")))?;
    Ok(result)
}

pub(crate) fn run_shell_command(
    base_dir: &Path,
    command: &str,
    effective_env: &RuntimeEnv,
    output_mode: OutputMode,
) -> CjResult<CommandResult> {
    let mut child = Command::new("/bin/sh");
    let exported_values = effective_env.exported_values();
    child
        .arg("-c")
        .arg(command)
        .current_dir(base_dir)
        .env_clear()
        .envs(&exported_values)
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit());

    let result = run_child(child, output_mode, true)
        .map_err(|err| CjError::new(format!("failed to run shell command '{command}': {err}")))?;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenCommandSpec {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
}

pub(crate) fn open_command_spec(url: &str) -> OpenCommandSpec {
    #[cfg(target_os = "macos")]
    {
        OpenCommandSpec {
            program: "open".to_string(),
            args: vec![url.to_string()],
        }
    }
    #[cfg(target_os = "windows")]
    {
        OpenCommandSpec {
            program: "cmd".to_string(),
            args: vec![
                "/C".to_string(),
                "start".to_string(),
                String::new(),
                url.to_string(),
            ],
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        OpenCommandSpec {
            program: "xdg-open".to_string(),
            args: vec![url.to_string()],
        }
    }
}

pub(crate) fn run_open_url(
    url: &str,
    effective_env: &RuntimeEnv,
    output_mode: OutputMode,
) -> CjResult<CommandResult> {
    let spec = open_command_spec(url);
    let mut child = Command::new(&spec.program);
    let exported_values = effective_env.exported_values();
    child
        .args(&spec.args)
        .env_clear()
        .envs(&exported_values)
        .stdin(Stdio::null())
        .stderr(Stdio::inherit());

    let result = run_child(child, output_mode, false)
        .map_err(|err| CjError::new(format!("failed to open URL '{url}': {err}")))?;
    Ok(result)
}

fn run_child(
    mut child: Command,
    output_mode: OutputMode,
    inherits_stdin: bool,
) -> io::Result<CommandResult> {
    ensure_signal_handler();
    if interrupted() {
        return Ok(CommandResult {
            status: 130,
            output: String::new(),
        });
    }

    let isolated_process_group =
        should_isolate_child_process_group(inherits_stdin, io::stdin().is_terminal());
    configure_child(&mut child, isolated_process_group);
    match output_mode {
        OutputMode::Inherit => {
            let mut child = child.stdout(Stdio::inherit()).spawn()?;
            let active_child = ActiveChild {
                id: child.id(),
                isolated_process_group,
            };
            register_child(active_child);
            if interrupted() {
                terminate_child(active_child);
            }
            let status = child.wait()?;
            unregister_child(active_child.id);
            Ok(CommandResult {
                status: status.code().unwrap_or_else(interrupted_status),
                output: String::new(),
            })
        }
        OutputMode::Capture => {
            let child = child.stdout(Stdio::piped()).spawn()?;
            let active_child = ActiveChild {
                id: child.id(),
                isolated_process_group,
            };
            register_child(active_child);
            if interrupted() {
                terminate_child(active_child);
            }
            let output = wait_with_output(child);
            unregister_child(active_child.id);
            let output = output?;
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            append_captured_output(&text);
            Ok(CommandResult {
                status: output.status.code().unwrap_or_else(interrupted_status),
                output: text,
            })
        }
    }
}

fn wait_with_output(child: Child) -> io::Result<std::process::Output> {
    child.wait_with_output()
}

pub(crate) fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

fn interrupted_status() -> i32 {
    if interrupted() {
        130
    } else {
        1
    }
}

fn ensure_signal_handler() {
    SIGNAL_HANDLER.call_once(|| {
        let _ = ctrlc::set_handler(|| {
            INTERRUPTED.store(true, Ordering::SeqCst);
            terminate_active_children();
        });
    });
}

fn active_children() -> &'static Mutex<Vec<ActiveChild>> {
    ACTIVE_CHILDREN.get_or_init(|| Mutex::new(Vec::new()))
}

fn register_child(child: ActiveChild) {
    active_children()
        .lock()
        .expect("active children lock")
        .push(child);
}

fn unregister_child(child_id: u32) {
    active_children()
        .lock()
        .expect("active children lock")
        .retain(|active| active.id != child_id);
}

pub(crate) fn terminate_active_children() {
    let children = active_children()
        .lock()
        .expect("active children lock")
        .clone();
    for child in children {
        terminate_child(child);
    }
}

pub(crate) fn should_isolate_child_process_group(
    inherits_stdin: bool,
    stdin_is_terminal: bool,
) -> bool {
    cfg!(unix) && !(inherits_stdin && stdin_is_terminal)
}

#[cfg(unix)]
fn configure_child(child: &mut Command, isolated_process_group: bool) {
    if !isolated_process_group {
        return;
    }
    unsafe {
        child.pre_exec(|| {
            if libc::setpgid(0, 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn configure_child(_child: &mut Command, _isolated_process_group: bool) {}

#[cfg(unix)]
fn terminate_child(child: ActiveChild) {
    unsafe {
        if child.isolated_process_group {
            libc::killpg(child.id as libc::pid_t, libc::SIGINT);
        } else {
            libc::kill(child.id as libc::pid_t, libc::SIGINT);
        }
    }
}

#[cfg(not(unix))]
fn terminate_child(_child: ActiveChild) {}

pub(crate) fn interpolate_argv(
    command: &str,
    effective_env: &HashMap<String, String>,
) -> CjResult<Vec<String>> {
    split_words(command)?
        .into_iter()
        .map(|word| interpolate_text(&word, effective_env, QuoteMode::None))
        .collect()
}

pub(crate) fn interpolate_shell_text(
    command: &str,
    effective_env: &RuntimeEnv,
) -> CjResult<String> {
    interpolate_text(command, &effective_env.vars, QuoteMode::Shell)
}

pub(crate) fn contains_variable_interpolation(input: &str) -> bool {
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if chars.peek() == Some(&'$') {
                chars.next();
            }
            continue;
        }
        if ch != '$' {
            continue;
        }

        match chars.peek().copied() {
            Some('{') => return true,
            Some(next) if is_env_start(next) => return true,
            _ => {}
        }
    }
    false
}

pub(crate) fn variable_references(input: &str) -> Vec<String> {
    let mut references = Vec::new();
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
                .split_once('?')
                .map_or(expression.as_str(), |value| value.0);
            if validate_env_name(name).is_ok() {
                references.push(name.to_string());
            }
            continue;
        }
        if !is_env_start(next) {
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
        if !name.is_empty() {
            references.push(name);
        }
    }
    references
}

pub(crate) fn unescape_variable_literals(input: &str) -> String {
    input.replace("\\$", "$")
}

pub(crate) fn interpolate_text(
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
    if expression.contains(":-") {
        return Err(CjError::new(format!(
            "invalid variable interpolation '{expression}': use ${{NAME?fallback}}"
        )));
    }
    if let Some((name, fallback)) = expression.split_once('?') {
        validate_env_name(name).map_err(|err| {
            CjError::new(format!(
                "invalid variable interpolation '{expression}': {err}"
            ))
        })?;
        return match effective_env.get(name) {
            Some(value) => Ok(value.clone()),
            None if fallback.is_empty() => Err(CjError::new(format!("missing variable: {name}"))),
            None => Ok(unquote_fallback(fallback).to_string()),
        };
    }

    validate_env_name(expression).map_err(|err| {
        CjError::new(format!(
            "invalid variable interpolation '{expression}': {err}"
        ))
    })?;
    Ok(effective_env.get(expression).cloned().unwrap_or_default())
}

fn unquote_fallback(fallback: &str) -> &str {
    fallback
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(fallback)
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

pub(crate) fn split_words(command: &str) -> CjResult<Vec<String>> {
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
