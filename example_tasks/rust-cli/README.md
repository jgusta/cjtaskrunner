# Rust CLI Example

This tiny Rust project demonstrates task arguments, `@switch`, parallel CI,
version conditionals, exported app environment, and guarded version bump usage.

## Tasks

- `check`: verifies files and runs `cargo check`.
- `envcheck`: prints taskfile environment values.
- `build ($PROFILE)`: accepts `debug` or `release`.
- `run ($GREETING)`: exports `APP_GREETING` and runs the CLI.
- `test`: runs `cargo test`.
- `ci`: runs environment, check, and test branches in parallel.
- `version`: prints the taskfile app version when it satisfies `>= 0.1.0`.
- `release ($LEVEL)`: accepts `patch`, `minor`, or `major`, bumps the version, and builds release.

## Run

```sh
cj ci
cj build debug
cj run hello
cj version
cj release patch
```

`release` intentionally edits this example's `@version app` value.
