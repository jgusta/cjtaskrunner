# PySide6 App Example

This minimal GUI project demonstrates environment defaults for headless checks,
task arguments, exported Qt platform values, and exported window titles.

## Tasks

- `check`: prints Python info and awaits file validation.
- `pyinfo`: captures the Python executable and prints Qt environment values.
- `makevenv`: creates `.venv` only when missing.
- `install`: installs PySide6.
- `platform ($NAME)`: accepts `offscreen`, `cocoa`, `xcb`, or `windows`.
- `run ($PLATFORM)`: selects a Qt platform and starts the GUI.
- `smoke ($TITLE)`: exports `PYSIDE_WINDOW_TITLE` and prints the configured title.

## Run

```sh
cj check
cj makevenv
cj install
cj smoke "CJ Window"
cj run offscreen
```

`run` and `smoke` require PySide6 to be installed.
