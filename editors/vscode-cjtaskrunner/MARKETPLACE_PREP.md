# VS Code Marketplace Release Checklist

This checklist is for shipping the `cjtaskrunner-vscode` extension to the public Visual Studio Marketplace.

## 1) Publisher and access setup

- [ ] Create or verify the Marketplace publisher at `https://marketplace.visualstudio.com/manage/publishers/`.
- [ ] Ensure the extension `publisher` in `package.json` exactly matches the publisher ID.
- [ ] Create an Azure DevOps Personal Access Token (PAT) with Marketplace publish/manage scopes.
- [ ] Store the token securely (for example: CI secret as `VSCE_PAT`).

References:
- https://code.visualstudio.com/api/working-with-extensions/publishing-extension
- https://learn.microsoft.com/en-us/azure/devops/extend/publish/overview?view=azure-devops

## 2) Marketplace metadata readiness

- [ ] Keep `private` unset/false in `package.json` (publishing is blocked if `private: true`).
- [ ] Confirm `name`, `displayName`, `description`, `version`, `publisher`, and `engines.vscode` are correct.
- [ ] Add `keywords` for discoverability.
- [ ] Add `icon` (minimum 128x128, recommended 256x256).
- [ ] Add `bugs.url` and `homepage`/`repository` links.
- [ ] Verify categories are accurate for discovery.

Reference:
- https://code.visualstudio.com/api/references/extension-manifest

## 3) Repository and release files

- [ ] Ensure extension README includes install/usage notes and feature list.
- [ ] Add and maintain `CHANGELOG.md` for versioned release notes.
- [ ] Ensure `LICENSE` is present in the extension folder.
- [ ] Optionally add `.vscodeignore` so packaged VSIX excludes unnecessary files.

## 4) Local validation and packaging

Run from `editors/vscode-cjtaskrunner`:

```sh
npm ci
npm run compile
npx @vscode/vsce package
```

- [ ] Build is clean (`npm run compile`).
- [ ] VSIX package succeeds without validation errors.
- [ ] Install the VSIX locally and smoke-test language features + task runner view.

## 5) Publish flow

- [ ] Log in for publish tooling: `npx @vscode/vsce login <publisher>`.
- [ ] Publish first version: `npx @vscode/vsce publish`.
- [ ] Verify listing visibility and metadata in Marketplace page.
- [ ] Tag release in git and record notes in changelog.

## 6) Recommended CI automation

- [ ] Add CI job that runs compile + package checks on pull requests.
- [ ] Add protected/manual release workflow for tagged versions.
- [ ] Publish via CI using `VSCE_PAT` secret and explicit version bump strategy.

## Current repo gaps to close before first publish

- [ ] `private` is currently `true` in `package.json` and must be removed/false.
- [ ] `keywords` are not set.
- [ ] `icon` path is not set.
- [ ] `bugs.url` is not set.
- [ ] `CHANGELOG.md` is not present in the extension directory.
