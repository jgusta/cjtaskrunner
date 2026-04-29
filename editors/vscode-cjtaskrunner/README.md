# CJTaskrunner VS Code Extension

Language support and task execution tools for CJTaskrunner taskfiles.

## Features

- Syntax highlighting for CJTaskrunner task files.
- Diagnostics, document symbols, directive hovers, task/variable completions, and formatting.
- Go-to-definition for `@task` references.
- Explorer `CJTaskrunner` view for task discovery.
- One-click task execution from the tree view and command palette.
- Language server status, restart, and output commands for troubleshooting.
- Extension-side Outline symbols when `cj lsp` is unavailable.

The Explorer task view is hidden unless a workspace root contains `cjtasks` or
one of the standard overlay files. Root detection does not scan descendants.
After a root taskfile enables the view, descendant folders are scanned for
recognized taskfile names, with `cjtasks` taking precedence in each folder.

Creating the first root taskfile in an already-open workspace may require
opening that file or reloading the window so VS Code activates the extension.

## Requirements

- VS Code `1.85.0` or newer.
- `cj` installed on `PATH`, or configured with `cjtaskrunner.path`.

## Extension Settings

- `cjtaskrunner.path`: Path to `cj`. Empty uses `cj` from `PATH`; relative paths resolve from the first workspace folder.
- `cjtaskrunner.lsp.trace.server`: LSP trace mode (`off`, `messages`, `verbose`).

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

Themes choose the colors and font styles for these scopes. Only lines whose first non-whitespace character is `#` are comments. Variable-shaped text keeps its variable scope inside documentation prose so invalid references remain visible, but CJTaskrunner rejects variables in `@desc` and `@help:` text.

## Commands

- `CJTaskrunner: Language Server Status`: Open a status document showing the resolved `cj` path, server state, trace setting, Outline mode, and last error.
- `CJTaskrunner: Restart Language Server`: Stop and start `cj lsp` after changing settings or rebuilding the binary.
- `CJTaskrunner: Show Language Server Output`: Open the language server output channel.

## Development

From repository root:

```sh
cargo build --bin cj
cargo install --path .
cd editors/vscode-cjtaskrunner
npm install
npm run compile
code .
```

Ensure `cj` is on `PATH`, or set `cjtaskrunner.path` in the Extension Development Host settings. The extension starts the language server with `cj lsp`. Press `F5` in VS Code to launch an Extension Development Host.

## Marketplace Release Notes

Release history is tracked in [`CHANGELOG.md`](./CHANGELOG.md). Marketplace publishing steps are tracked in [`MARKETPLACE_PREP.md`](./MARKETPLACE_PREP.md).
