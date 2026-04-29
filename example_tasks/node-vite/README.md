# Node Vite Example

This is a lightweight Vite-style project. It demonstrates npm script commands, taskfile environment overrides, fallback ports, and `.env` loading.

## Notable Files

- `cjt`: taskfile discovered by CJTasks.
- `.env`: provides `VITE_API_BASE` and `VITE_FEATURE_FLAG`.
- `package.json`: defines `dev`, `build`, and `preview` npm scripts.
- `index.html`: Vite entry HTML.
- `src/main.js`: browser entry module.

## Tasks

- `envcheck`: prints merged Vite-related environment values.
- `base`: verifies the expected files exist.
- `dev`: runs the Vite dev server on `127.0.0.1` and `PORT`.
- `build`: runs the Vite production build.
- `preview`: runs Vite preview on `127.0.0.1` and `PORT`.

## Run

Safe checks that do not require installed npm dependencies:

```sh
cargo run -- example_tasks/node-vite envcheck
cargo run -- example_tasks/node-vite base
```

Dependency-backed commands:

```sh
cargo run -- example_tasks/node-vite dev
cargo run -- example_tasks/node-vite build
cargo run -- example_tasks/node-vite preview
```

## Prerequisites and Caveats

`dev`, `build`, and `preview` intentionally are not run unless Node.js is installed and dependencies have been installed with `npm install`. `dev` and `preview` are long-running server commands.
