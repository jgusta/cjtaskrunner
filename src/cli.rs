use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ansi::{paint, Style};
use crate::command_text::split_words;
use crate::directive_info::DIRECTIVES;
use crate::environment::build_effective_env;
use crate::formatter::format_taskfile_source;
use crate::help_output::{format_task_help, format_task_listing, format_top_help};
use crate::project_init::{auto_import_tasks, init_taskfile};
use crate::runner::run_task;
use crate::runtime::{CwdState, RuntimeEnv};
use crate::task_file::{
    parse_task_file, parse_task_file_layers, split_line_expressions, validate_task_name,
    EnvEntries, TaskFile,
};
use crate::taskfile_discovery::{existing_taskfile_path, is_recognized_taskfile};
use crate::{CjError, CjResult};

pub fn run_cli(args: &[String]) -> CjResult<i32> {
    run_cli_from_cwd(args, &env::current_dir()?)
}

pub(crate) fn run_cli_from_cwd(args: &[String], cwd: &Path) -> CjResult<i32> {
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h" || arg == "--cli-help")
    {
        print_help();
        return Ok(0);
    }

    if args.first().is_some_and(|arg| arg == "--directives") {
        print_directives();
        return Ok(0);
    }

    if args.first().is_some_and(|arg| arg == "--init") {
        require_no_arguments(&args[1..], "--init")?;
        return init_taskfile(cwd);
    }

    if args.first().is_some_and(|arg| arg == "--auto") {
        require_no_arguments(&args[1..], "--auto")?;
        return auto_import_tasks(cwd);
    }

    if args.first().is_some_and(|arg| arg == "-e") {
        require_no_arguments(&args[1..], "-e")?;
        return open_taskfile_in_editor(cwd);
    }

    if args.first().is_some_and(|arg| arg == "--run") {
        return run_single_line(&args[1..], cwd);
    }

    if args.first().is_some_and(|arg| arg == "--completions") {
        let shell = parse_shell_arg(&args[1..], "--completions")?;
        print_completions(shell);
        return Ok(0);
    }

    if args
        .first()
        .is_some_and(|arg| arg == "--install-completions")
    {
        let shell = parse_shell_arg(&args[1..], "--install-completions")?;
        return install_completions(shell);
    }

    if args.first().is_some_and(|arg| arg == "--format") {
        let task_file = resolve_format_target(&args[1..], cwd)?;
        return format_task_file_in_place(&task_file);
    }

    if args.first().is_some_and(|arg| arg == "help") {
        let (task_file, section) = resolve_help_invocation(&args[1..], cwd)?;
        return print_taskfile_help(&task_file, section.as_deref(), cwd);
    }

    let (task_file, task_name, arguments) = match resolve_invocation_from(args, cwd)? {
        Invocation::List { task_file } => return list_tasks(&task_file, cwd),
        Invocation::Run {
            task_file,
            task_name,
            arguments,
        } => (task_file, task_name, arguments),
    };
    let base_dir = task_file_base_dir(&task_file);
    let parsed = load_task_file(&task_file)?;
    validate_no_task_directory_conflicts(base_dir, &parsed)?;
    let mut env = RuntimeEnv::new(build_effective_env(base_dir, &parsed.env)?);
    let mut cwd = CwdState::new(base_dir);

    run_task(
        &parsed,
        &task_name,
        &arguments,
        &mut env,
        &mut cwd,
        &mut Vec::new(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Invocation {
    List {
        task_file: PathBuf,
    },
    Run {
        task_file: PathBuf,
        task_name: String,
        arguments: Vec<String>,
    },
}

pub(crate) fn resolve_invocation_from(args: &[String], cwd: &Path) -> CjResult<Invocation> {
    if args.is_empty() {
        return Ok(Invocation::List {
            task_file: discover_task_file(cwd)?,
        });
    }

    if args[0] == "--default" || args[0] == "-d" {
        return match args.len() {
            1 => Ok(Invocation::Run {
                task_file: discover_task_file(cwd)?,
                task_name: "default".to_string(),
                arguments: Vec::new(),
            }),
            2 => Ok(Invocation::Run {
                task_file: resolve_task_file_target(&args[1], cwd)?,
                task_name: "default".to_string(),
                arguments: Vec::new(),
            }),
            _ => Err(CjError::new("usage: cj --default [taskfile-or-directory]")),
        };
    }

    let raw_target = PathBuf::from(&args[0]);
    let target = if raw_target.is_absolute() {
        raw_target
    } else {
        cwd.join(raw_target)
    };
    if args.len() >= 2 && target.is_file() && !is_recognized_taskfile(&target) {
        return Err(CjError::new(format!(
            "unrecognized taskfile name: {}",
            target.display()
        )));
    }
    let explicit_task_file = if args.len() >= 2 && target.is_dir() {
        Some(discover_task_file(&target)?)
    } else if args.len() >= 2 && target.is_file() && is_recognized_taskfile(&target) {
        Some(target)
    } else {
        None
    };

    let (task_file, task_name, arguments) = if let Some(task_file) = explicit_task_file {
        (task_file, args[1].clone(), args[2..].to_vec())
    } else {
        (
            discover_task_file(cwd)?,
            args[0].clone(),
            args[1..].to_vec(),
        )
    };
    validate_task_name(&task_name)
        .map_err(|err| CjError::new(format!("invalid task name '{task_name}': {err}")))?;
    Ok(Invocation::Run {
        task_file,
        task_name,
        arguments,
    })
}

fn print_help() {
    println!(
        "{}\n\n{}\n  cj\n  cj --help\n  cj --cli-help\n  cj --directives\n  cj --init\n  cj --auto\n  cj -e\n  cj --run <line>\n  cj lsp\n  cj help [section]\n  cj <task> [arguments...]\n  cj <taskfile-or-directory> <task> [arguments...]\n  cj --default [taskfile-or-directory]\n  cj -d [taskfile-or-directory]\n  cj --format [taskfile-or-directory]\n  cj --completions <bash|zsh|fish>\n  cj --install-completions <bash|zsh|fish>\n\nNo task name lists tasks in the detected taskfile.\nUse --init to create a taskfile.\nUse --auto to import tasks from common project task systems.\nUse -e to open the detected taskfile in $EDITOR.\nUse --default or -d to run the default task.\nUse --run to execute one task line without requiring a taskfile.\nUse cj help to show taskfile help.",
        paint("CJTaskrunner", Style::Header),
        paint("Usage:", Style::Section),
    );
}

fn require_no_arguments(args: &[String], command: &str) -> CjResult<()> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(CjError::new(format!("usage: cj {command}")))
    }
}

fn print_directives() {
    println!("{}", paint("CJTaskrunner directives:", Style::Header));
    for directive in DIRECTIVES {
        println!(
            "  {}{} {}",
            paint(format!("@{}", directive.name), Style::Directive),
            " ".repeat(15usize.saturating_sub(directive.name.len() + 1)),
            paint(directive.description, Style::Description)
        );
    }
}

fn open_taskfile_in_editor(cwd: &Path) -> CjResult<i32> {
    let task_file = discover_task_file(cwd)?;
    run_editor(&task_file)
}

fn run_editor(task_file: &Path) -> CjResult<i32> {
    let editor = env::var("EDITOR")
        .map_err(|_| CjError::new("EDITOR is not set; set EDITOR to use cj -e"))?;
    let argv = split_words(&editor)?;
    let Some((program, args)) = argv.split_first() else {
        return Err(CjError::new("EDITOR is empty; set EDITOR to use cj -e"));
    };
    let status = Command::new(program)
        .args(args)
        .arg(task_file)
        .status()
        .map_err(|err| CjError::new(format!("failed to run EDITOR '{program}': {err}")))?;
    Ok(status.code().unwrap_or(1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

fn parse_shell_arg(args: &[String], command: &str) -> CjResult<CompletionShell> {
    if args.len() != 1 {
        return Err(CjError::new(format!("usage: cj {command} <bash|zsh|fish>")));
    }
    match args[0].as_str() {
        "bash" => Ok(CompletionShell::Bash),
        "zsh" => Ok(CompletionShell::Zsh),
        "fish" => Ok(CompletionShell::Fish),
        shell => Err(CjError::new(format!(
            "unsupported shell '{shell}'; expected bash, zsh, or fish"
        ))),
    }
}

fn print_completions(shell: CompletionShell) {
    print!("{}", completion_script(shell));
}

pub(crate) fn install_completions(shell: CompletionShell) -> CjResult<i32> {
    let path = completion_install_path(shell)?;
    let parent = path
        .parent()
        .ok_or_else(|| CjError::new(format!("invalid completion path: {}", path.display())))?;
    fs::create_dir_all(parent)?;
    fs::write(&path, completion_script(shell))
        .map_err(|err| CjError::new(format!("failed to write {}: {err}", path.display())))?;
    println!("installed completions to {}", path.display());
    if shell == CompletionShell::Zsh {
        println!(
            "add {} to your zsh fpath before running compinit",
            parent.display()
        );
    }
    Ok(0)
}

pub(crate) fn completion_install_path(shell: CompletionShell) -> CjResult<PathBuf> {
    Ok(match shell {
        CompletionShell::Bash => xdg_data_home()?.join("bash-completion/completions/cj"),
        CompletionShell::Zsh => xdg_data_home()?.join("zsh/site-functions/_cj"),
        CompletionShell::Fish => xdg_config_home()?.join("fish/completions/cj.fish"),
    })
}

fn xdg_data_home() -> CjResult<PathBuf> {
    if let Some(path) = non_empty_os_env("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path));
    }
    Ok(home_dir()?.join(".local/share"))
}

fn xdg_config_home() -> CjResult<PathBuf> {
    if let Some(path) = non_empty_os_env("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(path));
    }
    Ok(home_dir()?.join(".config"))
}

fn home_dir() -> CjResult<PathBuf> {
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| CjError::new("HOME is not set"))
}

fn non_empty_os_env(key: &str) -> Option<std::ffi::OsString> {
    env::var_os(key).filter(|value| !value.is_empty())
}

pub(crate) fn completion_script(shell: CompletionShell) -> &'static str {
    match shell {
        CompletionShell::Bash => BASH_COMPLETIONS,
        CompletionShell::Zsh => ZSH_COMPLETIONS,
        CompletionShell::Fish => FISH_COMPLETIONS,
    }
}

const BASH_COMPLETIONS: &str = r#"_cj_tasks() {
  NO_COLOR=1 cj 2>/dev/null | awk '
    /^(\+| )  +[A-Za-z0-9_-]+(:[A-Za-z0-9_-]+)*/ {
      if ($1 == "+") print $2; else print $1
    }
  '
}

_cj() {
  local cur prev
  COMPREPLY=()
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev="${COMP_WORDS[COMP_CWORD-1]}"

  if [[ "$prev" == "--completions" || "$prev" == "--install-completions" ]]; then
    COMPREPLY=( $(compgen -W "bash zsh fish" -- "$cur") )
    return 0
  fi

  if [[ "${COMP_WORDS[1]}" == "help" && "$COMP_CWORD" -eq 2 ]]; then
    COMPREPLY=( $(compgen -W "$(_cj_tasks)" -- "$cur") )
    return 0
  fi

  if [[ "$COMP_CWORD" -eq 1 ]]; then
    COMPREPLY=( $(compgen -W "--help --cli-help --directives --init --auto -e --run --default -d --format --completions --install-completions help lsp $(_cj_tasks)" -- "$cur") )
    return 0
  fi

  if [[ "$prev" == "--format" || "$prev" == "--run" || "$prev" == "--default" || "$prev" == "-d" ]]; then
    compopt -o default 2>/dev/null
    return 0
  fi

  COMPREPLY=( $(compgen -W "$(_cj_tasks)" -- "$cur") )
}

complete -F _cj cj
"#;

const ZSH_COMPLETIONS: &str = r#"#compdef cj

_cj_tasks() {
  NO_COLOR=1 cj 2>/dev/null | awk '/^(\+| )  +[A-Za-z0-9_-]+(:[A-Za-z0-9_-]+)*/ { if ($1 == "+") print $2; else print $1 }'
}

_cj() {
  local -a commands shells tasks
  commands=(--help --cli-help --directives --init --auto -e --run --default -d --format --completions --install-completions help lsp)
  shells=(bash zsh fish)
  tasks=(${(f)"$(_cj_tasks)"})

  if (( CURRENT > 2 )) && [[ ${words[CURRENT-1]} == "--completions" || ${words[CURRENT-1]} == "--install-completions" ]]; then
    _describe 'shell' shells
    return
  fi

  if [[ ${words[2]} == "help" && $CURRENT -eq 3 ]]; then
    _describe 'task help' tasks
    return
  fi

  if (( CURRENT == 2 )); then
    _describe 'command' commands
    _describe 'task' tasks
    return
  fi

  if [[ ${words[CURRENT-1]} == "--format" || ${words[CURRENT-1]} == "--run" || ${words[CURRENT-1]} == "--default" || ${words[CURRENT-1]} == "-d" ]]; then
    _files
    return
  fi

  _describe 'task' tasks
}

_cj "$@"
"#;

const FISH_COMPLETIONS: &str = r#"function __cj_tasks
    NO_COLOR=1 cj 2>/dev/null | awk '
        /^(\+| )  +[A-Za-z0-9_-]+(:[A-Za-z0-9_-]+)*/ {
            marked = ($1 == "+")
            task = marked ? $2 : $1
            start = marked ? 3 : 2
            desc = ""
            for (i = start; i <= NF; i++) {
                desc = desc (i > start ? " " : "") $i
            }
            if (desc == "") {
                print task
            } else {
                print task "\t" desc
            }
        }
    '
end

complete -c cj -f -n '__fish_use_subcommand' -a '(__cj_tasks)' -d 'Task'

complete -c cj -f -n '__fish_use_subcommand' -a 'help' -d 'Show taskfile help'
complete -c cj -f -n '__fish_use_subcommand' -a 'lsp' -d 'Start language server'
complete -c cj -f -n '__fish_seen_subcommand_from help' -a '(__cj_tasks)' -d 'Task help'

complete -c cj -l help -d 'Show CLI help'
complete -c cj -l cli-help -d 'Show CLI help'
complete -c cj -l directives -d 'List directives'
complete -c cj -l init -d 'Create a cjtasks file'
complete -c cj -l auto -d 'Import common project tasks'
complete -c cj -s e -d 'Open taskfile in $EDITOR'
complete -c cj -l run -r -d 'Run one task line'
complete -c cj -l default -s d -r -d 'Run default task'
complete -c cj -l format -r -d 'Format taskfile'
complete -c cj -l completions -f -a 'bash zsh fish' -d 'Print shell completions'
complete -c cj -l install-completions -f -a 'bash zsh fish' -d 'Install shell completions'
"#;

fn resolve_help_invocation(args: &[String], cwd: &Path) -> CjResult<(PathBuf, Option<String>)> {
    match args.len() {
        0 => Ok((discover_task_file(cwd)?, None)),
        1 => Ok((discover_task_file(cwd)?, Some(args[0].clone()))),
        _ => Err(CjError::new("usage: cj help [section]")),
    }
}

fn resolve_format_target(args: &[String], cwd: &Path) -> CjResult<PathBuf> {
    match args.len() {
        0 => discover_task_file(cwd),
        1 => resolve_task_file_target(&args[0], cwd),
        _ => Err(CjError::new("usage: cj --format [taskfile-or-directory]")),
    }
}

fn resolve_task_file_target(raw: &str, cwd: &Path) -> CjResult<PathBuf> {
    let raw_target = PathBuf::from(raw);
    let target = if raw_target.is_absolute() {
        raw_target
    } else {
        cwd.join(raw_target)
    };

    if target.is_dir() {
        discover_task_file(&target)
    } else if target.is_file() {
        if is_recognized_taskfile(&target) {
            Ok(target)
        } else {
            Err(CjError::new(format!(
                "unrecognized taskfile name: {}",
                target.display()
            )))
        }
    } else if target.exists() {
        Err(CjError::new(format!(
            "path is neither a recognized taskfile nor a directory: {}",
            target.display()
        )))
    } else {
        Err(CjError::new(format!(
            "path does not exist: {}",
            target.display()
        )))
    }
}

fn format_task_file_in_place(task_file: &Path) -> CjResult<i32> {
    let source = fs::read_to_string(task_file)
        .map_err(|err| CjError::new(format!("failed to read {}: {err}", task_file.display())))?;
    let formatted = format_taskfile_source(&source);
    if formatted != source {
        fs::write(task_file, formatted).map_err(|err| {
            CjError::new(format!("failed to write {}: {err}", task_file.display()))
        })?;
    }
    println!("formatted {}", task_file.display());
    Ok(0)
}

fn list_tasks(task_file: &Path, invocation_dir: &Path) -> CjResult<i32> {
    let parsed = load_task_file(task_file)?;
    validate_no_task_directory_conflicts(task_file_base_dir(task_file), &parsed)?;

    println!(
        "{}",
        format_task_listing(&parsed, display_task_file(task_file, invocation_dir))
    );
    Ok(0)
}

fn print_taskfile_help(
    task_file: &Path,
    section: Option<&str>,
    invocation_dir: &Path,
) -> CjResult<i32> {
    let parsed = load_task_file(task_file)?;
    validate_no_task_directory_conflicts(task_file_base_dir(task_file), &parsed)?;
    let display_path = display_task_file(task_file, invocation_dir);
    let help = match section {
        Some(name) => format_task_help(&parsed, name, display_path)?,
        None => format_top_help(&parsed, display_path),
    };
    println!("{}\n\nrun `cj --help` for program help", help.trim_end());
    Ok(0)
}

fn display_task_file<'a>(task_file: &'a Path, invocation_dir: &Path) -> &'a Path {
    task_file.strip_prefix(invocation_dir).unwrap_or(task_file)
}

