# Emulated systems

Classic Classics hosts four cartridge systems alongside the native games. NES
runs the pure-Rust TetaNES core in-process. Super Nintendo, Game Boy Advance and
Atari 2600 run pinned libretro cores loaded from `app/cores/`. The collection
owns cartridge discovery, catalog identity, input translation, persistence and
audio output; the system crates only adapt one machine to `ArcadeGame`.

Every built-in cartridge is its own catalog card after the eight native games.
Cards show the system badge (`NES`, `SNES`, `GBA`, `ATARI 2600`) and normalized
box art. User-discovered cartridges also get individual cards but use the system
motif until artwork exists. `--catalog-index N` selects a card for headless
screenshots and diagnostics.

## Cores

| System | Core | Upstream pin | License |
|---|---|---|---|
| NES | `tetanes-core = 0.17.0` | `lukexor/tetanes` | MIT OR Apache-2.0 |
| Super Nintendo | `snes9x_libretro.so` | `libretro/snes9x` `fae2fea` | Snes9x free/non-commercial |
| Game Boy Advance | `mgba_libretro.so` | `libretro/mgba` `7a12d6d` | MPL-2.0 |
| Atari 2600 | `stella2023_libretro.so` | `libretro/stella2023` `ba52c43` | GPL-2.0 |

The shared libraries are separate works loaded at runtime. This keeps the main
executable MIT, makes a missing core a friendly error instead of a link failure,
and isolates GPL Stella and non-commercial Snes9x from the rest of the program.
`app/cores/SOURCE.txt`, `app/cores/LICENSES/` and `DevUtils/cores.lock` record
the exact sources and notices. The release archive **must** contain every core;
`DevUtils/package.py` and `DevUtils/verify_package.py` enforce that.

Rebuild the bundles with:

```sh
python3 DevUtils/build_cores.py
```

Sources are downloaded into `data/cache/cores-src` (ignored), built from the
pinned commits, and copied to `app/cores/`. Building needs `g++`, `make` and
`cmake`; normal Rust builds only load the checked-in `.so` files.

## Cartridge discovery

Every emulated card scans, in order:

1. `app/games/<system>/roms` in a release, or
   `crates/classicclassics-emulators/assets/<system>/roms` in a checkout.
2. `data/roms/<system>`.
3. `data/roms` (flat).
4. `rom_dirs` in `data/config/emulators.toml`.
5. Each repeatable `--rom-dir PATH`.

`--rom PATH` starts one cartridge immediately and also supplies the initial
selection in headless runs. Extensions are:

| System | Extensions |
|---|---|
| NES | `.nes` |
| Super Nintendo | `.sfc`, `.smc` |
| Game Boy Advance | `.gba`, `.agb` |
| Atari 2600 | `.a26`, `.bin` |

Entries are de-duplicated by a stable 64-bit FNV-1a identity over the file
bytes. Display names are uppercased because the shared bitmap font has only
uppercase glyphs. The collection does not unzip archives in this version.

Built-in cartridges are accompanied by `manifest.json` files containing the
display name, SHA-256, size and import source. They are release-owned: runtime
never writes into `app/`.

### Box art

Built-in box art lives in `crates/classicclassics-emulators/assets/<system>/covers/`
and is named by the cartridge's stable id (`<fnv-id>.png`). Covers are normalized
to a 450×600 (3:4) canvas with letterboxing so the catalog never stretches an
image. `DevUtils/fetch_artwork.py` downloads the matching named box art from
`thumbnails.libretro.com`, applies a few documented hack/homebrew overrides
(Micro Mages uses its own cover), and writes the normalized PNGs. The release
packages them beside the ROMs under `app/games/<system>/covers/`.

Dynamic user cartridges are not scanned for artwork in this version; they fall
back to the system motif and badge. Box-art rights remain with the respective
publishers; this artwork is included for the project's private catalog use.
`DevUtils/fetch_artwork.py` is the reproducible provenance and normalization
step.

NES headers written by old dumpers can contain a `DiskDude!` tag in bytes 7–16.
The NES adapter clears that tag while preserving the mapper low nibble, and
leaves NES 2.0 headers alone.

## Timing, video and audio

