---
name: commit-and-push
description: Commit and push Classic Classics changes, then watch GitHub Actions until every workflow for the pushed commit succeeds. Use only when the user explicitly asks to make a commit and push it.
---

# Commit, push, and verify workflows

Use this workflow only when the user explicitly asks to commit and push. Do not
run it automatically during ordinary edits. Never force-push, rewrite history,
delete tags, or modify another repository.

## 1. Preflight

```sh
git status --short
git diff --check
git branch --show-current
git log -5 --oneline
```

Review every changed and untracked path. Never stage generated or local data:

- `target/`
- `dist/`
- `data/`
- local screenshots, JSON presets, or logs
- credentials, tokens, or anything outside the repository

If an unexpected change belongs to the user, stop and ask instead of committing
or deleting it.

## 2. Verify locally

Run from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --no-default-features
```

If packaging, metadata, artwork, the catalog entry, or release workflows changed:

```sh
python3 DevUtils/package.py
python3 DevUtils/verify_package.py dist/ClassicClassics-*-linux-x64.zip
```

The full native test may need a Vulkan 1.3 desktop. Headless tests must still
pass without it. Do not commit if a required local check fails.

## 3. Stage and commit

Stage only the reviewed paths. For an intentional all-files change:

```sh
git add -A
git status --short
```

Otherwise stage explicit paths:

```sh
git add src/ README.md docs/ .agents/ .github/ DevUtils/ Cargo.toml Cargo.lock ClassicClassics.json
```

Write a descriptive Conventional Commit message, for example:

```sh
git commit -m "feat(settings): add in-game settings overlay and fullscreen controls"
```

Do not amend a commit that is already pushed. Make a follow-up commit instead.

## 4. Push

```sh
git push origin HEAD
```

If the remote moved, inspect it and create a merge/rebase that preserves the
user's work. Never use `git push --force` or `--force-with-lease` without explicit
permission.

## 5. Watch every workflow

Get the pushed SHA and active runs:

```sh
git rev-parse HEAD
gh run list --branch "$(git branch --show-current)" --limit 10
```

Watch each run for this commit:

```sh
gh run watch <run-id> --exit-status
```

Requirements:

- Every workflow triggered by the push must finish successfully.
- If `CI` fails, fetch only the failing logs:

  ```sh
  gh run view <run-id> --log-failed
  ```

- Fix the cause, rerun the local verification, commit the fix, push again, and
  watch the new runs.
- If a tag release workflow fails because the tag and Cargo version differ, fix
  the version or tag deliberately. Do not delete or move a published tag without
  explicit user approval.
- If `gh` is unavailable or unauthenticated, report that clearly and provide the
  commit URL plus the exact command the user can run to watch the run. Do not
  claim success without evidence.

## 6. Final report

Report:

- commit SHA and message
- branch and push result
- workflow names, run IDs, URLs and final conclusions
- any check that could not be run and why

Leave the working tree clean unless the user asked to keep further changes
uncommitted.
