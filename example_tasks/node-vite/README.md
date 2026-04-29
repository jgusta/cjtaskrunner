# Node Vite Example

This Vite-style project demonstrates modern CJTaskrunner workflow syntax:
runtime variables, task arguments, membership conditionals, parallel startup,
and browser opening.

## Tasks

- `check`: prints environment values and awaits file validation.
- `mode ($MODE)`: accepts `local`, `staging`, or `production`, then exports `VITE_MODE`.
- `dev`: simulates client/server startup with `@await` and opens the dev URL.
- `browser`: builds a runtime URL with `@set` and opens it.
- `build ($MODE)`: validates files, selects a mode, and runs `npm run build`.
- `preview`: builds production assets and starts Vite preview.

## Run

```sh
cj check
cj mode staging
cj dev
cj build production
cj preview
```

`build` and `preview` require Node.js dependencies to be installed.
