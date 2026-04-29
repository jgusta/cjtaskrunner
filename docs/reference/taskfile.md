# Taskfile Format

## Filesystem

A CJTaskrunner taskfile is a plain-text file named `cjtasks`.

The CLI does not search parent or child directories.

CJTaskrunner uses the taskfile's directory as the project root. Relative paths
inside the taskfile are resolved from that directory.

## File syntax

The simplest taskfile defines a task name and a command:

```cjtasks
hello:
  echo "hello world"
```

Run the task from the same directory:
```shell
cj hello
```

Running `cj` without arguments displays the available tasks:

```shell
> cj
Tasks in /Users/me/my-project/cjtasks:
  hello
```

Adding `@desc` to a task displays a short description in the summary:

In the `cjtasks` file:
```cjtasks
hello:
  @desc prints "hello world" and exits
  echo "hello world"

goodbye:
  @desc prints "goodbye" and exits
  echo "goodbye"
```

## Command Lines

Tasks are bash-like in that they resemble `bash` commands, and most of the time
you should be able to just use any bash command as a task. Commands are split
into arguments before invoking them on the named executable. Variables are
substituted before the command is run, so the command will only see the resolved
value.

Glob arguments will not work because they are not expanded by CJTaskrunner and
they will not be seen by a shell. So you cannot have a task for example `ls *`
because the `*` is a `bash` token that expands to a list of file names before
being sent to `ls`.

However, if you do need to use shell-specific features, you can use the `@shell` directive.

CJTaskrunner uses a small domain-specific language, so taskfiles require no
language runtime beyond the operating system shell used by `@shell`.

The language is line-based and indentation-sensitive. The official format uses
two spaces per indentation level. The parser also accepts one leading tab per
level when the whole file uses tabs consistently. Indented lines beneath a
header form a block. Task definitions may nest one level.

Top-level entries are task definitions, `@help:`, `@env:`, or `@version`
headers.

Blank lines are ignored. Comment-only lines start with optional indentation
followed by `#` and are ignored.

Top-level entries are either:

- `@version <name> <value>`
- `@help:`
- `@env:`
- `<task-name>:`
- `<task-name> (<argument>, ...):`

Indented entries must use full indentation levels. A file may not mix leading
spaces and leading tabs for indentation. Run `cj --format` to normalize
indentation to spaces.

Inline comments are not stripped. `echo # hi` passes `#` and `hi` as command arguments.

## Taskfile Discovery

When discovering inside a directory, CJTaskrunner loads `cjtasks` as the base
file when it exists. It then loads existing overlays from lowest to highest
precedence:

1. `cjtasks`
2. `production.cjtasks`
3. `staging.cjtasks`
4. `development.cjtasks`
5. `local.cjtasks`

Before validation and execution, CJTaskrunner flattens the selected files into
one effective taskfile.

Task definitions from a higher precedence replace lower ones, but must have the
same number of arguments. Tasks not replaced by a higher layer remain available,
and cross-layer task references are valid.

The bottom file must be named `cjtasks` and is the base layer if it exists. This
is the only layer that can hold `@version` directives, and any version related
directives use the `@version` from the base layer.

You can overwrite a parent task alone and it will still inherit subtasks. You
can overwrite subtasks as well, but due to the way they are defined, you will
also need to overwrite the parent task. A syntax to extend subtasks may be added
in the future.

Environment entries replace lower entries by name. Optionality (`:` or `?:`) is overwritten as well.

The highest-precedence `@help:` block wins.

## Tasks

Task keys are top-level lines, or nested task headings inside another task:

```cjtasks
build:
  cargo build

ext:
  @desc extension tasks
  build:
    @desc build extension assets
    cargo build
```

The nested `build:` task above is addressed as `ext:build`. Task definitions are
limited to one nested level.

Tasks may declare required named positional arguments:

```cjtasks
say (NAME, PUNCTUATION):
  echo hello $NAME$PUNCTUATION
```