fn run_single_line(args: &[String], cwd: &Path) -> CjResult<i32> {
    if args.len() != 1 {
        return Err(CjError::new("usage: cj --run <line>"));
    }
    let line = args[0].trim();
    validate_single_line_task(line)?;

    let source = format!("__run:\n  {line}\n");
    let parsed = parse_task_file(&source, Path::new("<cj --run>"))?;
    let mut env = RuntimeEnv::new(build_effective_env(cwd, &EnvEntries::default())?);
    let mut cwd = CwdState::new(cwd);
    run_task(&parsed, "__run", &[], &mut env, &mut cwd, &mut Vec::new())
}

fn validate_single_line_task(line: &str) -> CjResult<()> {
    if line.is_empty() {
        return Err(CjError::new("--run line cannot be empty"));
    }
    if line.contains('\n') || line.contains('\r') {
        return Err(CjError::new("--run accepts exactly one line"));
    }
    for expression in split_line_expressions(line) {
        let trimmed = expression.trim();
        if trimmed.ends_with(':') && !trimmed.starts_with('@') {
            return Err(CjError::new("--run does not accept task labels"));
        }
        let Some(rest) = trimmed.strip_prefix('@') else {
            continue;
        };
        let (name, args) = crate::directives::split_directive(rest);
        if name == "help:"
            || is_single_line_block_directive(name)
            || is_set_capture_args(name, args)
        {
            return Err(CjError::new("--run does not accept block directives"));
        }
    }
    Ok(())
}

