use super::*;

#[test]
fn discovers_default_cjtasks() {
    let dir = test_path("discover");
    fs::create_dir_all(&dir).expect("mkdir");
    File::create(dir.join("local.cjtasks")).expect("local.cjtasks");
    File::create(dir.join("cjtasks")).expect("cjtasks");

    let discovered = discover_task_file(&dir).expect("discover");
    assert_eq!(discovered.file_name().unwrap(), "cjtasks");

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn discovers_highest_precedence_overlay_when_cjtasks_is_absent() {
    let dir = test_path("overlay-discover");
    fs::create_dir_all(&dir).expect("mkdir");
    File::create(dir.join("ignored.cjtasks")).expect("ignored.cjtasks");
    File::create(dir.join("production.cjtasks")).expect("production.cjtasks");
    File::create(dir.join("development.cjtasks")).expect("development.cjtasks");
    File::create(dir.join("local.cjtasks")).expect("local.cjtasks");

    let discovered = discover_task_file(&dir).expect("discover overlay");
    assert_eq!(discovered.file_name().unwrap(), "local.cjtasks");

    fs::remove_file(dir.join("local.cjtasks")).expect("remove local.cjtasks");
    fs::remove_file(dir.join("development.cjtasks")).expect("remove development.cjtasks");
    fs::remove_file(dir.join("production.cjtasks")).expect("remove production.cjtasks");
    let err = discover_task_file(&dir).expect_err("ignore arbitrary .cjtasks files");
    assert!(err.to_string().contains("no recognized taskfile found"));

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
fn open_directive_requires_exactly_one_url() {
    let dir = test_path("open-args");
    fs::create_dir_all(&dir).expect("mkdir");
    let missing = parse_task_file("run:\n  @open\n", Path::new("cjtasks")).expect("parse");
    let mut env = minimal_env();
    let err = run_task_from_dir(&dir, &missing, "run", &mut env)
        .expect_err("@open missing url should fail");
    assert!(err.to_string().contains("@open expects exactly one URL"));

    let extra = parse_task_file(
        "run:\n  @open https://example.com extra\n",
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();
    let err =
        run_task_from_dir(&dir, &extra, "run", &mut env).expect_err("@open extra args should fail");
    assert!(err.to_string().contains("@open expects exactly one URL"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn open_directive_only_accepts_http_urls() {
    let dir = test_path("open-url-scheme");
    fs::create_dir_all(&dir).expect("mkdir");
    let parsed = parse_task_file(
        "run:\n  @open file:///tmp/report.html\n",
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();

    let err = run_task_from_dir(&dir, &parsed, "run", &mut env)
        .expect_err("@open non-http url should fail");

    assert!(err
        .to_string()
        .contains("@open URL must begin with http:// or https://"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn task_calls_snapshot_runtime_state_and_resume_callers() {
    let parsed = parse_task_file(
        r#"run:
  @set ORDER root
  @if true
    @set ORDER $ORDER-if
    @task child
    @set ORDER $ORDER-after-child
  @set ORDER $ORDER-after-if
  @shell printf "$ORDER" > order.txt
child:
  @set ORDER $ORDER-child
  @if true
    @task grandchild
  @set ORDER $ORDER-child-after
grandchild:
  @set ORDER $ORDER-grandchild
"#,
        Path::new("cjtasks"),
    )
    .expect("parse");
    let dir = test_path("nested-task-state");
    fs::create_dir_all(&dir).expect("mkdir");
    let mut env = minimal_env();

    let code = run_task_from_dir(&dir, &parsed, "run", &mut env).expect("run");
    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(dir.join("order.txt")).expect("order"),
        "root-if-after-child-after-if"
    );
    assert!(!env.vars.contains_key("ORDER"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn task_exports_update_calling_context() {
    let parsed = parse_task_file(
        r#"run:
  @set LOCAL parent
  @task child
  @shell printf "$LOCAL:$SHARED" > state.txt
child:
  @set LOCAL child
  @export SHARED child
"#,
        Path::new("cjtasks"),
    )
    .expect("parse");
    let dir = test_path("task-export-state");
    fs::create_dir_all(&dir).expect("mkdir");
    let mut env = minimal_env();

    let code = run_task_from_dir(&dir, &parsed, "run", &mut env).expect("run");
    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(dir.join("state.txt")).expect("state"),
        "parent:child"
    );
    assert_eq!(env.exported_values()["SHARED"], "child");

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn nested_task_cycle_reports_inner_call_chain() {
    let parsed = parse_task_file(
        r#"root:
  @task alpha
alpha:
  @if true
    @task beta
beta:
  @task gamma
gamma:
  @task alpha
"#,
        Path::new("cjtasks"),
    )
    .expect("parse");
    let dir = test_path("nested-task-cycle");
    fs::create_dir_all(&dir).expect("mkdir");
    let mut env = minimal_env();

    let err =
        run_task_from_dir(&dir, &parsed, "root", &mut env).expect_err("nested cycle should fail");
    assert!(err
        .to_string()
        .contains("recursive @task cycle detected: alpha -> beta -> gamma -> alpha"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn shared_await_tasks_run_once() {
    let dir = test_path("await-tasks-dedupe");
    fs::create_dir_all(&dir).expect("mkdir");
    let parsed = parse_task_file(
        r#"root:
  @await left right
left:
  @await shared
right:
  @await shared
shared:
  @shell printf x >> count.txt
"#,
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();

    let code = run_task_from_dir(&dir, &parsed, "root", &mut env).expect("run");

    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(dir.join("count.txt")).expect("count"),
        "x"
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

    let code = run_task_from_dir(&dir, &parsed, "run", &mut env).expect("run");
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
  @if-not-exists stale.txt
    @success
"#,
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();

    let code = run_task_from_dir(&dir, &parsed, "run", &mut env).expect("run");
    assert_eq!(code, 0);
    assert!(!dir.join("stale.txt").exists());

    let parsed =
        parse_task_file("run:\n  @stop nope\n  true\n", Path::new("cjtasks")).expect("parse");
    let code = run_task_from_dir(&dir, &parsed, "run", &mut env).expect("run");
    assert_eq!(code, 1);

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn clean_cannot_remove_scope_base_or_parent_directories() {
    let dir = test_path("clean-scope-base");
    fs::create_dir_all(dir.join("child/grandchild")).expect("mkdir");
    fs::write(dir.join("child/grandchild/stale.txt"), "stale").expect("stale");

    let parsed = parse_task_file(
        r#"clean-child:
  @clean child/grandchild/stale.txt
clean-scope:
  @clean .
clean-parent:
  @cd child
  @if true
    @clean ..
"#,
        Path::new("cjtasks"),
    )
    .expect("parse");

    let mut env = minimal_env();
    let code = run_task_from_dir(&dir, &parsed, "clean-child", &mut env).expect("clean child");
    assert_eq!(code, 0);
    assert!(!dir.join("child/grandchild/stale.txt").exists());

    let mut env = minimal_env();
    let err = run_task_from_dir(&dir, &parsed, "clean-scope", &mut env)
        .expect_err("cleaning scope base should fail");
    assert!(err
        .to_string()
        .contains("@clean cannot remove the current scope directory or any parent directory"));
    assert!(dir.is_dir());

    let mut env = minimal_env();
    let err = run_task_from_dir(&dir, &parsed, "clean-parent", &mut env)
        .expect_err("cleaning parent should fail");
    assert!(err
        .to_string()
        .contains("@clean cannot remove the current scope directory or any parent directory"));
    assert!(dir.join("child").is_dir());

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn set_block_capture_requires_colon() {
    let dir = test_path("capture-colon");
    fs::create_dir_all(&dir).expect("mkdir");
    let parsed = parse_task_file(
        "run:\n  @set OUT\n    @echo captured\n",
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();

    let err =
        run_task_from_dir(&dir, &parsed, "run", &mut env).expect_err("missing colon should fail");
    assert!(err.to_string().contains("@set expects NAME and value"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn cd_and_back_manage_scoped_working_directories() {
    let dir = test_path("cd");
    fs::create_dir_all(dir.join("sub/child")).expect("mkdir");
    let parsed = parse_task_file(
        r#"run:
  @shell pwd > root.txt
  @cd sub
  @shell pwd > sub.txt
  @if true
    @cd child
    @shell pwd > child.txt
  @shell pwd > after-block.txt
  @back
  @shell pwd > after-back.txt
  @back
  @shell pwd > after-extra-back.txt
"#,
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();

    let code = run_task_from_dir(&dir, &parsed, "run", &mut env).expect("run");
    assert_eq!(code, 0);

    let canonical_dir = fs::canonicalize(&dir).expect("dir");
    let canonical_sub = fs::canonicalize(dir.join("sub")).expect("sub");
    let canonical_child = fs::canonicalize(dir.join("sub/child")).expect("child");

    assert_eq!(
        fs::canonicalize(
            fs::read_to_string(dir.join("root.txt"))
                .expect("root")
                .trim()
        )
        .expect("root pwd"),
        canonical_dir
    );
    assert_eq!(
        fs::canonicalize(
            fs::read_to_string(dir.join("sub/sub.txt"))
                .expect("sub")
                .trim()
        )
        .expect("sub pwd"),
        canonical_sub
    );
    assert_eq!(
        fs::canonicalize(
            fs::read_to_string(dir.join("sub/child/child.txt"))
                .expect("child")
                .trim()
        )
        .expect("child pwd"),
        canonical_child
    );
    assert_eq!(
        fs::canonicalize(
            fs::read_to_string(dir.join("sub/after-block.txt"))
                .expect("after block")
                .trim()
        )
        .expect("after block pwd"),
        canonical_sub
    );
    assert_eq!(
        fs::canonicalize(
            fs::read_to_string(dir.join("after-back.txt"))
                .expect("after back")
                .trim()
        )
        .expect("after back pwd"),
        canonical_dir
    );
    assert_eq!(
        fs::canonicalize(
            fs::read_to_string(dir.join("after-extra-back.txt"))
                .expect("after extra back")
                .trim()
        )
        .expect("after extra back pwd"),
        canonical_dir
    );

    fs::remove_dir_all(dir).expect("cleanup");
}