The 120 Hz collection clock drives each system's native frame rate through
`VideoClock` (60.0988 Hz NES/SNES, 59.7275 Hz GBA, 60 Hz Atari 2600). Frames are
presented through `PixelFrame`, so the existing native texture path, scaling,
filtering, CRT effect and headless rasteriser are reused unchanged.

libretro cores deliver `0RGB1555`, `RGB565` or `XRGB8888`; the host converts each
to the shared 32-bit RGB framebuffer and tracks geometry changes (Stella can
change its visible height). TetaNES delivers RGBA directly.

Emulators return native-rate PCM through `ArcadeGame::take_audio`. The collection
feeds it to a `quasar_audio::ProceduralSource` on the shared effects bus, with
linear resampling to 48 kHz, a bounded queue and silence/hold on underrun.
Headless runs simply discard the batches. Pausing the collection pauses the
voice and flushes queued samples.

## Saves and records

Battery-backed cartridge RAM is written to
`data/save/emulators/<system>/<id>.srm`. Automatic save states are written to
`.state` beside it when pausing, leaving for the picker/catalog, changing games,
or exiting. Both use a same-directory temporary file, `sync_all` and rename.

On starting a cartridge the adapter restores the battery file, then the save
state if present. `--no-save` disables both reads and writes, along with the
collection's records and settings. Each emulated game's catalog record shows the
number of discovered cartridges; the persistent `Record` additionally keeps the
last title and play time.

Emulator machine state is deliberately not part of the JSON `--state` snapshot.
The snapshot records the picker and selected cartridge identity; the binary
`.state` file is the authoritative machine snapshot.

## Controls

The shared control preset gains `shoulder_left` (C / LB) and `shoulder_right`
(V / RB). Face buttons map as follows:

| Action | Emulator button |
|---|---|
| Fire | A (Atari 2600: joystick fire) |
| Jump | B |
| Secondary | X / Y depending on family |
| Hyperspace | X |
| Confirm | Start |
| Mode | Select |
| Movement | D-pad or left stick |

Player two uses the second controller slot and the existing player-two keyboard
bindings. Enter/Backspace/P/R/Escape keep their collection meanings; Backspace
leaves a running cartridge and returns directly to the catalog. The cartridge
picker remains available when a system is launched explicitly with
`--game nes` (or `--game snes`, etc.).

The picker and the running game render the active mapping rather than only a
generic "jump/fire" hint. Examples:

```text
NES   MOVE A/D - A:MOUSE1 - B:SPACE - START:ENTER - SELECT:TAB - ESC:ESCAPE
GBA   MOVE A/D - A:MOUSE1 - B:SPACE - L:C - R:V - START:ENTER - SELECT:TAB - ESC:ESCAPE
SNES  MOVE A/D - A:MOUSE1 - B:SPACE - X:X - Y:SHIFT - L:C - R:V - START:ENTER - SELECT:TAB - ESC:ESCAPE
2600  MOVE A/D - FIRE:MOUSE1 - RESET:ENTER - SELECT:TAB - ESC:ESCAPE
```

The `SELECT` row is the `mode` action (Tab by default); `START` is `confirm`
(Enter); these can be changed in Settings → Controls.

## Limits

- Snes9x is free for non-commercial use. Review the notice before any paid
  distribution.
- Stella is GPL-2.0. The separate shared library and source/notice files satisfy
  the release obligations; the executable itself remains MIT.
- mGBA uses its built-in BIOS implementation when no BIOS file is supplied.
  Some titles may require a user-provided BIOS or specific save hardware.
- Special-chip SNES cartridges depend on Snes9x support. Atari 2600 compatibility
  depends on Stella's bank-switching implementation.
- Zip/7z cartridge archives are not unpacked automatically.

## Verification

The cartridges in `crates/classicclassics-emulators/assets/` are the acceptance
set. Useful checks:

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo test --locked --workspace --no-default-features

DevUtils/smoke_emulators.sh
python3 DevUtils/fetch_artwork.py   # only needed when adding built-in ROMs

python3 DevUtils/package.py
python3 DevUtils/verify_package.py dist/ClassicClassics-*-linux-x64.zip
```

The package verifier rejects an archive missing any core, license, built-in
cartridge or manifest.
