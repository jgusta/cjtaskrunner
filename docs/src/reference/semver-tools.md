# SemVer Tools

CJTaskrunner assists with versioning through built-in directives.

`@version name value` declares a top-level component version and creates
`$VERSION_NAME`, uppercasing the name and converting hyphens to underscores.

```cjtasks
@version cli 0.1.0
@version lsp 0.0.1-alpha.1
```

Version names must contain only ASCII letters, digits, hyphens, and underscores.
Each declaration creates an override environment entry named `VERSION_<NAME>`,
where the name is uppercased and hyphens are converted to underscores.

Version values must be SemVer 2.x `MAJOR.MINOR.PATCH` values with an optional
prerelease suffix. Build metadata is not supported.

`@version` is only valid as a top-level header in `cjtasks`.

Use `@patch`, `@minor`, `@major`, `@pre`, or `@release` inside tasks to change a version.

Examples:

- `@version cli 0.1.0` creates `$VERSION_CLI` with value `0.1.0`.
- `@version language-server 0.0.1` creates `$VERSION_LANGUAGE_SERVER` with value `0.0.1`.
- `@version app 1.2.3-beta.1` creates `$VERSION_APP` with value `1.2.3-beta.1`.
- `@patch app` increments `$VERSION_APP` by one patch version.
- `@pre app alpha` sets the version prerelease to `alpha`.
- `@pre app alpha.` sets the prerelease to `alpha.0`, or increments the trailing number when the current prerelease is already `alpha.N`.

Duplicate version variables are errors.

## Other rules

Each version can be bumped at most once per invocation. Bump state is shared
with normal sequential `@task` calls. Tasks reached through `@await` are
isolated and cannot use version bump directives.

Overlay files may use version conditionals and may call a base task that
performs a bump, but they may not contain `@version` or version bump directives.
A bump always updates `cjtasks` on disk.

## Version conditionals

There is a dedicated set of conditional directives for versioning:

`@if-bumped`
`@if-patch`
`@if-minor`
`@if-major`
`@if-pre`
`@if-release`
