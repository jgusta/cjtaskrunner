# Variables

Variables are used inside [task definitions](taskfile.md#tasks) using the dollar sign `$`. Tasks you run will not be aware of variables used like this because CJTaskrunner swaps them out for their values before sending arguments to the task.

Variables in CJTaskrunner are [runtime values](definitions.md#runtime-variable), [exported](definitions.md#exported-variable), or [environment variables](definitions.md#environment-variable).

[Environment variables](definitions.md#environment-variable) can be used the same way as [runtime variables](definitions.md#runtime-variable), and are also visible to child processes. For example, `$PATH` is an [environment variable](definitions.md#environment-variable) that programs read from the shell [environment](definitions.md#environment).

[`@set NAME value`](directives.md#set) sets a CJTaskrunner [runtime variable](definitions.md#runtime-variable).

[Runtime values](definitions.md#runtime-variable) are passed into subordinate [`@task`](directives.md#task) and [`@await`](directives.md#await) calls as a snapshot, but [runtime variables](definitions.md#runtime-variable) set inside of a task do not persist to the calling task.

[`@set NAME:`](directives.md#set) runs its indented block with stdout capture enabled, trims trailing line endings, and stores the captured text. Capture fails when the block's final status is non-zero.

```cjtasks
branch:
  @set BRANCH:
    git rev-parse --abbrev-ref HEAD
  @echo $BRANCH
```

[`@export`](directives.md#export) also exposes the value to child processes, making it an [environment variable](definitions.md#environment-variable) for processes within the taskfile execution context.

[`@unset`](directives.md#unset) removes a [runtime value](definitions.md#runtime-variable) and [export](definitions.md#exported-variable).

Runtime variables are reset after the task exits.

Variables are not allowed in [`@desc`](directives.md#desc) or [`@help:`](directives.md#help) text.

## Environment

Set multiple variables at once using an [`@env:`](directives.md#env) block:

```cjtasks
@env:
  NAME: value
  FALLBACK?: fallback value
```

`NAME: value` overrides an inherited value. `NAME?: value` only applies when the variable is absent.

Additionally, you can use [`@export`](directives.md#export) to set [environment variables](definitions.md#environment-variable) or turn a [runtime variable](definitions.md#runtime-variable) into an [environment variable](definitions.md#environment-variable).

CJTaskrunner runs in an **isolated snapshot** of your **[environment](definitions.md#environment)**, which we refer to as the **[taskfile execution context](definitions.md#taskfile-execution-context)**.

- Existing [environment variables](definitions.md#environment-variable) can be used within [taskfiles](taskfile.md).
- If you set a variable using an existing variable name in your taskfile, that new value will take precedence for the duration of the [task context](definitions.md#task-context).
- The overridden [environment variable](definitions.md#environment-variable) will be restored after the [task context](definitions.md#task-context) ends unless exported.
- Variables that are exported will propagate to other tasks within the same **[taskfile execution context](definitions.md#taskfile-execution-context)**, for example when tasks call other tasks. These propagate back to a calling task as well.
- [Exported variables](definitions.md#exported-variable) work essentially the same way as [environment variables](definitions.md#environment-variable). In case of a conflict, [exported variables](definitions.md#exported-variable) take precedence.


## Interpolation

Variables can be used in [directives](directives.md) or [task commands](taskfile.md#command-lines). There is only simple [interpolation](#interpolation) available.

Variables are resolved before sending to commands.

[Directive](directives.md) operands are treated as strings after [interpolation](#interpolation); when a directive expects a variable name, the interpolated string is validated as the name.

```cjtasks
show:
  @echo $NAME
  @echo "${NAME}"
  @echo ${REQUIRED?}
  @echo ${PORT?5173}
  @echo ${MESSAGE?"hello world"}
```

Rules:

- `$NAME` expands to the current value or an empty string.
- `${NAME}` expands to the current value or an empty string.
- `${NAME?}` errors when the [variable](definitions.md#variable) is missing.
- `${NAME?fallback}` uses `fallback` when the [variable](definitions.md#variable) is missing.
- `${NAME?"fallback value"}` supports a quoted fallback.
- In [ordinary commands](taskfile.md#ordinary-commands), an interpolated value remains one argv value.
- In [`@shell`](directives.md#shell), interpolated values are shell-quoted before `/bin/sh -c`.

## Version Variables

```cjtasks
@version cli 0.1.0
```

[Version declarations](directives.md#version) create variables named `VERSION_<NAME>`, with the name uppercased and hyphens converted to underscores. The example above creates `$VERSION_CLI`. See [SemVer Tools](semver-tools.md) for more info.
