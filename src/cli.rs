pub fn run_cli(args: &[String]) -> CjResult<i32> {
    run_cli_from_cwd(args, &env::current_dir()?)
}

fn run_cli_from_cwd(args: &[String], cwd: &Path) -> CjResult<i32> {
    if args.first().is_some_and(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(0);
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

No task name lists tasks in the detected taskfile.
Use --default or -d to run the default task.
Use cj help to show taskfile help.
"
    );
}

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
