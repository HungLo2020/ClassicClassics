---
name: release
description: Explicitly publish a Classic Classics Basalt ZIP through the manual Release workflow, replacing the single rolling Latest release and verifying the downloaded asset. Ordinary commits and pushes do not release.
---

# Publish Classic Classics Latest

Use only when the user explicitly asks to publish a release or invokes this skill.
Creating/editing release configuration and normal commit-and-push requests do not
authorize dispatch. `.github/workflows/release.yml` is manual-only; CI never calls it.

## Prepare the exact commit

Work only in Classic Classics. Resolve `origin/main` and the intended full SHA.
The local checkout used for verification must match that SHA and be clean. Do
not include unrelated working changes or commit implicitly; use `commit-and-push`
when the user also authorized committing/pushing the release changes.

Run the repository's required local checks and build/verify the Basalt archive:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --no-default-features
python3 -m unittest DevUtils.test_release
python3 DevUtils/package.py
python3 DevUtils/verify_package.py dist/ClassicClassics-*-linux-x64.zip
```

Check GitHub's `main` SHA and watch CI for that exact commit:

```sh
gh api repos/HungLo2020/ClassicClassics/branches/main --jq .commit.sha
gh run list --workflow ci.yml --branch main --commit <sha>
gh run watch <ci-run-id> --exit-status
```

If CI has not run, dispatch `ci.yml` on `main` and watch it. Release only after
successful CI. If main moved, resolve and review the new target rather than
silently releasing a different commit.

## Explicit publication

Record existing Release run IDs so the new run can be identified. Then dispatch:

```sh
gh workflow run release.yml --ref main -f commit=<full-sha>
gh run list --workflow release.yml --branch main --event workflow_dispatch \
  --json databaseId,headSha,displayTitle,status,conclusion,url
gh run watch <new-release-run-id> --exit-status
```

The run title includes the requested SHA. The workflow checks out that SHA,
requires successful CI and the current main head, builds/verifies the ZIP, and
uploads both an Actions artifact and a GitHub Release asset.

`DevUtils/release.py` reuses a single release named/tagged **Latest**. It verifies
the uploaded bytes before replacing the old matching ZIP, removes stale version
ZIPs, moves **only** `Latest` to the requested SHA, and marks it as GitHub's latest
non-draft/non-prerelease. Moving this rolling tag is part of explicit release
authorization. Never create a version tag or delete unrelated releases/tags.
The ZIP remains `ClassicClassics-<Cargo version>-linux-x64.zip`, rooted only at
`app/`; user data never belongs in the archive.

If the workflow fails, inspect `gh run view <id> --log-failed`. Correct an
understood failure within the authorized scope and retry once. Stop and report
an unresolved failure rather than repeatedly mutating releases. A failed first
upload may leave a draft; the next attempt reuses it. Never claim publication
based only on dispatch success or an Actions artifact.

## Verify what Basalt will download

Read `releases/latest` and `git/ref/tags/Latest` through `gh api`. Require:

- `tag_name` is exactly `Latest`, not a draft or prerelease;
- the tag's commit matches the requested SHA;
- exactly one asset matches `ClassicClassics.json`, with the expected Cargo
  version filename and a nonzero size.

Download both the Release ZIP and the workflow's Actions artifact into separate
fresh directories under ignored `target/`:

```sh
gh release download Latest --repo HungLo2020/ClassicClassics \
  --pattern 'ClassicClassics-*-linux-x64.zip' --dir <release-dir>
gh run download <release-run-id> --name ClassicClassics-linux-x64 --dir <actions-dir>
python3 DevUtils/verify_package.py <release-dir>/ClassicClassics-<version>-linux-x64.zip
sha256sum <release-dir>/*.zip <actions-dir>/*.zip
```

Require equal SHA-256 hashes, also matching GitHub's asset digest when provided.
Report the release URL, direct ZIP link, commit, workflow run and verification
result. Publishing or updating a rolling release does not change the Cargo
version automatically.
