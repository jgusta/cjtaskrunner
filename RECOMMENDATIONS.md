# CJTasks Round 2 Format Recommendations

This document records the round 2 direction for extending the CJTasks task file format with direct argv execution, explicit shell execution, safe interpolation, conditionals, switch statements, runtime variables, and task composition.

## Core Principle

Keep ordinary command lines looking like shell commands, but execute them as direct argv commands by default. Make CJTasks-only control syntax visually distinct.

Recommended rule:

- If a task body line starts with a reserved marker such as `@`, parse it as a CJTasks directive.
- Otherwise, parse it as a command plus argv and execute it directly.

This avoids shell injection by default while still keeping common task lines familiar.

## Recommended Meta Syntax

Use `@` for CJTasks directives.

Only top-level task keys and `env:` use trailing colons. Directive lines do not use trailing colons.

Example:

```yaml
env:
  MODE?: development
  PORT?: 5173

dev:
  @task setup
  @if $MODE == production
    npm run build
    npm run preview -- --port $PORT
  @else
    npm run dev -- --port $PORT

setup:
  test -f package.json
  npm install
```

Normal commands stay normal:

```yaml
build:
  cargo build --release
  cp target/release/cj ./bin/cj
```

CJTasks behavior stays explicit:

```yaml
deploy:
  @task test
  @task build
  @switch $DEPLOY_TARGET
    @case staging
      ./scripts/deploy-staging.sh
    @case prod
      ./scripts/deploy-prod.sh
    @default
      echo "unknown target: $DEPLOY_TARGET"
      exit 1
```

Shell behavior is explicit:

```yaml
bundle:
  @shell mkdir -p dist && cat src/*.js > dist/app.js
```

## Variable Interpolation

Use `$NAME` for common variable references, and allow `${NAME}` when braces are needed to separate the variable name from surrounding text.

```text
$NAME
${NAME}
${NAME:-fallback}
```

Recommended MVP behavior:

- `$NAME` expands to the environment variable named `NAME`.
- `${NAME}` errors if the variable is missing.
- `${NAME:-fallback}` uses the fallback if the variable is missing or empty.
- Interpolation applies to direct command argv tokens, `@shell` command text, and directive arguments.
- Interpolated values are inserted as one atomic string value, never as raw shell text.
- Literal interpolation can be escaped with `\$NAME` or `\${NAME}`.

This is a security and predictability rule. If `NAME` has the value `-p dir/mydir`, then this task:

```yaml
example:
  mkdir $NAME
```

must behave as if the user passed exactly one argument with the literal value `-p dir/mydir` to `mkdir`. It must not become:

```sh
mkdir -p dir/mydir
```

and it must not allow injected shell syntax such as pipes, redirects, semicolons, command substitutions, or extra arguments.

Implementation note: ordinary command lines should be parsed into argv tokens and executed with `std::process::Command`. `@shell` is the explicit escape hatch for commands that intentionally need shell features. In `@shell` command text, interpolated values must be shell-escaped before the shell sees them.

Avoid complex shell-like expansion in the CJTasks layer for now:

- No pattern replacement such as `${VAR/foo/bar}`.
- No command substitution.
- No arithmetic expansion.
- No nested expansion unless there is a strong later need.

Example:

```yaml
env:
  APP_NAME?: cjtasks
  BUILD_DIR?: target/release

release:
  cargo build --release
  cp ${BUILD_DIR}/cj dist/${APP_NAME}
```

## Task Composition

Use `@task` to call another task from the same task file.

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

Recommended semantics:

- `@task name` runs another task through CJTasks, not through the shell.
- The called task uses the same task file and base directory.
- The called task receives the effective environment.
- Detect and report recursive task cycles.
- Stop the parent task if the called task fails.

There are two useful composition modes:

```yaml
ci:
  @task fmt
  @task test
```

This uses CJTasks semantics directly.

```yaml
ci-shell:
  cj fmt
  cj test
```

This shells out to the installed `cj` executable if written under `@shell`, or runs direct argv commands if written as ordinary lines. It is useful, but less direct than `@task`.

## Command Execution Model

Ordinary task lines execute directly by default. CJTasks should split each ordinary command line into argv tokens, expand variables into single argv tokens, and run the command with `std::process::Command`.

Example:

```yaml
build:
  cargo build --release
  npm run build
```

These should execute as:

