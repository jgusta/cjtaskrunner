# Install

CJTaskrunner installs one executable: `cj`.

## Homebrew

```sh
brew install jgusta/cjtaskrunner/cj
```

## Shell Installer

```sh
curl -fsSL https://raw.githubusercontent.com/jgusta/cjtaskrunner/main/install.sh | bash
```

The installer downloads the latest release for Linux x86_64, macOS Intel, or
macOS Apple Silicon, verifies it against `SHA256SUMS`, and installs `cj` into
`$CJ_INSTALL_DIR` or `$HOME/.local/bin`.

To install a specific version:

```sh
CJ_VERSION=0.1.0 curl -fsSL https://raw.githubusercontent.com/jgusta/cjtaskrunner/main/install.sh | bash
```

To install somewhere else:

```sh
CJ_INSTALL_DIR="$HOME/bin" curl -fsSL https://raw.githubusercontent.com/jgusta/cjtaskrunner/main/install.sh | bash
```

Make sure the install directory is in your `PATH`.

For Bash or Zsh:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

For Fish:

```fish
fish_add_path -Ux "$HOME/.local/bin"
```

## From Source

```sh
cargo install --git https://github.com/jgusta/cjtaskrunner.git cjtaskrunner
```

## Check The Install

```sh
cj --help
```

After installing, create a taskfile with:

```sh
cj --init
```
