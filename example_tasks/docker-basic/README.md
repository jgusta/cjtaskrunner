# Docker Basic Example

This example demonstrates Docker-oriented tasks in a project that uses the `cjtasks` filename as the default taskfile name. It also shows `.env` loading for container configuration.

## Notable Files

- `cjtasks`: taskfile discovered by CJTaskrunner.
- `.env`: provides `APP_PORT` and `APP_MESSAGE`.
- `Dockerfile`: image definition for the tiny Python HTTP server.
- `compose.yaml`: Docker Compose entrypoint for the same app.
- `app/server.py`: application code copied into the image.

## Tasks

- `envcheck`: prints merged environment values.
- `base`: verifies the expected files exist.
- `build`: builds the Docker image.
- `run`: runs the image and publishes `APP_PORT`.
- `composeconfig`: renders Docker Compose configuration.
- `composeup`: starts the Compose app with build enabled.

## Run

Safe checks that do not require Docker:

```sh
cargo run -- example_tasks/docker-basic envcheck
cargo run -- example_tasks/docker-basic base
```

Docker-backed commands:

```sh
cargo run -- example_tasks/docker-basic build
cargo run -- example_tasks/docker-basic run
cargo run -- example_tasks/docker-basic composeconfig
cargo run -- example_tasks/docker-basic composeup
```

## Prerequisites and Caveats

`build`, `run`, `composeconfig`, and `composeup` intentionally are not run unless Docker and Docker Compose are installed and available on `PATH`. `composeup` is long-running until stopped.
