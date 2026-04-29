# CJTasks Plan

CJTasks is a Rust-based CLI task runner with executable `cj`. It runs named tasks from a local task file using a small CJTasks-specific format, deterministic environment merging, task-file-directory execution, `.env` loading, and Python virtual environment awareness.

Round 2 updates the original MVP command model:

- Ordinary task lines execute directly as argv by default.
- Shell behavior is explicit through `@shell`.
- CJTasks control flow uses `@` directives with no trailing colons.
- Variables interpolate with `$NAME`, `${NAME}`, and `${NAME:-fallback}`.
- Runtime variable state is managed with `@set`, `@export`, and `@unset`.
- Task composition uses `@task`.
- Conditional blocks use `@if`, `@else`, `@if-exists`, `@if-missing`, `@if-set`, and `@if-unset`.
- Switch blocks use `@switch`, `@case`, and `@default`.

The sections below describe the current implementation direction after the round 2 update.

This plan is written so a builder can implement the MVP without pausing for design decisions.

## MVP Decisions

- Executable name: `cj`.
- Recognized task file names: `cjt`, `cjtasks`.
- Task names are slugs: `^[A-Za-z0-9_-]+$`.
- `cj <task>` searches only the current directory.
- `cj <taskfile-or-directory> <task>` uses the given task file, or searches only the given directory.
- If both `cjt` and `cjtasks` exist in a searched directory, `cjt` wins.
- The task file's directory is always the command working directory.
- Ordinary command lines run as direct argv commands.
- Shell behavior is explicit with `@shell`, which uses `/bin/sh -c` on Unix.
- Each command line is independent; process state such as `cd` does not persist to the next line.
- Commands stop on the first non-zero exit status.
- Standard input, output, and error are inherited.
- `.env` is loaded from the task file directory only.
- `.env` values do not override inherited process environment variables.
- Task file env overrides always override inherited process env and `.env`.
- Task file env fallbacks apply only when a variable is absent, not when it is present with an empty value.
- A detected or already-active Python virtualenv prepends its executable directory to `PATH` after all other env merging.
- An already-active virtualenv takes precedence over a discovered local `.venv`.

## CLI Behavior

Supported invocations:

```text
cj <task>
cj <taskfile-or-directory> <task>
```

Single argument behavior:

- Treat the argument as the task name.
- Discover a task file from the current working directory only.
- Run the named task from the discovered task file.

Two argument behavior:

- Treat the second argument as the task name.
- Treat the first argument as either:
  - a direct path to a task file named `cjt` or `cjtasks`, if it is a file with one of those names, or
  - a directory containing a recognized task file.
- If the first argument is a directory, discover a task file in that directory only.

Out of scope for MVP:

- Multiple tasks in one invocation.
- Flags and options.
- Task arguments after the task name.
- Recursive parent-directory discovery.
- Shell selection by user configuration.

## Task File Discovery

Recognized task file names:

- `cjt`
- `cjtasks`

Discovery rules:

- For `cj <task>`, check the current working directory for recognized task files.
- For `cj <dir> <task>`, check the provided directory for recognized task files.
- For `cj <taskfile> <task>`, use the provided task file directly.
- If a searched directory contains both `cjt` and `cjtasks`, use `cjt`.
- The task file's directory is always the base directory for execution.

Direct task file paths:

- A direct task file path must point to a regular file.
- The file name must be exactly `cjt` or `cjtasks`.
- If the first two-argument value exists but is neither a recognized file nor a directory, return an error.

## Task File Syntax

The format is CJTasks-specific and intentionally only YAML-like. Do not use a general YAML parser for MVP; the accepted grammar is smaller and line-oriented.

### Tasks

- Task keys are top-level lines with a slug followed by `:`.
- Task names must match `^[A-Za-z0-9_-]+$`.
- Example task keys: `build:`, `dev:`, `test123:`, `build-prod:`.
- `env:` is reserved and cannot be used as a task name.

Commands:

- Task commands are non-empty lines indented by at least two spaces under the task key.
- The command text is the line after removing the leading two spaces.
- Multiple indented command lines run in order.
- Nested directive blocks use additional two-space indentation.
- Blank lines inside or between tasks are ignored.
- Comment-only lines are ignored.
- Inline comments are not stripped from command lines; `echo # hi` is passed to the command parser as written.

### Comments

