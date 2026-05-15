# CJTaskrunner Example Projects

These directories are small example projects that show how `cj` runs project-local taskfiles named `cjtasks` or ending in `.cjtasks`.

From the repository root, run examples like:

```sh
cargo run -- example_tasks/node-vite envcheck
cargo run -- example_tasks/python-venv pyinfo
cargo run -- example_tasks/docker-basic base
```

After `cargo install --path .`, the same examples can be run with `cj`:

```sh
cj example_tasks/node-vite envcheck
cj example_tasks/python-venv pyinfo
cj example_tasks/docker-basic base
```

You can also run from inside an example directory with one argument:

```sh
cd example_tasks/rust-cli
cj check
```

CJTaskrunner behavior demonstrated here:

- Commands run from the taskfile directory, even when `cj` is invoked from elsewhere.
- `.env` is loaded from the taskfile directory only.
- Taskfile `env:` overrides replace inherited environment values.
- Taskfile fallback entries such as `PORT?: 5173` only apply when the variable is absent.
- Ordinary task lines use the round 2 direct argv execution model.
- Shell behavior such as redirects, globbing, command chaining, and shell-local variables belongs behind `@shell`.
- Python examples show `.venv`, `CJ_VENV`, and active `VIRTUAL_ENV` path behavior.

Most example taskfiles were first written for the original shell-per-line MVP. When updating them for round 2, convert shell-dependent lines to `@shell` and keep simple tool invocations as ordinary direct argv lines.

No package installers or Docker builds are required to inspect these examples. Some tasks are realistic commands that need external tools or dependencies before they will succeed; each example README calls those out.

## Examples

- `node-vite`: Vite-style frontend project using `.env`, npm scripts, and fallback ports. Safe tasks without dependencies: `envcheck`, `base`.
- `node-ssr`: Minimal server-side rendered Node app with build and server tasks. Safe tasks with Node installed: `envcheck`, `base`, `build`.
- `python-venv`: Python package layout oriented around `python -m venv .venv`. Safe tasks with Python installed: `pyinfo`, `base`, `makevenv`, `run`, `test`.
- `python-pipenv`: Pipenv-oriented Python app with `pipenv run` tasks. Safe task without Pipenv: `envcheck`.
- `python-cli-venv`: Venv-oriented Python command line project with module execution. Safe tasks with Python installed: `pathcheck`, `makevenv`, `run`, `package`.
- `pyside6-app`: Minimal PySide6 app taskfile for running and smoke-checking GUI code. Safe tasks without PySide6: `pyinfo`, `base`, `makevenv`.
- `docker-basic`: Simple Dockerfile plus Docker Compose task examples. Safe tasks without Docker: `envcheck`, `base`.
- `rust-cli`: Tiny Rust CLI project with cargo tasks. Safe tasks with Cargo installed: `envcheck`, `base`, `check`, `run`, `test`.
- `git-gibberish`: Local git workflow that generates a file, writes a commit message, and commits it. Safe tasks with Git installed: `base`, `initrepo`, `status`.
