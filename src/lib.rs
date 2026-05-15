use std::cell::RefCell;
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

thread_local! {
    static CAPTURED_OUTPUT: RefCell<String> = const { RefCell::new(String::new()) };
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Inherit,
    Capture,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CommandResult {
    status: i32,
    output: String,
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

include!("cli.rs");
include!("task_file.rs");
include!("environment.rs");
include!("runner.rs");
include!("directives.rs");
include!("command_text.rs");

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
        env::temp_dir().join(format!("cjtaskrunner-{name}-{id}"))
    }

    fn minimal_env() -> RuntimeEnv {
        RuntimeEnv::new(HashMap::from([(
            "PATH".to_string(),
            env::var("PATH").unwrap_or_default(),
        )]))
    }

    #[test]
    fn parses_env_and_tasks() {
        let path = Path::new("cjtasks");
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
            Path::new("cjtasks"),
        )
        .expect_err("duplicate env should fail");

        assert!(err.to_string().contains("duplicate env entry 'NAME'"));
    }

    #[test]
    fn rejects_bad_indentation() {
        let err = parse_task_file("run:\n   echo hi\n", Path::new("cjtasks"))
            .expect_err("bad indentation should fail");

        assert!(err.to_string().contains("even number of spaces"));
    }

    #[test]
    fn rejects_trailing_colon_directives() {
        let err = parse_task_file("run:\n  @if true:\n    echo hi\n", Path::new("cjtasks"))
            .expect_err("directive colon should fail");

        assert!(err
            .to_string()
            .contains("directives do not use trailing ':'"));
    }

    #[test]
    fn discovers_default_cjtasks() {
        let dir = test_path("discover");
        fs::create_dir_all(&dir).expect("mkdir");
        File::create(dir.join("build.cjtasks")).expect("build.cjtasks");
        File::create(dir.join("cjtasks")).expect("cjtasks");

        let discovered = discover_task_file(&dir).expect("discover");
        assert_eq!(discovered.file_name().unwrap(), "cjtasks");

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn discovers_single_cjtasks_extension_and_rejects_ambiguous_extensions() {
        let dir = test_path("extension-discover");
        fs::create_dir_all(&dir).expect("mkdir");
        File::create(dir.join("build.cjtasks")).expect("build.cjtasks");

        let discovered = discover_task_file(&dir).expect("discover extension");
        assert_eq!(discovered.file_name().unwrap(), "build.cjtasks");

        File::create(dir.join("deploy.cjtasks")).expect("deploy.cjtasks");
        let err = discover_task_file(&dir).expect_err("ambiguous extensions should fail");
        assert!(err.to_string().contains("multiple *.cjtasks"));

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
            Path::new("cjtasks"),
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
            Path::new("cjtasks"),
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
            Path::new("cjtasks"),
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
            Path::new("cjtasks"),
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
            Path::new("cjtasks"),
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
    fn semicolons_split_same_level_expressions() {
        let parsed = parse_task_file(
            "run:\n  @set MODE prod; @if $MODE == prod\n    true\n",
            Path::new("cjtasks"),
        )
        .expect("parse");

        let lines = &parsed.tasks["run"];
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].text, "@set MODE prod");
        assert_eq!(lines[1].text, "@if $MODE == prod");
        assert_eq!(lines[2].text, "true");
    }

    #[test]
    fn echo_clean_stop_and_status_chains() {
        let dir = test_path("fish-controls");
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("stale.txt"), "stale").expect("stale");
        let parsed = parse_task_file(
            r#"run:
  false; @or
    @echo recovered
  @and
    @clean stale.txt
  @if-missing stale.txt
    @success
"#,
            Path::new("cjtasks"),
        )
        .expect("parse");
        let mut env = minimal_env();

        let code = run_task(&dir, &parsed, "run", &mut env, &mut Vec::new()).expect("run");
        assert_eq!(code, 0);
        assert!(!dir.join("stale.txt").exists());

        let parsed =
            parse_task_file("run:\n  @stop nope\n  true\n", Path::new("cjtasks")).expect("parse");
        let code = run_task(&dir, &parsed, "run", &mut env, &mut Vec::new()).expect("run");
        assert_eq!(code, 1);

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn set_block_captures_stdout() {
        let dir = test_path("set-capture");
        fs::create_dir_all(&dir).expect("mkdir");
        let parsed = parse_task_file(
            r#"run:
  @set RESULT:
    @shell printf captured
  @shell test "$RESULT" = captured
"#,
            Path::new("cjtasks"),
        )
        .expect("parse");
        let mut env = minimal_env();

        let code = run_task(&dir, &parsed, "run", &mut env, &mut Vec::new()).expect("run");
        assert_eq!(code, 0);
        assert_eq!(env.vars["RESULT"], "captured");

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn resolves_single_arg_from_current_directory() {
        let dir = test_path("single-arg");
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("cjtasks"), "run:\n  true\n").expect("write cjtasks");

        let resolved = resolve_invocation_from(&["run".to_string()], &dir).expect("resolve");
        assert_eq!(resolved.0.file_name().unwrap(), "cjtasks");
        assert_eq!(resolved.1, "run");

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn resolves_two_arg_directory_and_direct_file() {
        let dir = test_path("two-arg");
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("cjtasks"), "run:\n  true\n").expect("write cjtasks");
        fs::write(dir.join("build.cjtasks"), "run:\n  true\n").expect("write build.cjtasks");

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

        let from_extension = resolve_invocation_from(
            &[
                dir.join("build.cjtasks").to_string_lossy().to_string(),
                "run".to_string(),
            ],
            &dir,
        )
        .expect("resolve extension file");
        assert_eq!(from_extension.0, dir.join("build.cjtasks"));

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn rejects_unrecognized_taskfile_name() {
        let dir = test_path("unrecognized-taskfile");
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("tasks"), "run:\n  true\n").expect("write tasks");

        let err = resolve_invocation_from(
            &[
                dir.join("tasks").to_string_lossy().to_string(),
                "run".to_string(),
            ],
            &dir,
        )
        .expect_err("unrecognized taskfile name should fail");
        assert!(err.to_string().contains("'.cjtasks' extension"));

        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn bare_relative_task_file_runs_from_current_directory() {
        let dir = test_path("relative-file");
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("cjtasks"), "run:\n  @shell pwd > out.txt\n").expect("write cjtasks");

        let code =
            run_cli_from_cwd(&["cjtasks".to_string(), "run".to_string()], &dir).expect("run");
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
