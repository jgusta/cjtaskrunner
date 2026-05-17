# CJTaskrunner VS Code Extension

Local VS Code language support for `cjtasks` and `*.cjtasks` files.

## Development

From repository root:

```sh
cargo build --bin cjtaskrunner-lsp
cd editors/vscode-cjtaskrunner
npm install
npm run compile
code .
```

Press `F5` in VS Code to launch an Extension Development Host. Open a file named `cjtasks` or ending in `.cjtasks`.

If the extension cannot find the server, set `cjtaskrunner.lsp.path` to the absolute path of `target/debug/cjtaskrunner-lsp`.

## Features

- diagnostics
- task document symbols
- directive, task, and variable completions
- directive hover
- go to definition for `@task`
- document formatting
- Explorer `CJTaskrunner` view for `cjtasks` and `*.cjtasks`
- run tasks from the tree view or `CJTaskrunner: Run Task`
- task descriptions from direct `@desc` lines

Task runs use `cjtaskrunner.executable.path` when set. Otherwise the extension uses bundled binaries, then workspace `target/debug`, then `PATH`. The LSP uses `cjtaskrunner.lsp.path` with the same fallback behavior.
