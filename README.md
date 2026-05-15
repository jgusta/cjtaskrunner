# CJTaskrunner

CJTaskrunner is a small Rust task runner with executable names `cj` and `cjtaskrunner`. It runs named tasks from a project-local taskfile, usually named `cjtasks`. If a taskfile needs an extension, or if a project has more than one taskfile, use the `.cjtasks` extension.

The taskfile format is intentionally small: ordinary task lines run commands directly by default, while CJTaskrunner behavior is written with explicit `@` directives.

See `SPEC.md` for the canonical cjtasks format specification.

## Install, Build, and Run

Build the CLI:

```sh
cargo build
```

Run it through Cargo:

```sh
cargo run -- <task>
cargo run -- <taskfile-or-directory> <task>
cargo run --bin cjtaskrunner -- <task>
```

Run an example from the repository root:

```sh
cargo run -- example_tasks/node-vite envcheck
cargo run -- example_tasks/python-venv pyinfo
cargo run -- example_tasks/rust-cli check
```

Install the local binary into your Cargo bin directory:

```sh
cargo install --path .
```

Then run:

```sh
cj <task>
cj <taskfile-or-directory> <task>
cjtaskrunner <task>
cjtaskrunner <taskfile-or-directory> <task>
```

## Taskfile Discovery

CJTaskrunner recognizes these taskfile forms:

- `cjtasks`
- `*.cjtasks`

Discovery is deliberately local:

- `cj <task>` looks only in the current directory.
- `cj <directory> <task>` looks only in the given directory.
- `cj <taskfile> <task>` uses that exact file, but the file must be named `cjtasks` or use the `.cjtasks` extension.
- If `cjtasks` exists in a searched directory, it wins.
- If no `cjtasks` file exists and exactly one `*.cjtasks` file exists, CJTaskrunner uses that file.
- If multiple `*.cjtasks` files exist, pass the intended taskfile path explicitly.
- Commands always run with the taskfile's directory as the working directory.

There is no parent-directory search in the current MVP.

## Taskfile Syntax

The format is CJTaskrunner-specific and YAML-like, but not general YAML. Top-level keys end with `:`. Indented entries must use exactly two spaces.

```yaml
env:
  NODE_ENV: development
  PORT?: 5173

setup:
  test -f package.json
  test -f src/main.js

dev:
  @task setup
  npm run dev -- --host 127.0.0.1 --port $PORT

dist:
  @shell mkdir -p dist && cp src/*.js dist/
```

Tasks:

- Task names are ASCII letters, digits, hyphens, and underscores, such as `build`, `test123`, or `build-prod`.
- `env` is reserved for the global environment section.
- Each non-empty command line under a task runs in order.
- Semicolons split multiple expressions on one physical line at the same indentation level. Semicolons inside quotes are preserved.
- Ordinary command lines are split into argv and executed directly.
- Shell syntax requires `@shell`.
- Standard input, output, and error are inherited.
- Execution stops at the first command that exits non-zero.
- Each command line is independent, so process working-directory changes and shell-local state do not persist to the next taskfile command line.

Directives:

- Directive lines start with `@`.
- Most directives do not use trailing colons. `@set NAME:` is the block-capture form.
- Nested directive bodies use another two spaces of indentation.
- Supported directives are `@task`, `@shell`, `@echo`, `@clean`, `@stop`, `@set`, `@export`, `@unset`, `@return`, `@success`, `@fail`, `@and`, `@or`, `@if`, `@else`, `@switch`, `@case`, and `@default`.

Comments and blank lines:

- Blank lines are ignored.
- Comment-only lines are ignored.
- Inline comments are not stripped from command or environment values.

Environment entries:

- `NAME: value` is an override and replaces inherited values.
- `NAME?: value` is a fallback and applies only when the variable is absent.
- Environment names must start with a letter or underscore, then use only letters, digits, and underscores.
- Matching single or double quotes around a whole value are stripped for convenience.

## Command Execution

Ordinary task lines run as direct argv commands:

```yaml
build:
  cargo build --release
  npm run build
```

Those lines execute like:

```text
["cargo", "build", "--release"]
["npm", "run", "build"]
```

This means pipes, redirects, command chaining, glob expansion, and shell builtins are not interpreted on ordinary lines. A token such as `>` is passed as a literal argument.

