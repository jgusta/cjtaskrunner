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

### Homebrew
CJTaskrunner can be installed from the project tap:

```sh
brew install jgusta/cjtaskrunner
```

### "YOLO"-style install with cURL and shell script

We are obligated to tell you that you shouldn't run untrusted scripts directly from the internet.

You can ignore this warning and install CJTaskrunner by using this command which runs a script directly from the internet:

```sh
curl -fsSL https://raw.githubusercontent.com/jgusta/cjtaskrunner/main/install.sh | bash
```

This script installs the latest release into `$CJ_INSTALL_DIR`, `$XDG_BIN_HOME/` or `$HOME/.local/bin` in that order, if found.

It supports Linux x86_64 and Mac Intel and Apple Silicon. It verifies the archive against the release `SHA256SUMS` file. 

### Download from Github
Official releases are available on Github ([https://jgusta/cjtaskrunner](https://github.com/jgusta/cjtaskrunner))


### Add your path

Make sure the install directory is in your PATH. So if your install directory is `$HOME/.local/bin`, add with:

`bash/zsh`:
```sh
export PATH="$HOME/.local/bin:$PATH"`
```

`fish`
```sh
fish_add_path -Ux $HOME/.local/bin:$PATH
```

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

