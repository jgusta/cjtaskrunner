# CJTaskrunner

<div style="display:flex;align-items:center;justify-content:center;margin-bottom:0;">
  <img src="../logo/cj-logo-color-f.svg" alt="CJTaskrunner logo" style="margin:0;display:block;height:auto;width:80px;" />
  <img src="../logo/cj-words.svg" alt="CJTaskrunner words logo" style="margin:0;display:block;width:320px;height:auto;" />
</div>
<div style="margin:0;text-align:center;font-size:1.4em;">
  Independent task wrangler
</div>
<div style="display:flex;align-items:center;justify-content:center;margin-bottom:0;">
  <img alt="license" src="https://img.shields.io/github/license/jgusta/cjtaskrunner">
  <img alt="version" src="https://img.shields.io/github/v/release/jgusta/cjtaskrunner">
</div>

## First Taskfile

Create a [taskfile](../reference/taskfile.md):

```sh
cj --init
```

Add a [task](../reference/taskfile.md#tasks):

```cjtasks
hello:
  @echo hello
```

Run it:

```sh
cj hello
```

Run [`cj`](../reference/cli.md#running-tasks) without a task name to list visible tasks.

See [CLI Reference](../reference/cli.md) and [Taskfile Reference](../reference/taskfile.md).

## Descriptions And Help

Use [`@desc`](../reference/directives.md#desc) for the one-line summary shown by `cj`.

```cjtasks
check:
  @desc run the test suite
  cargo test --locked
```

Use [`@help:`](../reference/directives.md#help) for block-form help text shown by [`cj help`](../reference/cli.md#help) and `cj help <task>`.

[Tasks](../reference/taskfile.md#tasks) beginning with `_` stay hidden from summary mode.

```cjtasks
_clean-cache:
  rm -rf .cache
```

See [`@desc`](../reference/directives.md#desc), [`@help:`](../reference/directives.md#help), and [`@selfhelp`](../reference/directives.md#selfhelp).

## Commands

[Ordinary commands](../reference/taskfile.md#ordinary-commands) run directly, without shell parsing.

```cjtasks
check:
  cargo test --locked
```

Use [`@shell`](../reference/directives.md#shell) when you need redirection, pipes, glob expansion, or command chaining.

```cjtasks
bundle:
  @shell mkdir -p dist && cat src/*.js > dist/app.js
```

See [Taskfile Reference](../reference/taskfile.md#ordinary-commands) and [`@shell`](../reference/directives.md#shell).

## Nested Tasks

[Nested tasks](../reference/taskfile.md#tasks) are addressed with colon-separated names. Tasks can only be nested one level.

```cjtasks
build:
  @desc build commands

  cli:
    @desc build the CLI
    cargo build --release
```

```sh
cj build:cli
```

See [Taskfile Reference](../reference/taskfile.md#tasks).

## Task Calls

Use [`@task`](../reference/directives.md#task) when one task should run another task.

```cjtasks
ci:
  @task fmt
  @task test

fmt:
  cargo fmt --check

test:
  cargo test --locked
```

See [`@task`](../reference/directives.md#task).

## Arguments

Declare required [arguments](../reference/taskfile.md#tasks) beside the task name.

```cjtasks
greet (NAME):
  @echo hello $NAME
```

```sh
cj greet Ada
```

[Arguments](../reference/taskfile.md#tasks) are required. Optional, default, and variadic arguments are not part of the format.

See [Taskfile Reference](../reference/taskfile.md#tasks).

## Working directory
- Always starts in the same directory of the [taskfile](../reference/taskfile.md), no matter where it is called from.
- Directory changes via [`@cd`](../reference/directives.md#cd) persist for later lines in the same task context.
- Called tasks receive the caller's current directory as their baseline, but their directory changes reset when they return.

## Variable definition

CJTaskrunner runs in an **isolated snapshot** of your **[environment](../reference/definitions.md#environment)** which inherits all your [environment variables](../reference/definitions.md#environment-variable). They can be used in your tasks by prefixing their name with a dollar sign (`$`) and CJTaskrunner will replace them with their value before doing anything else.

Define and set new [variables](../reference/variables.md) using [`@set`](../reference/directives.md#set).

```cjtasks
dev:
  @set foo BAR
  @echo foo $foo
```

Use its block form, [`@set VAR:`](../reference/directives.md#set) to capture the output of commands:
```cjtasks
branch:
  @set BRANCH:
    git rev-parse --abbrev-ref HEAD
  @echo $BRANCH
```

In order to persist the variable across taskrunner tasks and make it visible to child processes, use [`@export`](../reference/directives.md#export).

```cjtasks
run:
  @export NODE_ENV production
  node server.js
```

> [`@export`](../reference/directives.md#export) can be used with the variable name and no value assignment and it will export whatever the existing value is.

Use [`@env:`](../reference/directives.md#env) to set multiple taskfile-wide values. These are automatically exported. A `?` means only set if not yet set.

```cjtasks
@env:
  PORT?: 5173
  MYHOST?: http://localhost
  NODE_ENV?: production

dev:
  npm run dev -- --port $PORT --host $MYHOST
```

## Variable usage

You can use [variables](../reference/variables.md) by prefixing the name with a `$`. This is done before the task is parsed. You can substitute variable for a [directive](../reference/directives.md) argument.

You can set a default for a variable or make a value mandatory when using them. Use [`${NAME?}`](../reference/variables.md#interpolation) when a value must exist, and [`${NAME?fallback}`](../reference/variables.md#interpolation) for local fallback values.

You cannot use a variable as the name of a command in a [task definition](../reference/taskfile.md#tasks), nor can you substitute the name of a [directive](../reference/directives.md) using a variable.

[Directives](../reference/directives.md) that require the name of a variable ([`@if-set`](../reference/directives.md#if-set), [`@set`](../reference/directives.md#set), [`@export`](../reference/directives.md#export)) expect a string value; if you include a `$` in front of a variable name, it will resolve the value as the name.


- Unexported variables set with [`@set`](../reference/directives.md#set) are only accessible from the same [task context](../reference/definitions.md#task-context). Subordinate tasks receive them as a snapshot, but changes do not propagate back.

- Setting an environment variable using the [`@env:`](../reference/directives.md#env) directive, or using [`@export`](../reference/directives.md#export) on a variable will affect the [taskfile execution context](../reference/definitions.md#taskfile-execution-context).

- Pre-existing [environment variables](../reference/definitions.md#environment-variable) will be restored after the task runs. Variables and environment variable changes within the taskrunner context will be discarded after the task run.

See [Variables Reference](../reference/variables.md).

## Parallel Tasks

Use [`@await`](../reference/directives.md#await) when independent task branches can run at the same time.

```cjtasks
dev:
  client:
    npm run dev
  server:
    node server.js

  @await dev:client dev:server
    @open http://localhost:5173
  @or
    @echo startup failed
    @stop
```

[`@await`](../reference/directives.md#await) waits for all named tasks. Its block runs only when all awaited tasks succeed.

See [`@await`](../reference/directives.md#await).

## Watch Tasks

Use [`@watch`](../reference/directives.md#watch) to restart one long-running line when files change.

```cjtasks
docs:
  @watch docs
    mdbook serve
```

See [`@watch`](../reference/directives.md#watch).

## Conditional Logic

Use [`@if`](../reference/directives.md#if), [`@else`](../reference/directives.md#else), [`@and`](../reference/directives.md#and), and [`@or`](../reference/directives.md#or) for status-based flow.

```cjtasks
cond:
  @set MODE release
  @if $MODE == release
    cargo build --release
  @else
    cargo build
```

Use [`@if-in`](../reference/directives.md#if-in) for list membership.

```cjtasks
compile (TARGET):
  @if-in $TARGET linux macos windows
    @echo supported target
```

Every [`@if`](../reference/directives.md#if) directive has a matching negative form such as [`@if-not`](../reference/directives.md#if-not) or [`@if-not-in`](../reference/directives.md#if-not-in).

See [Flow Control Directives](../reference/directives.md#and).

## Versions

Declare component versions with [`@version`](../reference/directives.md#version), then bump them with [`@patch`](../reference/directives.md#patch), [`@minor`](../reference/directives.md#minor), [`@major`](../reference/directives.md#major), [`@pre`](../reference/directives.md#pre), or [`@release`](../reference/directives.md#release).

```cjtasks
@version cli 0.1.0

bump:
  @patch cli
  @echo $VERSION_CLI
```

See [SemVer Tools](../reference/semver-tools.md).
