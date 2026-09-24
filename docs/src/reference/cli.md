# CLI

```text
cj [task] [arguments...]
cj <taskfile-or-directory>
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

## Running Tasks

From a valid CJTaskruner context:

```sh
cj check
cj greet Ada
cj example_tasks/docker-basic check
```

A valid CJTaskrunner context is a directory in which a valid [task file](taskfile.md) is found.

If the first operand is an existing directory or recognized taskfile, it
selects that taskfile location. Otherwise the first operand is the task name.

## Listing mode

Aside from running named tasks, CJTaskrunner can also display information about the available tasks via "listing mode".

In a valid CJTaskrunner context, type `cj` without any arguments to list the available tasks and their descriptions (provided by the [`@desc`](directives.md#desc) directive).

## CJTaskrunner context

Your CJTaskrunner context depends on your working directory and whether or not it has a valid task file at the same level. For most purposes, you will want to use the default context, but you can also run CJTaskrunner in another context.

### Default context

The default CJTaskrunner context is your current working directory when there is a valid task file in it.

When you run the `cj` executable from a default context you simply specify the name of a task to run as the first argument to run it. In default context, running `cj` with no argument engages listing mode.

Listing mode in default context:

```
cj
```

### Non-default context

Specify a directory or file path as first argument. The directory must have a task file in its top level. Paths can be relative, ending slash is optional.

```
cj subdirectory
cj ../parentdir/cjtasks
cj /home/project/development.cjtasks
```

Running a task named "mytask" in another context:

```
cj subdirectory mytask
cj ../parentdir/cjtasks mytask
cj development.cjtasks mytask
```

Standard task file cascade rules applies to all contexts.

### Task names cannot share names with a directory

You **cannot** give a task the same name as a directory in the same context.

For instance, if you have a `cjtasks` file in your project root but there is a folder named `src` in the same project, then you cannot name your task `src`. Additionally, you can't name a task after a task file.

By design, the cli syntax is designed to minimize typing. The lazy ability to specify a different context as an argument comes with this one major tradeoff. In a CJTaskrunner context, you cannot have a task that shares a name with a directory.

The first argument is always checked if it is a valid directory or file name. Then task names are assumed.

This rule eliminates ambiguity. If you try to run a CJTaskrunner task from a context containing a directory with the same name, there will be an error. Relative and absolute paths can be used, trailing slashes on directory names are optional

Passing an existing directory or recognized taskfile by itself also lists that location's visible tasks.


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