```text
["cargo", "build", "--release"]
["npm", "run", "build"]
```

This keeps commands familiar while avoiding shell interpretation. The following line passes one literal argument after `mkdir`:

```yaml
example:
  mkdir $NAME
```

If `NAME` is `-p dir/mydir`, CJTasks executes:

```text
program: mkdir
argv: ["mkdir", "-p dir/mydir"]
```

It does not execute:

```text
/bin/sh -c "mkdir -p dir/mydir"
```

Shell-only syntax must use `@shell`:

```yaml
write-file:
  @shell echo hi > out.txt

pipeline:
  @shell cat input.txt | sort | uniq > output.txt
```

Recommended semantics:

- Ordinary lines are direct argv execution.
- `@shell` lines run through the configured shell.
- Start with `/bin/sh -c` on Unix for `@shell`.
- Variable interpolation in ordinary lines is argv-safe.
- Variable interpolation in `@shell` is shell-quoted before execution.
- Redirection, pipes, command chaining, glob expansion, and shell builtins require `@shell`.
- A shell-looking token such as `>` in an ordinary line is just a literal argv token.

Recommended parsing/library direction:

- Use a shell-word parser such as Rust's `shlex` crate to split ordinary lines into words.
- Track variable references during or after tokenization so each interpolation remains a single argv token.
- Use `std::process::Command` for ordinary lines.
- Use shell quoting, such as `shell-quote` or `shlex::try_quote`, when interpolating values into `@shell` text.

Task arguments can be added later, but should not be part of the first composition MVP unless they are immediately needed.

Possible future syntax:

```yaml
deploy:
  @task build release

build:
  cargo build --profile ${1:-dev}
```

## Variables And Assignment

CJTasks should maintain a run-global variable map. Execution order matters: variables set by earlier directives are visible to later commands, conditionals, switches, `@shell` lines, and composed tasks.

Use `@set` to assign a CJTasks variable:

```yaml
build:
  @set MODE production
  @task compile

compile:
  @if $MODE == production
    cargo build --release
  @else
    cargo build
```

Recommended `@set` syntax:

```text
@set NAME value
```

Recommended `@set` semantics:

- `NAME` is written without `$`.
- The value is the rest of the directive line after the variable name.
- Values are strings.
- `$VAR` and `${VAR}` inside the value are interpolated before assignment.
- `@set` updates the run-global CJTasks variable map.
- `@set` does not automatically export the variable to child process environments.
- Later CJTasks directives and interpolations see the updated value.
- Called tasks see the updated value.
- Changes made inside called tasks remain visible after the called task returns.

Use `@export` to make a variable available to later process executions:

```yaml
run:
  @set MODE development
  @export MODE
  npm run dev
```

Recommended `@export` syntax:

```text
@export NAME
@export NAME value
```

Recommended `@export` semantics:

- `@export NAME` exports the current CJTasks variable value for `NAME`.
- `@export NAME value` sets `NAME` and exports it in one step.
- Exported variables are included in the environment for later ordinary command lines and `@shell` lines.
- Exported variables are visible to later composed tasks.
- Export state is global and order-dependent.
- Exporting an unset variable is an error unless a value is provided.

Use `@unset` to remove a variable:

```yaml
clean-env:
  @unset DEBUG
  @task build
```

Recommended `@unset` semantics:

- `@unset NAME` removes the variable from the CJTasks variable map.
- If `NAME` was exported, it is removed from the exported environment overlay for later process executions.
- Unsetting a missing variable is not an error.

The initial run-global variable map should be built from the existing environment handling rules:

1. Start with the inherited process environment.
2. Apply `.env` absent-only values.
3. Apply task file fallbacks and overrides from `env:`.
4. Apply Python virtual environment path adjustments.
5. Execute task lines in order, allowing `@set`, `@export`, and `@unset` to mutate later state.

The `env:` block should continue to define initial environment values. Runtime mutation belongs in task bodies through `@set`, `@export`, and `@unset`.

Example of order-dependent global state:

```yaml
a:
  @set MODE production
  @task b

b:
  echo $MODE
```

This prints `production`.

```yaml
a:
  @task b
  @set MODE production

b:
  echo $MODE
```

This prints the previous value of `MODE`, because `@set MODE production` has not run yet.

## Conditionals

Keep conditionals intentionally small. Do not start with a full expression language.

