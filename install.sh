#!/usr/bin/env bash

set -euo pipefail

REPOSITORY="jgusta/cjtaskrunner"
INSTALL_DIR="${CJ_INSTALL_DIR:-}"
VERSION="${CJ_VERSION:-}"
TEMP_DIR=
INSTALL_TEMP=

say() {
  printf 'cj installer: %s\n' "$*"
}

die() {
  printf 'cj installer: %s\n' "$*" >&2
  exit 1
}

cleanup() {
  if [[ -n "$INSTALL_TEMP" ]]; then
    rm -f "$INSTALL_TEMP"
  fi
  if [[ -n "$TEMP_DIR" ]]; then
    rm -rf "$TEMP_DIR"
  fi
}

trap cleanup EXIT

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

if [[ -z "$INSTALL_DIR" ]]; then
  [[ -n "${HOME:-}" ]] || die "HOME or CJ_INSTALL_DIR must be set"
  INSTALL_DIR="$HOME/.local/bin"
fi

download() {
  local url=$1
  local destination=$2

  if command_exists curl; then
    curl --fail --location --silent --show-error \
      --retry 3 --connect-timeout 10 \
      --output "$destination" "$url"
  elif command_exists wget; then
    wget --quiet --tries=3 --timeout=10 --output-document="$destination" "$url"
  else
    die "curl or wget is required"
  fi
}

sha256() {
  local file=$1

  if command_exists sha256sum; then
    sha256sum "$file" | awk '{print $1}'
  elif command_exists shasum; then
    shasum -a 256 "$file" | awk '{print $1}'
  elif command_exists openssl; then
    openssl dgst -sha256 "$file" | awk '{print $NF}'
  else
    die "sha256sum, shasum, or openssl is required for checksum verification"
  fi
}

if [[ -n "$VERSION" ]]; then
  VERSION="${VERSION#v}"
  if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$ ]]; then
    die "CJ_VERSION must be a semantic version"
  fi
fi

OS="$(uname -s)"
ARCH="$(uname -m)"
PLATFORM=
ARCHIVE_KIND=
BINARY_NAME="cj"

case "$OS:$ARCH" in
  Linux:x86_64|Linux:amd64)
    PLATFORM="linux-x86_64"
    ARCHIVE_KIND="tar.gz"
    ;;
  Darwin:x86_64|Darwin:amd64)
    PLATFORM="macos-x86_64"
    ARCHIVE_KIND="tar.gz"
    ;;
  Darwin:arm64|Darwin:aarch64)
    PLATFORM="macos-aarch64"
    ARCHIVE_KIND="tar.gz"
    ;;
  MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64)
    die "Windows is not (yet) supported."
    ;;
  *)
    die "unsupported platform: $OS $ARCH"
    ;;
esac

ARCHIVE_NAME="cjtaskrunner-$PLATFORM.$ARCHIVE_KIND"
if [[ -n "$VERSION" ]]; then
  RELEASE_BASE="https://github.com/$REPOSITORY/releases/download/v$VERSION"
  RELEASE_LABEL="v$VERSION"
else
  RELEASE_BASE="https://github.com/$REPOSITORY/releases/latest/download"
  RELEASE_LABEL="latest"
fi

TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cjtaskrunner-install.XXXXXX")"
ARCHIVE_PATH="$TEMP_DIR/$ARCHIVE_NAME"
CHECKSUM_PATH="$TEMP_DIR/SHA256SUMS"
EXTRACT_DIR="$TEMP_DIR/extract"

say "downloading $RELEASE_LABEL for $PLATFORM"
download "$RELEASE_BASE/$ARCHIVE_NAME" "$ARCHIVE_PATH"
download "$RELEASE_BASE/SHA256SUMS" "$CHECKSUM_PATH"

EXPECTED_CHECKSUM="$(
  awk -v name="$ARCHIVE_NAME" \
    '$2 == name || $2 == ("*" name) { print $1; exit }' \
    "$CHECKSUM_PATH" | tr -d '\r'
)"
[[ -n "$EXPECTED_CHECKSUM" ]] || die "SHA256SUMS does not contain $ARCHIVE_NAME"

ACTUAL_CHECKSUM="$(sha256 "$ARCHIVE_PATH")"
EXPECTED_CHECKSUM="$(printf '%s' "$EXPECTED_CHECKSUM" | tr '[:upper:]' '[:lower:]')"
ACTUAL_CHECKSUM="$(printf '%s' "$ACTUAL_CHECKSUM" | tr '[:upper:]' '[:lower:]')"
[[ "$ACTUAL_CHECKSUM" == "$EXPECTED_CHECKSUM" ]] || die "checksum verification failed for $ARCHIVE_NAME"

mkdir -p "$EXTRACT_DIR"
case "$ARCHIVE_KIND" in
  tar.gz)
    command_exists tar || die "tar is required to extract $ARCHIVE_NAME"
    tar -xzf "$ARCHIVE_PATH" -C "$EXTRACT_DIR"
    ;;
  zip)
    command_exists unzip || die "unzip is required to extract $ARCHIVE_NAME"
    unzip -q "$ARCHIVE_PATH" -d "$EXTRACT_DIR"
    ;;
esac

BINARY_PATH="$(find "$EXTRACT_DIR" -type f -name "$BINARY_NAME" -print -quit)"
[[ -n "$BINARY_PATH" ]] || die "$BINARY_NAME was not found in $ARCHIVE_NAME"

mkdir -p "$INSTALL_DIR"
INSTALL_TEMP="$INSTALL_DIR/.$BINARY_NAME.tmp.$$"
cp "$BINARY_PATH" "$INSTALL_TEMP"
chmod 755 "$INSTALL_TEMP"
mv -f "$INSTALL_TEMP" "$INSTALL_DIR/$BINARY_NAME"
INSTALL_TEMP=

say "installed $INSTALL_DIR/$BINARY_NAME"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    say "add $INSTALL_DIR to PATH to run cj"
    ;;
esac
