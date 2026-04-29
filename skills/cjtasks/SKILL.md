---
name: cjtasks
description: Work with CJTaskrunner, a one-file executable named cj that uses a cjtasks file as a simple, self-discovering task catalog alongside other task runners, with a terse purpose-built language whose core syntax can be learned in seconds and optional directives that save time.
---

# Work With CJTaskrunner

Prefer the repository's existing task names and organization. Do not invent
syntax; use `cj --help` for CLI help and `cj --directives` when a directive
detail is not covered here.
CJTaskrunner is indentation-sensitive but is not YAML.

## Create And Discover Taskfiles

Use `cjtasks` as the base. Optional overlays cascade in this order:
`production.cjtasks`, `staging.cjtasks`, `development.cjtasks`, then
`local.cjtasks`. Higher layers replace whole tasks and env entries; task
overrides must preserve arity. Only the base may declare `@version` or contain
version bump directives. Discovery checks only the selected directory and does not search
parents or descendants.

Create an empty taskfile with `cj --init`. Import root-level `package.json`,
`deno.json`, `Makefile`, and argument-free `Justfile` tasks with `cj --auto`.
Imports never overwrite CJ tasks. Package scripts are considered first;
collisions use `build`, `build2`, `build3`, without an inserted separator.

## Follow The File Shape

Use two spaces per indentation level in generated or edited taskfiles.
CJTaskrunner accepts a file that consistently uses tabs, but `cj --format`
always normalizes leading indentation to spaces. Full-line `#` comments are
comments; inline `#` text is command input. Keep `@version`, `@help:`, and
`@env:` headers before task definitions.

```cjtasks
@version app 1.2.0

@env:
  MODE?: development
  API_URL: https://example.com

@help:
  Project development tasks

build:
  @desc build the project
  cargo build
```

Use `@env:` and `@help:` exactly. Plain `env:` and `help:` are ordinary task
names. Variables are forbidden in `@desc` prose and `@help:` blocks.

## Define And Invoke Tasks

Use ASCII letters, digits, hyphens, and underscores in task-name parts. Nest
task headings one level to create colon-addressed names.

```cjtasks
web:
  @desc web tasks
  build:
    npm run build

deploy (TARGET, TAG):
  deploy-tool $TARGET $TAG
```

Run these as `cj web:build` and `cj deploy production v1.2.3`. Declared task
arguments are required positional values. Do not invent optional, default, or
variadic argument syntax. Task argument variables are local to that call.

Tasks whose name begins with `_` are hidden from summary mode. If any parent
name begins with `_`, its descendants are hidden too. A task name cannot match
a directory beside the taskfile.

## Choose Direct Commands Or Shell Commands

Use ordinary lines for direct argv execution. They do not process pipes,
redirects, globbing, command substitution, chaining, or shell builtins.

```cjtasks
test:
  cargo test --all-targets

bundle:
  @shell mkdir -p dist && cat src/*.js > dist/app.js
```

Use `@shell` only for actual shell syntax. Use `@open` only with one
`http://` or `https://` URL.

## Interpolate Variables Correctly

- `$NAME` and `${NAME}` expand to an empty string when absent.
- `${NAME?}` errors when absent.
- `${NAME?fallback}` and `${NAME?"fallback text"}` use a fallback when absent.
- `\$NAME` writes a literal variable marker.
- An interpolated ordinary-command value remains one argv value.
- `@shell` shell-quotes interpolated values before shell execution.

## Manage Environment And Captured Output

In top-level `@env:`, use `NAME: value` to override inherited values and
`NAME?: value` to provide an absent-only fallback.

Inside tasks:

- Use `@set NAME value` for a CJ runtime variable.
- Use `@set NAME:` with an indented block to capture its stdout.
- Use `@export NAME` to expose a runtime variable to child processes.
- Use `@unset NAME` to remove the runtime value and export.

Runtime variables are internal until exported. Child processes receive the
exported environment.

## Compose Tasks And Status Flow

Use `@task name arguments...` for sequential composition. It shares runtime
state and current working directory while restoring called-task argument and
directory scopes on return. Recursive task calls are errors.

Use `@and` after success and `@or` after failure:

```cjtasks
check:
  cargo test
  @and
    @echo tests passed
  @or
    @stop tests failed
```

Use `@success`, `@fail`, `@return`, or `@stop` for explicit status behavior.
Use `@if`, `@if-not`, `@if-in`, `@if-not-in`, `@else`, `@if-exists`,
`@if-not-exists`, `@if-set`, `@if-not-set`, `@switch`, `@case`, and `@default`
for branching. Membership syntax is `@if-in $VALUE one two three`.

## Run Parallel Tasks With Await

Use `@await` to run named argument-free tasks in parallel. Its optional block
runs only after every awaited task succeeds; handle failure with same-level
`@or`.

```cjtasks
dev:
  @await server client
    @task open-browser
  @or
    @stop development services failed
```

Awaited tasks receive cloned runtime and directory state. They cannot use
`@set`, `@export`, `@unset`, or version bump directives, including through static `@task`
calls. They may change the filesystem. Await cycles and missing tasks are
parse errors. Set `CJ_JOBS` to a positive integer to limit parallelism.

## Manage Versions

Declare SemVer without build metadata at the top level. A version named
`app` creates `$VERSION_APP`.

```cjtasks
@version app 1.2.0

release (LEVEL):
  @switch $LEVEL
    @case patch
      @patch app
    @case minor
      @minor app
    @case major
      @major app
  @if-bumped
    @echo some version changed
  @if-major app
    @echo major release
```

Use `@major`, `@minor`, `@patch`, `@pre`, or `@release`. Each named
version may be bumped once per invocation. `@if-bumped` with no arguments
matches any bump; `@if-not-bumped` with no arguments matches no bumps. Use
`@if-version`, `@if-not-version`, `@if-bumped`, `@if-not-bumped`, and the
kind-specific `@if-patch` / `@if-not-patch` style directives for version
conditions.

## Work With Paths

Use `@cd` and `@back` for scoped directory changes. Use `@mkdir`, `@clean`,
`@cp`, `@cpdir`, and `@rename` for filesystem operations. Relative paths use
the task's current working directory.

## Document Tasks

Use `@desc` for one-line summary text and `@help:` for indented detailed text.
Use `@selfhelp` to print the current task's help and stop successfully.

## Use The CLI

- `cj` lists visible tasks.
- `cj <task> [arguments...]` runs a task.
- `cj <directory-or-taskfile> <task> [arguments...]` selects a taskfile.
- `cj help [task]` prints taskfile or task help.
- `cj --init` creates an empty `cjtasks` without overwriting a taskfile.
- `cj --auto` imports common root-level task systems additively.
- `cj --format [directory-or-taskfile]` formats a taskfile in place.
- `cj --run <line>` executes one non-block task line without a taskfile.
- `cj --directives` lists directives.
- `cj --completions <bash|zsh|fish>` prints completions.
- `cj --install-completions <bash|zsh|fish>` installs completions.
- `cj lsp` starts the built-in language server over stdio.

Set `NO_COLOR` to a non-empty value for stable plain-text output in scripts
and tests.

## Validate Changes

After editing a taskfile:

1. Run `cj --format`.
2. Run `cj` to parse the file and inspect summary visibility.
3. Run `cj help <task>` for changed help or nested tasks.
4. Run the narrowest affected task.
5. Confirm `@await` targets are argument-free and mutation-safe.
