use super::*;

#[test]
fn parses_env_and_tasks() {
    let path = Path::new("cjtasks");
    let parsed = parse_task_file(
        r#"
# Project tasks
@env:
  NODE_ENV: development
  PORT?: 5173
  EMPTY:

dev:
  @desc start development server
  echo # retained

test123:
  cargo test

build:dev:
  @desc build dev
  @help:
    Build dev artifacts.

    Use during local work.
  cargo build
"#,
        path,
    )
    .expect("parse");

    assert_eq!(parsed.env.overrides["NODE_ENV"], "development");
    assert_eq!(parsed.env.overrides["EMPTY"], "");
    assert_eq!(parsed.env.fallbacks["PORT"], "5173");
    assert_eq!(parsed.task_order, vec!["dev", "test123", "build:dev"]);
    assert_eq!(parsed.descriptions["dev"], "start development server");
    assert_eq!(parsed.descriptions["build:dev"], "build dev");
    assert_eq!(
        parsed.task_help["build:dev"],
        "Build dev artifacts.\n\nUse during local work."
    );
    assert_eq!(parsed.tasks["dev"][0].text, "echo # retained");
    assert_eq!(parsed.tasks["test123"][0].text, "cargo test");
    assert_eq!(parsed.tasks["build:dev"][0].text, "cargo build");
}

#[test]
fn parses_task_arguments() {
    let parsed = parse_task_file(
        r#"
deploy (TARGET, TAG):
  echo $TARGET $TAG

release:
  publish (REGISTRY):
    echo $REGISTRY
"#,
        Path::new("cjtasks"),
    )
    .expect("parse task arguments");

    assert_eq!(
        parsed.task_order,
        vec!["deploy", "release", "release:publish"]
    );
    assert_eq!(parsed.task_arguments["deploy"], vec!["TARGET", "TAG"]);
    assert_eq!(parsed.task_arguments["release"], Vec::<String>::new());
    assert_eq!(parsed.task_arguments["release:publish"], vec!["REGISTRY"]);
    assert_eq!(parsed.tasks["deploy"][0].text, "echo $TARGET $TAG");
}

#[test]
fn rejects_invalid_task_argument_declarations() {
    for (source, expected) in [
        ("run ():\n  true\n", "task argument list cannot be empty"),
        (
            "run (FIRST SECOND):\n  true\n",
            "task arguments must be separated by commas",
        ),
        (
            "run (VALUE, VALUE):\n  true\n",
            "duplicate task argument 'VALUE'",
        ),
        (
            "run (bad-name):\n  true\n",
            "invalid task argument 'bad-name'",
        ),
        ("run (VALUE:\n  true\n", "invalid task argument declaration"),
    ] {
        let err = parse_task_file(source, Path::new("cjtasks"))
            .expect_err("invalid task argument declaration should fail");
        assert!(
            err.to_string().contains(expected),
            "expected {expected:?} in {err}"
        );
    }
}

#[test]
fn parses_version_headers_into_env_entries() {
    let parsed = parse_task_file(
        "@version cli 0.1.0\n@version lsp 0.0.1-alpha.1\nshow:\n  @echo $VERSION_CLI $VERSION_LSP\n",
        Path::new("cjtasks"),
    )
    .expect("parse");

    assert_eq!(parsed.env.overrides["VERSION_CLI"], "0.1.0");
    assert_eq!(parsed.env.overrides["VERSION_LSP"], "0.0.1-alpha.1");
    assert_eq!(parsed.task_order, vec!["show"]);
}

#[test]
fn version_headers_must_be_semver_without_build_metadata() {
    let leading_zero = parse_task_file("@version app 01.2.3\nrun:\n  true\n", Path::new("cjtasks"))
        .expect_err("leading zeros should fail");
    assert!(leading_zero
        .to_string()
        .contains("version '01.2.3' must be semantic version"));

    let build_metadata = parse_task_file(
        "@version app 1.2.3+build\nrun:\n  true\n",
        Path::new("cjtasks"),
    )
    .expect_err("build metadata should fail");
    assert!(build_metadata.to_string().contains("semantic"));
}

#[test]
fn parses_top_level_help_and_allows_help_task() {
    let parsed = parse_task_file(
        "@help:\n  Project help.\n\nrun:\n  true\n",
        Path::new("cjtasks"),
    )
    .expect("parse");
    assert_eq!(parsed.help.as_deref(), Some("Project help."));

    let parsed = parse_task_file("help:\n  ok\n", Path::new("cjtasks")).expect("parse help task");
    assert_eq!(parsed.help, None);
    assert_eq!(parsed.task_order, vec!["help"]);
    assert_eq!(parsed.tasks["help"][0].text, "ok");
}

