# CJTaskrunner VS Code Extension

Language support and task execution tools for `cjtasks` and `*.cjtasks` files.

## Features

- Syntax highlighting for CJTaskrunner task files.
- Diagnostics, document symbols, directive hovers, task/variable completions, and formatting.
- Go-to-definition for `@task` references.
- Explorer `CJTaskrunner` view for task discovery.
- One-click task execution from the tree view and command palette.

## Requirements

- VS Code `1.85.0` or newer.
- `cj` or `cjtaskrunner` and `cjtaskrunner-lsp` binaries available via bundled assets, workspace `target/debug`, or `PATH`.

## Extension Settings

- `cjtaskrunner.executable.path`: Absolute path to `cj` or `cjtaskrunner`.
- `cjtaskrunner.lsp.path`: Absolute path to `cjtaskrunner-lsp`.
- `cjtaskrunner.lsp.trace.server`: LSP trace mode (`off`, `messages`, `verbose`).

## Development

From repository root:

```sh
cargo build --bin cjtaskrunner-lsp
cd editors/vscode-cjtaskrunner
npm install
npm run compile
code .
```

Press `F5` in VS Code to launch an Extension Development Host.

## Marketplace Release Notes

Release history is tracked in [`CHANGELOG.md`](./CHANGELOG.md). Marketplace publishing steps are tracked in [`MARKETPLACE_PREP.md`](./MARKETPLACE_PREP.md).