Use `@shell` when a task intentionally needs shell behavior:

```yaml
bundle:
  @shell mkdir -p dist && cat src/*.js > dist/app.js
```

On Unix-like systems, `@shell` uses `/bin/sh -c`.

## Interpolation

CJTaskrunner interpolates variables in ordinary command argv tokens, `@shell` command text, and directive arguments.

Supported forms:

```text
$NAME
${NAME}
${NAME:-fallback}
```

Rules:

- `$NAME` and `${NAME}` read the current CJTaskrunner variable value.
- `${NAME:-fallback}` uses `fallback` when `NAME` is missing or empty.
- Missing variables without a fallback are errors.
- Interpolated values in ordinary command lines stay one argv value. If `NAME` is `-p dir/mydir`, then `mkdir $NAME` passes one argument with that exact value; it does not become two arguments.
- Interpolated values in `@shell` are quoted before shell execution.
- Escape literal interpolation with `\$NAME` or `\${NAME}`.

CJTaskrunner does not support shell-style command substitution, arithmetic expansion, pattern replacement, or nested expansion. Use `@set NAME:` when you need to capture task output into a variable.

## Task Composition

Use `@task` to run another task from the same taskfile:

```yaml
ci:
  @task fmt
  @task test
  @task build

fmt:
  cargo fmt --check

test:
  cargo test

build:
  cargo build
```

`@task name` uses CJTaskrunner semantics directly. The called task runs with the same taskfile, base directory, and effective variable state. Execution stops if the called task fails, and recursive task cycles are reported as errors.

## Runtime Variables

The `env:` block defines the initial environment. Task bodies can mutate later runtime state with `@set`, `@export`, and `@unset`.

```yaml
release:
  @set MODE production
  @export MODE
  @task build

build:
  @if $MODE == production
    cargo build --release
  @else
    cargo build
```

Runtime variable directives:

- `@set NAME value` sets a CJTaskrunner variable for later interpolation and directives, but does not export it to child processes.
- `@set NAME:` runs an indented block and stores its captured stdout in `NAME`. Trailing newlines are trimmed.
- `@export NAME` exports the current variable value to later child processes.
- `@export NAME value` sets and exports a value in one step.
- `@unset NAME` removes the variable and removes any later export overlay for that name.

Runtime state is order-dependent and shared with composed tasks. Changes made inside a task called with `@task` remain visible after that task returns.

Capture example:

```yaml
capture:
  @set RESULT:
    @shell printf "build-%s" "$MODE"
  @echo $RESULT
```

The capture block uses the same task semantics as normal execution, but child stdout and `@echo` output are collected instead of inherited.

## Utility and Status Directives

Use `@echo`, `@clean`, and `@stop` for common task-runner behavior without dropping into shell:

```yaml
clean:
  @clean dist
  @echo cleaned

guard:
  @if-missing package.json
    @stop missing package.json
```

Utility directives:

- `@echo text` writes text plus a newline to stdout after interpolation.
- `@clean path` removes one file or directory relative to the taskfile directory. Missing paths are ok.
- `@stop text` writes text plus a newline when provided, then stops the current flow with status `1`.
- `@return value` writes `value` without adding a newline and returns a status derived from it: `true` is `0`, `false` is `1`, numeric values are that status code, other truthy strings are `0`, and empty/`0`/`false` values are `1`.
- `@return` with an indented block runs that block and returns its status.
- `@success` returns status `0`.
- `@fail` returns status `1`.

Status chains are inspired by fish-style `and`/`or` flow:

```yaml
build-or-clean:
  @task build; @and
    @echo build ok
  @or
    @clean dist
    @echo cleaned after failed build
```

- `@and` runs its indented block only when previous expression returned status `0`.
- `@or` runs its indented block only when previous expression returned non-zero.
- A failed expression can be followed by same-level `@or`; otherwise execution stops on non-zero status.
- Skipped `@and` returns status `1`; skipped `@or` returns status `0`.

## Conditionals and Switches

Conditionals intentionally use a small expression set:

```yaml
install:
  @if-exists package-lock.json
    npm ci
  @else
    npm install

build:
  @if $MODE == production
    npm run build
  @else
    npm run build:dev
```

Supported conditional forms:

