# Ecosystem

## Command-line client

The `cj` executable discovers and runs tasks, formats taskfiles, generates
shell completions, and exposes the language server. See
[Getting started](../index.md) and the [taskfile format](../reference/taskfile.md).

## Built-in LSP

CJTaskrunner serves its language protocol implementation through `cj lsp`.
It provides diagnostics, completion, hover information, document symbols,
task definitions, and formatting without requiring a second executable.

## VS Code extension

The repository includes a
[VS Code extension](../../../editors/vscode-cjtaskrunner/README.md) for syntax
highlighting, Outline symbols, task discovery and execution, and integration
with the built-in language server.

## Installation

See [Install](../install.md).

## Shell completion

`cj` can generate or install completions for Bash, Zsh, and Fish:

```sh
cj --completions zsh
cj --install-completions zsh
```

Replace `zsh` with `bash` or `fish` as needed.

See [CLI Reference](../reference/cli.md).

## Example projects

The [`example_tasks`](../../../example_tasks/README.md) directory contains taskfiles
for Rust, Node, Python, Docker, and other common project layouts.

## Syntax Highlighting

### VSCode extension
The official extension brings syntax highlighting, formatting, taskfile comprehension and more to Visual Studio Code.
