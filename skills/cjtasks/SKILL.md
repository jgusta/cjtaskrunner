---
name: cjtasks
description: Create, edit, run, and troubleshoot CJTaskrunner taskfiles and CLI workflows.
---

# CJTaskrunner

- Preserve existing task names/organization.
- Never invent syntax. Use `cj --help`; use `cj --directives` for omitted directive details.
- Sources: [manual](https://jgusta.github.io/cjtaskrunner/), [directives](https://jgusta.github.io/cjtaskrunner/reference/directives.html), [repo](https://github.com/jgusta/cjtaskrunner).
- Errors usually state the fix; follow that guidance first.
- Syntax is indentation-sensitive, not YAML.

## Taskfiles

- Base: `cjtasks`.
- Overlay precedence (low to high): `production.cjtasks`, `staging.cjtasks`, `development.cjtasks`, `local.cjtasks`.
- Overlays replace whole tasks/env entries; task overrides must preserve arity.
- Only base may declare `@version` or version bumps.
- Discovery checks only the selected directory; never parents/descendants.
- `cj --init`: create empty taskfile; never overwrite one.
- `cj --auto`: add root `package.json`, `deno.json`, `Makefile`, and argument-free `Justfile` tasks; never overwrite CJ tasks. Package scripts take priority; rename collisions `build`, `build2`, `build3` (no separator).

## Shape

- Generate/edit with 2-space indents. Consistent tabs parse; `cj --format` converts leading indentation to spaces.
- Full-line `#` is a comment; inline `#` is command input.
- Put `@version`, `@help:`, `@env:` before tasks.
- Spell `@env:`/`@help:` exactly; `env:`/`help:` define tasks.
- Variables are forbidden in `@desc`/`@help:`; all indented `@help:` content is literal text.

## Tasks

- Name parts: ASCII letters, digits, `-`, `_`.
- One-level nested headings produce `parent:child`; invoke `cj web:build`.
- Declare required positional args as `deploy (TARGET, TAG):`; invoke `cj deploy production v1.2.3`.
- No optional/default/variadic argument syntax. Argument variables are call-local.
- Leading `_` hides a task from summaries; descendants of hidden parents are hidden.
- Task names cannot match directories beside the taskfile.

## Commands

- Ordinary lines execute argv directly: no pipes, redirection, globs, command substitution, chaining, or shell builtins.
- Use `@shell` only for shell syntax.
- `@open` accepts exactly one `http://` or `https://` URL.

## Variables/environment

- `$NAME`, `${NAME}`: empty if absent.
- `${NAME?}`: error if absent.
- `${NAME?fallback}`, `${NAME?"fallback text"}`: fallback if absent.
- `\$NAME`: literal marker.
- Interpolated ordinary-command value stays one argv; `@shell` shell-quotes interpolations.
- Top-level `@env:`: `NAME: value` overrides inherited value; `NAME?: value` sets only if absent.
- `@set NAME value`: runtime variable.
- `@set NAME:` + indented block: capture stdout.
- `@export NAME`: expose runtime variable to children.
- `@unset NAME`: remove runtime value/export.
- Runtime variables remain internal until exported.

## Flow/composition

- `@task name args...`: sequential call sharing runtime/cwd; restores callee argument/directory scopes afterward. Recursion errors.
- `@and` runs after success; `@or` after failure.
- Status: `@success`, `@fail`, `@return`, `@stop`.
- Branching: `@if`, `@if-not`, `@if-in`, `@if-not-in`, `@else`, `@if-exists`, `@if-not-exists`, `@if-set`, `@if-not-set`, `@switch`, `@case`, `@default`.
- Membership: `@if-in $VALUE one two three`.
- `@await task...`: parallel argument-free tasks. Its optional block runs after all succeed; same-level `@or` handles failure.
- Awaited tasks clone runtime/cwd, may mutate files, but cannot use `@set`, `@export`, `@unset`, or version bumps, including via static `@task` calls.
- Await cycles/missing targets are parse errors. Positive-integer `CJ_JOBS` limits parallelism.

## Versions

- Top-level SemVer excludes build metadata; `@version app 1.2.0` creates `$VERSION_APP`.
- Bumps: `@major`, `@minor`, `@patch`, `@pre`, `@release`; each named version may bump once/invocation.
- Conditions: `@if-version`, `@if-not-version`, `@if-bumped`, `@if-not-bumped`, and kind pairs such as `@if-patch`/`@if-not-patch`.
- Argumentless `@if-bumped` matches any bump; argumentless `@if-not-bumped` matches no bumps.

## Paths/docs

- `@cd`/`@back`: scoped cwd changes.
- `@mkdir`, `@clean`, `@cp`, `@cpdir`, `@rename`: filesystem operations; relative paths use task cwd.
- `@desc`: one-line summary. `@help:`: indented details. For help/subtask-only tasks, `@selfhelp` prints current help, then succeeds/stops.

## CLI

- `cj`: list visible tasks.
- `cj <task> [args...]`: run.
- `cj <directory-or-taskfile> <task> [args...]`: select taskfile and run.
- `cj help [task]`: taskfile/task help.
- `cj --init`: initialize.
- `cj --auto`: additive import.
- `cj --format [directory-or-taskfile]`: format in place.
- `cj --run <line>`: execute one non-block line without taskfile.
- `cj --directives`: list directives.
- `cj --completions <bash|zsh|fish>`: print completions.
- `cj --install-completions <bash|zsh|fish>`: install completions.
- `cj lsp`: stdio language server.
- Nonempty `NO_COLOR`: stable plain output for scripts/tests.

## Validate edits

1. `cj --format`.
2. `cj` (parse + summary visibility).
3. `cj help <task>` for changed help/nesting.
4. Run narrowest affected task.
5. Verify `@await` targets are argument-free/mutation-safe.
