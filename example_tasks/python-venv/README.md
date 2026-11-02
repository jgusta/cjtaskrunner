# Python venv Example

This package-style Python project demonstrates local venv discovery, task
arguments, runtime exports, filesystem directives, and awaited checks.

## Tasks

- `check`: prints Python info, then awaits the test task.
- `pyinfo`: captures the Python executable with [`@set NAME:`](../../docs/src/reference/directives.md#set).
- `makevenv`: creates `.venv` only when it is missing.
- `install ($MODE)`: accepts `editable` or `normal`.
- `run ($MESSAGE)`: exports `APP_MESSAGE` and runs the module.
- `test`: runs unittest discovery.
- `package`: stages source under `build/package` with [`@mkdir`](../../docs/src/reference/directives.md#mkdir), [`@cp`](../../docs/src/reference/directives.md#cp), and [`@cpdir`](../../docs/src/reference/directives.md#cpdir).
- `clean`: removes `build`.

## Run

```sh
cj check
cj makevenv
cj install editable
cj run hello
cj package
cj clean
```

CJTaskrunner automatically prepends a local `.venv/bin` to `PATH` after the
virtual environment exists.
