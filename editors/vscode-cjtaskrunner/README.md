# CJTaskrunner VS Code Extension

Language support and task execution tools for [CJTaskrunner](https://github.com/jgusta/cjtaskrunner) taskfiles.

## Features

- Syntax highlighting for CJTaskrunner `cjtasks` task files.
- Diagnostics, document symbols, directive hovers, task/variable completions, and formatting.
- Go-to-definition for `@task` references.
- `CJTasks` panel for task discovery and execution.
- Lightweight and optimized < 5ms startup time

The task panel is hidden unless a workspace root contains `cjtasks` or
one of the standard overlay files. 

It will also scan for descendant task files if and only if a taskfile is found at the directory root.

After creating the `cjtasks` file, the panel may not appear until a reload. This is deliberate to keep the extension as light as possible.

## Requirements

- VS Code `1.85.0` or newer.
- CJTaskrunner executable `cj`.

## Extension Settings

- `cjtaskrunner.path`: Path to `cj`. Leave empty to use `cj` from `PATH`.

## Syntax Scopes

CJTaskrunner emits theme-controlled TextMate scopes:

- Directive name -> `keyword.control.directive.cjtasks`
- Task name or @task reference -> `entity.name.function.task.cjtasks`
- Ordinary task line or directive argument -> `meta.task-line.cjtasks`
- Variable -> `variable.other.cjtasks`
- Syntactic colon -> `punctuation.separator.colon.cjtasks`
- @help:, @env:, or capture-form @set -> `keyword.control.directive.block.cjtasks`
- Full-line comment -> `comment.line.number-sign.cjtasks`
- Double-quoted string -> `string.quoted.double.cjtasks`
- Single-quoted string -> `string.quoted.single.cjtasks`
- @help: or @desc prose -> `comment.block.documentation.cjtasks`

Variable-shaped text keeps its variable scope inside documentation prose so invalid references remain visible, but CJTaskrunner rejects variables in `@desc` and `@help:` text.

## Commands

- `CJTaskrunner: Language Server Status`: Open a status document showing the resolved `cj` path, server state, trace setting, Outline mode, and last error.
- `CJTaskrunner: Restart Language Server`: Stop and start `cj lsp` after changing settings or rebuilding the binary.
- `CJTaskrunner: Show Language Server Output`: Open the language server output channel.

