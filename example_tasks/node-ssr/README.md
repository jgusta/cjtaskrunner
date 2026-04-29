# Node SSR Example

This example is a tiny server-side rendered Node app. It uses the `cjtasks` filename and demonstrates `.env` plus taskfile fallback environment values for server configuration.

## Notable Files

- `cjtasks`: taskfile discovered by CJTasks.
- `.env`: provides `SSR_GREETING`.
- `package.json`: documents equivalent npm scripts.
- `src/server.js`: starts the HTTP server.
- `src/render.js`: writes static HTML to stdout.

## Tasks

- `envcheck`: prints merged Node environment values.
- `base`: verifies expected files exist.
- `start`: runs `node src/server.js`.
- `build`: creates `dist/index.html` with `src/render.js`.
- `clean`: removes `dist`.

## Run

Inspection and build commands:

```sh
cargo run -- example_tasks/node-ssr envcheck
cargo run -- example_tasks/node-ssr base
cargo run -- example_tasks/node-ssr build
```

Server command:

```sh
cargo run -- example_tasks/node-ssr start
```

## Prerequisites and Caveats

All runtime tasks expect Node.js on `PATH`. `start` is intentionally long-running. `build` writes `dist/index.html`; `clean` removes `dist`.
