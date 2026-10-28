# Architecture

Classic Classics is a Cargo workspace with six crates and one executable.

| Package | Location | Responsibility |
|---|---|---|
| `classicclassics` | `src/` | Native window/event loop and headless command-line host |
| `classicclassics-core` | `crates/classicclassics-core/` | `ArcadeGame`, `GameInfo`, input, sound events, HUD hints, vector drawing, RNG and particles |
| `classicclassics-collection` | `crates/classicclassics-collection/` | Registration, session, catalog, settings overlay, bindings, persistence and audio synthesis |
| `classicclassics-pong` | `crates/classicclassics-pong/` | Pong and its record schema/catalog motif |
| `classicclassics-lunar` | `crates/classicclassics-lunar/` | Lunar Lander and its record schema/catalog motif |
| `classicclassics-asteroids` | `crates/classicclassics-asteroids/` | Asteroids and its record schema/catalog motif |

The dependency direction is host → collection → games → core. Each game depends
only on core, Serde and Quasar's draw-list API. Games never depend on each other,
windowing, storage or device backends. Core is independent of all game crates.
The host's `desktop`, `gamepad` and `audio` features enable platform backends;
the collection's `audio` feature only adds native audio output.

`ArcadeGame` defines fixed-step update, draw list, autoplay, record update,
summary and catalog presentation. Each game owns a serializable `Game` and
`Record`. The collection's `games!` registration table generates `GameKind`,
the serializable `Game` enum, dispatch and the aggregate `Records` schema.
Adding a registration automatically updates CLI lookup/help and catalog entries.
The existing enum serialization tags and version-one record field names remain
unchanged. See [Adding a game](adding-a-game.md).

All packages are default workspace members, so `cargo test` and `cargo clippy`
cover every crate. The workspace owns versions and external dependency pins.
The root package remains named `classicclassics`, preserving Cargo run commands,
package metadata lookup, executable paths and the release archive layout.

## Catalog and logical canvases

The catalog uses a 1920×1080 draw list with six card slots in a three-column,
two-row grid. Page selection follows the highlighted game; subsequent games
appear on later pages. Left/right advances through games, and up/down moves
within a column, wrapping over existing rows. With only one row, up/down
also advances through games. Only registered games are shown.
Game metadata, motifs and record labels come from their own crates.

Games retain their 960×600 logical playfields to preserve physics and rendering.
Quasar scales either canvas to the actual window. Settings overlays fit their
960×600 design into the current canvas while preserving aspect ratio, and
screenshots use the scene's dimensions instead of assuming one global size.

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

`classicclassics-collection/src/controls.rs` declares twenty-three named actions. `quasar_input::Bindings` maps
keyboard, mouse and gamepad sources to them. The desktop host reads two
`InputState`s: the window's keyboard/mouse state and the gamepad hub's player
state, then merges the same action from both. Catalog stick axes also come from named
actions, rather than reading a hardcoded physical stick. This means rebinding only changes
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

## Shared controls editor

The global preset is owned by `Session::controls`, separate from each game's
interpretation of the logical `Input`. `settings_screen.rs` opens the shared
`controls_screen.rs` editor from the catalog or any paused game. Quasar UI owns
its `Binding`/`Button` widget tree, modal focus, layout and capture isolation;
Quasar's renderer adapter appends its draw list to the same canvas as the game.
The Gameplay/Menus/System categories expose every action in a table with
Keyboard/Mouse and Controller columns. Clicking or selecting a cell opens a
capture dialog naming the action and device. A cell displays all of that device's
assignments, joined with OR; editing it replaces those assignments and retains
the other device. Delete clears the cell, and the footer contains only Restore
Defaults and Back. Directional prompts name Left/Right or Up/Down explicitly.

Pointer events are mapped through Quasar's `Viewport::to_source`, then into the
same fitted design coordinates used by the UI layout. Quasar handles widget hit
testing. Arrow navigation retains the current device column when changing rows;
Tab also traverses widgets. Keyboard and mouse capture accepts only that device,
and controller capture accepts controller buttons/axes.

