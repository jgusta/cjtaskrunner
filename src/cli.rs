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
