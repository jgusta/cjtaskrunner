# CJTaskrunner Plan

Most original MVP implementation work is done. Stable behavior now lives in `SPEC.md`.

This file tracks remaining work and open product decisions only.

## Remaining Product Work

- Add command-line flags.
- Add task arguments after task name, such as `cj test -- --filter foo`.
- Add multiple-task invocation, if useful.
- Decide whether parent-directory taskfile discovery belongs in CJTaskrunner.
- Decide whether task-level environment blocks belong in the format.
- Decide whether `.env.local` or named env files belong in the format.
- Define Windows shell behavior for `@shell`.
- Decide whether `@return` should become a true flow-control return instead of current status/output expression behavior.
- Decide whether `@set NAME:` should require exactly one expression, or continue accepting a normal block.
- Decide whether `@and` / `@or` should support inline expressions in addition to indented blocks.

## Engineering Work

- Improve diagnostics with column positions where practical.
- Add integration tests for installed binary aliases `cj`, `cjtaskrunner`, and `cjtaskrunner-lsp`.
- Add end-to-end LSP protocol tests for initialize, diagnostics, symbols, completion, hover, and definition.
- Add fixture-based parser tests for valid and invalid taskfiles.
- Add tests for explicit `*.cjtasks` invocation and ambiguous discovery errors at CLI level.
- Add tests for signal termination behavior on Unix.
- Consider converting `include!` feature files into true Rust modules once module boundaries stabilize.
- Review example taskfiles and remove any remaining shell-dependent ordinary lines.
- Consider sharing one structured parser between executor and LSP while preserving LSP error recovery.

## Known Limitations

- No command-line flags.
- No task arguments after the task name.
- No multiple-task invocation.
- No parent-directory taskfile discovery.
- No task-level environment blocks.
- No shell configuration; Unix `@shell` commands use `/bin/sh -c`.
- No Windows shell strategy yet.
- No general YAML parsing.
- No variable expansion in `.env` values.
- No `.env.local` or parent `.env` discovery.
- No full expression AST; control flow is still line and block based.