Recommended examples:

```yaml
install:
  @if-exists package-lock.json
    npm ci
  @else
    npm install
```

```yaml
build:
  @if $MODE == production
    npm run build
  @else
    npm run build:dev
```

Useful MVP conditions:

```text
$VAR == value
$VAR != value
${VAR} == value
${VAR} != value
@if-exists path
@if-missing path
@if-set $VAR
@if-unset $VAR
```

Recommended semantics:

- Paths in `@if-exists` and `@if-missing` are relative to the task file base directory.
- String comparisons are literal after interpolation.
- Missing variables in `$VAR` or `${VAR}` should produce a clear error in comparisons.
- Use `${VAR:-fallback}` when missing values are acceptable.
- `@if-set` and `@if-unset` check the run-global variable map, including values created by `@set`.
- Only one `@else` is allowed per `@if`.
- Nested conditionals can be supported after the parser has block handling.

## Switch Statements

Switch statements are useful for env-driven task selection.

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
      echo "unknown APP_KIND=$APP_KIND"
      exit 1
```

Recommended MVP behavior:

- The switch value is interpolated first.
- `@case` values are literal strings.
- No glob matching or regex matching at first.
- At most one matching case runs.
- `@default` runs if no case matches.
- Missing `@default` is allowed; no matching case means no commands run.

## Syntax Boundary

Do not let meta syntax masquerade as shell.

Good:

```yaml
check:
  @if-exists Cargo.toml
    cargo test
```

Risky:

```yaml
check:
  if exists Cargo.toml:
    cargo test
```

The second form looks like shell or YAML but is neither. The `@` marker makes the CJTasks language boundary obvious.

## Suggested MVP Grammar

Informal grammar:

```text
file        = item*
item        = env_block | task_block | comment | blank

task_block  = task_name ":" newline body_line*
task_name   = [A-Za-z0-9_-]+

body_line   = indent command_line
            | indent directive_line
            | indent comment
            | blank

directive_line = "@task" task_name
               | "@shell" shell_text
               | "@set" variable value
               | "@export" variable value?
               | "@unset" variable
               | "@if" condition
               | "@else"
               | "@if-exists" path
               | "@if-missing" path
               | "@if-set" variable
               | "@if-unset" variable
               | "@switch" value
               | "@case" value
               | "@default"
```

The current implementation uses two-space indentation for task command lines. Once block syntax is added, the parser should define indentation levels explicitly:

- Top-level task keys and `env:` start at column 0.
- Task keys and `env:` require trailing colons.
- CJTasks directive lines do not use trailing colons.
- Task body lines start at two spaces.
- Nested directive bodies use additional two-space indentation.

Example:

```yaml
deploy:
  @switch $DEPLOY_TARGET
    @case staging
      ./scripts/deploy-staging.sh
    @case prod
      ./scripts/deploy-prod.sh
```

## Task Names

The original plan says task names are alphanumeric slugs. In practice, allow hyphen and underscore:

```text
[A-Za-z0-9_-]+
```

This supports common names such as:

```text
build-prod
db_migrate
test123
```

## Implementation Order

Recommended implementation sequence:

1. Add interpolation for command lines and directive arguments.
2. Change ordinary command execution from `/bin/sh -c` to direct argv execution.
3. Add `@shell` for explicit shell execution.
4. Add run-global variables with `@set`, `@export`, and `@unset`.
5. Add `@task` composition with cycle detection.
6. Add parser support for nested blocks.
7. Add `@if` and `@else`.
8. Add `@switch`, `@case`, and `@default`.
9. Add richer diagnostics for directive parse errors.

This order keeps each step testable and avoids requiring the full control-flow parser before composition is useful.

## Summary Recommendation

Use:

- `@task name` for composition.
- Direct argv execution for ordinary command lines.
- `@shell ...` for explicit shell execution.
- `@set NAME value`, `@export NAME`, `@export NAME value`, and `@unset NAME` for order-dependent global variable state.
- `@if ...` and `@else` for conditionals.
- `@switch ...`, `@case`, and `@default` for branching.
- `$VAR`, `${VAR}`, and `${VAR:-fallback}` for interpolation.
- Simple predicate directives such as `@if-exists path`, `@if-missing path`, `@if-set $VAR`, and `@if-unset $VAR`.

This gives CJTasks a small language without turning it into a shell clone.
