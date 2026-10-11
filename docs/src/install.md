# Install

CJTaskrunner has one executable: `cj`.

## Shell Installer (recommended)

You should never run arbitrary shell scripts that you download off the internet. 

That said, the easiest and recommended way to install CJTaskrunner is by doenloading and running the install script in one line here:

```sh
curl -fsSL https://raw.githubusercontent.com/jgusta/cjtaskrunner/main/install.sh | bash
```

The installer downloads the latest release for Linux x86_64, macOS Intel, or
macOS Apple Silicon, verifies it against `SHA256SUMS`, and installs `cj` into `$CJ_INSTALL_DIR` or `$HOME/.local/bin`.

Make sure the install directory is in your `PATH`.

For Bash or Zsh:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

For Fish shell:

```fish
fish_add_path "$HOME/.local/bin"
```

## From Source

```sh
cargo install --git https://github.com/jgusta/cjtaskrunner.git cjtaskrunner
```

## Homebrew

Homebrew no longer supports intel-based macs. So if you are on an intel mac, this formula will build from scratch. 

```sh
brew install jgusta/cjtaskrunner/cj
```

## Check The Install

```sh
cj --help
```

