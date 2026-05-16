# cjtasks Specification

This document specifies CJTaskrunner taskfiles.

## Names

- Project: `CJTaskrunner`
- Executables: `cj`, `cjtaskrunner`
- Default taskfile: `cjtasks`
- Additional taskfiles: files ending in `.cjtasks`

## Invocation

```text
cj <task>
cj <taskfile-or-directory> <task>
cjtaskrunner <task>
cjtaskrunner <taskfile-or-directory> <task>
```

Single-argument invocation treats the argument as a task name and discovers a taskfile in the current directory only.

Two-argument invocation treats the second argument as the task name. The first argument must be either:

- a directory containing a discoverable taskfile, or
- a taskfile named `cjtasks` or ending in `.cjtasks`

Task names must contain only ASCII letters, digits, hyphens, and underscores.

## Taskfile Discovery

When discovering inside a directory:

1. If `cjtasks` exists, use it.
2. Else if exactly one `*.cjtasks` file exists, use it.
3. Else if multiple `*.cjtasks` files exist, error and require an explicit taskfile path.
4. Else error.

Discovery does not search parent directories.

Tasks start with the selected taskfile's directory as the working directory.

## File Format

The format is line-oriented and YAML-like, but not general YAML.

Top-level entries are either:

- `env:`
- `<task-name>:`

Blank lines are ignored. Comment-only lines start with optional spaces followed by `#` and are ignored.

Indented entries must use an even number of spaces, at least two. Tabs are not part of the format.

Inline comments are not stripped. `echo # hi` passes `#` and `hi` as command arguments.

## Global Environment Section

There may be at most one `env:` section. It applies to all tasks.

Entries are indented exactly two spaces:

```yaml
env:
  NAME: value
  FALLBACK?: fallback value
```

Environment names must match:

```text
^[A-Za-z_][A-Za-z0-9_]*$
```

Entry forms:

- `NAME: value` overrides inherited values.
- `NAME?: value` sets fallback values only when the variable is absent.

Value rules:

- Value is the remainder after the first `:`.
- One leading space after `:` is removed when present.
- Empty values are allowed.
- Matching single or double quotes around the whole value are stripped.
- Escape processing is not performed.
- Inline comments are retained as value text.
- Duplicate environment entries are errors.

## Tasks

Task keys are top-level lines:

```yaml
build:
  cargo build
```

Task names must match:

```text
^[A-Za-z0-9_-]+$
```

`env` is reserved and cannot be used as a task name.

Task lines are non-empty indented lines under a task. They run in order. Execution stops on first non-zero status unless a *same-level* `@or` handles it.

Semicolons split multiple *same-level* expressions on one physical line. Semicolons inside single or double quotes are preserved.

Semicolons *cannot* replace a newline where an indentation increase follows immediately.

```yaml
run:
  @set MODE prod; @if $MODE == prod
    cargo build --release
```

## Ordinary Commands

Ordinary task lines execute directly as argv commands through the platform process API.

```yaml
build:
  cargo build --release
```

This executes as:

```text
["cargo", "build", "--release"]
```

Ordinary commands do not interpret shell syntax:

- pipes
- redirects
- globbing
- command chaining
- shell builtins

Use `@shell` for shell behavior.

Each command line is independent. Process state such as `cd` does not persist to later taskfile lines.

Child stdin, stdout, and stderr are inherited unless a command is being captured by `@set NAME:`.

## Word Splitting

CJTaskrunner splits command and directive arguments with shell-like quote handling:

- Whitespace separates words outside quotes.
- Single and double quotes group text.
- Quotes are removed.
- Backslash can escape the next character.
- Unterminated quotes are errors.

This is not a full shell parser.

## Interpolation

Interpolation applies to ordinary command argv tokens, `@shell` text, and directive arguments.

Supported forms:

```text
$NAME
${NAME}
${NAME:-fallback}
```

Rules:

- `$NAME` reads the current CJTaskrunner variable value, or an empty string when absent.
- `${NAME}` reads the current variable value and errors when absent.
- `${NAME:-fallback}` uses `fallback` when `NAME` is missing or empty.
- `\$NAME` and `\${NAME}` escape interpolation.
- In ordinary commands, an interpolated value remains one argv value.
- In `@shell`, interpolated values are shell-quoted before `/bin/sh -c`.

CJTaskrunner does not support shell-style command substitution, arithmetic expansion, pattern replacement, or nested expansion.

## Directives

Directive lines start with `@`.

Most directives do not use trailing colons. `@set NAME:` is the block-capture form.

Nested directive bodies use two additional spaces of indentation.

### `@shell`

```yaml
bundle:
  @shell mkdir -p dist && cat src/*.js > dist/app.js
```

Runs the interpolated command through `/bin/sh -c` on Unix.

### `@desc`

```yaml
build:
  @desc compile project
  cargo build
```

Defines task description metadata. `cj` and `cjtaskrunner` show it when run without a task name. `@desc` does not run a command.

### `@task`

```yaml
ci:
  @task fmt
  @task test
```

Runs another task from the same taskfile with the current working directory and runtime variable state.

Recursive task cycles are errors.

The called task inherits the current working directory. Directory changes made inside the called task reset when it returns.

### `@cd`, `@back`

