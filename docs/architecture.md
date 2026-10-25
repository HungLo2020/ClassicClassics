# Architecture

Classic Classics is one Rust binary with two layers:

1. **Library (`src/lib.rs`)** — menu, simulations, draw lists, persistence
   models and audio synthesis. No window, GPU, device or filesystem access is
   required to build or run the simulations.
2. **Desktop host (`src/desktop.rs`)** — Quasar window, input routing,
   GPU presentation, audio device and the winit application loop. It is compiled
   only with the `desktop` feature.

This split is the same one Quasar's Pong reference uses. It is what makes the
headless test path meaningful: the exact same tick and the exact same draw list
that reach the GPU can be rasterised by `quasar_render_2d::reference`.

## Module map

| Module | Responsibility |
|---|---|
| `src/main.rs` | CLI, Basalt paths, settings/controls/records loading, headless run |
| `src/cli.rs` | Game-specific flags layered over `quasar_config::CommonArgs` |
| `src/session.rs` | Catalog state, game wrapper, shared `Input`, records and JSON state |
| `src/menu.rs` | Catalog draw list and cabinet motifs |
| `src/settings_screen.rs` | Modal settings overlay: display, window, volume, mute, reset, quit |
| `src/controls.rs` | Action names, default bindings, validation and HUD hints |
| `src/paint.rs` | Vector vocabulary over `quasar_render_2d::Canvas` |
| `src/audio.rs` | Generated `quasar_audio::Clip` bank and optional `AudioEngine` wrapper |
| `src/settings.rs` | Machine-local display/audio settings |
| `src/games/*.rs` | Pong, Lunar Lander and Asteroids simulations and draw lists |
| `src/desktop.rs` | Native event loop, renderer, gamepad hub, audio playback, saving |

## Fixed step

Every simulation and the menu advance at `TICK_HZ = 120`. The desktop host uses
`quasar_core::FixedStep` with a bounded catch-up (eight ticks), and the headless
runner simply calls the same `Session::tick` in a loop. Inputs carry both held
values (thrust, fire) and one-tick edges (serve, pause, confirm). Edges are
consumed after each tick through `quasar_input::InputState::consume_edges`, so a
fast press/release between frames is not lost and a single press cannot trigger
twice.

Each game owns its RNG state and any particle state, so a serialized game is a
complete deterministic snapshot. The Lunar Lander terrain, Asteroids spawns and
sound synthesis are all deterministic; no wall-clock time or ambient randomness
enters the simulation.

## Input

`controls.rs` declares twenty-one named actions. `quasar_input::Bindings` maps
keyboard, mouse and gamepad sources to them. The desktop host reads two
`InputState`s: the window's keyboard/mouse state and the gamepad hub's player
state, then merges the same action from both. This means rebinding only changes
the preset, never game code, and the HUD labels are derived from the first
binding.

**Escape** opens the settings overlay; **Backspace** returns from a game to the
catalog. The overlay is a modal layer owned by `Session`: while it is open the
base game still exists but receives no ticks, so the simulation freezes exactly
where it was. Analog stick menu navigation is latched in the host so a fast
`-1 -> +1` sweep produces one edge, not two.

Gamepads are discovered and routed by `quasar_input::GilrsGamepads` and
`GamepadHub`. Deadzones, trigger thresholds, hotplug and disconnect handling all
live in Quasar. The `gamepad` feature can be disabled without touching game
logic.

## Rendering

Quasar draws ordered quads. The vector games use `Paint::line`, which sets a
local rotation/translation transform and emits one thin quad, then restores the
identity transform. Polylines, circles, polygon outlines, glows, dashed lines
and the vector ship/lander are built from that primitive. Text uses Quasar's
built-in 5×7 bitmap font atlas.

Both games that ask for a "vector look" use outlines and additive-looking halos
rather than filled raster art. The result is a single deterministic draw list per
frame (`Canvas`), presented by `quasar_render_2d::Renderer2D` or rasterised by
`quasar_render_2d::reference::rasterize`.

Presentation settings use Quasar's `DisplayOptions`:

- **F2** toggles nearest/linear sampling.
- **F3** toggles fit/integer scaling.
- **F11** or **Alt+Enter** toggles fullscreen.
- Resize and DPI handling remain in `quasar_window::GameWindow`.

## Settings overlay and host application

`Session` owns the live `Settings` and the optional `SettingsScreen`. The
overlay edits those settings in place and never touches the window, renderer or
audio device directly. After each tick the desktop host compares
`Session::settings` with the last applied copy in `App::sync_settings`:

- fullscreen requests go to `quasar_window::GameWindow::set_fullscreen`;
- window-size presets use the native logical `request_inner_size`;
- filter/scaling changes call `Renderer2D::set_options`;
- mute and volume call the Quasar audio engine.

This one-way flow keeps the simulation platform-independent and means a
headless run can screenshot the settings overlay through the same draw list as
the GPU. Settings are written to `data/config/display.json` on a normal exit.

## Audio

`audio.rs` synthesises every effect into a `quasar_audio::Clip` at startup. There
are no audio files in the release. When the `audio` feature is enabled, a native
`AudioEngine` is opened with a 48 kHz stereo request and an "effects" bus. A
missing device is not fatal: `AudioEngine::try_open` returns `None` and the game
continues silently. **M** mutes the bus and persists the preference.

## Persistence

`quasar_storage::GamePaths` implements the Basalt directory contract, including
`BASALT_*` overrides. `quasar_storage::read_json` distinguishes a missing file
from a corrupt one; corrupt files are preserved rather than overwritten.
`write_json` is atomic (temporary file, `sync_all`, rename). The files are:

| File | Category | Contents |
|---|---|---|
| `data/save/records.json` | save (synced, backed up) | Best rally, wins, matches, best scores, landings, crashes, best level |
| `data/config/display.json` | config (backed up, not synced) | Size, fullscreen, filtering, scaling, audio volume and mute |
| `data/config/shared/controls.json` | shared config (synced, backed up) | Portable bindings preset |

Nothing is stored outside the install root, and `app/` is never written at
runtime.

## Error policy

The binary prints `classicclassics: <reason>` to standard error and exits
non-zero on a fatal error. A normal quit, including **Q** or **Escape** in the
catalog, exits zero. Invalid saved settings are reported once, preserved, and
replaced in memory by defaults.
