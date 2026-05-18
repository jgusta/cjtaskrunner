# Release Guide

## One-time setup

1. Create a publisher in Visual Studio Marketplace.
2. Create an Azure DevOps PAT with Marketplace `Manage` scope.
3. Log in once on your machine:

```sh
cd editors/vscode-cjtaskrunner
npx @vscode/vsce login cjtaskrunner
```

## Pre-release checks

```sh
cd editors/vscode-cjtaskrunner
npm ci
npm run compile
npm run package:ci
npm run publish:dry-run
```

## Publish

```sh
cd editors/vscode-cjtaskrunner
npx @vscode/vsce publish
```

## Post-publish

1. Verify listing metadata, icon, and README rendering.
2. Verify install in a clean VS Code profile.
3. Tag release in git and update `CHANGELOG.md`.