- A comment-only line starts with optional spaces followed by `#`.
- Comments can be indented to match surrounding content.
- Comment-only lines are ignored by the parser.

### Global Environment Section

MVP supports one optional top-level `env:` section before, between, or after tasks. It applies to all tasks.

Environment entries are indented by exactly two spaces under `env:`:

```yaml
env:
  NAME: value
  FALLBACK?: fallback value
```

Environment entry rules:

- Override entry syntax: `  NAME: value`
- Fallback entry syntax: `  NAME?: value`
- Variable names must match `^[A-Za-z_][A-Za-z0-9_]*$`.
- The value is the remainder of the line after the first `:` with one optional leading space removed.
- Empty values are allowed and mean the empty string.
- Matching single or double quotes around the whole value may be stripped for convenience, but escape processing is not required for MVP.
- Inline comments are not stripped from env values.
- Duplicate env entries are an error.
- Multiple `env:` sections are an error.

Task-level env blocks are deferred. The MVP has global task-file env only.

### Example

```yaml
# Project tasks
env:
  NODE_ENV: development
  PORT?: 5173

dev:
  npm run vite

test:
  # Run Rust tests
  cargo test

build:
  npm run build
  cargo build --release
```

Round 2 directive example:

```yaml
deploy:
  @set MODE production
  @task build
  @if $MODE == production
    @shell ./scripts/deploy.sh > deploy.log
  @else
    echo skip
```

## Environment Handling

Build the effective environment in this order:

1. Start with the current `cj` process environment.
2. Load `.env` from the task file directory, adding only variables that are absent.
3. Apply global task-file fallback entries, adding only variables that are absent.
4. Apply global task-file override entries, replacing any existing value.
5. Apply Python virtual environment path adjustments.

Important details:

- "Absent" means the variable key is not present in the environment map.
- A present variable with an empty string value is not considered absent.
- Env variable names are case-sensitive in CJTasks' internal map. Platform-specific process behavior may still differ.
- MVP does not support task-level environment values.

## `.env` Loading

CJTasks reads `.env` from the task file's base directory if present.

MVP `.env` parsing:

- Accept `NAME=value` lines.
- Ignore blank lines and comment-only lines.
- Variable names must match `^[A-Za-z_][A-Za-z0-9_]*$`.
- Values are the remainder of the line after `=`.
- Matching single or double quotes around the whole value may be stripped.
- Escape processing and variable expansion are not required.
- `export NAME=value` is not required.
- `.env.local`, task-specific env files, and parent-directory `.env` discovery are out of scope.
- Invalid `.env` lines should return a clear error with file path and line number.

## PATH and Executable Behavior

Ordinary commands execute directly with `std::process::Command` after CJTasks splits the line into argv tokens and performs safe interpolation:

- `npm run vite`
- `cargo test`
- `python -m pytest`

Shell-only behavior requires `@shell`:

- Pipes.
- Redirects.
- Globbing.
- Command chaining.
- Shell builtins.

Execution rules:

- The child process working directory is the task file directory.
- The child environment is the effective environment described above.
- Executables are resolved through the effective `PATH`.
- Interpolated values in ordinary commands are inserted as single argv values.
- Interpolated values in `@shell` are shell-quoted before `/bin/sh -c` sees them.

Non-Unix direct argv execution should use the platform process API. If Windows `@shell` support is added later, define the platform shell strategy explicitly.

## Python Virtual Environment Awareness

CJTasks should make Python virtual environments visible by adjusting `PATH`.

Detection order:

1. If `VIRTUAL_ENV` is set and non-empty in the effective environment, use that virtualenv.
2. Else if `CJ_VENV` is set and non-empty, use that path as the virtualenv directory.
3. Else if `.venv` exists as a directory under the task base directory, use it.
4. Else make no virtualenv adjustment.

Path adjustment:

- On Unix, prepend `<venv>/bin` to `PATH`.
- If `PATH` is absent, set it to the virtualenv executable directory.
- Set or update `VIRTUAL_ENV` to the selected virtualenv path.
- Leave `PYTHONHOME` untouched for MVP.
- Do not walk parent directories for `.venv` in MVP.
- If the selected virtualenv directory exists but its executable directory does not, return an actionable error.

## Execution Model

For each invocation:

1. Validate argument count.
2. Resolve the task file.
3. Set the task file's directory as the base directory.
4. Parse the task file.
5. Validate and resolve the requested task by name.
6. Build the effective environment.
7. Run the task's steps in order from the base directory.
8. Stop on the first failing command.
9. Exit with the failing command status, or `0` if all commands succeed.

