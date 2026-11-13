#!/usr/bin/env python3
"""Fetch and normalize box art for built-in cartridges.

Artwork comes from the libretro-thumbnails Named_Boxarts collection. Covers are
letterboxed into a 3:4 canvas and stored as covers/<fnv-id>.png beside the ROMs.
Dynamically discovered user cartridges intentionally have no artwork.
"""

from __future__ import annotations

import html
import io
import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "crates" / "classicclassics-emulators" / "assets"
BASE = "https://thumbnails.libretro.com"
SYSTEMS = {
    "nes": "Nintendo - Nintendo Entertainment System",
    "snes": "Nintendo - Super Nintendo Entertainment System",
    "gba": "Nintendo - Game Boy Advance",
    "atari2600": "Atari - 2600",
}
COVER_SIZE = (450, 600)

# ROM hacks and homebrew whose exact box art is not in the named library.
OVERRIDES = {
    "advance wars returns (v3.1).gba": "Advance Wars 2 - Black Hole Rising (USA).png",
    "pokemon radical red.gba": "Pokemon - FireRed Version (USA).png",
    "pokemon shiny gold sigma.gba": "Pokemon - FireRed Version (USA).png",
    "super mario bros 3 (u) (prg 1) [h2].nes": "Super Mario Bros. 3 (USA).png",
    "super mario bros.nes": "Super Mario Bros. (World).png",
    "duck hunt (world).nes": "Duck Hunt (World).png",
    "tetris (u) [!].nes": "Tetris (1989-11)(Nintendo)(US).png",
    "lifeforce (u).nes": "Life Force (USA).png",
    "legend of zelda, the.nes": "Legend of Zelda, The (USA).png",
    "zelda ii - the adventure of link (usa).nes": "Zelda II - The Adventure of Link (USA).png",
    "metroid (u).nes": "Metroid (USA).png",
    "mega man (usa).nes": "Mega Man (USA).png",
    "mega man 2 (usa).nes": "Mega Man 2 (USA).png",
    "mega man 3 (usa).nes": "Mega Man 3 (USA).png",
    "pokemon emerald.gba": "Pokemon - Emerald Version (USA, Europe).png",
    "legend of zelda, the - the minish cap (usa).gba": "Legend of Zelda, The - The Minish Cap (USA).png",
    "pitfall 2 - the lost caverns.a26": "Pitfall II - Lost Caverns (USA).png",
    "pitfall!.a26": "Pitfall! - Pitfall Harry's Jungle Adventure (USA).png",
    "asteroids.a26": "Asteroids (USA).png",
    "micro mages.nes": "__MICRO_MAGES__",
}


def fnv1a(path: Path) -> int:
    value = 0xCBF29CE484222325
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            for byte in chunk:
                value ^= byte
                value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return value


def normalize(text: str) -> str:
    text = text.lower()
    text = re.sub(r"\([^)]*\)|\[[^\]]*\]", " ", text)
    text = text.replace("&", " and ").replace("_", " ")
    text = re.sub(r"[^a-z0-9]+", " ", text)
    words = text.split()
    words = [word for word in words if word not in ("the", "and")]
    return " ".join(words)


def index(system: str) -> list[str]:
    url = f"{BASE}/{SYSTEMS[system].replace(' ', '%20')}/Named_Boxarts/"
    request = urllib.request.Request(url, headers={"User-Agent": "classicclassics"})
    with urllib.request.urlopen(request, timeout=90) as response:
        body = response.read().decode("utf-8", "replace")
    names = [
        urllib.parse.unquote(html.unescape(value))
        for value in re.findall(r'href="([^"]+\.png)"', body, flags=re.IGNORECASE)
    ]
    return sorted(set(names))


def match(name: str, candidates: list[str]) -> str | None:
    target = normalize(name)
    scored = []
    for candidate in candidates:
        value = normalize(candidate)
        if value == target:
            score = 0
        elif value.startswith(target + " "):
            score = 10 + len(value) - len(target)
        elif target in value:
            score = 100 + len(value) - len(target)
        else:
            continue
        lowered = candidate.lower()
        if "(usa" in lowered or "(world" in lowered or "(us)" in lowered:
            score -= 2
        if "(europe" in lowered or "(japan" in lowered:
            score += 3
        if "[b" in lowered or "(beta" in lowered or "(proto" in lowered:
            score += 8
        for variant in (
            "virtual console",
            "collector",
            "demo",
            "kiosk",
            "playchoice",
            "tengen",
            "(unl)",
            "alternate",
            "sample",
            "hack",
            "translation",
        ):
            if variant in lowered:
                score += 15
        scored.append((score, candidate))
    return min(scored)[1] if scored else None


def normalize_cover(source: bytes, destination: Path) -> None:
    image = Image.open(io.BytesIO(source)).convert("RGB")
    image.thumbnail(COVER_SIZE, Image.Resampling.LANCZOS)
    canvas = Image.new("RGB", COVER_SIZE, (10, 16, 26))
    x = (COVER_SIZE[0] - image.width) // 2
    y = (COVER_SIZE[1] - image.height) // 2
    canvas.paste(image, (x, y))
    destination.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(destination, "PNG", optimize=True)


def main() -> None:
    missing = []
    for system in SYSTEMS:
        cache = Path("/tmp") / f"classicclassics-thumbs-{system}.txt"
        if cache.is_file():
            candidates = [
                urllib.parse.unquote(line) for line in cache.read_text().splitlines()
            ]
        else:
            candidates = index(system)
            cache.write_text("\n".join(candidates))
        covers = ASSETS / system / "covers"
        roms = sorted((ASSETS / system / "roms").glob("*"))
        for rom in roms:
            if not rom.is_file():
                continue
            destination = covers / f"{fnv1a(rom):016x}.png"
            if destination.is_file():
                print("cached", system, rom.name)
                continue
            override = OVERRIDES.get(rom.name.lower())
            if override == "__MICRO_MAGES__":
                source = ROOT / "crates/classicclassics-micromages/assets/basalt/cover.png"
                normalize_cover(source.read_bytes(), destination)
                print("micro ", system, rom.name)
                continue
            chosen = override or match(rom.stem, candidates)
            if not chosen:
                missing.append((system, rom.name))
                print("MISS ", system, rom.name)
                continue
            url = f"{BASE}/{SYSTEMS[system].replace(' ', '%20')}/Named_Boxarts/{urllib.parse.quote(chosen)}"
            request = urllib.request.Request(url, headers={"User-Agent": "classicclassics"})
            with urllib.request.urlopen(request, timeout=90) as response:
                normalize_cover(response.read(), destination)
            print("ok   ", system, rom.name, "<-", chosen)
    if missing:
        print("\nmissing artwork:", file=sys.stderr)
        for system, name in missing:
            print(f"  {system}: {name}", file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
