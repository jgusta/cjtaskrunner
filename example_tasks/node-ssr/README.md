# Node SSR Example

This tiny server-side rendered Node app demonstrates captured `@set` blocks,
task arguments, `@switch`, filesystem directives, `@open`, and taskfile
versions.

## Tasks

- `check`: runs safe checks, then renders static HTML.
- `summary`: captures and prints an SSR environment summary.
- `mode ($MODE)`: accepts `development` or `production` and exports `NODE_ENV`.
- `start ($MODE)`: starts the SSR server in the selected mode.
- `server:open`: opens the configured local server URL.
- `build ($MODE)`: cleans and recreates `dist/index.html`.
- `clean`: removes `dist`.
- `version`: prints the taskfile `site` version.
- `release`: builds, bumps the example version, and reports the bump.

## Run

```sh
cj check
cj summary
cj build production
cj start development
cj server:open
cj version
```

`start` is long-running. `release` intentionally edits this example's
`@version site` value.
