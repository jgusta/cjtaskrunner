# CJTasks

CJTasks is a small Rust task runner with the executable name `cj`. It runs named tasks from a project-local taskfile named `cjt` or `cjtasks`, merges environment values in a predictable order, loads a local `.env`, and makes Python virtual environments easier to use by adjusting `PATH`.

The MVP is intentionally simple: it is a line-oriented task runner, not a shell replacement, Make clone, or full YAML interpreter.

## Install, Build, and Run

Build the CLI:

```sh
cargo build
```

Run it through Cargo:

```sh
cargo run -- <task>
cargo run -- <taskfile-or-directory> <task>
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
```

## Taskfile Discovery

CJTasks recognizes two taskfile names:

- `cjt`
- `cjtasks`

Discovery is deliberately local:

- `cj <task>` looks only in the current directory.
- `cj <directory> <task>` looks only in the given directory.
- `cj <taskfile> <task>` uses that exact file, but the file must be named `cjt` or `cjtasks`.
- If both `cjt` and `cjtasks` exist in a searched directory, `cjt` wins.
- Commands always run with the taskfile's directory as the working directory.

There is no parent-directory search in the current MVP.

## Taskfile Syntax

The format is CJTasks-specific and YAML-like, but not general YAML. Top-level keys end with `:`. Indented entries must use exactly two spaces.

```yaml
env:
  NODE_ENV: development
  PORT?: 5173

base:
  test -f package.json
  test -f src/main.js

dev:
  npm run dev -- --host 127.0.0.1 --port "$PORT"
```

Tasks:

- Task names are ASCII letters and digits only, such as `build`, `test`, or `test123`.
- `env` is reserved for the global environment section.
- Each non-empty command line under a task runs in order.
- Commands run through `/bin/sh -c`.
- Standard input, output, and error are inherited.
- Execution stops at the first command that exits non-zero.
- Each command line is independent, so `cd`, shell variables, and `export` do not persist to the next taskfile command line.

Comments and blank lines:

- Blank lines are ignored.
- Comment-only lines are ignored.
- Inline comments are not stripped from command or environment values.

Environment entries:

- `NAME: value` is an override and replaces inherited values.
- `NAME?: value` is a fallback and applies only when the variable is absent.
- Environment names must start with a letter or underscore, then use only letters, digits, and underscores.
- Matching single or double quotes around a whole value are stripped for convenience.

## Environment and .env Behavior

For every task, CJTasks builds the child environment in this order:

1. Start with the current `cj` process environment.
2. Load `.env` from the taskfile directory, adding only variables that are absent.
3. Apply taskfile `NAME?: value` fallbacks, adding only variables that are absent.
4. Apply taskfile `NAME: value` overrides, replacing existing values.
5. Apply Python virtual environment path handling.

The `.env` parser accepts `NAME=value` lines, ignores blank lines and comment-only lines, and does not expand variables. It only reads `.env` beside the taskfile; it does not search parents and does not load `.env.local`.

## Python Virtual Environment Behavior

CJTasks can prepend a virtualenv executable directory to `PATH`.

Detection order:

1. Use active `VIRTUAL_ENV` when it is set and non-empty.
2. Else use `CJ_VENV` when it is set and non-empty.
3. Else use `.venv` under the taskfile directory when it exists.
4. Else leave Python paths alone.

On Unix-like systems, the selected virtualenv contributes `<venv>/bin` to the front of `PATH`, and `VIRTUAL_ENV` is set to the selected directory. If a selected virtualenv exists without a `bin` directory, CJTasks returns an error instead of silently continuing.

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

## Current MVP Limitations

- No command-line flags.
- No task arguments after the task name.
- No multiple-task invocation.
- No parent-directory taskfile discovery.
- No task dependencies or task composition.
- No task-level environment blocks.
- No shell configuration; Unix commands use `/bin/sh -c`.
- No Windows shell strategy yet.
- No general YAML parsing.
- No variable expansion in `.env` or taskfile environment values.
- No `.env.local` or parent `.env` discovery.