Argument values become runtime variables only for the called task. Previous
values with the same names are restored when the task returns. Missing, extra,
or undeclared arguments are errors before task execution begins.

Task names must match:

```text
^[A-Za-z0-9_-]+(:[A-Za-z0-9_-]+)*$
```

`env` and `help` are valid task names. The directives are distinguished by
their `@` prefix.

Task names must not match a directory in the selected taskfile's directory.

Task lines are non-empty indented lines under a task. They run in order.
Execution stops on first non-zero status unless a *same-level* `@or` handles it.

Execution stops with an error after 100,000 task steps as possible infinite-loop protection.

Semicolons split multiple *same-level* expressions on one physical line.
Semicolons inside single or double quotes are preserved.

Semicolons *cannot* replace a newline where an indentation increase follows immediately.

```cjtasks
run:
  @set MODE prod; @if $MODE == prod
    cargo build --release
```

`@if` accepts a truthy value, `==`, and `!=` string comparisons. Use `@if-in`
or `@if-not-in` for exact case-sensitive list membership:

```cjtasks
compile (TARGET):
  @if-in $TARGET linux macos windows
    @echo supported target
```

Membership requires at least one candidate. Quoted values remain one word, so
`@if-in $TARGET linux "mac os" windows` can match `mac os`.

## Ordinary Commands

Ordinary task lines execute directly as argv commands through the platform
process API.

```cjtasks
build:
  cargo build --release
```

This executes as:

```text
["cargo", "build", "--release"]
```

Ordinary commands do not interpret shell syntax:

- pipes
- redirects
- globbing
- command chaining
- shell builtins
- `~` home-directory expansion
- `NAME=value command` environment prefixes
- backslash line continuations

Use `@shell` for shell behavior.

For command-specific environment variables, prefer exported task variables:

```cjtasks
electron-install:
  rm -rf node_modules
  rm -rf $HOME/.electron-gyp/9.4.4
  @export npm_config_runtime electron
  @export npm_config_target 9.4.4
  @export npm_config_disturl https://electronjs.org/headers
  @export npm_config_arch x64
  @export npm_config_force_process_config true
  npm install
```

Or keep the shell syntax on a single `@shell` line:

```cjtasks
electron-install:
  rm -rf node_modules
  rm -rf $HOME/.electron-gyp/9.4.4
  @shell npm_config_runtime=electron npm_config_target=9.4.4 npm_config_disturl=https://electronjs.org/headers npm_config_arch=x64 npm_config_force_process_config=true npm install
```

Child stdin, stdout, and stderr are inherited unless a command is being captured
by `@set NAME:`.

## Word Splitting

CJTaskrunner splits command and directive arguments with shell-like quote handling:

- Whitespace separates words outside quotes.
- Single and double quotes group text.
- Quotes are removed.
- Backslash can escape the next character.
- Unterminated quotes are errors.

This is not a full shell parser.

## Interpolation

Interpolation applies to ordinary command argv tokens, `@shell` text, and directive arguments.

Supported forms:

```text
$NAME
${NAME}
${NAME?}
${NAME?fallback}
${NAME?"fallback value"}
```

Rules:

- `$NAME` reads the current CJTaskrunner variable value, or an empty string when absent.
- `${NAME}` reads the current variable value, or an empty string when absent.
- `${NAME?}` reads the current variable value and errors when absent.
- `${NAME?fallback}` and `${NAME?"fallback value"}` use the fallback when `NAME` is absent.
- `\$NAME` and `\${NAME}` escape interpolation.
- Variable references are invalid in `@desc` text and every line of an `@help:` block. Escape the dollar sign to include literal variable-shaped text there; the escape is not displayed.
- Directive operands are interpolated as strings before validation, including operands that name variables.
- In ordinary commands, an interpolated value remains one argv value.
- In `@shell`, interpolated values are shell-quoted before `/bin/sh -c`.

CJTaskrunner does not support shell-style command substitution, arithmetic
expansion, pattern replacement, or nested expansion.
