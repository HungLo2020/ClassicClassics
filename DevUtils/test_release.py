"""Exercise rolling publication against an in-memory GitHub API; never publishes."""
from __future__ import annotations

from contextlib import redirect_stdout
from copy import deepcopy
from io import StringIO
import hashlib
from pathlib import Path
import tempfile
import unittest
from urllib.parse import parse_qs, urlparse
import zipfile

from DevUtils.release import publish, validate_target

SHA = "a" * 40
NEXT_SHA = "b" * 40


class FakeGitHub:
    def __init__(self):
        self.main = SHA
        self.ci = True
        self.release = None
        self.assets = []
        self.refs = {"refs/tags/Latest-other": "c" * 40}
        self.writes = []
        self.sequence = 100
        self.fail_upload = False
        self.corrupt_upload = False

    def pages(self, route):
        if route == "releases":
            return iter(deepcopy([self.release] if self.release else []))
        if route == "releases/1/assets":
            return iter(deepcopy(self.assets))
        raise AssertionError(route)

    def request(self, method, route, payload=None, *, data=None):
        if method != "GET":
            self.writes.append((method, route, payload))
        if route == "branches/main":
            return {"commit": {"sha": self.main}}
        if route.startswith("actions/workflows/ci.yml/runs?"):
            return {"workflow_runs": [{"head_sha": self.main, "head_branch": "main", "event": "push",
                                       "status": "completed", "conclusion": "success" if self.ci else "failure"}]}
        if route == "releases" and method == "POST":
            self.release = dict(payload, id=1, upload_url="https://uploads.github.com/repos/test/releases/1/assets{?name}",
                                html_url="https://github.com/HungLo2020/ClassicClassics/releases/tag/Latest")
            return deepcopy(self.release)
        if route.startswith("https://uploads.github.com/"):
            if self.fail_upload:
                raise RuntimeError("simulated upload failure")
            self.sequence += 1
            asset = {"id": self.sequence, "name": parse_qs(urlparse(route).query)["name"][0], "size": len(data),
                     "digest": "sha256:" + ("0" * 64 if self.corrupt_upload else hashlib.sha256(data).hexdigest())}
            self.assets.append(asset)
            return deepcopy(asset)
        if route.startswith("releases/assets/"):
            asset_id = int(route.rsplit("/", 1)[1])
            asset = next(asset for asset in self.assets if asset["id"] == asset_id)
            if method == "DELETE":
                self.assets.remove(asset)
                return None
            asset.update(payload)
            return deepcopy(asset)
        if route == "git/matching-refs/tags/Latest":
            return [{"ref": ref, "object": {"sha": sha}} for ref, sha in self.refs.items() if ref.startswith("refs/tags/Latest")]
        if route == "git/refs" and method == "POST":
            self.refs[payload["ref"]] = payload["sha"]
            return {}
        if route == "git/refs/tags/Latest" and method == "PATCH":
            assert payload["force"] is True
            self.refs["refs/tags/Latest"] = payload["sha"]
            return {}
        if route == "git/ref/tags/Latest":
            return {"object": {"sha": self.refs["refs/tags/Latest"]}}
        if route == "releases/1" and method == "PATCH":
            self.release.update(payload)
            return deepcopy(self.release)
        if route == "releases/latest":
            return dict(deepcopy(self.release), assets=deepcopy(self.assets))
        raise AssertionError((method, route, payload))


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.enterContext(redirect_stdout(StringIO()))
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.archive = Path(self.directory.name) / "ClassicClassics-1.0.0-linux-x64.zip"
        with zipfile.ZipFile(self.archive, "w") as output:
            executable = zipfile.ZipInfo("app/bin/classicclassics")
            executable.external_attr = 0o100755 << 16
            output.writestr(executable, b"test executable")
            for name in ["app/basalt/cover.png", "app/basalt/icon.png", "app/CLASSICCLASSICS-RUNTIME.txt", "app/LICENSE"]:
                output.writestr(name, b"test fixture")
        self.api = FakeGitHub()

    def test_first_release_then_replacement_keeps_one_release_one_zip_and_moves_only_latest(self):
        first = publish(self.api, SHA, self.archive, "1.0.0")
        old_asset = self.api.assets[0]["id"]
        self.api.main = NEXT_SHA
        second = publish(self.api, NEXT_SHA, self.archive, "1.0.0")
        self.assertEqual(first["id"], second["id"])
        self.assertEqual(len(self.api.assets), 1)
        self.assertNotEqual(self.api.assets[0]["id"], old_asset)
        self.assertEqual(self.api.assets[0]["name"], self.archive.name)
        self.assertEqual(self.api.refs["refs/tags/Latest"], NEXT_SHA)
        self.assertEqual(self.api.refs["refs/tags/Latest-other"], "c" * 40)
        self.assertIn(NEXT_SHA, second["body"])
        self.assertFalse(second["draft"] or second["prerelease"])
        self.assertEqual(sum(route == "releases" and method == "POST" for method, route, _ in self.api.writes), 1)

    def test_new_version_removes_the_old_zip(self):
        publish(self.api, SHA, self.archive, "1.0.0")
        new = self.archive.with_name("ClassicClassics-1.1.0-linux-x64.zip")
        new.write_bytes(self.archive.read_bytes())
        publish(self.api, SHA, new, "1.1.0")
        self.assertEqual([asset["name"] for asset in self.api.assets], [new.name])

    def test_failed_upload_preserves_the_previous_release_and_tag(self):
        publish(self.api, SHA, self.archive, "1.0.0")
        before = deepcopy(self.api.assets)
        self.api.main = NEXT_SHA
        self.api.fail_upload = True
        with self.assertRaisesRegex(RuntimeError, "simulated upload failure"):
            publish(self.api, NEXT_SHA, self.archive, "1.0.0")
        self.assertEqual(self.api.assets, before)
        self.assertEqual(self.api.refs["refs/tags/Latest"], SHA)

    def test_bad_upload_checksum_preserves_old_zip_and_retry_recovers(self):
        publish(self.api, SHA, self.archive, "1.0.0")
        self.api.main = NEXT_SHA
        self.api.corrupt_upload = True
        with self.assertRaisesRegex(ValueError, "checksum"):
            publish(self.api, NEXT_SHA, self.archive, "1.0.0")
        self.assertTrue(any(asset["name"] == self.archive.name for asset in self.api.assets))
        self.assertEqual(self.api.refs["refs/tags/Latest"], SHA)
        self.api.corrupt_upload = False
        publish(self.api, NEXT_SHA, self.archive, "1.0.0")
        self.assertEqual([asset["name"] for asset in self.api.assets], [self.archive.name])

    def test_stale_commit_or_failed_ci_does_not_mutate_github(self):
        self.api.main = NEXT_SHA
        with self.assertRaisesRegex(ValueError, "main moved"):
            publish(self.api, SHA, self.archive, "1.0.0")
        self.api.main = SHA
        self.api.ci = False
        with self.assertRaisesRegex(ValueError, "CI must"):
            publish(self.api, SHA, self.archive, "1.0.0")
        with self.assertRaisesRegex(ValueError, "full lowercase"):
            validate_target(self.api, "main")
        self.assertEqual(self.api.writes, [])

    def test_invalid_archive_does_not_mutate_github(self):
        with zipfile.ZipFile(self.archive, "a") as output:
            output.writestr("data/save/records.json", "{}")
        with self.assertRaisesRegex(ValueError, "invalid release archive"):
            publish(self.api, SHA, self.archive, "1.0.0")
        self.assertEqual(self.api.writes, [])

    def test_retries_reuse_an_unpublished_draft(self):
        self.api.fail_upload = True
        with self.assertRaises(RuntimeError):
            publish(self.api, SHA, self.archive, "1.0.0")
        self.assertTrue(self.api.release["draft"])
        self.api.fail_upload = False
        publish(self.api, SHA, self.archive, "1.0.0")
        self.assertEqual(sum(route == "releases" and method == "POST" for method, route, _ in self.api.writes), 1)
        self.assertFalse(self.api.release["draft"])


if __name__ == "__main__":
    unittest.main()