Command semantics:

- Ordinary commands execute directly as child processes.
- `@shell` commands execute as their own shell process.
- Process state does not persist across command lines.
- Standard input, output, and error are inherited.
- If a command is terminated by signal on Unix, return a non-zero failure code and include the signal in the error message when possible.

## Error Handling

CJTasks should return clear, actionable errors for:

- Invalid CLI argument count.
- No task file found.
- First two-argument value is neither a recognized task file nor a directory.
- Task file parse failure.
- Requested task not found.
- Invalid task name syntax.
- Reserved task name such as `env`.
- Invalid indentation.
- Invalid or unsupported environment declaration syntax.
- Duplicate task names.
- Duplicate env entries.
- Invalid `.env` line.
- Selected virtualenv has no executable directory.
- Command spawn failure.
- Command exits with non-zero status.

Errors should include:

- The path to the task file when relevant.
- The task name when relevant.
- Line and column information for parse errors when possible.
- The failing command and exit status for command failures.

## Implementation Phases

### Phase 1: Minimal Runner

- Create Rust CLI crate for executable `cj`.
- Implement argument parsing for one-argument and two-argument forms.
- Implement task file discovery for `cjt` and `cjtasks`, with `cjt` precedence.
- Parse task keys, comment-only lines, blank lines, and exactly two-space-indented command lines.
- Run ordinary commands directly from the task file's directory.
- Add explicit `@shell` for shell-dependent commands.
- Inherit process environment and current `PATH`.
- Report basic errors.

### Phase 2: Environment and `.env`

- Add `.env` loading from the base directory.
- Add global `env:` syntax for overrides and fallbacks.
- Implement the environment merge order from this plan.
- Add tests for process env, `.env`, fallback, and override precedence.

### Phase 3: Python Virtual Environment Awareness

- Detect `VIRTUAL_ENV`, `CJ_VENV`, and base-directory `.venv`.
- Prepend the selected virtualenv executable directory to `PATH`.
- Set or update `VIRTUAL_ENV`.
- Add tests for active env precedence and local `.venv` detection.

### Phase 4: Parser and Diagnostics Hardening

- Improve parse errors with line and column details.
- Add validation for task names, indentation, comments, reserved names, duplicate tasks, and unsupported constructs.
- Add fixtures for valid and invalid task files.

### Phase 5: Later Compatibility

- Consider task arguments such as `cj test -- --filter foo`.
- Consider parent-directory taskfile discovery.
- Consider task-level env blocks.
- Consider `.env.local` and additional env file names.
- Consider non-Unix shell behavior.

## Test Checklist

- `cj dev` finds `./cjt` before `./cjtasks`.
- `cj ./some/dir dev` runs from `./some/dir` when that directory contains `cjt`.
- `cj ./some/dir/cjt dev` uses that exact task file.
- Unknown task returns a not-found error listing the requested task.
- Invalid task name returns a validation error before command execution.
- Commands run sequentially and stop after the first failure.
- `@shell cd subdir` on one line does not affect the next line.
- `npm run vite` runs as direct argv and uses `PATH`.
- `.env` adds absent variables but does not replace process variables.
- `env:` fallback entries add absent variables but do not replace empty present variables.
- `env:` override entries replace existing values.
- `VIRTUAL_ENV` takes precedence over `CJ_VENV` and `.venv`.
- `.venv/bin` is prepended to `PATH` when no active virtualenv exists.

## Resolved Questions

- Env override syntax: top-level `env:` section with `NAME: value`.
- Missing-variable fallback syntax: top-level `env:` section with `NAME?: value`.
- If both `cjt` and `cjtasks` exist, `cjt` wins.
- Task discovery searches only the current or explicitly provided directory.
- `.env` does not override inherited process environment variables.
- `.env.local` and other env file names are not supported in MVP.
- Ordinary commands run as direct argv; `@shell` runs through `/bin/sh -c` on Unix.
- Each command line is independent and does not share process state.
- Task names allow letters, numbers, hyphens, and underscores.
- Task arguments are deferred.
- Alternate Python virtualenv variable for CJTasks is `CJ_VENV`; active Python env uses standard `VIRTUAL_ENV`.
- An active virtual environment overrides a discovered `.venv`.
