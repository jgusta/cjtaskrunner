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
    tasks: HashMap<String, Vec<String>>,
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

pub fn run_cli(args: &[String]) -> CjResult<i32> {
    run_cli_from_cwd(args, &env::current_dir()?)
}

fn run_cli_from_cwd(args: &[String], cwd: &Path) -> CjResult<i32> {
    let (task_file, task_name) = resolve_invocation_from(args, cwd)?;
    let base_dir = task_file_base_dir(&task_file);
    let parsed = parse_task_file_path(&task_file)?;
    let commands = parsed
        .tasks
        .get(&task_name)
        .ok_or_else(|| CjError::new(format!("task not found: {task_name}")))?;
    let env = build_effective_env(base_dir, &parsed.env)?;

    run_commands(base_dir, commands, &env)
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
    let mut tasks: HashMap<String, Vec<String>> = HashMap::new();
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

        if !line.starts_with("  ") || line.starts_with("   ") {
            return Err(line_error(
                path,
                line_number,
                "indented entries must use exactly two leading spaces",
            ));
        }

        match section {
            Section::Env => parse_env_entry(&line[2..], &mut env, path, line_number)?,
            Section::Task => {
                let task_name = current_task
                    .as_ref()
                    .ok_or_else(|| line_error(path, line_number, "command without a task"))?;
                if line[2..].is_empty() {
                    continue;
                }
                tasks
                    .get_mut(task_name)
                    .expect("current task must exist")
                    .push(line[2..].to_string());
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
    if name.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        Ok(())
    } else {
        Err("task names must contain only ASCII letters and digits")
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

fn run_commands(
    base_dir: &Path,
    commands: &[String],
    effective_env: &HashMap<String, String>,
) -> CjResult<i32> {
    for command in commands {
        let status = Command::new("/bin/sh")
            .arg("-c")
            .arg(command)
            .current_dir(base_dir)
            .env_clear()
            .envs(effective_env)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|err| CjError::new(format!("failed to run command '{command}': {err}")))?;

        if !status.success() {
            return Ok(status.code().unwrap_or(1));
        }
    }

    Ok(0)
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
        assert_eq!(parsed.tasks["dev"], vec!["echo # retained"]);
        assert_eq!(parsed.tasks["test123"], vec!["cargo test"]);
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

        assert!(err.to_string().contains("exactly two leading spaces"));
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
    fn commands_are_independent_and_run_from_base_dir() {
        let dir = test_path("run");
        fs::create_dir_all(&dir).expect("mkdir");
        let commands = vec![
            "cd /".to_string(),
            "printf '%s' \"$PWD\" > pwd.txt".to_string(),
        ];
        let env = HashMap::from([("PATH".to_string(), env::var("PATH").unwrap_or_default())]);

        let code = run_commands(&dir, &commands, &env).expect("run");
        assert_eq!(code, 0);
        let reported = fs::read_to_string(dir.join("pwd.txt")).expect("pwd");
        assert_eq!(
            fs::canonicalize(reported).expect("reported pwd"),
            fs::canonicalize(&dir).expect("dir")
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
        fs::write(dir.join("cjt"), "run:\n  printf '%s' \"$PWD\" > out.txt\n").expect("write cjt");

        let code = run_cli_from_cwd(&["cjt".to_string(), "run".to_string()], &dir).expect("run");
        assert_eq!(code, 0);
        let reported = fs::read_to_string(dir.join("out.txt")).expect("out");
        assert_eq!(
            fs::canonicalize(reported).expect("reported pwd"),
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
