# Git Gibberish Example

This project demonstrates shell command execution around a local git workflow:

1. initialize a local git repo,
2. generate or modify a gibberish file,
3. write a commit message,
4. commit the result.

It is useful for checking command ordering, `.env` values, quoted shell variables, and git commands that use taskfile fallback environment values.

## Notable Files

- `cjt`: taskfile discovered by CJTasks.
- `.env`: sets `GIBBERISH_FILE` and `COMMIT_MESSAGE_FILE`.
- `scripts/generate-gibberish.sh`: writes sample file content.
- `scripts/write-commit-message.sh`: writes the commit message file.
- `README.md`: this documentation file.

## Tasks

- `base`: verifies required scripts exist.
- `initrepo`: runs `git init`.
- `gibberish`: writes the gibberish output file.
- `message`: writes the commit message file.
- `status`: runs `git status --short`.
- `commit`: stages generated files and commits them.
- `all`: initializes, generates, writes a message, stages, and commits.

## Run

Safe checks:

```sh
cargo run -- example_tasks/git-gibberish base
cargo run -- example_tasks/git-gibberish status
```

Workflow commands:

```sh
cargo run -- example_tasks/git-gibberish initrepo
cargo run -- example_tasks/git-gibberish gibberish
cargo run -- example_tasks/git-gibberish message
cargo run -- example_tasks/git-gibberish commit
```

## Prerequisites and Caveats

These tasks expect Git and `sh` to be available. `initrepo`, `gibberish`, `message`, `commit`, and `all` intentionally mutate this example directory by creating `.git` data and generated text files. The `commit` task uses local `git -c user.name=... -c user.email=...` values, so it does not depend on global git identity.
