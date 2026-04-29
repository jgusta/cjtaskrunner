# Python Pipenv Example

This project demonstrates a Pipenv-oriented taskfile. It keeps Pipenv commands in the taskfile without requiring the example setup to create an environment.

## Notable Files

- `cjtasks`: taskfile discovered by CJTasks.
- `.env`: provides `PIPENV_DOTENV_LOCATION` and `PIPENV_EXAMPLE_MESSAGE`.
- `Pipfile`: empty app and dev dependency sets for the example.
- `app/pipenv_app.py`: module executed by the `run` task.
- `tests/test_pipenv_app.py`: unittest target.

## Tasks

- `envcheck`: prints merged Pipenv-related environment values.
- `install`: runs `pipenv install --dev`.
- `run`: runs the app through `pipenv run`.
- `test`: runs unittest discovery through `pipenv run`.
- `where`: prints the Pipenv virtualenv path.

## Run

Safe check without Pipenv:

```sh
cargo run -- example_tasks/python-pipenv envcheck
```

Pipenv-backed commands:

```sh
cargo run -- example_tasks/python-pipenv install
cargo run -- example_tasks/python-pipenv where
cargo run -- example_tasks/python-pipenv run
cargo run -- example_tasks/python-pipenv test
```

## Prerequisites and Caveats

`install`, `where`, `run`, and `test` intentionally are not run unless `pipenv` is installed and available on `PATH`. `install` may need network access.
