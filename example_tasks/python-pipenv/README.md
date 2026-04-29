# Python Pipenv Example

This Pipenv project demonstrates safe checks that do not install dependencies,
hidden prerequisite tasks, ordered composition, and exported runtime messages.

## Tasks

- `check`: runs dependency-free environment and file checks in parallel.
- `envcheck`: prints Pipenv-related environment values.
- `install`: verifies `pipenv`, then installs dependencies.
- `run ($MESSAGE)`: exports `PIPENV_EXAMPLE_MESSAGE` before running the app.
- `test`: runs unittest discovery through Pipenv.
- `where`: prints the Pipenv virtualenv path.
- `ci`: installs and tests through Pipenv.

## Run

```sh
cj check
cj install
cj run hello
cj test
cj where
cj ci
```

Pipenv-backed tasks require `pipenv` on `PATH`.
