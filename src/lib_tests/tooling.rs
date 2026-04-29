use super::*;

#[test]
fn execution_step_limit_detects_possible_infinite_loop() {
    let dir = test_path("step-limit");
    fs::create_dir_all(&dir).expect("mkdir");
    let parsed = parse_task_file("run:\n  true\n", Path::new("cjtasks")).expect("parse");
    let mut env = minimal_env();
    env.steps = MAX_EXECUTION_STEPS;

    let err =
        run_task_from_dir(&dir, &parsed, "run", &mut env).expect_err("step limit should fail");
    assert!(err.to_string().contains("possible infinite loop"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn formats_taskfile_source_without_ast() {
    let source =
        "@env:\t\n\tNAME: value\t\ndeploy ( TARGET ,TAG ):\n   @desc build\t\n\tcargo build\t\n\n";
    let formatted = format_taskfile_source(source);
    assert_eq!(
        formatted,
        "@env:\n  NAME: value\ndeploy (TARGET, TAG):\n  @desc build\n  cargo build\n\n"
    );
}

#[test]
fn format_flag_formats_discovered_taskfile() {
    let dir = test_path("format-flag");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("cjtasks"), "run:\t\n\ttrue\t\n").expect("write cjtasks");

    let code = run_cli_from_cwd(&["--format".to_string()], &dir).expect("format");
    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(dir.join("cjtasks")).expect("read"),
        "run:\n  true\n"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn completions_command_prints_supported_shells() {
    let code = run_cli_from_cwd(
        &["--completions".to_string(), "fish".to_string()],
        Path::new("."),
    )
    .expect("completions");
    assert_eq!(code, 0);

    let err = run_cli_from_cwd(
        &["--completions".to_string(), "powershell".to_string()],
        Path::new("."),
    )
    .expect_err("unsupported shell");
    assert!(err.to_string().contains("unsupported shell"));
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
