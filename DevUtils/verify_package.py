#!/usr/bin/env python3
"""Verify a Classic Classics ZIP against the Basalt release layout rules."""

from __future__ import annotations

import argparse
import json
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def verify(archive: Path, version: str) -> list[str]:
    errors: list[str] = []
    if not archive.name.startswith(f"ClassicClassics-{version}-linux-x64") or not archive.name.endswith(".zip"):
        errors.append(f"unexpected asset name: {archive.name}")
    with zipfile.ZipFile(archive) as bundle:
        if bundle.testzip() is not None:
            errors.append("archive is corrupt")
        names = bundle.namelist()
        if not names:
            errors.append("archive is empty")
        for name in names:
            path = Path(name)
            if path.is_absolute() or ".." in path.parts:
                errors.append(f"unsafe path: {name}")
            if not name.startswith("app/"):
                errors.append(f"entry outside app/: {name}")
            if name.startswith(("data/", ".basalt/")) or "/data/" in name:
                errors.append(f"user-data entry in release: {name}")
        required = {
            "app/bin/classicclassics",
            "app/basalt/cover.png",
            "app/basalt/icon.png",
            "app/CLASSICCLASSICS-RUNTIME.txt",
        }
        # Both sprite games need their complete exported asset trees at runtime.
        for game in ("micromages", "jackal"):
            root = ROOT / "crates" / f"classicclassics-{game}" / "assets"
            required.update(f"app/games/{game}/{path.relative_to(root).as_posix()}"
                            for path in root.rglob("*") if path.is_file())
        missing = sorted(required - set(names))
        if missing:
            errors.append(f"missing release files: {', '.join(missing)}")
        executable = bundle.getinfo("app/bin/classicclassics")
        if not (executable.external_attr >> 16) & 0o111:
            errors.append("app/bin/classicclassics is not executable")
        cover = bundle.getinfo("app/basalt/cover.png")
        if cover.file_size > 2 * 1024 * 1024:
            errors.append("cover exceeds 2 MB")
    return errors


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("--catalog", type=Path, default=ROOT / "ClassicClassics.json")
    args = parser.parse_args()
    catalog = json.loads(args.catalog.read_text())
    version = json.loads(
        __import__("subprocess").check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
            cwd=ROOT,
            text=True,
        )
    )
    package_version = next(
        package["version"] for package in version["packages"] if package["name"] == "classicclassics"
    )
    errors = verify(args.archive, package_version)
    if catalog.get("schema_version") != 2 or catalog.get("standard_version") != 1:
        errors.append("catalog entry must use schema_version 2 and standard_version 1")
    if catalog.get("id") != "ClassicClassics":
        errors.append("catalog id must be ClassicClassics")
    if catalog.get("launch", {}).get("executable") != "app/bin/classicclassics":
        errors.append("catalog launch executable does not match the archive")
    if catalog.get("source", {}).get("repository") != "HungLo2020/ClassicClassics":
        errors.append("catalog repository is not HungLo2020/ClassicClassics")
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
    print(f"verified {args.archive} and {args.catalog}")


if __name__ == "__main__":
    main()