The editor transforms portable device events into Quasar `Binding::Button`,
`Chord`, `Axis` or `GamepadAxis` values. Digital axis mappings collect the
negative and positive buttons; stick mappings record orientation. Buttons and
triggers map digital actions. The host translates native key/mouse events using
Quasar identifiers and polls Quasar's gamepad input states; it arms stick capture
only after neutral to avoid recording the movement used to open the editor.

`App::sync_controls` validates and installs a changed preset in the window,
controller player and fallback `InputState`s. It releases pending/held input at
capture and configuration boundaries so the capture cannot fire in gameplay.
Session rendering derives HUD hints from the current preset. The session ignores
all gameplay/global commands while the editor is capturing; focus loss cancels
capture. Global quit/fullscreen/mute/filter/scaling commands now follow the same
named-action tick path for both keyboards and gamepads.

Normal exit saves valid presets as TOML using the existing atomic adapter;
explicit `--bindings`, `--no-save` and corrupt/newer saved files prevent writes.
Version-one JSON/TOML presets remain readable. Missing new catalog-axis actions
are filled from defaults, while explicitly empty actions remain disabled.
Escape plus arrow/Enter navigation inside Settings are reserved recovery paths,
so remapping navigation never makes the shared editor inaccessible. Games receive
only the logical input and own no physical keys or controller bindings.

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

On first run, settings request borderless fullscreen and automatic monitor
resolution, with a 1920×1080 fallback. The host obtains monitor information via
Quasar's re-exported winit handles; Quasar's borderless window fills the native
monitor without changing its display mode. Logical window sizes account for
monitor DPI. Saved preferences, TOML/JSON run configuration and explicit common
CLI/environment flags override defaults. Explicit dimensions disable automatic
resolution unless configured otherwise. Manual resizing and resolution presets
also select a fixed window size; Auto restores monitor sizing.

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
the GPU. Settings are written to `data/config/display.toml` on a normal exit.

## Audio

`classicclassics-collection/src/audio.rs` synthesises every effect into a `quasar_audio::Clip` at startup. There
are no audio files in the release. When the `audio` feature is enabled, a native
`AudioEngine` is opened with a 48 kHz stereo request and an "effects" bus. A
missing device is not fatal: `AudioEngine::try_open` returns `None` and the game
continues silently. **M** mutes the bus and persists the preference.

## Persistence

`quasar_storage::GamePaths` implements the Basalt directory contract, including
`BASALT_*` overrides. `quasar_storage::read_json` distinguishes a missing file
from a corrupt one; corrupt files are preserved rather than overwritten.
`write_json` is atomic (temporary file, `sync_all`, rename). Preferences are
parsed by Quasar Config. Its `Config` handles defaults/file/CLI precedence and
validates typed values through Serde. TOML configuration is written by a small
`config_file` adapter with the same atomic contract, since Quasar 0.2.16 exposes
no TOML writer. Legacy JSON is read only when the TOML file is absent; valid
preferences migrate on normal exit without deleting JSON. Corrupt/newer files
prevent replacement, and saves retain their version-one JSON schema. The files are:

| File | Category | Contents |
|---|---|---|
| `data/save/records.json` | save (synced, backed up) | Best rally, wins, matches, best scores, landings, crashes, best level |
| `data/config/display.toml` | config (backed up, not synced) | Size, fullscreen, filtering, scaling, audio volume and mute |
| `data/config/shared/controls.toml` | shared config (synced, backed up) | Portable bindings preset |

Nothing is stored outside the install root, and `app/` is never written at
runtime.

## Error policy

The binary prints `classicclassics: <reason>` to standard error and exits
non-zero on a fatal error. A normal quit, including **Q** or **Backspace** in the
catalog, exits zero. Invalid saved settings are reported once, preserved, and
replaced in memory by defaults.
