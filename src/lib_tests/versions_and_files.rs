use super::*;

#[test]
fn version_bump_directives_record_single_bump_kind_for_conditionals() {
    let dir = test_path("version-bump-state");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("cjtasks"),
        r#"@version app 1.2.3
run:
  @patch app
  @if-bumped
    @shell printf bumped > bumped.txt
  @if-bumped app
    @shell printf any > any.txt
  @if-patch app
    @shell printf patch > patch.txt
  @if-not-patch app
    @shell printf not-patch > not-patch.txt
  @if-not-bumped
    @shell printf none > none.txt
  @if-minor app
    @shell printf minor > minor.txt
  @if-not-minor app
    @shell printf not-minor > not-minor.txt
"#,
    )
    .expect("write cjtasks");

    let code = run_cli_from_cwd(&["run".to_string()], &dir).expect("run");

    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(dir.join("bumped.txt")).expect("bumped"),
        "bumped"
    );
    assert_eq!(fs::read_to_string(dir.join("any.txt")).expect("any"), "any");
    assert_eq!(
        fs::read_to_string(dir.join("patch.txt")).expect("patch"),
        "patch"
    );
    assert!(
        !dir.join("not-patch.txt").exists(),
        "@if-not-patch must not run after a patch bump"
    );
    assert!(
        !dir.join("none.txt").exists(),
        "no-arg @if-not-bumped must not run after a bump"
    );
    assert!(
        !dir.join("minor.txt").exists(),
        "@if-minor must not run after a patch bump"
    );
    assert_eq!(
        fs::read_to_string(dir.join("not-minor.txt")).expect("not-minor"),
        "not-minor"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn version_bump_directive_interpolates_version_name() {
    let dir = test_path("version-bump-name-interpolated");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("cjtasks"),
        r#"@version app 1.2.3
run (NAME):
  @patch $NAME
  @if-patch $NAME
    @shell printf matched > matched.txt
"#,
    )
    .expect("write cjtasks");

    let code = run_cli_from_cwd(&["run".to_string(), "app".to_string()], &dir).expect("run");

    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(dir.join("matched.txt")).expect("matched"),
        "matched"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn negative_version_condition_inverts_version_condition() {
    let dir = test_path("version-conditionals-negative");
    fs::create_dir_all(&dir).expect("mkdir");
    let parsed = parse_task_file(
        r#"@version app 1.2.3-beta.1
run:
  @if-not-version app release
    @shell printf not-release > not-release.txt
  @if-not-version app >= 2.0.0
    @shell printf below-two > below-two.txt
  @if-not-version app prerelease
    @shell printf not-pre > not-pre.txt
"#,
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();
    env.export("VERSION_APP".to_string(), "1.2.3-beta.1".to_string());

    let code = run_task_from_dir(&dir, &parsed, "run", &mut env).expect("run");

    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(dir.join("not-release.txt")).expect("not-release"),
        "not-release"
    );
    assert_eq!(
        fs::read_to_string(dir.join("below-two.txt")).expect("below-two"),
        "below-two"
    );
    assert!(!dir.join("not-pre.txt").exists());

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn unbumped_without_arguments_matches_when_no_version_was_bumped() {
    let dir = test_path("version-no-bump-state");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("cjtasks"),
        r#"@version app 1.2.3
run:
  @if-bumped
    @shell printf bumped > bumped.txt
  @if-not-bumped
    @shell printf none > none.txt
"#,
    )
    .expect("write cjtasks");

    let code = run_cli_from_cwd(&["run".to_string()], &dir).expect("run");

    assert_eq!(code, 0);
    assert!(
        !dir.join("bumped.txt").exists(),
        "no-arg @if-bumped must not run before a bump"
    );
    assert_eq!(
        fs::read_to_string(dir.join("none.txt")).expect("none"),
        "none"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn version_bump_directives_reject_second_bump_for_same_version() {
    let dir = test_path("version-bump-duplicate");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("cjtasks"),
        "@version app 1.2.3\nrun:\n  @patch app\n  @minor app\n",
    )
    .expect("write cjtasks");

    let err = run_cli_from_cwd(&["run".to_string()], &dir).expect_err("second bump should fail");

    assert!(err
        .to_string()
        .contains("version 'app' was already bumped as patch in this run"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn version_conditionals_use_semver_precedence() {
    let dir = test_path("version-conditionals");
    fs::create_dir_all(&dir).expect("mkdir");
    let parsed = parse_task_file(
        r#"@version app 1.2.3-beta.1
run:
  @if-version app < 1.2.3
    @shell printf prerelease-lower > lower.txt
  @if-version app prerelease
    @shell printf pre > pre.txt
  @if-version app release
    @shell printf release > release.txt
"#,
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();
    env.export("VERSION_APP".to_string(), "1.2.3-beta.1".to_string());

    let code = run_task_from_dir(&dir, &parsed, "run", &mut env).expect("run");

    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(dir.join("lower.txt")).expect("lower"),
        "prerelease-lower"
    );
    assert_eq!(fs::read_to_string(dir.join("pre.txt")).expect("pre"), "pre");
    assert!(!dir.join("release.txt").exists());

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn version_directive_is_header_only() {
    let late = parse_task_file("run:\n  true\n@version cli 0.1.0\n", Path::new("cjtasks"))
        .expect_err("@version after tasks should fail");
    assert!(late
        .to_string()
        .contains("@version must appear before tasks"));

    let parsed = parse_task_file(
        "@version cli 0.1.0\nbump:\n  @version inc patch cli\n",
        Path::new("cjtasks"),
    )
    .expect("parse");
    let dir = test_path("version-header-only");
    fs::create_dir_all(&dir).expect("mkdir");
    let mut env = minimal_env();

    let err = run_task_from_dir(&dir, &parsed, "bump", &mut env)
        .expect_err("task-body @version should fail");

    assert!(err
        .to_string()
        .contains("@version can only be used as a top-level header"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn filesystem_directives_copy_create_and_rename() {
    let dir = test_path("filesystem-directives");
    fs::create_dir_all(dir.join("srcdir/nested")).expect("mkdir srcdir");
    fs::write(dir.join("a.txt"), "a").expect("write a");
    fs::write(dir.join("b.txt"), "b").expect("write b");
    fs::write(dir.join("srcdir/nested/c.txt"), "c").expect("write c");
    let parsed = parse_task_file(
        r#"run:
  @mkdir out/files out/dirs
  @cp a.txt b.txt out/files
  @cp a.txt out/single.txt
  @cpdir srcdir out/dirs
  @cpdir srcdir/ out/contents
  @rename out/single.txt out/renamed.txt
"#,
        Path::new("cjtasks"),
    )
    .expect("parse");
    let mut env = minimal_env();

    let code = run_task_from_dir(&dir, &parsed, "run", &mut env).expect("run");
    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(dir.join("out/files/a.txt")).expect("a"),
        "a"
    );
    assert_eq!(
        fs::read_to_string(dir.join("out/files/b.txt")).expect("b"),
        "b"
    );
    assert_eq!(
        fs::read_to_string(dir.join("out/dirs/srcdir/nested/c.txt")).expect("dir c"),
        "c"
    );
    assert_eq!(
        fs::read_to_string(dir.join("out/contents/nested/c.txt")).expect("contents c"),
        "c"
    );
    assert_eq!(
        fs::read_to_string(dir.join("out/renamed.txt")).expect("renamed"),
        "a"
    );
    assert!(!dir.join("out/single.txt").exists());

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn rename_cannot_move_between_directories() {
    let dir = test_path("rename-no-move");
    fs::create_dir_all(dir.join("other")).expect("mkdir");
    fs::write(dir.join("a.txt"), "a").expect("write a");
    let parsed = parse_task_file("run:\n  @rename a.txt other/a.txt\n", Path::new("cjtasks"))
        .expect("parse");
    let mut env = minimal_env();

    let err =
        run_task_from_dir(&dir, &parsed, "run", &mut env).expect_err("rename should not move");
    assert!(err.to_string().contains("cannot move across directories"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn task_names_cannot_conflict_with_directories() {
    let dir = test_path("task-dir-conflict");
    fs::create_dir_all(dir.join("build")).expect("mkdir build");
    fs::write(dir.join("cjtasks"), "build:\n  true\n").expect("write cjtasks");

    let err = run_cli_from_cwd(&[], &dir).expect_err("task/folder conflict should fail");
    assert!(err
        .to_string()
        .contains("task name conflicts with directory"));

    fs::remove_dir_all(dir).expect("cleanup");
}
