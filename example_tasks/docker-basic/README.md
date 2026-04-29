# Docker Basic Example

This Docker example demonstrates task arguments, runtime exports, hidden helper
tasks, awaited validation, and nested Compose helpers.

## Tasks

- `check`: prints environment values and awaits file validation.
- `image ($TAG)`: builds a Docker image with `local`, `test`, or `latest`.
- `run ($TAG)`: builds and runs the tagged image.
- `compose:config`: renders Docker Compose configuration.
- `compose:up ($TAG)`: exports `IMAGE_REF` and starts Compose.
- `clean`: removes local scratch output.

## Run

Safe checks:

```sh
cj check
```

Docker-backed commands:

```sh
cj image local
cj run local
cj compose:config
cj compose:up local
```

Docker and Docker Compose must be installed for Docker-backed commands.
