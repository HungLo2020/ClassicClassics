#!/bin/sh
# Load every built-in cartridge through its emulator and render 30 frames.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
BIN=${1:-"$ROOT/target/debug/classicclassics"}
OUT=${TMPDIR:-/tmp}/classicclassics-smoke
mkdir -p "$OUT"
fail=0
for system in nes snes gba atari2600; do
    for rom in "$ROOT/crates/classicclassics-emulators/assets/$system/roms/"*; do
        [ -f "$rom" ] || continue
        if "$BIN" --headless --no-save --game "$system" --rom "$rom" \
            --ticks 30 --screenshot "$OUT/$system.png" >/dev/null 2>&1; then
            echo "ok   $system $(basename "$rom")"
        else
            echo "FAIL $system $(basename "$rom")"
            fail=1
        fi
    done
done
exit "$fail"
