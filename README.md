# CJTaskrunner

CJTaskrunner is a lightweight Rust task runner for project-local taskfiles (`cjtasks` or `*.cjtasks`).
It ships with two CLI names (`cj` and `cjtaskrunner`) plus an optional LSP server (`cjtaskrunner-lsp`).

## Basics First

- **Taskfile name:** `cjtasks` (preferred) or `*.cjtasks`.
- **Execution model:** each task line runs in order, stopping on the first non-zero exit.
- **Default command style:** command lines run directly as argv (no shell parsing unless you use `@shell`).
- **Directives:** behavior is configured with `@` directives.

Minimal `cjtasks` example:

```yaml
env:
  PORT?: 3000

setup:
  test -f package.json

dev:
  @task setup
  node server.js --port $PORT
```

## Installation

### Prerequisites

- Rust toolchain (Cargo)

### Install from this repository

```sh
cargo install --path .
```

Then use either command name:

```sh
cj --help
cjtaskrunner --help
```

### Build without installing

```sh
cargo build
cargo run -- --help
```

### Build the language server

```sh
cargo build --bin cjtaskrunner-lsp
```

## Usage

### Run tasks

```sh
cj <task>
cj <taskfile-or-directory> <task>
```

Examples:

```sh
cargo run -- example_tasks/node-vite envcheck
cargo run -- example_tasks/python-venv pyinfo
cargo run -- example_tasks/rust-cli check
```

### Useful CLI options

```sh
cj --default
cj --format [taskfile-or-directory]
cj --completions <bash|zsh|fish>
cj --install-completions <bash|zsh|fish>
cj help [section]
```

### Taskfile discovery behavior

- `cj <task>` searches only the current directory.
- `cj <directory> <task>` searches only that directory.
- `cj <taskfile> <task>` uses that exact file path.
- If both are present, `cjtasks` is preferred over `*.cjtasks`.
- If multiple `*.cjtasks` files exist, pass the explicit file path.

## Directives (Brief)

Supported directives:

- Flow/composition: `@task`, `@and`, `@or`, `@if`, `@else`, `@switch`, `@case`, `@default`, `@return`, `@stop`, `@success`, `@fail`
- Commands/filesystem: `@shell`, `@cd`, `@back`, `@echo`, `@clean`, `@mkdir`, `@cp`, `@cpdir`, `@rename`
- Variables/environment: `@set`, `@export`, `@unset`
- Documentation: `@desc`, `@help:`

For exact directive semantics and grammar rules, see `SPEC.md`.

## LSP and Editor Support

Run the language server:

```sh
cjtaskrunner-lsp
```

Current support includes diagnostics, directive/task completions, hover for directives, symbols, go-to-definition for `@task` references, and formatting.

A VS Code extension is included at `editors/vscode-cjtaskrunner`.

## License

MIT. Copyright (c) 2026 jgusta.
