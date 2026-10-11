mod common;

use common::{assert_failure, assert_success, run_cj, temp_path};
use std::fs;
use std::process::Command;

#[test]
fn run_flag_executes_one_line_task_without_taskfile() {
    let dir = temp_path("run-line");
    fs::create_dir_all(&dir).expect("mkdir");

    assert_success(&run_cj(
        &dir,
        &["--run", "@shell printf run-line > out.txt"],
    ));
    assert_eq!(
        fs::read_to_string(dir.join("out.txt")).expect("out"),
        "run-line"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn run_flag_rejects_labels_and_block_only_directives() {
    let dir = temp_path("run-line-reject");
    fs::create_dir_all(&dir).expect("mkdir");
    let label = assert_failure(&run_cj(&dir, &["--run", "build:"]));
    assert!(label.contains("--run does not accept task labels"));

    let nested = assert_failure(&run_cj(&dir, &["--run", "@if true"]));
    assert!(nested.contains("--run does not accept block directives"));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn edit_flag_opens_detected_taskfile_with_editor() {
    let dir = temp_path("edit-taskfile");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("cjtasks"), "build:\n  @success\n").expect("write cjtasks");
    fs::write(dir.join("local.cjtasks"), "local:\n  @success\n").expect("write local");

    let output = Command::new(env!("CARGO_BIN_EXE_cj"))
        .arg("-e")
        .current_dir(&dir)
        .env("EDITOR", "printf %s")
        .env("NO_COLOR", "1")
        .output()
        .expect("run cj -e");

    let stdout = assert_success(&output);
    assert_eq!(
        fs::canonicalize(stdout).expect("opened path"),
        fs::canonicalize(dir.join("cjtasks")).expect("expected path")
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn edit_flag_requires_editor() {
    let dir = temp_path("edit-taskfile-no-editor");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("cjtasks"), "build:\n  @success\n").expect("write cjtasks");

    let output = Command::new(env!("CARGO_BIN_EXE_cj"))
        .arg("-e")
        .current_dir(&dir)
        .env_remove("EDITOR")
        .env("NO_COLOR", "1")
        .output()
        .expect("run cj -e");

    let stderr = assert_failure(&output);
    assert!(stderr.contains("EDITOR is not set"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn version_flag_works_without_a_taskfile_and_rejects_arguments() {
    let dir = temp_path("version-flag");
    fs::create_dir_all(&dir).expect("mkdir");
    let output = run_cj(&dir, &["--version"]);
    assert_eq!(
        assert_success(&output),
        format!("cj {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(assert_failure(&run_cj(&dir, &["--version", "extra"])).contains("usage: cj --version"));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn removed_default_flags_do_not_run_a_task() {
    let dir = temp_path("removed-default-flags");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("cjtasks"), "default:\n  @success\n").expect("taskfile");
    for flag in ["--default", "-d"] {
        assert_failure(&run_cj(&dir, &[flag]));
    }
    assert_success(&run_cj(&dir, &["default"]));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn lsp_flag_rejects_arguments_and_lsp_is_an_ordinary_task_name() {
    let dir = temp_path("lsp-flag");
    fs::create_dir_all(&dir).expect("mkdir");
    assert!(assert_failure(&run_cj(&dir, &["--lsp", "extra"])).contains("usage: cj --lsp"));
    fs::write(dir.join("cjtasks"), "lsp:\n  @echo ordinary-task\n").expect("taskfile");
    assert!(assert_success(&run_cj(&dir, &["lsp"])).contains("ordinary-task"));
    fs::remove_dir_all(dir).expect("cleanup");
}
