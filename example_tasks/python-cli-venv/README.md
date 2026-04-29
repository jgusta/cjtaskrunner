# Python CLI venv Example

This command-line Python example demonstrates explicit runtime modes, exported
environment values, filesystem packaging, and cleanup.

## Tasks

- `check`: prints path information, then awaits the packaging task.
- `pathcheck`: captures and prints Python and venv information.
- `makevenv`: creates `.venv` only when missing.
- `run ($MODE)`: accepts `dryrun` or `live` and exports `CLI_MODE`.
- `package`: stages source under `build/cli` and byte-compiles it.
- `clean`: removes build output and Python bytecode caches.

## Run

```sh
cj check
cj makevenv
cj run dryrun
cj package
cj clean
```

Set `CJ_VENV=/path/to/venv` before running `cj` to select a specific virtual
environment.
