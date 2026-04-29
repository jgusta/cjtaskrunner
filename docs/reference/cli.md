# CLI

```text
cj [task] [arguments...]
cj <taskfile-or-directory> <task> [arguments...]
cj --init
cj --auto
cj -e
cj --format [taskfile-or-directory]
cj --run <line>
cj --directives
cj --completions <bash|zsh|fish>
cj --install-completions <bash|zsh|fish>
cj lsp
```

Running `cj` without a task lists visible tasks in the discovered taskfile.

## Running Tasks

```sh
cj check
cj greet Ada
cj example_tasks/docker-basic check
```

If the first operand is an existing directory or recognized taskfile, it
selects that taskfile location. Otherwise the first operand is the task name.

`cj --run <line>` executes one task line in the current directory without requiring a taskfile. The line must not contain newlines, task labels, or block directives such as `@if`, `@switch`, `@and`, `@or`, `@help:`, or `@set NAME:`.

## Creating Taskfiles

`cj --init` creates an empty `cjtasks` file in the current directory.

`cj --auto` imports common task definitions from `package.json`, `deno.json`, `Makefile`, and argument-free `Justfile` recipes. It creates `cjtasks` when no base taskfile exists, otherwise it appends missing tasks to `cjtasks`.

`package.json` scripts are considered first, followed by Deno, Make, and Just tasks. Each wrapper takes the shortest available normalized name. Name or directory conflicts add a number without a separator: `build`, `build2`, `build3`. Existing CJ tasks are never overwritten. This command is **not** idempotent; you probably don't want to run it more than once as it will create new tasks each time it runs.

## Editing

```sh
cj -e
```

`cj -e` opens the detected taskfile in `$EDITOR`. If `cjtasks` exists, it is
opened. Otherwise CJTaskrunner opens the highest-precedence overlay taskfile it
can find.

## Formatting

```sh
cj --format
cj --format path/to/project
```

Note this command targets the directory, not the file itself.

Formatting normalizes indentation and trailing whitespace while preserving blank lines. Formatter output always uses spaces for leading indentation.

## Shell Completion

```sh
cj --completions zsh
cj --install-completions zsh
```
`bash`, `fish` and `zsh` are supported.


## Help

`cj --cli-help` prints CLI usage.

`cj --directives` prints supported directives and a brief description for each one.

## Language server

`cj lsp` starts the language server over stdio. The language server is part of the main executable; there is no separate LSP binary.

```sh
cj lsp
```

## Miscellaneous

CJTaskrunner may use ANSI color in its own help, listing, and directive output. A non-empty `NO_COLOR` environment variable disables ANSI color.

The language server is built into the `cj` executable and runs over stdio.

See [Taskfile Format](taskfile.md) for discovery and invocation details.