```yaml
build-docs:
  @cd docs
  npm run build
  @back
```

- `@cd path` changes the current working directory.
- Relative `@cd` paths resolve from the current working directory.
- `@back` undoes one `@cd` in the current scope.
- `@back` does nothing at the root directory for the current scope.
- Directory changes persist for later commands in the same block.
- Nested blocks inherit the parent directory and restore their starting directory when the block ends.
- Tasks inherit the caller's current directory and restore their starting directory when they return.

### `@set`, `@export`, `@unset`

```yaml
run:
  @set MODE production
  @export MODE
  @unset MODE
```

- `@set NAME value` sets a runtime variable for later CJTaskrunner interpolation and directives. It does not export the value to child processes.
- `@export NAME` exports an existing runtime variable to later child processes.
- `@export NAME value` sets and exports a value in one step.
- `@unset NAME` removes the runtime variable and export overlay.

Runtime state is order-dependent and shared with composed tasks.

### `@set NAME:` Capture

```yaml
capture:
  @set RESULT:
    @shell printf captured
  @echo $RESULT
```

Runs the indented block with stdout capture enabled. Captured stdout is stored in `NAME`. Trailing `\r` and `\n` characters are trimmed.

The capture fails when the block's final status is non-zero.

### `@echo`, `@clean`, `@stop`

```yaml
clean:
  @clean dist
  @echo cleaned
```

- `@echo text` writes text plus newline to stdout after interpolation.
- `@clean path` removes one file or directory relative to the current working directory. Missing paths are not errors.
- `@stop text` writes text plus newline when text is provided, then returns status `1`.

### `@return`, `@success`, `@fail`

```yaml
run:
  @return true
```

- `@return value` writes `value` without adding a newline and returns a status derived from it.
- `@return` with an indented block runs that block and returns its status.
- `@success` returns status `0`.
- `@fail` returns status `1`.

Status derivation for `@return value`:

- `true` -> `0`
- `false` -> `1`
- numeric value -> that status code
- other truthy string -> `0`
- empty string, `0`, or `false` -> `1`

### `@and`, `@or`

```yaml
build-or-clean:
  @task build; @and
    @echo build ok
  @or
    @clean dist
```

- `@and` runs its block only when the previous expression returned `0`.
- `@or` runs its block only when the previous expression returned non-zero.
- Skipped `@and` returns `1`.
- Skipped `@or` returns `0`.
- A failed expression may be followed by same-level `@or`; otherwise non-zero status stops execution.

### Conditionals

```yaml
build:
  @if $MODE == production
    cargo build --release
  @else
    cargo build
```

Supported forms:

```text
@if value
@if left == right
@if left != right
@if-exists path
@if-missing path
@if-set NAME
@if-unset NAME
@else
```

Truthiness: empty string, `0`, and `false` are false. Other values are true.

Paths in `@if-exists` and `@if-missing` are relative to the current working directory.

### Switches

```yaml
serve:
  @switch $APP_KIND
    @case node
      npm run dev
    @case rust
      cargo run
    @default
      @echo unknown app kind
```

`@switch` takes one value. `@case` takes one value. `@default` takes no arguments.

At most one case body runs. If no case matches, `@default` runs when present.

## Environment Merge

For each invocation, CJTaskrunner builds the effective environment in this order:

1. Start with the current process environment.
2. Load `.env` from the taskfile directory, adding only absent variables.
3. Apply taskfile fallback entries, adding only absent variables.
4. Apply taskfile override entries, replacing existing values.
5. Apply Python virtual environment path adjustment.

"Absent" means the key is not present. A present empty string is not absent.

Runtime variables and exported child-process environment start from this effective environment.

## `.env`

CJTaskrunner reads `.env` from the taskfile directory only.

Accepted lines:

```text
NAME=value
```

Rules:

- Blank lines are ignored.
- Comment-only lines are ignored.
- Names must match `^[A-Za-z_][A-Za-z0-9_]*$`.
- Values are the text after `=`.
- Matching single or double quotes around the whole value are stripped.
- Escape processing and interpolation are not performed.
- Invalid lines are errors with path and line number.

`.env.local`, parent `.env` discovery, and task-specific env files are not part of this spec.

## Python Virtual Environments

CJTaskrunner adjusts `PATH` for Python virtual environments.

Detection order:

1. Use `VIRTUAL_ENV` when set and non-empty.
2. Else use `CJ_VENV` when set and non-empty.
3. Else use `.venv` under the taskfile directory when it exists.
4. Else make no adjustment.

On Unix-like systems:

- prepend `<venv>/bin` to `PATH`
- set `VIRTUAL_ENV` to the selected virtualenv directory

If a selected virtualenv exists without a `bin` directory, execution errors.

## Errors

Errors should include relevant path, line, task, directive, or command context when available.

Errors include:

- invalid invocation
- missing taskfile
- ambiguous `*.cjtasks` discovery
- unrecognized explicit taskfile name
- invalid task name
- duplicate task
- duplicate env entry
- invalid indentation
- invalid directive syntax
- unknown directive
- invalid `.env` line
- missing variable in `${NAME}` interpolation
- recursive `@task` cycle
- failed command spawn
- selected virtualenv missing `bin`

Child process non-zero statuses are returned as CJTaskrunner exit statuses.
