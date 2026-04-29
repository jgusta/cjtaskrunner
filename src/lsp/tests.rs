use super::*;

#[test]
fn lsp_and_editor_outline_share_taskfile_fixture() {
    let analysis = analyze(include_str!("../../tests/fixtures/outline.cjtasks"));

    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );
    assert_eq!(analysis.task_order, ["build", "build:dev", "help", "env"]);
    assert_eq!(
        analysis.tasks["build"].description.as_deref(),
        Some("build tasks")
    );
    assert_eq!(
        analysis.tasks["build:dev"].description.as_deref(),
        Some("build dev assets")
    );
}

#[test]
fn lsp_analyzes_multiple_diagnostics_and_symbols() {
    let analysis = analyze(
        r#"@env:
  NAME: one
  NAME?: two
build:
  @task test
  @unknown
test:
  true
"#,
    );

    assert!(analysis.tasks.contains_key("build"));
    assert!(analysis.tasks.contains_key("test"));
    assert!(analysis.variables.contains("NAME"));
    assert!(analysis
        .diagnostics
        .iter()
        .any(|diag| diag.message.contains("duplicate env entry")));
    assert!(analysis
        .diagnostics
        .iter()
        .any(|diag| diag.message.contains("unknown directive")));
}

#[test]
fn lsp_rejects_late_header_entries() {
    let analysis = analyze(
        r#"run:
  true
@env:
  NAME: value
@version cli 0.1.0
"#,
    );

    assert!(analysis
        .diagnostics
        .iter()
        .any(|diag| diag.message.contains("@env: must appear before tasks")));
    assert!(analysis
        .diagnostics
        .iter()
        .any(|diag| diag.message.contains("@version must appear before tasks")));
}

#[test]
fn lsp_allows_env_and_help_task_names() {
    let analysis = analyze(
        r#"env:
  echo env
help:
  echo help
"#,
    );

    assert!(analysis.diagnostics.is_empty());
    assert!(analysis.tasks.contains_key("env"));
    assert!(analysis.tasks.contains_key("help"));
}

#[test]
fn lsp_accepts_desc_text() {
    let analysis = analyze(
        r#"build:
  @desc build project
  true
"#,
    );

    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn lsp_accepts_help_colon_block() {
    let analysis = analyze(
        r#"@help:
  Project help.

build:
  @help:
    Build help.
  true
"#,
    );

    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn lsp_accepts_consistent_tabs_and_rejects_mixed_indentation() {
    let tabs = analyze("run:\n\t@echo tabbed\n");
    assert!(
        tabs.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        tabs.diagnostics
    );
    assert!(tabs.tasks.contains_key("run"));

    let mixed = analyze("run:\n  @echo spaces\n\t@echo tabs\n");
    assert!(mixed
        .diagnostics
        .iter()
        .any(|diag| diag.message.contains("cj --format")));
}

#[test]
fn lsp_guides_pasted_shell_line_continuations_and_env_prefixes() {
    let analysis = analyze(
        "install:\n  npm_config_runtime=electron \\\n  npm_config_target=9.4.4 \\\n  npm install\ncheck:\n  RUST_LOG=debug cargo test\n",
    );

    let messages = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert!(messages
        .iter()
        .any(|message| message.contains("line continuations")));
    assert!(messages
        .iter()
        .any(|message| message.contains("NAME=value command prefixes")));
    assert!(messages
        .iter()
        .all(|message| message.contains("@shell") && message.contains("@export")));
}

#[test]
fn lsp_rejects_variables_in_descriptions_and_help_text() {
    let analysis = analyze(
        r#"@help:
  Home is $HOME
build:
  @desc build $TARGET
  @help:
    Missing: ${NAME?fallback}
  true
literal:
  @desc Literal \${NAME}
  @help:
    Literal \$HOME
  true
"#,
    );

    let messages = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        messages
            .iter()
            .filter(|message| message.contains("text cannot contain variables"))
            .count(),
        3
    );
    assert!(!analysis.variables.contains("HOME"));
    assert!(!analysis.variables.contains("TARGET"));
    assert!(!analysis.variables.contains("NAME"));
    assert_eq!(
        analysis.tasks["literal"].description.as_deref(),
        Some("Literal ${NAME}")
    );
}

#[test]
fn lsp_accepts_selfhelp_directive() {
    let analysis = analyze(
        r#"cli:
  @desc cli commands
  @help:
    CLI help.
  @selfhelp
"#,
    );

    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn lsp_validates_if_membership_conditions() {
    let valid = analyze(
        r#"run:
  @if-in $TARGET linux "mac os" windows
    true
  @if-not-in staging dev prod
    true
"#,
    );
    assert!(
        valid.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        valid.diagnostics
    );

    let invalid = analyze(
        r#"run:
  @if-in prod
    true
"#,
    );
    assert!(invalid.diagnostics.iter().any(|diagnostic| diagnostic
        .message
        .contains("@if-in expects '<needle> <candidate>...'")));
}

#[test]
fn lsp_analyzes_nested_task_blocks() {
    let analysis = analyze(
        r#"build:
  @desc build tasks
  dev:
    @desc build development assets
    true
"#,
    );

    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );
    assert!(analysis.tasks.contains_key("build"));
    assert!(analysis.tasks.contains_key("build:dev"));
    assert_eq!(
        analysis.tasks["build"].description.as_deref(),
        Some("build tasks")
    );
    assert_eq!(
        analysis.tasks["build:dev"].description.as_deref(),
        Some("build development assets")
    );
}

#[test]
fn lsp_analyzes_task_arguments_without_changing_symbol_names() {
    let analysis = analyze(
        r#"deploy (TARGET, TAG):
  @desc deploy a release
  true
release:
  publish (REGISTRY):
    true
"#,
    );

    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );
    assert!(analysis.tasks.contains_key("deploy"));
    assert!(analysis.tasks.contains_key("release:publish"));
    assert!(analysis.variables.contains("TARGET"));
    assert!(analysis.variables.contains("TAG"));
    assert!(analysis.variables.contains("REGISTRY"));

    let symbols = document_symbols(&analysis);
    assert_eq!(symbols[0].name, "deploy");
    assert_eq!(symbols[1].name, "release");
    assert_eq!(
        symbols[1].children.as_ref().expect("release children")[0].name,
        "release:publish"
    );
}