#[test]
fn rejects_variables_in_descriptions_and_help_text() {
    for (source, expected_line, expected_context) in [
        (
            "run:\n  @desc deploy $TARGET\n  true\n",
            "cjtasks:2",
            "@desc text cannot contain variables",
        ),
        (
            "@help:\n  Home is ${HOME}\nrun:\n  true\n",
            "cjtasks:2",
            "@help: text cannot contain variables",
        ),
        (
            "run:\n  @help:\n    Missing: ${NAME?fallback}\n  true\n",
            "cjtasks:3",
            "@help: text cannot contain variables",
        ),
    ] {
        let err = parse_task_file(source, Path::new("cjtasks"))
            .expect_err("metadata variables should fail");
        let message = err.to_string();
        assert!(message.contains(expected_line), "{message}");
        assert!(message.contains(expected_context), "{message}");
    }

    let parsed = parse_task_file(
        "@help:\n  Literal \\$HOME\nrun:\n  @desc Literal \\${NAME}\n  true\n",
        Path::new("cjtasks"),
    )
    .expect("escaped dollars are literal metadata text");
    assert_eq!(parsed.help.as_deref(), Some("Literal $HOME"));
    assert_eq!(parsed.descriptions["run"], "Literal ${NAME}");
}

#[test]
fn rejects_task_groups_deeper_than_one_nested_level() {
    let err = parse_task_file(
        "build:\n  dev:\n    fast:\n      true\n",
        Path::new("cjtasks"),
    )
    .expect_err("deep task group should fail");

    assert!(err.to_string().contains("limited to one level"));
}

#[test]
fn rejects_duplicate_env_entries() {
    let err = parse_task_file(
        "@env:\n  NAME: one\n  NAME?: two\nrun:\n  echo hi\n",
        Path::new("cjtasks"),
    )
    .expect_err("duplicate env should fail");

    assert!(err.to_string().contains("duplicate env entry 'NAME'"));
}

#[test]
fn parses_env_as_plain_task_name() {
    let parsed = parse_task_file(
        "env:\n  NAME: value\nrun:\n  echo hi\n",
        Path::new("cjtasks"),
    )
    .expect("parse env task");

    assert_eq!(parsed.task_order, vec!["env", "run"]);
    assert_eq!(parsed.tasks["env"][0].text, "NAME: value");
}

#[test]
fn rejects_env_after_task_definitions() {
    let err = parse_task_file(
        "run:\n  echo hi\n@env:\n  NAME: value\n",
        Path::new("cjtasks"),
    )
    .expect_err("@env after tasks should fail");

    assert!(err.to_string().contains("before"));
}

#[test]
fn rejects_bad_indentation() {
    let err = parse_task_file("run:\n   echo hi\n", Path::new("cjtasks"))
        .expect_err("bad indentation should fail");

    assert!(err.to_string().contains("indentation levels"));
}

#[test]
fn accepts_tabs_when_the_file_uses_tabs_consistently() {
    let parsed = parse_task_file(
        "@env:\n\tNAME: value\nrun:\n\t@help:\n\t\tRun help.\n\t@echo $NAME\n",
        Path::new("cjtasks"),
    )
    .expect("tab-indented taskfile");

    assert_eq!(parsed.env.overrides["NAME"], "value");
    assert_eq!(parsed.task_help["run"], "Run help.");
    assert_eq!(parsed.tasks["run"][0].indent, 2);
    assert_eq!(parsed.tasks["run"][0].text, "@echo $NAME");
}

#[test]
fn rejects_files_that_mix_leading_spaces_and_tabs() {
    let err = parse_task_file("run:\n  @echo spaces\n\t@echo tabs\n", Path::new("cjtasks"))
        .expect_err("mixed indentation should fail");

    let message = err.to_string();
    assert!(message.contains("leading spaces and tabs"));
    assert!(message.contains("cj --format"));
}

#[test]
fn rejects_trailing_colon_directives() {
    parse_task_file("run:\n  @if true:\n    echo hi\n", Path::new("cjtasks"))
        .expect_err("directive colon should fail");
}

#[test]
fn rejects_pasted_shell_line_continuations_and_env_prefixes() {
    let continuation = parse_task_file(
        "install:\n  npm_config_runtime=electron \\\n  npm install\n",
        Path::new("cjtasks"),
    )
    .expect_err("shell line continuation should fail");
    let message = continuation.to_string();
    assert!(message.contains("line continuations"));
    assert!(message.contains("@shell"));
    assert!(message.contains("@export"));

    let env_prefix = parse_task_file(
        "install:\n  npm_config_runtime=electron npm install\n",
        Path::new("cjtasks"),
    )
    .expect_err("shell env prefix should fail");
    assert!(env_prefix
        .to_string()
        .contains("NAME=value command prefixes"));
}
