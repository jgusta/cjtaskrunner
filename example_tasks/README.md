# CJTaskrunner Example Projects

These examples are small projects with `cjtasks` files. They are meant to show the capabilities of CJTaskrunner syntax. We don't recommend doing everything in CJTaskrunner, but for some really simple pesky tasks such as making 

Run an example from inside its directory:

```sh
cd example_tasks/node-vite
cj
cj check
```

## Features demonstrated

- Task arguments such as `run ($MODE)` and `build ($PROFILE)`.
- Hidden helper tasks beginning with `_`.
- Runtime variables with [`@set`](../docs/src/reference/directives.md#set), captured output with [`@set:`](../docs/src/reference/directives.md#set), [`@export`](../docs/src/reference/directives.md#export), and [`@unset`](../docs/src/reference/directives.md#unset).
- Conditional blocks with [`@if`](../docs/src/reference/directives.md#if), [`@if-not`](../docs/src/reference/directives.md#if-not), [`@if-in`](../docs/src/reference/directives.md#if-in), [`@if-not-in`](../docs/src/reference/directives.md#if-not-in),
  [`@if-exists`](../docs/src/reference/directives.md#if-exists), [`@if-not-exists`](../docs/src/reference/directives.md#if-not-exists), [`@if-version`](../docs/src/reference/directives.md#if-version), and [`@switch`](../docs/src/reference/directives.md#switch).
- Parallel branches with [`@await`](../docs/src/reference/directives.md#await) plus [`@or`](../docs/src/reference/directives.md#or) failure handling.
- Browser opening with [`@open`](../docs/src/reference/directives.md#open).
- File operations with [`@clean`](../docs/src/reference/directives.md#clean), [`@mkdir`](../docs/src/reference/directives.md#mkdir), [`@cp`](../docs/src/reference/directives.md#cp), and [`@cpdir`](../docs/src/reference/directives.md#cpdir).
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
