# PySide6 App Example

This is a minimal PySide6 project. It demonstrates Python path handling, `.env` loading, a GUI-friendly fallback environment value, and optional local `.venv` use.

## Notable Files

- `cjt`: taskfile discovered by CJTasks.
- `.env`: provides `PYSIDE_WINDOW_TITLE`.
- `app/simple_window.py`: minimal PySide6 module and smoke-test helper.

## Tasks

- `pyinfo`: prints Python executable, virtualenv status, and Qt platform value.
- `base`: verifies expected files exist.
- `makevenv`: creates `.venv`.
- `install`: installs PySide6 into the active Python environment.
- `run`: launches the PySide6 module.
- `smoke`: imports the module and prints the configured window title.

## Run

Safe checks that do not require PySide6:

```sh
cargo run -- example_tasks/pyside6-app pyinfo
cargo run -- example_tasks/pyside6-app base
cargo run -- example_tasks/pyside6-app makevenv
```

Dependency-backed commands:

```sh
cargo run -- example_tasks/pyside6-app install
cargo run -- example_tasks/pyside6-app smoke
cargo run -- example_tasks/pyside6-app run
```

## Prerequisites and Caveats

`install` expects Python packaging access and may need network access. `run` and `smoke` intentionally are not run unless PySide6 is installed in the active environment or project `.venv`. The taskfile defaults `QT_QPA_PLATFORM` to `offscreen` when absent.
