#!/usr/bin/env python3
"""Build the pinned libretro cores into app/cores.

The release must always contain these shared libraries. Sources are downloaded
at the commits in DevUtils/cores.lock into data/cache/cores-src (ignored).
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import tarfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOCK = ROOT / "DevUtils" / "cores.lock"
CACHE = ROOT / "data" / "cache" / "cores-src"
OUT = ROOT / "app" / "cores"


def run(command: list[str], cwd: Path) -> None:
    print("+", " ".join(command), f"(in {cwd})")
    subprocess.run(command, cwd=cwd, check=True)


def download(repository: str, commit: str) -> Path:
    CACHE.mkdir(parents=True, exist_ok=True)
    name = repository.split("/")[-1]
    source = CACHE / f"{name}-{commit}"
    if source.is_dir():
        return source
    archive = CACHE / f"{name}-{commit}.tar.gz"
    if not archive.is_file():
        url = f"https://codeload.github.com/{repository}/tar.gz/{commit}"
        print("downloading", url)
        urllib.request.urlretrieve(url, archive)
    with tarfile.open(archive, "r:gz") as bundle:
        bundle.extractall(CACHE, filter="data")
    extracted = CACHE / f"{name}-{commit}"
    if not extracted.is_dir():
        # GitHub archives use <repo>-<full sha>.
        candidates = [p for p in CACHE.iterdir() if p.is_dir() and p.name.startswith(name)]
        if not candidates:
            raise SystemExit(f"cannot locate extracted source for {repository}")
        extracted = candidates[0]
        if extracted != source:
            extracted.rename(source)
    return source


def build_snes9x(source: Path) -> Path:
    run(["make", "-C", "libretro", f"-j{max(1, __import__('os').cpu_count() or 1)}"], source)
    return source / "libretro" / "snes9x_libretro.so"


def build_mgba(source: Path) -> Path:
    build = source / "build-libretro"
    run(
        [
            "cmake",
            "-S",
            ".",
            "-B",
            "build-libretro",
            "-DBUILD_LIBRETRO=ON",
            "-DBUILD_QT=OFF",
            "-DBUILD_SDL=OFF",
            "-DUSE_FFMPEG=OFF",
            "-DUSE_MINIZIP=OFF",
            "-DCMAKE_BUILD_TYPE=Release",
        ],
        source,
    )
    run(
        [
            "cmake",
            "--build",
            "build-libretro",
            "--target",
            "mgba_libretro",
            f"-j{max(1, __import__('os').cpu_count() or 1)}",
        ],
        source,
    )
    return build / "mgba_libretro.so"


def build_stella(source: Path) -> Path:
    run(
        ["make", "-C", "src/os/libretro", f"-j{max(1, __import__('os').cpu_count() or 1)}"],
        source,
    )
    return source / "src/os/libretro/stella2023_libretro.so"


BUILDERS = {
    "snes9x": build_snes9x,
    "mgba": build_mgba,
    "stella2023": build_stella,
}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only", choices=sorted(BUILDERS), action="append")
    parser.add_argument("--clean", action="store_true", help="remove cached downloads first")
    args = parser.parse_args()
    if args.clean and CACHE.exists():
        shutil.rmtree(CACHE)
    lock = json.loads(LOCK.read_text())
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "LICENSES").mkdir(exist_ok=True)
    for name, entry in lock.items():
        if args.only and name not in args.only:
            continue
        source = download(entry["repository"], entry["commit"])
        built = BUILDERS[name](source)
        if not built.is_file():
            raise SystemExit(f"build did not produce {built}")
        shutil.copy2(built, OUT / entry["core"])
        license_source = source / "LICENSE"
        if name == "stella2023":
            license_source = source / "License.txt"
        if license_source.is_file():
            shutil.copy2(license_source, OUT / entry["license"])
        print("installed", entry["core"])
    (OUT / "SOURCE.txt").write_text(
        "Classic Classics emulator cores\n\n"
        + "\n".join(
            f"{name}: https://github.com/{entry['repository']} @ {entry['commit']}"
            for name, entry in lock.items()
        )
        + "\n\nRebuild with DevUtils/build_cores.py. License texts are in LICENSES/.\n"
    )


if __name__ == "__main__":
    main()
