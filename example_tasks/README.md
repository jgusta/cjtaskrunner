# CJTaskrunner Example Projects

These examples are small projects with real `cjtasks` files. They are meant to
show current CJTaskrunner syntax, not just wrap package-manager commands.

Run an example from inside its directory:

```sh
cd example_tasks/node-vite
cj
cj check
```

## What The Examples Show

- Task arguments such as `run ($MODE)` and `build ($PROFILE)`.
- Hidden helper tasks beginning with `_`.
- Runtime variables with `@set`, captured output, `@export`, and `@unset`.
- Conditional blocks with `@if`, `@if-not`, `@if-in`, `@if-not-in`,
  `@if-exists`, `@if-not-exists`, `@if-version`, and `@switch`.
- Parallel branches with `@await` plus `@or` failure handling.
- Browser opening with `@open`.
- File operations with `@clean`, `@mkdir`, `@cp`, and `@cpdir`.
- Version headers and guarded version bump flows.

## Examples

- `node-vite`: parallel dev startup, runtime URLs, mode arguments, and browser opening.
- `node-ssr`: static rendering, captured environment summaries, version checks, and file cleanup.
- `docker-basic`: image tag arguments, exported `IMAGE_REF`, Compose helpers, and safe validation.
- `python-venv`: venv discovery, exported app messages, packaging with filesystem directives.
- `python-cli-venv`: explicit CLI modes, build staging, bytecode compilation, and cleanup.
- `python-pipenv`: safe checks without installing, hidden Pipenv prerequisite checks, exported messages.
- `pyside6-app`: GUI-friendly Qt platform selection and exported window titles.
- `rust-cli`: build profile arguments, parallel CI, version conditionals, and guarded bumps.

Most `check` tasks avoid network installs and heavy tools. Tasks such as Docker
builds, npm builds, PySide installs, Pipenv runs, and Rust release builds still
require their normal external tools.
