# Rust CLI Example

This is a tiny Rust CLI project with Cargo-oriented CJTasks. It demonstrates taskfile fallback environment values and commands that run from the example directory.

## Notable Files

- `cjt`: taskfile discovered by CJTasks.
- `.env`: provides `RUST_LOG`.
- `Cargo.toml`: Rust package metadata.
- `src/main.rs`: tiny CLI program.

## Tasks

- `envcheck`: prints merged Rust-related environment values.
- `base`: verifies expected files exist.
- `check`: runs `cargo check`.
- `run`: runs the CLI with `cargo run`.
- `test`: runs `cargo test`.
- `release`: builds with `cargo build --release`.

## Run

Common commands:

```sh
cargo run -- example_tasks/rust-cli envcheck
cargo run -- example_tasks/rust-cli base
cargo run -- example_tasks/rust-cli check
cargo run -- example_tasks/rust-cli run
cargo run -- example_tasks/rust-cli test
```

Release build:

```sh
cargo run -- example_tasks/rust-cli release
```

## Prerequisites and Caveats

These tasks expect Rust and Cargo to be installed. No crates beyond the standard library are required. `release` writes build output under this example's `target` directory.