```text
@if $VAR == value
@if $VAR != value
@if ${VAR} == value
@if ${VAR} != value
@if-exists path
@if-missing path
@if-set $VAR
@if-unset $VAR
@else
```

Paths in `@if-exists` and `@if-missing` are relative to the taskfile directory. String comparisons are literal after interpolation.

Use `@switch`, `@case`, and `@default` for one-of-many branching:

```yaml
serve:
  @switch $APP_KIND
    @case node
      npm run dev
    @case rust
      cargo run
    @case python
      python3 -m app
    @default
      echo unknown APP_KIND=$APP_KIND
```

`@case` values are literal strings. At most one case runs. A missing `@default` is allowed.

## Environment and .env Behavior

For every task, CJTaskrunner builds the child environment in this order:

1. Start with the current `cj` process environment.
2. Load `.env` from the taskfile directory, adding only variables that are absent.
3. Apply taskfile `NAME?: value` fallbacks, adding only variables that are absent.
4. Apply taskfile `NAME: value` overrides, replacing existing values.
5. Apply Python virtual environment path handling.

The `.env` parser accepts `NAME=value` lines, ignores blank lines and comment-only lines, and does not expand variables. It only reads `.env` beside the taskfile; it does not search parents and does not load `.env.local`.

## Python Virtual Environment Behavior

CJTaskrunner can prepend a virtualenv executable directory to `PATH`.

Detection order:

1. Use active `VIRTUAL_ENV` when it is set and non-empty.
2. Else use `CJ_VENV` when it is set and non-empty.
3. Else use `.venv` under the taskfile directory when it exists.
4. Else leave Python paths alone.

On Unix-like systems, the selected virtualenv contributes `<venv>/bin` to the front of `PATH`, and `VIRTUAL_ENV` is set to the selected directory. If a selected virtualenv exists without a `bin` directory, CJTaskrunner returns an error instead of silently continuing.

## Examples

Example projects live under `example_tasks/`:

- `node-vite`: Vite-style frontend taskfile with npm scripts and `.env`.
- `node-ssr`: Minimal Node server-side rendering workflow.
- `python-venv`: Python package layout with local `.venv` behavior.
- `python-cli-venv`: Python module execution with `CJ_VENV` demonstration.
- `python-pipenv`: Pipenv-oriented taskfile.
- `pyside6-app`: PySide6 app tasks with GUI-friendly environment defaults.
- `docker-basic`: Dockerfile and Docker Compose tasks.
- `rust-cli`: Cargo tasks for a tiny Rust CLI.
- `git-gibberish`: Local git workflow that generates files and commits them.

Run a safe inspection-style task:

```sh
cargo run -- example_tasks/node-vite envcheck
cargo run -- example_tasks/docker-basic base
cargo run -- example_tasks/git-gibberish base
```

Some example tasks intentionally require external tools or dependencies, such as `npm install`, `pipenv`, `PySide6`, Docker, or Cargo. See each example README for what can be run immediately and what is intentionally left as a realistic dependency-backed command.

Existing example taskfiles were originally written against the shell-per-line MVP. Commands that use redirects, globbing, command chaining, quoted shell variables, or shell builtins should be converted to `@shell` or direct argv-safe interpolation when using the round 2 execution model.

## Source Layout

The Rust source is grouped by CJTaskrunner feature area:

- `src/cli.rs`: invocation, taskfile discovery, base directory selection.
- `src/task_file.rs`: taskfile parsing, syntax validation, semicolon splitting.
- `src/environment.rs`: `.env`, taskfile env merge, Python virtualenv path handling.
- `src/runner.rs`: task and block execution.
- `src/directives.rs`: CJTaskrunner directives and control flow.
- `src/command_text.rs`: word splitting, interpolation, child process execution.

`src/lib.rs` keeps shared types and includes the feature files in one private namespace.

## Current Limitations

- No command-line flags.
- No task arguments after the task name.
- No multiple-task invocation.
- No parent-directory taskfile discovery.
- No task-level environment blocks.
- No shell configuration; Unix `@shell` commands use `/bin/sh -c`.
- No Windows shell strategy yet.
- No general YAML parsing.
- No variable expansion in `.env` values.
- No `.env.local` or parent `.env` discovery.
- No full expression AST; control flow is still line and block based.
