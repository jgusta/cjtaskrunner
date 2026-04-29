#!/bin/sh
set -eu

message_file="${1:-commit-message.txt}"
target="${2:-gibberish.txt}"

{
  printf 'Update generated gibberish\n\n'
  printf 'Generated or modified %s through a CJTasks workflow.\n' "$target"
} > "$message_file"
