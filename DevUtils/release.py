#!/usr/bin/env python3
"""Publish a verified Basalt ZIP to the explicitly requested rolling Latest release."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
from urllib.error import HTTPError
from urllib.parse import quote
from urllib.request import Request, urlopen

from DevUtils.package import version
from DevUtils.verify_package import verify

REPOSITORY = "HungLo2020/ClassicClassics"
TAG = "Latest"


class GitHub:
    def __init__(self, token: str):
        self.token = token

    def request(self, method: str, route: str, payload: dict | None = None, *, data: bytes | None = None):
        upload = route.startswith("https://uploads.github.com/")
        url = route if upload else f"https://api.github.com/repos/{REPOSITORY}/{route}"
        headers = {
            "Authorization": f"Bearer {self.token}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "ClassicClassics-release",
        }
        if payload is not None:
            data = json.dumps(payload).encode()
            headers["Content-Type"] = "application/json"
        elif data is not None:
            headers["Content-Type"] = "application/zip"
        request = Request(url, data=data, headers=headers, method=method)
        try:
            with urlopen(request, timeout=120) as response:
                body = response.read()
                return json.loads(body) if body else None
        except HTTPError as error:
            # Do not print authorization headers or token-bearing request objects.
            raise RuntimeError(f"GitHub {method} {route}: HTTP {error.code}: {error.read().decode()}") from None

    def pages(self, route: str):
        page = 1
        while True:
            separator = "&" if "?" in route else "?"
            values = self.request("GET", f"{route}{separator}per_page=100&page={page}")
            yield from values
            if len(values) < 100:
                return
            page += 1


def validate_target(api: GitHub, commit: str) -> None:
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("release commit must be a full lowercase commit SHA")
    if api.request("GET", "branches/main")["commit"]["sha"] != commit:
        raise ValueError("main moved; resolve and validate the intended release commit again")
    runs = api.request("GET", f"actions/workflows/ci.yml/runs?head_sha={commit}&branch=main&per_page=100")["workflow_runs"]
    if not any(run["head_sha"] == commit and run["head_branch"] == "main"
               and run["event"] in ("push", "workflow_dispatch")
               and run["status"] == "completed" and run["conclusion"] == "success" for run in runs):
        raise ValueError("CI must have completed successfully for this exact main commit")


def publish(api: GitHub, commit: str, archive: Path, package_version: str) -> dict:
    expected = f"ClassicClassics-{package_version}-linux-x64.zip"
    if archive.name != expected:
        raise ValueError(f"expected asset {expected}, got {archive.name}")
    errors = verify(archive, package_version)
    if errors:
        raise ValueError("invalid release archive: " + "; ".join(errors))
    payload = archive.read_bytes()
    digest = "sha256:" + hashlib.sha256(payload).hexdigest()
    validate_target(api, commit)  # Last check before any external mutation.

    # Reuse the same release id. List releases also finds a draft left by a failed attempt.
    release = next((release for release in api.pages("releases") if release["tag_name"] == TAG), None)
    if release is not None and release.get("immutable"):
        raise ValueError("Latest is immutable; rolling publication requires a mutable release")
    if release is None:
        release = api.request("POST", "releases", {
            "tag_name": TAG, "target_commitish": commit, "name": TAG,
            "draft": True, "prerelease": False,
        })
    release_id = release["id"]
    temporary = f"pending-{commit}-{expected}"
    assets = list(api.pages(f"releases/{release_id}/assets"))
    for asset in assets:
        if asset["name"] == temporary:
            api.request("DELETE", f"releases/assets/{asset['id']}")
    upload_url = release["upload_url"].split("{", 1)[0] + "?name=" + quote(temporary)
    uploaded = api.request("POST", upload_url, data=payload)
    if uploaded["size"] != len(payload) or (uploaded.get("digest") and uploaded["digest"] != digest):
        raise ValueError("uploaded ZIP failed size/checksum verification; previous published asset retained")

    # Keep the previous ZIP available until its replacement has uploaded successfully.
    for asset in list(api.pages(f"releases/{release_id}/assets")):
        if asset["id"] != uploaded["id"] and (
            asset["name"].startswith("ClassicClassics-") and asset["name"].endswith("-linux-x64.zip")
            or asset["name"].startswith("pending-") and "ClassicClassics-" in asset["name"]
        ):
            api.request("DELETE", f"releases/assets/{asset['id']}")
    api.request("PATCH", f"releases/assets/{uploaded['id']}", {"name": expected})

    # Moving this one tag is the rolling-release policy; other tags are untouched.
    ref = next((ref for ref in api.request("GET", "git/matching-refs/tags/Latest") if ref["ref"] == "refs/tags/Latest"), None)
    if ref is None:
        api.request("POST", "git/refs", {"ref": "refs/tags/Latest", "sha": commit})
    elif ref["object"]["sha"] != commit:
        api.request("PATCH", "git/refs/tags/Latest", {"sha": commit, "force": True})
    body = (
        f"Classic Classics {package_version}\n\n"
        f"Built from [{commit}](https://github.com/{REPOSITORY}/commit/{commit}).\n\n"
        f"Download `{expected}` for Linux x64. The Basalt v1 ZIP contains only `app/`; "
        "updating preserves saves and configuration in `data/`. Requires a Vulkan 1.3 driver.\n\n"
        f"SHA-256: `{digest.removeprefix('sha256:')}`\n\n"
        "This rolling release and its Latest tag are replaced only by an explicitly requested release."
    )
    api.request("PATCH", f"releases/{release_id}", {
        "tag_name": TAG, "target_commitish": commit, "name": TAG, "body": body,
        "draft": False, "prerelease": False, "make_latest": "true",
    })
    latest = api.request("GET", "releases/latest")
    published_assets = [asset for asset in latest["assets"] if asset["name"].startswith("ClassicClassics-")
                        and asset["name"].endswith("-linux-x64.zip")]
    if latest["id"] != release_id or latest["tag_name"] != TAG or latest["draft"] or latest["prerelease"]:
        raise ValueError("GitHub's latest endpoint does not point at the published Latest release")
    if len(published_assets) != 1 or published_assets[0]["name"] != expected or published_assets[0]["size"] != len(payload):
        raise ValueError("published release does not contain exactly the expected Basalt ZIP")
    if api.request("GET", "git/ref/tags/Latest")["object"]["sha"] != commit:
        raise ValueError("Latest tag does not point at the requested commit")
    print(f"Published {latest['html_url']}\nAsset: {expected}\nCommit: {commit}\n{digest}")
    return latest


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--check-only", action="store_true", help="read-only CI/commit verification")
    args = parser.parse_args()
    if os.environ.get("GITHUB_REF") != "refs/heads/main":
        parser.error("release workflow must be dispatched on main")
    if subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip() != args.commit:
        parser.error("checkout does not match the requested release commit")
    if subprocess.check_output(["git", "status", "--porcelain"], text=True).strip():
        parser.error("release checkout must be clean")
    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    if not token:
        parser.error("GH_TOKEN or GITHUB_TOKEN is required")
    api = GitHub(token)
    if args.check_only:
        validate_target(api, args.commit)
        print(f"Verified successful CI for main at {args.commit}")
    elif args.archive is None:
        parser.error("--archive is required to publish")
    else:
        publish(api, args.commit, args.archive, version())


if __name__ == "__main__":
    main()
