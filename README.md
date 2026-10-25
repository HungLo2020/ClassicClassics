# Classic Classics

A single-window arcade collection built almost entirely from
[Quasar](https://github.com/HungLo2020/Quasar) infrastructure: Pong, Lunar
Lander and Asteroids, with vector-style rendering, generated sound, persistent
high scores and a Basalt-ready Linux release.

![Classic Classics cover](app/basalt/cover.png)

This project follows the
[CODING-STANDARDS.md](https://github.com/HungLo2020/LinuxScripts/blob/master/CODING-STANDARDS.md)
and the [Basalt Game Standard](https://github.com/HungLo2020/Basalt/blob/main/STANDARDS.md).

## Games

| Game | Style | Rules |
|---|---|---|
| **Pong** | Neon arcade | First to seven. Solo versus the computer or local two-player. |
| **Lunar Lander** | Vector flight | Rotate and burn fuel to land softly, upright and on a multiplier pad. |
| **Asteroids** | Vector space | Inertial flight, wrap-around space, splitting rocks, three lives. |

The catalog screen is the entry point. Choose a game with **A/D**, **W/S** or
the arrow keys, then press **Enter**. **Escape** opens Settings from the catalog
or freezes a running game. **Backspace** returns from a game to the catalog.
**Q** always quits.

### Controls

| Action | Keyboard | Gamepad |
|---|---|---|
| Navigate catalog | W/S/A/D or arrows | D-pad or left stick |
| Confirm / serve / fire | Enter or Space | South |
| Settings | Escape | East |
| Back to catalog | Backspace | Select |
| Pong paddles | W/S, arrows in two-player | Left/right stick |
| Rotate ship / lander | A/D or Left/Right | Left stick |
| Thrust | W or Up | Triggers |
| Hyperspace | Shift | West |
| Pong mode | Tab | North |
| Pause | P | Start |
| Restart | R | LB |
| Mute | M | — |
| Fullscreen | F11 or Alt+Enter | — |
| Filter / scaling | F2 / F3 | — |

### Settings

**Escape** opens a settings overlay from either the catalog or a game. It freezes
the simulation and offers:

- **Fullscreen** — apply borderless fullscreen or return to the remembered window.
- **Window size** — cycle 800×500, 960×600, 1280×800, 1440×900 or 1600×1000.
- **Filtering** — Sharp (nearest) or Smooth (linear).
- **Scaling** — Fit or Integer.
- **Master volume** — adjust in 5% steps.
- **Mute** — toggle output without losing the saved volume.
- **Back to catalog** — leave a running game and save its records.
- **Restore defaults** — reset display and audio settings.
- **Quit Classic Classics** — exit with status 0.

Changes apply immediately. Display and audio settings are written to
`data/config/display.json` on a normal exit. **F11/Alt+Enter**, **F2** and **F3**
remain direct shortcuts outside the overlay.

Settings may also be supplied for one run through Quasar's common arguments.
The file is the same JSON shape as `display.json`; `--set` uses `key=value`:

```sh
cargo run -- --config /tmp/display.json --set width=1280 --set audio.volume=0.5
```

Gamepads are hotplugged through Quasar's `gilrs` adapter and share the same
serializable action preset as the keyboard. An explicit preset can be tested
without changing the saved one:

```sh
cargo run -- --bindings /tmp/controls.json
cargo run --no-default-features -- --write-default-bindings /tmp/controls.json
cargo run --no-default-features -- --check-bindings /tmp/controls.json
```

## Build and run

Requirements are a current stable Rust toolchain (1.94), CMake, Ninja or Make,
a C++ compiler, and the usual Linux window/audio/Vulkan development packages:

```sh
sudo apt install build-essential cmake ninja-build pkg-config \
  libxkbcommon-dev libxkbcommon-x11-0 libasound2-dev libudev-dev \
  libwayland-dev libx11-dev libxrandr-dev libxi-dev libxcursor-dev libvulkan-dev
cargo run
```

The native game needs a Vulkan 1.3 driver. Quasar's headless reference
renderer and every simulation run without a GPU or window:

```sh
cargo run --no-default-features -- --headless --ticks 2 \
  --screenshot /tmp/classicclassics-menu.png
cargo run --no-default-features -- --headless --game asteroids \
  --ticks 600 --autoplay --screenshot /tmp/asteroids.png \
  --state /tmp/asteroids.json
cargo run --no-default-features -- --headless --game asteroids --settings \
  --ticks 1 --screenshot /tmp/settings.png
```

Native smoke test (resizes, toggles fullscreen, changes filtering and scaling):

```sh
cargo run -- --game pong --autoplay --no-save \
  --smoke-frames 30 --exercise-window
```

## Quasar usage

Nothing in the game loop is hand-rolled infrastructure:

| Quasar crate | Used for |
|---|---|
| `quasar-core` | Fixed 120 Hz step and shared viewport/scaling types |
| `quasar-window` | Native window, desktop identity, keyboard/mouse state, winit events |
| `quasar-input` | Named actions, bindings, edge detection, deadzones, controller hotplug |
| `quasar-render-2d` | Ordered quad draw lists, bitmap font, GPU presentation, headless rasterisation |
| `quasar-audio` | Deterministic generated clips and the optional CPAL output backend |
| `quasar-storage` | Basalt directory discovery and atomic JSON persistence |
| `quasar-config` | Common CLI/environment parsing (`--headless`, `--resolution`, `--no-audio`, …) |

The vector look is produced with Quasar's public canvas API: rotated thin
quads become lines, polylines, circles and polygon outlines. The same draw list
is presented on the GPU and rasterised headlessly, so screenshots do not need a
separate renderer.

## Data and Basalt layout

The release is a single binary plus artwork. On first run the game creates only
the directories it needs and honors every Basalt directory variable, falling
back to paths relative to the install root when run outside Basalt.

```text
<install root>/
├── app/                       release-owned: executable, cover, icon, license
└── data/
    ├── save/records.json      synced high scores and progress
    ├── config/display.json    machine-local window/audio settings
    ├── config/shared/controls.json  portable bindings preset
    └── cache/                 reserved for rebuildable files
```

All JSON is written through `quasar_storage::write_json`, which writes a
temporary file in the same directory, syncs it and renames it into place.
Existing corrupt or newer files are preserved and defaults are used; nothing is
ever written into `app/`.

## Release and catalog

The root catalog entry is [`ClassicClassics.json`](ClassicClassics.json). Copy it
to `~/.config/basalt/games/` to try it locally:

```sh
cp ClassicClassics.json ~/.config/basalt/games/
basalt catalog
basalt install ClassicClassics --library default
```

Releases are produced by GitHub Actions on tags that match the Cargo version:

```sh
git tag v1.0.0
git push origin v1.0.0
```

The release asset is `ClassicClassics-<version>-linux-x64.zip` and contains only
`app/`, so Basalt can replace the release without touching `data/`. To build and
inspect a local archive:

```sh
python3 DevUtils/package.py
python3 DevUtils/verify_package.py dist/ClassicClassics-*-linux-x64.zip
```

## Documentation

- [Architecture](docs/architecture.md) — simulation, rendering, input and host boundaries
- [Basalt compliance](docs/basalt-compliance.md) — the standard's checklist, item by item
- [Testing](docs/testing.md) — deterministic, headless and native verification

## AGENTS

`AGENTS.md` is a symbolic link to this README; the README is the single source
of truth for agent instructions.

- Read this document and the linked coding standards before changing the project.
- Prefer Quasar crates for timing, windowing, input, rendering, audio, storage and configuration. Do not add parallel infrastructure without a documented reason.
- Keep every game a pure fixed-step simulation with a draw list. Headless tests must continue to run with `--no-default-features`.
- Never write to `app/` at runtime. Progress goes to `data/save/`, machine settings to `data/config/`, portable settings to `data/config/shared/`, rebuildable files to `data/cache/`.
- Write user data atomically through `quasar_storage`.
- Keep the root catalog entry and release asset names in sync with the Basalt standard.
- Keep settings changes flowing through `Session::settings`; the desktop host applies them through `sync_settings`, and the simulation never calls the window, renderer or audio device directly.
- Run `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`, and `cargo test --locked --no-default-features` before finishing.
- To commit and push, invoke the `commit-and-push` skill explicitly; it verifies local checks and watches every GitHub Actions workflow for the pushed commit.
- Do not modify other projects or repositories as part of work here.
