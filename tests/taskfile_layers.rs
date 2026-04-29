mod common;

use std::fs;

use common::{run_cj, temp_path};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = temp_path(name);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn layers_replace_tasks_env_and_help_by_precedence() {
    let dir = temp_dir("layers-precedence");
    fs::write(
        dir.join("cjtasks"),
        "@env:\n  VALUE: base\n@help:\n  base help\nshow (ARG):\n  @echo base-$VALUE-$ARG\n",
    )
    .unwrap();
    fs::write(
        dir.join("production.cjtasks"),
        "@env:\n  VALUE?: production\nshow (NAME):\n  @echo production-$VALUE-$NAME\n",
    )
    .unwrap();
    fs::write(
        dir.join("local.cjtasks"),
        "@env:\n  VALUE: local\n@help:\n  local help\nshow (ITEM):\n  @echo local-$VALUE-$ITEM\n",
    )
    .unwrap();

    let run = run_cj(&dir, &["show", "value"]);
    assert_eq!(run.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "local-local-value"
    );

    let help = run_cj(&dir, &["help"]);
    assert!(String::from_utf8_lossy(&help.stdout).contains("local help"));
}

#[test]
fn canonical_base_is_displayed_when_overlays_exist() {
    let dir = temp_dir("base-selection");
    fs::write(dir.join("cjtasks"), "show:\n  @echo base\n").unwrap();
    fs::write(dir.join("local.cjtasks"), "show:\n  @echo local\n").unwrap();

    let listing = run_cj(&dir, &[]);
    let output = String::from_utf8_lossy(&listing.stdout);
    assert!(output.starts_with("Tasks in cjtasks:"));
    assert!(output.contains("show"));

    let run = run_cj(&dir, &["show"]);
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "local");
}

#[test]
fn override_must_keep_task_arity() {
    let dir = temp_dir("layer-arity");
    fs::write(dir.join("cjtasks"), "build (TARGET):\n  @success\n").unwrap();
    fs::write(dir.join("local.cjtasks"), "build:\n  @success\n").unwrap();

    let result = run_cj(&dir, &[]);
    assert_ne!(result.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&result.stderr).contains("overrides arity 1 with arity 0"));
}

#[test]
fn overlays_cannot_declare_versions_or_bump_them() {
    let version_dir = temp_dir("layer-version");
    fs::write(version_dir.join("cjtasks"), "run:\n  @success\n").unwrap();
    fs::write(
        version_dir.join("local.cjtasks"),
        "@version cli 1.0.0\nrun:\n  @success\n",
    )
    .unwrap();
    let version = run_cj(&version_dir, &[]);
    assert!(
        String::from_utf8_lossy(&version.stderr).contains("@version is only allowed in cjtasks")
    );

    let bump_dir = temp_dir("layer-bump");
    fs::write(bump_dir.join("cjtasks"), "@version cli 1.0.0\n").unwrap();
    fs::write(bump_dir.join("local.cjtasks"), "release:\n  @patch cli\n").unwrap();
    let bump = run_cj(&bump_dir, &[]);
    assert!(String::from_utf8_lossy(&bump.stderr)
        .contains("version bump directives are only allowed in cjtasks"));
}

#[test]
fn overlay_can_reference_a_task_from_the_base() {
    let dir = temp_dir("layer-cross-reference");
    fs::write(dir.join("cjtasks"), "worker:\n  @echo worker\n").unwrap();
    fs::write(
        dir.join("development.cjtasks"),
        "dev:\n  @await worker\n    @echo complete\n",
    )
    .unwrap();

    let result = run_cj(&dir, &["dev"]);
    assert_eq!(result.status.code(), Some(0));
}

#[test]
fn base_bump_updates_the_base_when_overlays_are_loaded() {
    let dir = temp_dir("layer-base-bump");
    fs::write(
        dir.join("cjtasks"),
        "@version cli 1.0.0\nrelease:\n  @patch cli\n",
    )
    .unwrap();
    fs::write(dir.join("local.cjtasks"), "local:\n  @success\n").unwrap();

    let result = run_cj(&dir, &["release"]);
    assert_eq!(result.status.code(), Some(0));
    assert!(fs::read_to_string(dir.join("cjtasks"))
        .unwrap()
        .contains("@version cli 1.0.1"));
}
