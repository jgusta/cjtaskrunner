# Python venv Example

This project shows a conventional Python package layout and local `.venv` workflow. CJTaskrunner automatically prepends `.venv/bin` to `PATH` after the virtual environment exists.

## Notable Files

- `cjtasks`: taskfile discovered by CJTaskrunner.
- `.env`: provides `APP_MESSAGE`.
- `pyproject.toml`: package metadata and console script.
- `src/demo_app/__main__.py`: module executed by the `run` task.
- `tests/test_demo_app.py`: unittest target.

## Tasks

- `pyinfo`: prints Python executable and virtualenv status.
- `base`: verifies expected files exist.
- `makevenv`: creates `.venv`.
- `install`: installs the package editable into the active Python environment.
- `run`: runs `python3 -m demo_app`.
- `test`: runs unittest discovery.

## Run

Basic commands:

```sh
cargo run -- example_tasks/python-venv pyinfo
cargo run -- example_tasks/python-venv base
cargo run -- example_tasks/python-venv makevenv
cargo run -- example_tasks/python-venv run
cargo run -- example_tasks/python-venv test
```

Optional editable install:

```sh
cargo run -- example_tasks/python-venv install
```

## Prerequisites and Caveats

These tasks expect Python 3 on `PATH`. `install` is included as a realistic packaging task and may need Python packaging tooling or network access depending on the environment.
