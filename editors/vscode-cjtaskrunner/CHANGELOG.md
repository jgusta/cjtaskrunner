# Changelog
CJTaskrunner Official VS Code Extension

## Unreleased

- Recognize `cjtasks` and the standard overlay taskfiles, with `cjtasks` taking precedence.
- Hide the Explorer task view unless a workspace root contains a recognized taskfile.
- Avoid descendant taskfile discovery until a root taskfile enables the view.

### Added

- Language server status, restart, and output commands.
- Extension-side document symbols for VS Code Outline when `cj lsp` is unavailable.
- `cjtaskrunner.path` setting for resolving the `cj` executable.

## [0.0.1] - 2026-05-18

### Added

- Initial language support for `cjtasks` and standard overlay taskfiles.
- Syntax highlighting, diagnostics, symbols, hovers, completions, formatting, and definition support.
- Explorer task view with run and refresh commands.
