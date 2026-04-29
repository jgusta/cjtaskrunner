mod common;

use common::{assert_failure_contains, assert_success, run_cj, temp_path};
use std::fs;

#[test]
fn init_creates_cjtasks_without_overwriting_a_taskfile() {
    let dir = temp_path("init");
    fs::create_dir_all(&dir).expect("create project");

    let stdout = assert_success(&run_cj(&dir, &["--init"]));
    assert!(stdout.contains("created"));
    assert_eq!(
        fs::metadata(dir.join("cjtasks"))
            .expect("cjtasks metadata")
            .len(),
        0
    );

    assert_failure_contains(&run_cj(&dir, &["--init"]), "taskfile already exists");
    assert_eq!(
        fs::metadata(dir.join("cjtasks"))
            .expect("cjtasks metadata")
            .len(),
        0
    );

    fs::remove_dir_all(dir).expect("remove project");
}

#[test]
fn auto_imports_common_task_systems_and_is_idempotent() {
    let dir = temp_path("auto");
    fs::create_dir_all(&dir).expect("create project");
    fs::write(
        dir.join("package.json"),
        r#"{
  "packageManager": "pnpm@10.0.0",
  "scripts": {
    "build": "vite build",
    "lint:fix": "eslint --fix"
  }
}
"#,
    )
    .expect("write package.json");
    fs::write(
        dir.join("deno.json"),
        r#"{"tasks":{"build":"deno task build"}}
"#,
    )
    .expect("write deno.json");
    fs::write(
        dir.join("Makefile"),
        ".PHONY: build release\nVERSION := 1\nbuild:\n\ttrue\nrelease: build\n\ttrue\n",
    )
    .expect("write Makefile");
    fs::write(
        dir.join("Justfile"),
        "build:\n  cargo build\ndeploy target:\n  echo {{target}}\nvalue := 'x'\n",
    )
    .expect("write Justfile");

    let stdout = assert_success(&run_cj(&dir, &["--auto"]));
    assert!(stdout.contains("with 6 imported tasks"));
    let source = fs::read_to_string(dir.join("cjtasks")).expect("read generated cjtasks");
    assert!(source.starts_with("# Imported from package.json by cj --auto\n"));
    for expected in [
        "build:\n  pnpm run build",
        "lintfix:\n  pnpm run lint:fix",
        "build2:\n  deno task build",
        "build3:\n  make build",
        "release:\n  make release",
        "build4:\n  just build",
    ] {
        assert!(
            source.contains(expected),
            "missing generated task: {expected}"
        );
    }
    assert!(!source.contains("deploy:"));
    assert!(!source.contains("PHONY:"));
    assert!(!source.contains("npm-build"));
    assert_success(&run_cj(&dir, &[]));

    let second = assert_success(&run_cj(&dir, &["--auto"]));
    assert!(second.contains("no new tasks to add"));
    assert_eq!(
        fs::read_to_string(dir.join("cjtasks")).expect("read unchanged cjtasks"),
        source
    );

    fs::remove_dir_all(dir).expect("remove project");
}

#[test]
fn auto_preserves_existing_cjtasks() {
    let dir = temp_path("auto-existing");
    fs::create_dir_all(&dir).expect("create project");
    fs::write(
        dir.join("cjtasks"),
        "build:\n  @echo custom build\nbuild2:\n  @echo another custom build\n",
    )
    .expect("write taskfile");
    fs::write(
        dir.join("package.json"),
        r#"{"scripts":{"build":"build","test":"test"}}
"#,
    )
    .expect("write package.json");

    let stdout = assert_success(&run_cj(&dir, &["--auto"]));
    assert!(stdout.contains("with 2 imported tasks"));
    let source = fs::read_to_string(dir.join("cjtasks")).expect("read taskfile");
    assert!(source.contains("build:\n  @echo custom build"));
    assert!(source.contains("build2:\n  @echo another custom build"));
    assert!(source.contains("build3:\n  npm run build"));
    assert!(source.contains("test:\n  npm run test"));

    fs::remove_dir_all(dir).expect("remove project");
}

#[test]
fn auto_ignores_unrecognized_taskfiles_and_creates_canonical_taskfile() {
    let dir = temp_path("auto-unrecognized");
    fs::create_dir_all(&dir).expect("create project");
    fs::write(dir.join("unknown.cjtasks"), "custom:\n  @echo ignored\n")
        .expect("write unknown taskfile");
    fs::write(
        dir.join("package.json"),
        r#"{"scripts":{"test":"test"}}
"#,
    )
    .expect("write package.json");

    let stdout = assert_success(&run_cj(&dir, &["--auto"]));
    assert!(stdout.contains("created"));
    let source = fs::read_to_string(dir.join("cjtasks")).expect("read canonical taskfile");
    assert!(source.contains("test:\n  npm run test"));
    assert!(!source.contains("custom:"));

    fs::remove_dir_all(dir).expect("remove project");
}

#[test]
fn auto_requires_an_importable_task_source() {
    let dir = temp_path("auto-empty");
    fs::create_dir_all(&dir).expect("create project");

    assert_failure_contains(&run_cj(&dir, &["--auto"]), "no importable tasks found");
    assert!(!dir.join("cjtasks").exists());

    fs::remove_dir_all(dir).expect("remove project");
}
