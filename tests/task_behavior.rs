mod common;

use common::{assert_failure, assert_success, run_cj, run_cj_with_env, temp_path};
use std::fs;
use std::path::Path;

fn write_taskfile(dir: &Path, source: &str) {
    fs::write(dir.join("cjtasks"), source).expect("write cjtasks");
}

#[test]
fn ordinary_commands_do_not_shell_split_interpolated_values() {
    let dir = temp_path("direct");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        "@env:\n  CJTEST_VALUE: a b; echo injected\nrun:\n  sh -c 'test \"$1\" = \"a b; echo injected\"' ignored $CJTEST_VALUE\n",
    );

    assert_success(&run_cj(&dir, &["run"]));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn shell_directive_quotes_interpolated_values() {
    let dir = temp_path("shell");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        "@env:\n  CJTEST_VALUE: safe; echo bad\nrun:\n  @shell printf '%s' $CJTEST_VALUE > out.txt\n",
    );

    assert_success(&run_cj(&dir, &["run"]));
    assert_eq!(
        fs::read_to_string(dir.join("out.txt")).expect("out"),
        "safe; echo bad"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn braced_interpolation_defaults_to_empty_and_supports_required_forms() {
    let dir = temp_path("braced-vars");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"@env:
  EMPTY:
run:
  @shell printf '%s|%s|%s|%s|%s' ${MISSING} ${EMPTY} ${MISSING?fallback} ${MISSING?"quoted fallback"} ${EMPTY?} > out.txt
required:
  @echo ${MISSING?}
invalid-interpolation:
  @echo ${1BAD}
"#,
    );

    assert_success(&run_cj(&dir, &["run"]));
    assert_eq!(
        fs::read_to_string(dir.join("out.txt")).expect("out"),
        "||fallback|quoted fallback|"
    );

    let stderr = assert_failure(&run_cj(&dir, &["required"]));
    assert!(stderr.contains("missing variable: MISSING"));

    let stderr = assert_failure(&run_cj(&dir, &["invalid-interpolation"]));
    assert!(stderr.contains("invalid variable interpolation"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn task_composition_runs_tasks_and_reports_cycles() {
    let dir = temp_path("task");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        "first:\n  @task second\nsecond:\n  true\ncycle:\n  @task cycle\n",
    );

    assert_success(&run_cj(&dir, &["first"]));
    let stderr = assert_failure(&run_cj(&dir, &["cycle"]));
    assert!(stderr.contains("recursive @task cycle"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn await_block_runs_after_parallel_tasks_succeed() {
    let dir = temp_path("await-tasks");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"dev:
  client:
    @shell sleep 1 && printf client > client.txt
  server:
    @shell sleep 1 && printf server > server.txt
  @await dev:server dev:client
    @if-exists client.txt
      @if-exists server.txt
        @shell printf ready > ready.txt
"#,
    );
    let start = std::time::Instant::now();

    assert_success(&run_cj_with_env(&dir, &["dev"], &[("CJ_JOBS", "2")]));

    assert_eq!(
        fs::read_to_string(dir.join("ready.txt")).expect("ready"),
        "ready"
    );
    assert!(
        start.elapsed() < std::time::Duration::from_millis(1900),
        "awaited tasks should run in parallel"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn await_failure_skips_success_block_and_can_be_handled_by_or() {
    let dir = temp_path("await-or");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"root:
  @await failer
    @shell printf bad > should-not-exist.txt
  @or
    @shell printf failed > failed.txt
failer:
  @fail
"#,
    );

    assert_success(&run_cj(&dir, &["root"]));

    assert!(!dir.join("should-not-exist.txt").exists());
    assert_eq!(
        fs::read_to_string(dir.join("failed.txt")).expect("failed"),
        "failed"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn failed_if_else_can_skip_and_then_run_or_chain() {
    let dir = temp_path("if-else-and-or-chain");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"run:
  @set MYVAR arvo
  @if $MYVAR == arv2o
    @success
  @else
    @fail
  @and
    @echo there is success
  @or
    @echo we have failed
"#,
    );

    let stdout = assert_success(&run_cj(&dir, &["run"]));
    assert_eq!(stdout.trim(), "we have failed");

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn return_directive_is_status_only() {
    let dir = temp_path("return-status-only");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"run:
  @return hello
  @and
    @echo after
status:
  @return 7
"#,
    );

    let stdout = assert_success(&run_cj(&dir, &["run"]));
    assert_eq!(stdout, "after\n");

    let output = run_cj(&dir, &["status"]);
    assert_eq!(output.status.code(), Some(7));
    assert!(output.stdout.is_empty());

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn awaited_tasks_isolate_set_and_share_exports() {
    let dir = temp_path("await-mutation");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"root:
  @set PORT root
  @await setup expose
  @shell printf "$PORT:$EXPORTED" > out.txt
setup:
  @set PORT 3000
expose:
  @export EXPORTED yes
"#,
    );

    assert_success(&run_cj(&dir, &["root"]));
    assert_eq!(
        fs::read_to_string(dir.join("out.txt")).expect("out"),
        "root:yes"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn watch_blocks_reject_await() {
    let dir = temp_path("watch-await");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"root:
  @watch .
    @await child
child:
  @success
"#,
    );

    let stderr = assert_failure(&run_cj(&dir, &["root"]));
    assert!(stderr.contains("@await cannot be used inside @watch"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn watch_blocks_must_be_one_indented_line() {
    let dir = temp_path("watch-one-line");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"root:
  @watch .
    @echo first
    @echo second
"#,
    );

    let stderr = assert_failure(&run_cj(&dir, &["root"]));
    assert!(stderr.contains("@watch expects exactly one indented line"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn watch_blocks_reject_task_calls_that_use_await() {
    let dir = temp_path("watch-task-await");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"root:
  @watch .
    @task helper
helper:
  @await child
child:
  @success
"#,
    );

    let stderr = assert_failure(&run_cj(&dir, &["root"]));
    assert!(stderr.contains("@await cannot be used inside @watch"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn conditionals_switches_and_runtime_env_mutations_work_together() {
    let dir = temp_path("controls");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("exists.txt"), "").expect("exists");
    write_taskfile(
        &dir,
        r#"run:
  @set MODE prod
  @if $MODE == prod
    @shell printf yes > if.txt
  @else
    @shell printf no > if.txt
  @if-exists exists.txt
    @export FOUND 1
  @set FOUND_NAME FOUND
  @if-set $FOUND_NAME
    @shell printf found > found.txt
  @switch $MODE
    @case dev
      @shell printf dev > switch.txt
    @case prod
      @shell printf prod > switch.txt
    @default
      @shell printf default > switch.txt
  @unset $FOUND_NAME
  @if-not-set $FOUND_NAME
    @shell printf unset > unset.txt
"#,
    );

    assert_success(&run_cj(&dir, &["run"]));

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
fn directive_variable_name_operands_are_interpolated_strings() {
    let dir = temp_path("interpolated-variable-name-operands");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"run:
  @set TARGET_NAME MODE
  @set $TARGET_NAME release
  @if-set $TARGET_NAME
    @shell printf %s $MODE > mode.txt
  @set CAPTURE_NAME COMMIT
  @set $CAPTURE_NAME:
    @shell printf abc123
  @export $CAPTURE_NAME
  @shell printf %s $COMMIT > commit.txt
  @unset $TARGET_NAME
  @if-not-set $TARGET_NAME
    @shell printf unset > unset.txt
"#,
    );

    assert_success(&run_cj(&dir, &["run"]));

    assert_eq!(
        fs::read_to_string(dir.join("mode.txt")).expect("mode"),
        "release"
    );
    assert_eq!(
        fs::read_to_string(dir.join("commit.txt")).expect("commit"),
        "abc123"
    );
    assert_eq!(
        fs::read_to_string(dir.join("unset.txt")).expect("unset"),
        "unset"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn if_tests_string_membership_in_a_list() {
    let dir = temp_path("if-in");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"run (TARGET):
  @if-in $TARGET linux "mac os" windows
    @shell printf member > member.txt
  @else
    @shell printf missing > member.txt
  @if-in staging dev test prod
    @shell printf wrong > absent.txt
  @else
    @shell printf absent > absent.txt
  @if-not-in staging dev test prod
    @shell printf not-member > not-member.txt
  @if-not 0
    @shell printf falsey > falsey.txt
"#,
    );

    assert_success(&run_cj(&dir, &["run", "mac os"]));
    assert_eq!(
        fs::read_to_string(dir.join("member.txt")).expect("member"),
        "member"
    );
    assert_eq!(
        fs::read_to_string(dir.join("absent.txt")).expect("absent"),
        "absent"
    );
    assert_eq!(
        fs::read_to_string(dir.join("not-member.txt")).expect("not-member"),
        "not-member"
    );
    assert_eq!(
        fs::read_to_string(dir.join("falsey.txt")).expect("falsey"),
        "falsey"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn if_in_requires_at_least_one_candidate() {
    let dir = temp_path("if-in-arguments");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"run:
  @if-in prod
    @success
"#,
    );

    let stderr = assert_failure(&run_cj(&dir, &["run"]));
    assert!(stderr.contains("@if-in expects '<needle> <candidate>...'"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn if_rejects_inline_membership_syntax() {
    let dir = temp_path("if-inline-membership-rejected");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"run:
  @if prod in dev prod
    @success
"#,
    );

    let stderr = assert_failure(&run_cj(&dir, &["run"]));
    assert!(stderr.contains("@if expects a value, '<left> == <right>', or '<left> != <right>'"));

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn set_capture_blocks_capture_stdout_without_leaking_nested_output() {
    let dir = temp_path("set-capture");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        r#"run:
  @set OUT:
    @echo before
    @set INNER:
      @echo inner
    @echo after
  @shell printf %s $OUT > out.txt
  @shell printf %s $INNER > inner.txt
"#,
    );

    assert_success(&run_cj(&dir, &["run"]));
    assert_eq!(
        fs::read_to_string(dir.join("out.txt")).expect("out"),
        "before\nafter"
    );
    assert_eq!(
        fs::read_to_string(dir.join("inner.txt")).expect("inner"),
        "inner"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn cd_back_and_task_calls_restore_working_directories() {
    let dir = temp_path("cd-task");
    fs::create_dir_all(dir.join("sub/child")).expect("mkdir");
    write_taskfile(
        &dir,
        r#"run:
  @cd sub
  @task write
  @shell pwd > after-task.txt
write:
  @shell pwd > task-start.txt
  @cd child
  @shell pwd > task-child.txt
"#,
    );

    assert_success(&run_cj(&dir, &["run"]));

    let canonical_sub = fs::canonicalize(dir.join("sub")).expect("sub");
    let canonical_child = fs::canonicalize(dir.join("sub/child")).expect("child");
    assert_eq!(
        fs::canonicalize(
            fs::read_to_string(dir.join("sub/task-start.txt"))
                .expect("task start")
                .trim()
        )
        .expect("task start pwd"),
        canonical_sub
    );
    assert_eq!(
        fs::canonicalize(
            fs::read_to_string(dir.join("sub/child/task-child.txt"))
                .expect("task child")
                .trim()
        )
        .expect("task child pwd"),
        canonical_child
    );
    assert_eq!(
        fs::canonicalize(
            fs::read_to_string(dir.join("sub/after-task.txt"))
                .expect("after task")
                .trim()
        )
        .expect("after task pwd"),
        canonical_sub
    );

    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn version_bump_directives_update_taskfile_version_headers() {
    let dir = temp_path("version-bump");
    fs::create_dir_all(&dir).expect("mkdir");
    write_taskfile(
        &dir,
        "@version cli 0.1.0\n@version lsp 0.0.1-alpha.1\n@version api 1.2.3\nbump:\n  @set NAME cli\n  @patch $NAME\n  @shell printf %s $VERSION_CLI > cli-version.txt\n  @pre lsp beta.\n  @minor api\n",
    );

    assert_success(&run_cj(&dir, &["bump"]));

    let source = fs::read_to_string(dir.join("cjtasks")).expect("read cjtasks");
    assert!(source.contains("@version cli 0.1.1"));
    assert!(source.contains("@version lsp 0.0.1-beta.0"));
    assert!(source.contains("@version api 1.3.0"));
    assert_eq!(
        fs::read_to_string(dir.join("cli-version.txt")).expect("read captured version"),
        "0.1.1"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}
