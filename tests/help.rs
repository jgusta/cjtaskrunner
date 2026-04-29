mod common;

use common::{assert_success, run_cj, temp_path};
use std::fs;
use std::path::Path;

fn write_help_taskfile(dir: &Path) {
    fs::write(
        dir.join("cjtasks"),
        r#"@help:
  Project help.

build:
  @desc build tasks
  @help:
    Build task group.
  dev:
    @desc build development assets
    @help:
      Build development assets.

      Produces a local debug bundle.
    true
  docs:
    @desc build documentation
    true

deploy:
  prod:
    @desc deploy production assets
    @help:
      Deploy production assets.
    true

cli:
  @desc cli command group
  @help:
    CLI help body.
  @selfhelp
  @shell printf should-not-run > selfhelp-ran.txt
  build:
    @desc build cli
    true
"#,
    )
    .expect("write cjtasks");
}

fn write_no_top_help_taskfile(dir: &Path) {
    fs::write(
        dir.join("cjtasks"),
        r#"build:
  @desc build assets
  true
"#,
    )
    .expect("write cjtasks");
}

#[test]
fn help_command_prints_nested_child_name_desc_and_help_body() {
    let dir = temp_path("nested-help-child");
    fs::create_dir_all(&dir).expect("mkdir");
    write_help_taskfile(&dir);

    let stdout = assert_success(&run_cj(&dir, &["help", "build:dev"]));
    assert!(stdout.starts_with(
        "build:dev\n  build development assets\n\nBuild development assets.\n\nProduces a local debug bundle.\n"
    ));
    assert!(!stdout.contains("\nHelp available:\n"));
    assert!(stdout.ends_with("run `cj --help` for program help\n"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn help_command_prints_parent_name_desc_body_and_nested_descs() {
    let dir = temp_path("nested-help-parent");
    fs::create_dir_all(&dir).expect("mkdir");
    write_help_taskfile(&dir);

    let stdout = assert_success(&run_cj(&dir, &["help", "build"]));
    assert!(stdout.starts_with("build\n  build tasks\n\nBuild task group.\n"));
    assert!(stdout.contains("\nTasks:\n"));
    assert!(stdout.contains("    build:dev"));
    assert!(stdout.contains("build development assets"));
    assert!(stdout.contains("    build:docs"));
    assert!(stdout.contains("build documentation"));
    assert!(stdout.contains("+ commands with a + have help"));
    assert!(stdout.contains("run `cj help <task>` to view it"));
    assert!(!stdout.contains("\nHelp available:\n"));
    assert!(stdout.ends_with("run `cj --help` for program help\n"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn task_arguments_appear_in_summary_and_task_help() {
    let dir = temp_path("task-argument-help");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("cjtasks"),
        "deploy (TARGET, TAG):\n  @desc deploy a release\n  @help:\n    Deploy help.\n  true\n",
    )
    .expect("write taskfile");

    let summary = assert_success(&run_cj(&dir, &[]));
    assert!(summary.contains("+  deploy ($TARGET, $TAG)"));
    assert!(summary.contains("deploy a release"));

    let help = assert_success(&run_cj(&dir, &["help", "deploy"]));
    assert!(help.starts_with("deploy ($TARGET, $TAG)\n"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn help_marker_precedes_nested_task_indentation() {
    let dir = temp_path("help-marker-indentation");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("cjtasks"),
        "plain:\n  true\nbuild:\n  child:\n    @help:\n      Child help.\n    true\n",
    )
    .expect("write taskfile");

    let summary = assert_success(&run_cj(&dir, &[]));
    assert!(summary.contains("\n   plain\n"));
    assert!(summary.contains("\n   build\n"));
    assert!(summary.contains("\n+    build:child\n"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn task_listing_aligns_descriptions_after_widest_visible_task() {
    let dir = temp_path("aligned-summary-descriptions");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("cjtasks"),
        r#"a:
  @desc short task
  true

longer-task (TARGET):
  @desc longer task
  @help:
    More help.
  true
"#,
    )
    .expect("write taskfile");

    let summary = assert_success(&run_cj(&dir, &[]));
    let short = summary
        .lines()
        .find(|line| line.contains("short task"))
        .expect("short description row");
    let long = summary
        .lines()
        .find(|line| line.contains("longer task"))
        .expect("long description row");
    assert_eq!(
        short.find("short task"),
        long.find("longer task"),
        "summary descriptions should share one column\n{summary}"
    );
    assert!(
        long.contains("longer-task ($TARGET)  longer task"),
        "summary descriptions should start at least two spaces after the widest task label\n{summary}"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn help_command_prints_top_help_before_task_listing_and_sections() {
    let dir = temp_path("top-help-listing");
    fs::create_dir_all(&dir).expect("mkdir");
    write_help_taskfile(&dir);

    let stdout = assert_success(&run_cj(&dir, &["help"]));
    let help_index = stdout.find("Project help.").expect("top help");
    let tasks_index = stdout.find("Tasks in ").expect("task listing");
    assert!(help_index < tasks_index);
    assert!(stdout.contains("build"));
    assert!(stdout.contains("build tasks"));
    assert!(stdout.contains("build:dev"));
    assert!(stdout.contains("build development assets"));
    assert!(stdout.contains("build:docs"));
    assert!(stdout.contains("build documentation"));
    assert!(stdout.contains("deploy:prod"));
    assert!(stdout.contains("deploy production assets"));
    assert!(stdout.contains("+ commands with a + have help"));
    assert!(stdout.contains("run `cj help <task>` to view it"));
    assert!(!stdout.contains("Help available:"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn help_command_lists_tasks_without_top_help() {
    let dir = temp_path("help-without-top-help");
    fs::create_dir_all(&dir).expect("mkdir");
    write_no_top_help_taskfile(&dir);

    let stdout = assert_success(&run_cj(&dir, &["help"]));
    assert!(stdout.starts_with("Tasks in "));
    assert!(stdout.contains("  build"));
    assert!(stdout.contains("build assets"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn no_args_lists_tasks_without_top_help() {
    let dir = temp_path("nested-help-listing");
    fs::create_dir_all(&dir).expect("mkdir");
    write_help_taskfile(&dir);

    let stdout = assert_success(&run_cj(&dir, &[]));
    assert!(stdout.starts_with("Tasks in cjtasks:"));
    assert!(!stdout.contains("Project help."));
    assert!(stdout.contains("build:dev"));
    assert!(stdout.contains("build development assets"));
    assert!(stdout.contains("  build"));
    assert!(stdout.contains("    build:dev"));
    assert!(stdout.contains("+ commands with a + have help"));
    assert!(stdout.contains("run `cj help <task>` to view it"));
    assert!(!stdout.contains("Help available:"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn task_listing_uses_taskfile_path_relative_to_invocation_directory() {
    let dir = temp_path("relative-taskfile-heading");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("local.cjtasks"), "build:\n  true\n").expect("write taskfile");

    let stdout = assert_success(&run_cj(&dir, &[]));
    assert!(stdout.starts_with("Tasks in local.cjtasks:"));
    assert!(!stdout.contains(&dir.display().to_string()));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn no_args_summary_hides_underscore_tasks() {
    let dir = temp_path("hidden-summary-tasks");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("cjtasks"),
        r#"build:
  @desc visible build
  true

_internal:
  @desc hidden helper
  @help:
    Internal helper help.
  true
"#,
    )
    .expect("write cjtasks");

    let stdout = assert_success(&run_cj(&dir, &[]));
    assert!(stdout.contains("build"));
    assert!(stdout.contains("visible build"));
    assert!(!stdout.contains("_internal"));
    assert!(!stdout.contains("hidden helper"));

    let help = assert_success(&run_cj(&dir, &["help", "_internal"]));
    assert!(help.contains("_internal"));
    assert!(help.contains("Internal helper help."));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn selfhelp_prints_task_help_without_cli_hint_and_stops_execution() {
    let dir = temp_path("selfhelp");
    fs::create_dir_all(&dir).expect("mkdir");
    write_help_taskfile(&dir);

    let help_stdout = assert_success(&run_cj(&dir, &["help", "cli"]));
    let selfhelp_stdout = assert_success(&run_cj(&dir, &["cli"]));
    assert_eq!(
        selfhelp_stdout.trim_end(),
        help_stdout
            .strip_suffix("\nrun `cj --help` for program help\n")
            .expect("CLI help hint")
            .trim_end()
    );
    assert!(!selfhelp_stdout.contains("cj --help"));
    assert!(!dir.join("selfhelp-ran.txt").exists());

    fs::remove_dir_all(dir).expect("cleanup");
}
