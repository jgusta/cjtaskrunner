use super::*;

#[test]
fn resolves_single_arg_from_current_directory() {
    let dir = test_path("single-arg");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("cjtasks"), "run:\n  true\n").expect("write cjtasks");

    let resolved = resolve_invocation_from(&["run".to_string()], &dir).expect("resolve");
    assert_eq!(
        resolved,
        Invocation::Run {
            task_file: dir.join("cjtasks"),
            task_name: "run".to_string(),
            arguments: Vec::new(),
        }
    );

    let listed = resolve_invocation_from(&[], &dir).expect("resolve list");
    assert_eq!(
        listed,
        Invocation::List {
            task_file: dir.join("cjtasks")
        }
    );

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
    assert_eq!(
        from_dir,
        Invocation::Run {
            task_file: dir.join("cjtasks"),
            task_name: "run".to_string(),
            arguments: Vec::new(),
        }
    );

    let from_file = resolve_invocation_from(
        &[
            dir.join("cjtasks").to_string_lossy().to_string(),
            "run".to_string(),
        ],
        &dir,
    )
    .expect("resolve file");
    assert_eq!(
        from_file,
        Invocation::Run {
            task_file: dir.join("cjtasks"),
            task_name: "run".to_string(),
            arguments: Vec::new(),
        }
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn resolves_task_arguments_with_and_without_explicit_location() {
    let dir = test_path("task-arguments-invocation");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("cjtasks"), "deploy (TARGET, TAG):\n  true\n").expect("write cjtasks");

    let local = resolve_invocation_from(
        &[
            "deploy".to_string(),
            "production".to_string(),
            "v1.2.3".to_string(),
        ],
        &dir,
    )
    .expect("resolve local task arguments");
    assert_eq!(
        local,
        Invocation::Run {
            task_file: dir.join("cjtasks"),
            task_name: "deploy".to_string(),
            arguments: vec!["production".to_string(), "v1.2.3".to_string()],
        }
    );

    let explicit = resolve_invocation_from(
        &[
            dir.to_string_lossy().to_string(),
            "deploy".to_string(),
            "staging".to_string(),
            "v2.0.0".to_string(),
        ],
        &dir,
    )
    .expect("resolve explicit task arguments");
    assert_eq!(
        explicit,
        Invocation::Run {
            task_file: dir.join("cjtasks"),
            task_name: "deploy".to_string(),
            arguments: vec!["staging".to_string(), "v2.0.0".to_string()],
        }
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn task_arguments_bind_locally_and_restore_previous_values() {
    let dir = test_path("task-argument-bindings");
    fs::create_dir_all(&dir).expect("mkdir");
    let parsed = parse_task_file(
        "show (VALUE):\n  @shell printf '%s' $VALUE > value.txt\n",
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();
    env.vars.insert("VALUE".to_string(), "parent".to_string());

    let code = run_task_with_arguments_from_dir(
        &dir,
        &parsed,
        "show",
        &["child value".to_string()],
        &mut env,
    )
    .expect("run");

    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(dir.join("value.txt")).expect("value output"),
        "child value"
    );
    assert_eq!(env.vars["VALUE"], "parent");
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn task_argument_bindings_restore_after_failure_and_error() {
    let dir = test_path("task-argument-restore");
    fs::create_dir_all(&dir).expect("mkdir");
    let parsed = parse_task_file(
        "fails (VALUE):\n  @fail\nerrors (VALUE):\n  @unknown\n",
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();
    env.vars.insert("VALUE".to_string(), "parent".to_string());

    let code =
        run_task_with_arguments_from_dir(&dir, &parsed, "fails", &["child".to_string()], &mut env)
            .expect("run failure");
    assert_eq!(code, 1);
    assert_eq!(env.vars["VALUE"], "parent");

    let err =
        run_task_with_arguments_from_dir(&dir, &parsed, "errors", &["child".to_string()], &mut env)
            .expect_err("run error");
    assert!(err.to_string().contains("unknown directive"));
    assert_eq!(env.vars["VALUE"], "parent");

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn task_argument_arity_is_validated_before_execution() {
    let dir = test_path("task-argument-arity");
    fs::create_dir_all(&dir).expect("mkdir");
    let parsed = parse_task_file(
        "plain:\n  true\nplain:child:\n  true\ndeploy (TARGET, TAG):\n  true\n",
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();

    let missing =
        run_task_with_arguments_from_dir(&dir, &parsed, "deploy", &["prod".to_string()], &mut env)
            .expect_err("missing argument should fail");
    assert!(missing
        .to_string()
        .contains("task 'deploy' expects 2 arguments, received 1"));

    let extra =
        run_task_with_arguments_from_dir(&dir, &parsed, "plain", &["child".to_string()], &mut env)
            .expect_err("unexpected argument should fail");
    assert!(extra
        .to_string()
        .contains("task 'plain' expects 0 arguments, received 1"));
    assert!(extra.to_string().contains("Did you mean `plain:child`?"));

    let unrelated =
        run_task_with_arguments_from_dir(&dir, &parsed, "plain", &["other".to_string()], &mut env)
            .expect_err("unrelated argument should fail");
    assert!(!unrelated.to_string().contains("Did you mean"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn task_directive_forwards_interpolated_arguments() {
    let dir = test_path("task-directive-arguments");
    fs::create_dir_all(&dir).expect("mkdir");
    let parsed = parse_task_file(
        r#"parent:
  @set DESTINATION production
  @task child $DESTINATION "release tag"

child (TARGET, TAG):
  @shell printf '%s|%s' $TARGET $TAG > forwarded.txt
"#,
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();

    let code = run_task_from_dir(&dir, &parsed, "parent", &mut env).expect("run parent");

    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(dir.join("forwarded.txt")).expect("forwarded output"),
        "production|release tag"
    );
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn rejects_parameterized_tasks_in_await() {
    let err = parse_task_file(
        "dev:\n  @await server\nserver (PORT):\n  @echo $PORT\n",
        Path::new("cjtasks"),
    )
    .expect_err("parameterized awaited task should fail");

    assert!(err
        .to_string()
        .contains("awaited task 'server' requires arguments"));
}

#[test]
fn rejects_unrecognized_taskfile_name() {
    let dir = test_path("unrecognized-taskfile");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("unknown.cjtasks"), "run:\n  true\n").expect("write unknown.cjtasks");
    fs::write(dir.join("other-tasks"), "run:\n  true\n").expect("write other-tasks");

    resolve_invocation_from(
        &[
            dir.join("unknown.cjtasks").to_string_lossy().to_string(),
            "run".to_string(),
        ],
        &dir,
    )
    .expect_err("unrecognized taskfile name should fail");

    resolve_invocation_from(
        &[
            dir.join("other-tasks").to_string_lossy().to_string(),
            "run".to_string(),
        ],
        &dir,
    )
    .expect_err("unrecognized taskfile name should fail");

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn bare_relative_task_file_runs_from_current_directory() {
    let dir = test_path("relative-file");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("cjtasks"), "run:\n  @shell pwd > out.txt\n").expect("write cjtasks");

    let code = run_cli_from_cwd(&["cjtasks".to_string(), "run".to_string()], &dir).expect("run");
    assert_eq!(code, 0);
    let reported = fs::read_to_string(dir.join("out.txt")).expect("out");
    assert_eq!(
        fs::canonicalize(reported.trim()).expect("reported pwd"),
        fs::canonicalize(&dir).expect("dir")
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn no_args_lists_discovered_tasks() {
    let dir = test_path("list-tasks");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("cjtasks"),
        "@help:\n  Root help.\nbuild:\n  @desc compile project\n  @help:\n    Build help.\n  true\n",
    )
    .expect("write cjtasks");

    let code = run_cli_from_cwd(&[], &dir).expect("list");
    assert_eq!(code, 0);

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn help_command_prints_top_level_and_task_help() {
    let dir = test_path("help-command");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("cjtasks"),
        "@help:\n  Root help.\nbuild:dev:\n  @help:\n    Build dev help.\n  true\n",
    )
    .expect("write cjtasks");

    assert_eq!(
        run_cli_from_cwd(&["help".to_string()], &dir).expect("top help"),
        0
    );
    assert_eq!(
        run_cli_from_cwd(&["help".to_string(), "build:dev".to_string()], &dir).expect("task help"),
        0
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn rejects_help_without_colon() {
    let err = parse_task_file("run:\n  @help\n    nope\n", Path::new("cjtasks"))
        .expect_err("@help must use colon");
    assert!(err.to_string().contains("@help must use trailing ':'"));
}

#[test]
fn default_flag_runs_default_task() {
    let dir = test_path("default-flag");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("cjtasks"),
        "default:\n  @shell printf ok > default.txt\n",
    )
    .expect("write cjtasks");

    let code = run_cli_from_cwd(&["--default".to_string()], &dir).expect("default");
    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(dir.join("default.txt")).expect("read"),
        "ok"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}
