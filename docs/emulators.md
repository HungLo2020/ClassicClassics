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
`data/save/emulators/<system>/<id>.srm`. It is ordinary cartridge behavior and
saves automatically when pausing, leaving, changing games, or exiting. Writes
use a same-directory temporary file, `sync_all` and rename.

Save states are **manual and emulator-only**. While a cartridge is running, open
the Escape pause/settings overlay and choose **Save State**, **Load State** or
**Delete State**. There is one quick-state slot per cartridge. Starting a
cartridge restores only battery SRAM; it never loads `.state` automatically.
Existing `.state` files from older builds are treated as the quick-slot contents
and can be loaded explicitly.

`--no-save` disables battery/state reads and writes along with the collection's
records and settings. Each emulated game's catalog record shows the number of
discovered cartridges; the persistent `Record` additionally keeps the last title
and play time.

Emulator machine state is deliberately not part of the JSON `--state` snapshot.
The snapshot records the picker and selected cartridge identity; the binary
`.state` file is the authoritative manual save-state slot.

## Controls

Emulator buttons are a dedicated action set, separate from native Fire/Jump/
Secondary. The keyboard layout is a left-hand movement, right-hand face-button
cluster:

| Action | Keyboard | Controller |
|---|---|---|
| Movement | WASD or arrows | D-pad or left stick |
| A | K | East |
| B | J | South |
| X | I | North |
| Y | U | West |
| L / R | C / V | LB / RB |
| Start | Enter | Start |
| Select | Tab | Select |
| Settings | Escape | Start + Select |
| Back to catalog | Backspace | Left stick click |
| Pause | P | Right stick click |
| Restart | R | Mode / guide |

The controller face buttons follow the Nintendo diamond; NES uses B/A, GBA uses
B/A/L/R, SNES uses B/A/Y/X/L/R, and Atari 2600 uses B as its single fire button.
Unused buttons are ignored per family. Player two uses the second controller
slot; the keyboard player-two bindings also map to the emulator A/B/Y buttons.

The picker and the running game render the active mapping. Examples:

```text
NES   MOVE A/D - A:K - B:J - START:ENTER - SELECT:TAB - ESC:ESCAPE
GBA   MOVE A/D - A:K - B:J - L:C - R:V - START:ENTER - SELECT:TAB - ESC:ESCAPE
SNES  MOVE A/D - A:K - B:J - X:I - Y:U - L:C - R:V - START:ENTER - SELECT:TAB - ESC:ESCAPE
2600  MOVE A/D - FIRE:J - RESET:ENTER - SELECT:TAB - ESC:ESCAPE
```

`START` and `SELECT` are dedicated emulator actions. They are deliberately
separate from menu Confirm and native Mode, so Space (native jump) and controller
face buttons cannot start/pause a console by accident. All actions can be changed
under Settings → Controls → Emulator.

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
