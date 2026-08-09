# Integrations

## Auto discover

`cj --auto` imports common task definitions from `package.json`, `deno.json`, `Makefile`, and argument-free `Justfile` recipes. It creates `cjtasks` when no base taskfile exists, otherwise it appends missing tasks to `cjtasks`.

`package.json` scripts are considered first, followed by Deno, Make, and Just tasks. Each wrapper takes the shortest available normalized name. Name or directory conflicts add a number without a separator: `build`, `build2`, `build3`. Existing CJ tasks are never overwritten. This command is **not** idempotent; you probably don't want to run it more than once as it will create new tasks each time it runs.

See [CLI Reference](../reference/cli.md).

## Python virtual environments

CJTaskrunner adjusts `PATH` when it finds an active or project-local Python
virtual environment. Detection uses `VIRTUAL_ENV`, then `CJ_VENV`, then a
`.venv` directory beside the taskfile.

Tasks run with the Python executable from inside the virtual environment.

```cjtasks
setup:
  python -m venv .venv
  python -m pip install -e .

test:
  python -m pytest
```

See the
[Python virtual environment example](../../../example_tasks/python-venv/README.md).

## Node and Vite

Node package scripts remain the source of truth. CJTaskrunner can group them
with setup or validation steps:

```cjtasks
setup:
  npm install

dev:
  @task setup
  npm run dev

check:
  npm run build
```

See the [Vite example](../../../example_tasks/node-vite/README.md).

## Rust and Cargo

Cargo commands can be exposed alongside the rest of a repository's workflows:

```cjtasks
check:
  cargo fmt --check
  cargo test --locked
  cargo clippy --all-targets --all-features --locked -- -D warnings
```

See the [Rust CLI example](../../../example_tasks/rust-cli/README.md).

## Continuous integration

Install `cj` in the CI environment, then invoke the same public tasks used
locally:

```sh
cargo install --path .
cj check
```

Keep CI-only mechanics in underscore-prefixed tasks when they should remain
callable without appearing in summary mode.

## Editors

The [VS Code extension](../../../editors/vscode-cjtaskrunner/README.md) provides syntax highlighting, Outline symbols, task execution, language-server support, formatting, and an explorer panel.

See [Ecosystem](ecosystem.md).
