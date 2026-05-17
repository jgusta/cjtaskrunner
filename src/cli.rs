pub fn run_cli(args: &[String]) -> CjResult<i32> {
    run_cli_from_cwd(args, &env::current_dir()?)
}

fn run_cli_from_cwd(args: &[String], cwd: &Path) -> CjResult<i32> {
    if args.first().is_some_and(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(0);
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
        return print_taskfile_help(&task_file, section.as_deref());
    }

    let (task_file, task_name) = match resolve_invocation_from(args, cwd)? {
        Invocation::List { task_file } => return list_tasks(&task_file),
        Invocation::Run {
            task_file,
            task_name,
        } => (task_file, task_name),
    };
    let base_dir = task_file_base_dir(&task_file);
    let parsed = parse_task_file_path(&task_file)?;
    validate_no_task_directory_conflicts(base_dir, &parsed)?;
    let mut env = RuntimeEnv::new(build_effective_env(base_dir, &parsed.env)?);
    let mut cwd = CwdState::new(base_dir);

    run_task(&parsed, &task_name, &mut env, &mut cwd, &mut Vec::new())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Invocation {
    List { task_file: PathBuf },
    Run { task_file: PathBuf, task_name: String },
}

fn resolve_invocation_from(args: &[String], cwd: &Path) -> CjResult<Invocation> {
    match args.len() {
        0 => Ok(Invocation::List {
            task_file: discover_task_file(cwd)?,
        }),
        1 => {
            if args[0] == "--default" || args[0] == "-d" {
                return Ok(Invocation::Run {
                    task_file: discover_task_file(cwd)?,
                    task_name: "default".to_string(),
                });
            }
            let task_name = args[0].clone();
            validate_task_name(&task_name)
                .map_err(|err| CjError::new(format!("invalid task name '{task_name}': {err}")))?;
            Ok(Invocation::Run {
                task_file: discover_task_file(cwd)?,
                task_name,
            })
        }
        2 => {
            if args[0] == "--default" || args[0] == "-d" {
                return Ok(Invocation::Run {
                    task_file: resolve_task_file_target(&args[1], cwd)?,
                    task_name: "default".to_string(),
                });
            }
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
                Ok(Invocation::Run {
                    task_file: discover_task_file(&target)?,
                    task_name,
                })
            } else if target.is_file() {
                if is_recognized_task_file(&target) {
                    Ok(Invocation::Run {
                        task_file: target,
                        task_name,
                    })
                } else {
                    Err(CjError::new(format!(
                        "taskfile must be named 'cjtasks' or use the '.cjtasks' extension: {}",
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
        _ => Err(CjError::new(
            "usage: cj | cj --default [taskfile-or-directory] | cj <task> | cj <taskfile-or-directory> <task> | cj --format [taskfile-or-directory]",
        )),
    }
}

fn print_help() {
    println!(
        "\
CJTaskrunner

Usage:
  cj
  cj --help
  cj help [section]
  cj <task>
  cj <taskfile-or-directory> <task>
  cj --default [taskfile-or-directory]
  cj -d [taskfile-or-directory]
  cj --format [taskfile-or-directory]
  cj --completions <bash|zsh|fish>
  cj --install-completions <bash|zsh|fish>

No task name lists tasks in the detected taskfile.
Use --default or -d to run the default task.
Use cj help to show taskfile help.
"
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

fn parse_shell_arg(args: &[String], command: &str) -> CjResult<CompletionShell> {
    if args.len() != 1 {
        return Err(CjError::new(format!(
            "usage: cj {command} <bash|zsh|fish>"
        )));
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

fn install_completions(shell: CompletionShell) -> CjResult<i32> {
    let path = completion_install_path(shell)?;
    let parent = path
        .parent()
        .ok_or_else(|| CjError::new(format!("invalid completion path: {}", path.display())))?;
    fs::create_dir_all(parent)?;
    fs::write(&path, completion_script(shell))
        .map_err(|err| CjError::new(format!("failed to write {}: {err}", path.display())))?;
    println!("installed completions to {}", path.display());
    Ok(0)
}

fn completion_install_path(shell: CompletionShell) -> CjResult<PathBuf> {
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

fn completion_script(shell: CompletionShell) -> &'static str {
    match shell {
        CompletionShell::Bash => BASH_COMPLETIONS,
        CompletionShell::Zsh => ZSH_COMPLETIONS,
        CompletionShell::Fish => FISH_COMPLETIONS,
    }
}

const BASH_COMPLETIONS: &str = r#"_cj_tasks() {
  cj 2>/dev/null | awk '
    /^  [A-Za-z0-9_-]+(:[A-Za-z0-9_-]+)?/ { print $1 }
  '
}

_cj_help_sections() {
  cj 2>/dev/null | awk '
    $0 == "Help available:" { help = 1; next }
    help && /^  / { print $1 }
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
    COMPREPLY=( $(compgen -W "$(_cj_help_sections)" -- "$cur") )
    return 0
  fi

  if [[ "$COMP_CWORD" -eq 1 ]]; then
    COMPREPLY=( $(compgen -W "--help --default -d --format --completions --install-completions help $(_cj_tasks)" -- "$cur") )
    return 0
  fi

  if [[ "$prev" == "--format" || "$prev" == "--default" || "$prev" == "-d" ]]; then
    compopt -o default 2>/dev/null
    return 0
  fi

  COMPREPLY=( $(compgen -W "$(_cj_tasks)" -- "$cur") )
}

complete -F _cj cj
complete -F _cj cjtaskrunner
"#;

const ZSH_COMPLETIONS: &str = r#"#compdef cj cjtaskrunner

_cj_tasks() {
  cj 2>/dev/null | awk '/^  [A-Za-z0-9_-]+(:[A-Za-z0-9_-]+)?/ { print $1 }'
}

_cj_help_sections() {
  cj 2>/dev/null | awk '$0 == "Help available:" { help = 1; next } help && /^  / { print $1 }'
}

_cj() {
  local -a commands shells tasks help_sections
  commands=(--help --default -d --format --completions --install-completions help)
  shells=(bash zsh fish)
  tasks=(${(f)"$(_cj_tasks)"})
  help_sections=(${(f)"$(_cj_help_sections)"})

  if (( CURRENT > 2 )) && [[ ${words[CURRENT-1]} == "--completions" || ${words[CURRENT-1]} == "--install-completions" ]]; then
    _describe 'shell' shells
    return
  fi

  if [[ ${words[2]} == "help" && $CURRENT -eq 3 ]]; then
    _describe 'help section' help_sections
    return
  fi

  if (( CURRENT == 2 )); then
    _describe 'command' commands
    _describe 'task' tasks
    return
  fi

  if [[ ${words[CURRENT-1]} == "--format" || ${words[CURRENT-1]} == "--default" || ${words[CURRENT-1]} == "-d" ]]; then
    _files
    return
  fi

  _describe 'task' tasks
}

_cj "$@"
"#;

const FISH_COMPLETIONS: &str = r#"function __cj_tasks
    cj 2>/dev/null | awk '/^  [A-Za-z0-9_-]+(:[A-Za-z0-9_-]+)?/ { print $1 }'
end

function __cj_help_sections
    cj 2>/dev/null | awk '$0 == "Help available:" { help = 1; next } help && /^  / { print $1 }'
end

complete -c cj -f -n '__fish_use_subcommand' -a '(__cj_tasks)' -d 'Task'
complete -c cjtaskrunner -f -n '__fish_use_subcommand' -a '(__cj_tasks)' -d 'Task'

complete -c cj -f -n '__fish_use_subcommand' -a 'help' -d 'Show taskfile help'
complete -c cjtaskrunner -f -n '__fish_use_subcommand' -a 'help' -d 'Show taskfile help'
complete -c cj -f -n '__fish_seen_subcommand_from help' -a '(__cj_help_sections)' -d 'Help section'
complete -c cjtaskrunner -f -n '__fish_seen_subcommand_from help' -a '(__cj_help_sections)' -d 'Help section'

complete -c cj -l help -d 'Show CLI help'
complete -c cjtaskrunner -l help -d 'Show CLI help'
complete -c cj -l default -s d -r -d 'Run default task'
complete -c cjtaskrunner -l default -s d -r -d 'Run default task'
complete -c cj -l format -r -d 'Format taskfile'
complete -c cjtaskrunner -l format -r -d 'Format taskfile'
complete -c cj -l completions -f -a 'bash zsh fish' -d 'Print shell completions'
complete -c cjtaskrunner -l completions -f -a 'bash zsh fish' -d 'Print shell completions'
complete -c cj -l install-completions -f -a 'bash zsh fish' -d 'Install shell completions'
complete -c cjtaskrunner -l install-completions -f -a 'bash zsh fish' -d 'Install shell completions'
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
        _ => Err(CjError::new(
            "usage: cj --format [taskfile-or-directory]",
        )),
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
        if is_recognized_task_file(&target) {
            Ok(target)
        } else {
            Err(CjError::new(format!(
                "taskfile must be named 'cjtasks' or use the '.cjtasks' extension: {}",
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
        fs::write(task_file, formatted)
            .map_err(|err| CjError::new(format!("failed to write {}: {err}", task_file.display())))?;
    }
    println!("formatted {}", task_file.display());
    Ok(0)
}

fn list_tasks(task_file: &Path) -> CjResult<i32> {
    let parsed = parse_task_file_path(task_file)?;
    validate_no_task_directory_conflicts(task_file_base_dir(task_file), &parsed)?;

    println!("Tasks in {}:", task_file.display());
    for name in &parsed.task_order {
        if let Some(description) = parsed.descriptions.get(name) {
            println!("  {name:<20} {description}");
        } else {
            println!("  {name}");
        }
    }
    let help_sections = help_sections(&parsed);
    if !help_sections.is_empty() {
        println!();
        println!("Help available:");
        for section in help_sections {
            println!("  {section}");
        }
    }
    Ok(0)
}

fn print_taskfile_help(task_file: &Path, section: Option<&str>) -> CjResult<i32> {
    let parsed = parse_task_file_path(task_file)?;
    validate_no_task_directory_conflicts(task_file_base_dir(task_file), &parsed)?;
    let help = match section {
        Some(name) => parsed.task_help.get(name).ok_or_else(|| {
            CjError::new(format!(
                "no help section '{name}' found in {}",
                task_file.display()
            ))
        })?,
        None => parsed.help.as_ref().ok_or_else(|| {
            CjError::new(format!("no top-level help found in {}", task_file.display()))
        })?,
    };
    println!("{help}");
    Ok(0)
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

fn help_sections(parsed: &TaskFile) -> Vec<String> {
    let mut sections = Vec::new();
    if parsed.help.is_some() {
        sections.push("help".to_string());
    }
    for task in &parsed.task_order {
        if parsed.task_help.contains_key(task) {
            sections.push(task.clone());
        }
    }
    sections
}

fn discover_task_file(dir: &Path) -> CjResult<PathBuf> {
    let default = dir.join("cjtasks");
    if default.is_file() {
        return Ok(default);
    }

    let mut extension_matches = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && has_cjtasks_extension(&path) {
            extension_matches.push(path);
        }
    }

    match extension_matches.len() {
        1 => Ok(extension_matches.remove(0)),
        0 => Err(CjError::new(format!(
            "no cjtasks or *.cjtasks taskfile found in {}",
            dir.display()
        ))),
        _ => Err(CjError::new(format!(
            "multiple *.cjtasks taskfiles found in {}; pass one explicitly",
            dir.display()
        ))),
    }
}

fn is_recognized_task_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "cjtasks")
        || has_cjtasks_extension(path)
}

fn has_cjtasks_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "cjtasks")
}

fn task_file_base_dir(task_file: &Path) -> &Path {
    task_file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}
