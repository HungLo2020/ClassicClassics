#!/usr/bin/env python3
"""Build an app-only Basalt v1 release archive for Classic Classics."""

from __future__ import annotations

import argparse
import json
import platform
import subprocess
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = """Classic Classics {version}
Quasar-powered arcade collection.

Requires a Vulkan 1.3 desktop driver and a standard Linux desktop session
(X11 or Wayland). Bundles no runtime beyond the executable; libc and the
graphics/audio stack come from the host system.

Save, configuration and cache files are created under the Basalt install root:
  data/save/           records and progress
  data/config/         display and audio settings
  data/config/shared/  portable controls preset
  data/cache/          reserved for rebuildable files
"""


def version() -> str:
    output = subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
        cwd=ROOT,
        text=True,
    )
    metadata = json.loads(output)
    return next(package["version"] for package in metadata["packages"] if package["name"] == "classicclassics")


def game_assets():
    """Source assets mapped into the release-owned app tree."""
    for game in ("micromages", "jackal"):
        root = ROOT / "crates" / f"classicclassics-{game}" / "assets"
        for path in sorted(root.rglob("*")):
            if path.is_file():
                yield path, f"app/games/{game}/{path.relative_to(root).as_posix()}"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, help="package an existing native desktop build")
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    args = parser.parse_args()

    system = {"Linux": "linux", "Windows": "windows", "Darwin": "macos"}.get(platform.system())
    machine = {"x86_64": "x64", "AMD64": "x64", "aarch64": "arm64", "arm64": "arm64", "riscv64": "riscv64"}.get(
        platform.machine()
    )
    if system != "linux" or machine is None:
        parser.error("this release pipeline currently packages linux-x64 only")
    suffix = ".exe" if system == "windows" else ""
    if args.binary is None:
        subprocess.run(["cargo", "build", "--locked", "--release", "--bin", "classicclassics"], cwd=ROOT, check=True)
        binary = ROOT / "target" / "release" / ("classicclassics" + suffix)
    else:
        binary = args.binary.resolve()
    if not binary.is_file():
        parser.error(f"executable not found: {binary}")

    package_version = version()
    args.output.mkdir(parents=True, exist_ok=True)
    archive = args.output / f"ClassicClassics-{package_version}-{system}-{machine}.zip"
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as output:
        executable = zipfile.ZipInfo(f"app/bin/classicclassics{suffix}")
        executable.create_system = 3
        executable.external_attr = 0o100755 << 16
        executable.compress_type = zipfile.ZIP_DEFLATED
        output.writestr(executable, binary.read_bytes())
        output.write(ROOT / "app/basalt/cover.png", "app/basalt/cover.png")
        output.write(ROOT / "app/basalt/icon.png", "app/basalt/icon.png")
        output.write(ROOT / "LICENSE", "app/LICENSE")
        for path, destination in game_assets():
            output.write(path, destination)
        output.writestr(
            "app/CLASSICCLASSICS-RUNTIME.txt",
            RUNTIME.format(version=package_version),
        )
    print(archive)


if __name__ == "__main__":
    main()
