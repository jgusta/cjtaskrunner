# Python CLI venv Example

This is a venv-oriented Python example focused on module execution and explicit `CJ_VENV` selection.

## Notable Files

- `cjtasks`: taskfile discovered by CJTaskrunner.
- `.env`: provides `CLI_NAME`.
- `pyproject.toml`: package metadata for the example.
- `src/local_cli/__main__.py`: module executed by the `run` task.

## Tasks

- `pathcheck`: prints Python executable, virtualenv status, and first `PATH` entry.
- `makevenv`: creates `.venv`.
- `run`: runs `python3 -m local_cli`.
- `package`: byte-compiles `src`.
- `clean`: removes Python `__pycache__` directories under `src`.

## Run

Basic commands:

```sh
cargo run -- example_tasks/python-cli-venv pathcheck
cargo run -- example_tasks/python-cli-venv makevenv
cargo run -- example_tasks/python-cli-venv run
cargo run -- example_tasks/python-cli-venv package
```

Explicit virtualenv selection:

```sh
CJ_VENV=/path/to/venv cargo run -- example_tasks/python-cli-venv pathcheck
```

## Prerequisites and Caveats

These tasks expect Python 3 on `PATH`. If `CJ_VENV` is set, CJTaskrunner uses that virtual environment before looking for a local `.venv`; the selected environment must have a `bin` directory. `clean` removes generated `__pycache__` directories.
