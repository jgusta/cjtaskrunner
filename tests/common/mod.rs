#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn temp_path(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "cjtaskrunner-{name}-{}-{timestamp}-{sequence}",
        std::process::id()
    ))
}

pub fn run_cj(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cj"))
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .output()
        .expect("run cj")
}

pub fn run_cj_with_env(dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cj"));
    command
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1");
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("run cj")
}

pub fn assert_success(output: &Output) -> String {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("stdout utf8")
}

pub fn assert_failure(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stderr.clone()).expect("stderr utf8")
}

pub fn assert_failure_contains(output: &Output, message: &str) {
    let stderr = assert_failure(output);
    assert!(
        stderr.contains(message),
        "expected stderr to contain {message:?}\nstderr:\n{stderr}"
    );
}