fn is_single_line_block_directive(name: &str) -> bool {
    matches!(
        name,
        "and"
            | "or"
            | "if"
            | "if-not"
            | "if-in"
            | "if-not-in"
            | "else"
            | "if-exists"
            | "if-not-exists"
            | "if-set"
            | "if-not-set"
            | "if-version"
            | "if-not-version"
            | "if-bumped"
            | "if-not-bumped"
            | "if-patch"
            | "if-minor"
            | "if-major"
            | "if-pre"
            | "if-release"
            | "if-not-patch"
            | "if-not-minor"
            | "if-not-major"
            | "if-not-pre"
            | "if-not-release"
            | "switch"
            | "case"
            | "default"
    )
}

fn is_set_capture_args(name: &str, args: &str) -> bool {
    name == "set" && args.trim_end().ends_with(':')
}

fn validate_no_task_directory_conflicts(base_dir: &Path, parsed: &TaskFile) -> CjResult<()> {
    for task in &parsed.task_order {
        if base_dir.join(task).is_dir() {
            return Err(CjError::new(format!(
                "task name conflicts with directory: {task}"
            )));
        }
    }
    Ok(())
}

pub(crate) fn discover_task_file(dir: &Path) -> CjResult<PathBuf> {
    if let Some(path) = existing_taskfile_path(dir) {
        return Ok(path);
    }

    Err(CjError::new(format!(
        "no recognized taskfile found in {}",
        dir.display()
    )))
}

fn load_task_file(path: &Path) -> CjResult<TaskFile> {
    let directory = task_file_base_dir(path);
    parse_task_file_layers(directory)
}

fn task_file_base_dir(task_file: &Path) -> &Path {
    task_file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}
