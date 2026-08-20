---
tags: directive, reference
---

# Directives

Directives only exist to save time. They are optional and not needed to run tasks.

Directive names begin with `@`.  Some accept arguments, in which case the arguments are names, paths, or user-facing text. Directives have no options.

When using a directive, the line begins with the directive name.  Directives must always be the first thing on the line (except for indenting). This has several consequences:
  - The syntax unambiguos
  - Directives cannot be used in the middle of other lines
  - Directives do not "return" usable values other than exit codes
  - The '@' symbol can be used in strings freely

Some directives are 'block' directives, meaning that the lines after them are indented.

Some directives capture the string expression or standard out of lines indented below them. These directive lines end with a colon. See [`@help:`](#help) and [`@set:`](#set). One more special case is [`@env:`](#env).

Other than those three, all block directives have no colon at the end of the line.

Directive lines 

## Index

### Commands And Composition

- [`@shell`](#shell) - run a command through the platform shell.
- [`@open`](#open) - open an HTTP or HTTPS URL.
- [`@task`](#task) - call another task.
- [`@await`](#await) - run tasks in parallel and wait for their statuses.
- [`@watch`](#watch) - restart one line when files change.
- [`@echo`](#echo) - print text.
- [`@return`](#return) - return a derived or block status.
- [`@success`](#success) - return status `0`.
- [`@fail`](#fail) - return status `1`.
- [`@stop`](#stop) - print an optional message and return status `1`.

### Flow Control

- [`@and`](#and) - run a block after success.
- [`@or`](#or) - run a block after failure.
- [`@if`](#if) - test values or their inverse.
- [`@if-in`](#if-in) - test whether a value is in a list.
- [`@else`](#else) - alternate block for an `@if`.
- [`@if-exists`](#if-exists) - test whether a path exists.
- [`@if-set`](#if-set) - test whether a variable exists.
- [`@if-version`](#if-version) - test a component version.
- [`@if-bumped`](#if-bumped) - test whether a version was bumped.
- [`@if-patch`](#if-patch) - test whether a version received a patch bump.
- [`@if-minor`](#if-minor) - test whether a version received a minor bump.
- [`@if-major`](#if-major) - test whether a version received a major bump.
- [`@if-pre`](#if-pre) - test whether a version received a prerelease bump.
- [`@if-release`](#if-release) - test whether a version received a release bump.
- [`@switch`](#switch) - select a case by value.
- [`@case`](#case) - define a switch case.
- [`@default`](#default) - define the switch fallback.

### Variables And Versions

- [`@env:`](#env) - declare taskfile environment entries.
- [`@set`](#set) - set a runtime variable or capture block output.
- [`@export`](#export) - export a runtime variable to child processes.
- [`@unset`](#unset) - remove a runtime variable and export.
- [`@version`](#version) - declare a component version.
- [`@patch`](#patch) - patch-bump a declared component version.
- [`@minor`](#minor) - minor-bump a declared component version.
- [`@major`](#major) - major-bump a declared component version.
- [`@pre`](#pre) - prerelease-bump a declared component version.
- [`@release`](#release) - remove prerelease from a declared component version.

### Files And Directories

- [`@cd`](#cd) - change the current working directory.
- [`@back`](#back) - undo one `@cd`.
- [`@clean`](#clean) - remove a file or directory.
- [`@mkdir`](#mkdir) - create directories.
- [`@cp`](#cp) - copy files.
- [`@cpdir`](#cpdir) - copy directories.
- [`@rename`](#rename) - rename a file or directory.

### Documentation Metadata

- [`@desc`](#desc) - add a summary description to a task.
- [`@help:`](#help) - add taskfile or task help text.
- [`@selfhelp`](#selfhelp) - print the current task's help and return.

Nested directive bodies use two additional spaces of indentation. See the
[taskfile format](taskfile.md) for parsing, interpolation, and status
propagation rules.

## Commands And Composition

<a id="shell"></a>

### `@shell`

Runs the interpolated command through `/bin/sh -c` on Unix-like environments and MacOS.

```cjtasks
bundle:
  @shell mkdir -p dist && cat src/*.js > dist/app.js
```

CJTaskrunner is generally concerned with your relationship to your scripts and executables, invoking the commands directly executables. 

As such, shell-specific features such as globbing (`*[1,3]`), parameter expansion (`${VAR##/*/}`), inline flow control ( `&&`, `||`, `;`, `\`) and piping  (`<`, `|`, `>&2`) are not supported in ordinary task definitions.

But fear not; should the need to for example, process strings directly in a shell via terse-but-not-readable cleverness not be abated you can use the `@shell` directive to run any nonsense through whatever your system calls `/bin/sh`... one line at a time.

Variable interpolation is still observed before the command is sent to the shell, so if you need to send a dollar sign (`$`) to the shell, you must escape it in your task definition with a backslash: `\$`.

As a reminder, there are many standard command line built-in utilities that offer fantastic string manipuation capabilities, such as `awk`, `sed` and `grep` that can be invoked directly through CJTaskrunner. Additionally, several routine tasks such as making directories or deleting things can be done with CJTaskrunner.

Ultimately, you may find that it is best to use a script in a `.sh` file and use CJTaskrunner to call that file. That is the idiomatic CJTaskrunning way.

<a id="open"></a>

### `@open`

```cjtasks
dev:
  @open http://localhost:5173
```

Opens one interpolated HTTP or HTTPS URL with the system browser. The URL must begin with `http://` or `https://`.

Platform opener commands:

- macOS: `open <url>`
- Windows: `cmd /C start "" <url>`
- Linux and other Unix systems: `xdg-open <url>`

<a id="task"></a>

### `@task`

```cjtasks
ci:
  @task fmt
  @task image assets/logo.svg png

image (INPUT, FORMAT):
  image-tool $INPUT --format $FORMAT
```

Runs another task from the same taskfile with the current working directory and runtime variable state. Values after the task name are interpolated and passed as positional arguments.

Recursive task cycles are errors.

The called task inherits runtime variables and the current working directory as
a snapshot. Runtime variables and directory changes made inside the called task
reset when it returns. Values changed with `@export` are shared with the calling
task and later child processes.

<a id="await"></a>

### `@await`

```cjtasks
dev:
  client:
    @echo client starts
    sleep 1
    @success
  server:
    @echo server starts
    sleep 2
    @success

  @await dev:server dev:client
    @task browser-opens
  @or
    @echo awaited tasks failed

browser-opens:
  @echo browser opens
```

`@await task...` runs task branches in parallel and returns their status. When `@await` has an indented block, the block runs only after all awaited tasks succeed. If any awaited task fails, the block is skipped and `@await` returns the failing status, so normal `@or` and `@and` chaining applies.

Rules:

- `@await` is an executable directive and may appear wherever a task line can appear.
- Awaited tasks run in parallel when possible.
- Transitive awaits run before their waiting tasks.
- Shared awaited tasks run at most once per `@await` directive.
- `@await` without a block returns awaited task status directly.
- `@await` with a block returns awaited task failure status or the block status.
- Await cycles are parse errors.
- Missing awaited tasks are parse errors.
- Tasks with declared arguments cannot be named directly in `@await`; use an
  argument-free wrapper that calls them with `@task`.

Awaited tasks run with cloned runtime variables and current-directory state.
Runtime variables set inside awaited tasks never leak back to the parent task.
Values changed with `@export` or `@unset` are shared with the parent task and
later child processes. Tasks reachable through `@await` cannot use version bump
directives. This validation also follows static `@task name` calls inside
awaited tasks.

Awaited tasks may still mutate the filesystem through ordinary commands or filesystem directives.

The await parallelism limit defaults to the machine's available parallelism. Set `CJ_JOBS` to a positive integer to override it.

When CJTaskrunner receives Ctrl-C, it interrupts active child processes, including child process groups started by awaited tasks.

<a id="watch"></a>

### `@watch`

```cjtasks
docs:
  @watch docs
    mdbook serve
```

`@watch path...` runs its single indented line immediately and watches the named
files or directories while that line is running. When a change is detected, it
waits three seconds, collapses any other changes during that window into the
same rebuild signal, then stops and restarts the line.

After each restart, watching starts again. If the watched line exits before any
change is detected, `@watch` exits with that line's status. Directory watches are
recursive.

`@watch` requires at least one path and exactly one indented line. `@await` is
not allowed in the watched line or in a static `@task` called from it.

<a id="echo"></a>

### `@echo`

```cjtasks
build:
  @echo building $VERSION_APP
```

`@echo text` interpolates its arguments and writes the resulting text followed
by a newline. It returns status `0`. Unlike running `echo` as a task command, it doesn't spawn a process. It is also cross-platform and consistent.

You can also just use the `echo` (no `@`) task if you need the specific command behavior.

<a id="return"></a>

### `@return`

```cjtasks
check:
  @return
    test -f Cargo.toml

fail:
  @return 1
```

`@return value` returns a status derived from the value.
`true` and other non-empty non-numeric strings return `0`; `false` and an empty string return `1`; numeric text returns that status code.

With an indented block, `@return` runs the block and returns its final status.

<a id="success"></a>

### `@success`

```cjtasks
ready:
  @echo ready
  @success
```

`@success` takes no arguments and returns status `0`.

<a id="fail"></a>

### `@fail`

```cjtasks
unsupported:
  @echo unsupported configuration
  @fail
```

`@fail` takes no arguments and returns status `1`.

<a id="stop"></a>

### `@stop`

```cjtasks
sync:
  @if-not-set API_TOKEN
    @stop API_TOKEN is required
```

`@stop [text]` writes the optional text followed by a newline, then returns
status `1`, stopping normal task execution unless a same-level `@or` handles it.

## Flow Control

<a id="and"></a>

### `@and`

```cjtasks
build:
  cargo build
  @and
    @echo build succeeded
```

`@and` runs its indented block only when the previous same-level expression
returned status `0`. It takes no arguments. When skipped, it returns status `1`.

<a id="or"></a>

### `@or`

```cjtasks
build:
  cargo build
  @or
    @stop build failed
```

`@or` runs its indented block only when the previous same-level expression
returned non-zero. It takes no arguments. When skipped, it returns status `0`.

<a id="if"></a>
<a id="if-not"></a>

### `@if`

```cjtasks
build:
  @if $MODE == release
    cargo build --release
  @if $VERBOSE
    @echo verbose mode
  @if-not $SKIP_TESTS
    cargo test
```

Supported forms are `@if value`, `@if left == right`, and `@if left != right`.
Empty text, `0`, and `false` are false; other values are true. The indented
block runs only when the condition matches. `@if-not` accepts the same forms and
runs its indented block when the condition does not match.

<a id="if-in"></a>
<a id="if-not-in"></a>

### `@if-in`

```cjtasks
compile (TARGET):
  @if-in $TARGET linux macos windows
    @echo supported target
  @if-not-in $TARGET linux macos windows
    @stop unsupported target
```

`@if-in needle candidate...` runs its block when the needle exactly matches one
candidate. `@if-not-in needle candidate...` runs its block when the needle does
not match any candidate. Quote a value or candidate when it contains spaces.

<a id="else"></a>

### `@else`

```cjtasks
build:
  @if $MODE == release
    cargo build --release
  @else
    cargo build
```

`@else` runs its block when the immediately associated same-level `@if` family
condition did not run. It takes no arguments and must follow a compatible
conditional.

<a id="if-exists"></a>
<a id="if-not-exists"></a>

### `@if-exists`

```cjtasks
install:
  @if-exists package-lock.json
    npm ci
setup:
  @if-not-exists .venv
    python -m venv .venv
```

`@if-exists path` runs its block when the path exists. `@if-not-exists path`
runs its block when the path does not exist. Relative paths resolve from the
current working directory.

<a id="if-set"></a>
<a id="if-not-set"></a>

### `@if-set`

```cjtasks
private:
  @if-set API_TOKEN
    ./fetch-private-data
  @if-not-set API_TOKEN
    @stop API_TOKEN is required
```

`@if-set NAME` runs its block when the runtime variable exists. `NAME` may be
interpolated. `@if-not-set NAME` runs its block when the runtime variable is
absent. A present empty string is set, not unset.

<a id="if-version"></a>
<a id="if-not-version"></a>

### `@if-version`

```cjtasks
publish:
  @if-version app prerelease
    @stop refusing to publish a prerelease
  @if-version app >= 1.0.0
    @echo stable API
  @if-not-version app prerelease
    @echo release version
```

`@if-version name operator version` compares SemVer precedence using `==`,
`!=`, `<`, `<=`, `>`, or `>=`. The forms `@if-version name prerelease` and
`@if-version name release` test whether a prerelease suffix is present.
`@if-not-version` accepts the same forms and runs its block when the version
condition is false.

<a id="if-bumped"></a>
<a id="if-not-bumped"></a>

### `@if-bumped`

```cjtasks
release:
  @patch app
  @if-bumped
    @echo at least one version changed
  @if-not-bumped docs
    @echo docs version did not change
  @if-patch app
    @echo app received a patch bump
```

`@if-bumped` runs its block when any version was bumped during the current
invocation. `@if-not-bumped` runs its block when no version was bumped during
the current invocation. Both forms accept an optional version name to narrow the
check to one version.

<a id="if-patch"></a>
<a id="if-not-patch"></a>

### `@if-patch`

```cjtasks
release:
  @patch app
  @if-patch app
    @echo patch release
  @if-not-patch docs
    @echo docs did not receive a patch bump
```

`@if-patch name` runs its block when the named version received a patch bump
during the current invocation. `@if-not-patch name` runs its block when the
named version did not receive a patch bump.

<a id="if-minor"></a>
<a id="if-not-minor"></a>

### `@if-minor`

`@if-minor name` runs its block when the named version received a minor bump
during the current invocation. `@if-not-minor name` runs its block when the
named version did not receive a minor bump.

<a id="if-major"></a>
<a id="if-not-major"></a>

### `@if-major`

`@if-major name` runs its block when the named version received a major bump
during the current invocation. `@if-not-major name` runs its block when the
named version did not receive a major bump.

<a id="if-pre"></a>
<a id="if-not-pre"></a>

### `@if-pre`

`@if-pre name` runs its block when the named version received a prerelease bump
during the current invocation. `@if-not-pre name` runs its block when the named
version did not receive a prerelease bump.

<a id="if-release"></a>
<a id="if-not-release"></a>

### `@if-release`

`@if-release name` runs its block when the named version received a release bump
during the current invocation. `@if-not-release name` runs its block when the
named version did not receive a release bump.

<a id="switch"></a>

### `@switch`

```cjtasks
build:
  @switch $MODE
    @case release
      cargo build --release
    @default
      cargo build
```

`@switch value` selects at most one directly nested `@case` block. When no case
matches, a directly nested `@default` block runs when present.

<a id="case"></a>

### `@case`

```cjtasks
build:
  @switch $MODE
    @case debug
      cargo build
    @case release
      cargo build --release
```

`@case value` defines a branch directly inside `@switch`. At most one matching
case runs. A case outside a switch, a duplicate matching structure, or a case
without one value is an error.

<a id="default"></a>

### `@default`

```cjtasks
build:
  @switch $MODE
    @case release
      cargo build --release
    @default
      cargo build
```

`@default` defines the fallback branch directly inside `@switch`. It runs only
when no `@case` matches and takes no arguments.

## Variables And Versions

<a id="env"></a>

### `@env:`

```cjtasks
@env:
  MODE: development
  PORT?: 3000

serve:
  @echo $MODE on $PORT
```

`@env:` declares taskfile environment entries. It is valid only at the top
level, before task definitions.

- `NAME: value` overrides an inherited value.
- `NAME?: value` supplies a value only when the variable is absent.
- Values may be empty and may be wrapped in matching single or double quotes.
- Inline `#` text remains part of the value.
- Duplicate or invalid environment names are errors.

<a id="set"></a>

### `@set`

```cjtasks
build:
  @set MODE release
  @set COMMIT:
    git rev-parse --short HEAD
  @echo building $MODE at $COMMIT
```

`@set NAME value` sets a CJTaskrunner runtime variable. It does not export the
value to child processes.

`@set NAME:` runs its indented block with stdout capture enabled, trims trailing
line endings, and stores the captured text. Capture fails when the block's final
status is non-zero.

Runtime values are passed to sequential `@task` calls and `@await` tasks as a
snapshot. Changes made with `@set` do not persist back to the calling task.

<a id="export"></a>

### `@export`

```cjtasks
serve:
  @set PORT 3000
  @export PORT
  node server.js
```

`@export NAME` exports an existing runtime variable to later child processes.
`@export NAME value` sets and exports the value in one step. Export changes are
shared with later task contexts, including calling tasks after subordinate
`@task` or `@await` work returns.

<a id="unset"></a>

### `@unset`

```cjtasks
build:
  @set MODE release
  @export MODE
  @unset MODE
```

`@unset NAME` removes the runtime variable and any export overlay for later
commands. `NAME` may be interpolated. It does not modify the parent process
environment.

<a id="version"></a>

### `@version`

```cjtasks
@version cli 0.1.0
@version extension 0.0.1-beta.1

show:
  @echo $VERSION_CLI
```

`@version name value` declares a top-level component version and creates
`$VERSION_NAME`, uppercasing the name and converting hyphens to underscores.

Values use SemVer `MAJOR.MINOR.PATCH` with an optional prerelease suffix. Build
metadata is not supported. Version headers must appear before task definitions.

<a id="patch"></a>

### `@patch`

```cjtasks
@version cli 0.1.0
@version app 1.2.3-beta.1

bump-cli:
  @patch cli
  @pre app beta.
```

`@patch name` increments patch and removes prerelease. The version name is
interpolated before validation.

<a id="minor"></a>

### `@minor`

`@minor name` increments minor, then sets patch to `0` and removes prerelease.

<a id="major"></a>

### `@major`

`@major name` increments major, then sets minor and patch to `0` and removes
prerelease.

<a id="pre"></a>

### `@pre`

`@pre name prerelease` sets or increments the prerelease part. The version name
and prerelease identifier are interpolated before validation.

#### Prerelease bump rules:

- `@pre app alpha` sets the version prerelease to `alpha`.
- `@pre app alpha.` sets the prerelease to `alpha.0`, or increments the trailing number when the current prerelease is already `alpha.N`.

<a id="release"></a>

### `@release`

`@release name` removes the prerelease part.

Each version can be bumped at most once per invocation. Bump state is shared
with sequential `@task` calls. Tasks reached through `@await` are isolated and
cannot use version bump directives.

## Files And Directories

<a id="cd"></a>

### `@cd`

```cjtasks
build-docs:
  @cd docs
  npm run build
  @back
```

- `@cd path` changes the current working directory.
- Relative `@cd` paths resolve from the current working directory.

<a id="back"></a>

### `@back`

```cjtasks
build-docs:
  @cd docs
  npm run build
  @back
```

- `@back` undoes one `@cd` in the current scope.
- `@back` does nothing at the root directory for the current scope.
- Directory changes persist for later commands in the same block.
- Nested blocks inherit the parent directory and restore their starting directory when the block ends.
- Tasks inherit the caller's current directory and restore their starting directory when they return.

<a id="clean"></a>

### `@clean`

```cjtasks
clean:
  @clean dist
```

`@clean path` removes one file or directory relative to the current working
directory. A missing path is not an error.

<a id="mkdir"></a>

### `@mkdir`

```cjtasks
package:
  @mkdir dist dist/assets
```

`@mkdir path...` creates one or more directories, including missing parent
directories. Existing directories are not errors.

<a id="cp"></a>

### `@cp`

```cjtasks
package:
  @mkdir dist
  @cp README.md LICENSE dist
```

`@cp source... destination` copies files. With multiple sources, the
destination must be a directory. Paths resolve from the current working
directory.

<a id="cpdir"></a>

### `@cpdir`

```cjtasks
package:
  @cpdir assets dist
```

`@cpdir source... destination` copies directories recursively. A trailing slash
on a source copies that directory's contents into the destination. Multiple
sources require a directory destination.

<a id="rename"></a>

### `@rename`

```cjtasks
package:
  @rename dist/app.tmp dist/app
```

`@rename source destination` renames one file or directory. Source and
destination must be in the same directory; use copy directives when moving
between directories.

## Documentation Metadata

<a id="desc"></a>

### `@desc`

```cjtasks
build:
  @desc compile project
  cargo build
```

Defines task description metadata. `cj` shows it when run without a task name.
Variables are not allowed in the description. Use `\$NAME` or `\${NAME}` to
display literal variable-shaped text. `@desc` does not run a command.

<a id="help"></a>

### `@help:`

```cjtasks
@help:
  Project task help.

build:
  @help:
    Build help.
  cargo build
```

It's like `@desc` but indented, multiline and only prints when specifically asked for with `cj help`.

The entire block is read as a string. Variables are not allowed on any help-text line. However, for your sanity, the parser will catch whenever something looks like an attempt at using a variable. Use `\$NAME` or `\${NAME}` to display literal variable-shaped text. 

`cj help` prints top-level help text when present, followed by the task listing and available help sections. 

`cj help <task>` prints the task name, its `@desc` and `@help:` text, direct child tasks, and available help sections. 

Top-level `@help:` defines taskfile help. Task-level `@help:` defines help for
that task. Plain `@help` is invalid; `help:` without the `@` defines an ordinary
task named `help`. 

Tasks whose name or any nested task segment starts with `_` are hidden from the no-argument summary listing. They can still be run directly and viewed with `cj help <task>`.

<a id="selfhelp"></a>

### `@selfhelp`

```cjtasks
cli:
  @desc cli commands
  @help:
    CLI help.
  @selfhelp
```

Prints the same output as `cj help <current-task>`, then stops the current task successfully. `@selfhelp` takes no arguments.

The purpose of this directive is to serve as an indicator that there are no commands in this task, useful if it is just a container for subtasks. It is the closest thing to a 
