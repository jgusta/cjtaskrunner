<div style="display:flex;align-items:center;justify-content:center;margin-bottom:0;">
  <img src="theme/cj-logo-color-f.svg" alt="CJTaskrunner logo" style="margin:0;display:block;height:auto;width:80px;" />
  <img src="theme/cj-words.svg" alt="CJTaskrunner wordmark" style="margin:0;display:block;width:320px;height:auto;" />
</div>
<div style="margin:0;text-align:center;font-size:1.4em;">
  Independent task wrangler
</div>
<div style="display:flex;align-items:center;justify-content:center;margin-bottom:0;">
  <img alt="license" src="https://img.shields.io/github/license/jgusta/cjtaskrunner">
  <img alt="version" src="https://img.shields.io/github/v/release/jgusta/cjtaskrunner">
</div>

CJTaskrunner is a small task runner for project-local commands. Put the
commands a project needs in a `cjtasks` file, add short descriptions where they
help, and run them with `cj`.

```cjtasks
check:
  @desc run the project checks
  cargo test --locked
```

```sh
cj check
```

CJTaskrunner is meant to be a command catalog, not a replacement for Cargo,
npm, Python, Docker, or shell scripts. It gives those tools short, discoverable
project names.


{{#toc}}{{/toc}}


## Start Here

- [Manual](manual/index.md) - common taskfile patterns.
- [Integrations](manual/integrations.md) - wrap npm, Cargo, Python,
  Docker, and CI commands.
- [Ecosystem](manual/ecosystem.md) - editors, install paths, and examples.

## Reference

- [CLI](reference/cli.md)
- [Taskfile Format](reference/taskfile.md)
- [Directives](reference/directives.md)
- [Variables](reference/variables.md)

## Explanation

- [Philosophy](manual/philosophy.md)
- [Comparisons](manual/comparisons.md)
