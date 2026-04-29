use super::*;
use crate::cli::{discover_task_file, resolve_invocation_from, run_cli_from_cwd, Invocation};
use crate::environment::apply_python_venv;
use crate::formatter::format_taskfile_source;
use crate::runner::{run_task, MAX_EXECUTION_STEPS};
use crate::runtime::{CwdState, RuntimeEnv};
use crate::task_file::{parse_task_file, TaskFile};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
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

fn run_task_from_dir(
    dir: &Path,
    parsed: &TaskFile,
    task_name: &str,
    env: &mut RuntimeEnv,
) -> CjResult<i32> {
    let mut cwd = CwdState::new(dir);
    run_task(parsed, task_name, &[], env, &mut cwd, &mut Vec::new())
}

fn run_task_with_arguments_from_dir(
    dir: &Path,
    parsed: &TaskFile,
    task_name: &str,
    arguments: &[String],
    env: &mut RuntimeEnv,
) -> CjResult<i32> {
    let mut cwd = CwdState::new(dir);
    run_task(parsed, task_name, arguments, env, &mut cwd, &mut Vec::new())
}

mod command_text;
mod execution;
mod invocation;
mod parsing;
mod tooling;
mod versions_and_files;
