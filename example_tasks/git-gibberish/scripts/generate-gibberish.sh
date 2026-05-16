#!/bin/sh
set -eu

target="${1:-gibberish.txt}"
timestamp="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"

{
  printf 'gibberish generated at %s\n' "$timestamp"
  printf 'flim blorv snacket wobble\n'
  printf 'zint quabble nork floop\n'
} >> "$target"