#[test]
fn lsp_reports_invalid_task_argument_declarations() {
    let analysis = analyze(
        r#"bad (FIRST SECOND):
  true
duplicate (VALUE, VALUE):
  true
"#,
    );

    let messages = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert!(messages
        .iter()
        .any(|message| message.contains("task arguments must be separated by commas")));
    assert!(messages
        .iter()
        .any(|message| message.contains("duplicate task argument 'VALUE'")));
}

#[test]
fn lsp_rejects_task_groups_deeper_than_one_nested_level() {
    let analysis = analyze(
        r#"build:
  dev:
    fast:
      true
"#,
    );

    assert!(analysis
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("limited to one level")));
}

#[test]
fn lsp_document_symbols_are_nested_and_span_task_blocks() {
    let analysis = analyze(
        r#"build:
  @desc build tasks
  dev:
    @desc build development assets
    true
  docs:
    @desc build docs
    true
test:
  true
"#,
    );
    let symbols = document_symbols(&analysis);

    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0].name, "build");
    assert_eq!(symbols[0].range.start.line, 0);
    assert_eq!(symbols[0].range.end.line, 8);
    let children = symbols[0].children.as_ref().expect("build children");
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name, "build:dev");
    assert_eq!(children[0].range.start.line, 2);
    assert_eq!(children[0].range.end.line, 5);
    assert_eq!(children[1].name, "build:docs");
    assert_eq!(children[1].range.start.line, 5);
    assert_eq!(children[1].range.end.line, 8);
    assert_eq!(symbols[1].name, "test");
    assert!(symbols[1].children.is_none());
}

#[test]
fn lsp_accepts_version_headers_and_bump_directives() {
    let analysis = analyze(
        r#"@version cli 0.1.0
@version lsp 0.0.1-alpha.1
bump:
  @patch cli
  @set NAME cli
  @patch $NAME
  @pre lsp beta.
  @if-bumped
    @echo bumped
  @if-not-bumped
    @echo none
  @if-patch cli
    @echo bumped level
  @if-not-patch lsp
    @echo lsp was not patched
  @if-not-version cli < 0.1.0
    @echo cli is not below 0.1.0
  @echo $VERSION_CLI $VERSION_LSP
"#,
    );

    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );
    assert!(analysis.variables.contains("VERSION_CLI"));
    assert!(analysis.variables.contains("VERSION_LSP"));
    assert!(analysis.variables.contains("NAME"));
}
