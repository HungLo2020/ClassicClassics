# Adding a game

Games are statically linked workspace libraries. A game owns its simulation,
rendering, record schema, catalog artwork and tests. The collection owns selection,
settings, persistence and device access.

## Create the crate

Create `crates/classicclassics-<name>/Cargo.toml` and `src/lib.rs`. The workspace's
`crates/*` membership discovers it automatically. Inherit package settings and
dependencies:

```toml
[package]
name = "classicclassics-example"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
classicclassics-core.workspace = true
quasar-render-2d.workspace = true
serde.workspace = true
```

Implement `classicclassics_core::ArcadeGame` for the game's `Game` type. Pong is
the smallest existing example. The contract requires:

- `Game`: `Default`, `Clone`, `Debug`, Serde serialization/deserialization.
- `Record`: the same traits, with defaults for newly introduced fields.
- `INFO`: name, unique CLI aliases, tagline and catalog accent color.
- `tick`: advance exactly one 120 Hz tick from shared `Input`, appending `Sound`
  events. Keep RNG/particles in the game state; do not use ambient randomness.
- `scene`: produce a Quasar `Canvas`, using the supplied `ControlHints`.
- `record`: update only this game's record. Handle one-time counters without
  counting the same result twice when called on successive ticks.
- `autoplay` and `summary`: deterministic input and useful smoke-test output.
- `catalog_record` and `motif`: record label/value and vector catalog artwork.
  Artwork is centered at the supplied position inside a 512×142 area.

Keep game crates free of windowing, storage and device backends. Use Quasar for
infrastructure, and the shared `paint`/`simulation` helpers for existing drawing,
RNG and particles. Prefer game-local helpers until another game needs them.

## Register it

Add the path dependency under `[workspace.dependencies]` in the root manifest:

```toml
classicclassics-example = { path = "crates/classicclassics-example" }
```

In `crates/classicclassics-collection/Cargo.toml`, add:

```toml
classicclassics-example.workspace = true
```

Import the crate and add one entry to the registration table in
`crates/classicclassics-collection/src/games.rs`:

```rust
use classicclassics_example as example;
// Inside games! { ... }:
// Example => example, example, "04";
```

The fields are the enum variant, crate alias, persistent record key and cabinet
number. Choose stable identifiers: existing enum tags and record keys are part
of serialized snapshots/saves. New record fields default when older saves lack
them. More complex record migrations require an explicit schema migration.

Registration supplies catalog entries, CLI aliases/help, game dispatch and the
aggregate records automatically. The catalog lays out up to six games per page;
paging follows the selection, including incomplete final pages. No game-specific
menu match arms are needed.

Games choose the meaning of existing logical `Input` fields. They must not read
physical key or controller state. The shared Controls menu applies the same
Quasar preset to every game. If new actions are necessary, extend shared
`Input`/`ControlHints`, `ACTIONS` and the defaults in collection `controls.rs`,
then map the new named action in `src/desktop.rs`. Add any signed action to
`controls_screen::axis_directions` so its editor captures a button pair or stick.
Place each new action in the Gameplay, Menus or System list in `controls_screen.rs`;
the editor checks that every action appears exactly once. Do not create a
game-specific control editor or persistence file. Imported software renderers can use the documented compatibility adapter in
[Imported games](imported-games.md) without changing their gameplay. New sound events go
in core's `audio.rs`, with synthesis/playback in the collection's `audio.rs`.
Preserve old portable binding presets or provide an explicit migration.

## Verify it

```sh
cargo test --locked -p classicclassics-example
cargo run --locked --no-default-features -- --headless --no-save \
  --game example --autoplay --ticks 1200 --screenshot /tmp/example.png
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --no-default-features
```

After first adding the workspace dependency, run Cargo without `--locked` once
to update `Cargo.lock`. The shared registry tests exercise every registered game:
serialization, deterministic continuation, sounds, records and draw-list
validation. Add game-specific simulation tests in its own crate and inspect its
headless and native presentation before finishing. Packaging still emits one
`app/bin/classicclassics` binary; adding a game does not require another launcher
or a different Basalt catalog entry.

## Pixel games and cover art

An imported software renderer can implement `ArcadeGame::pixel_frame` and return a
`PixelFrame` containing a cached Quasar `Image`, aspect ratio and help text. Keep
`FrameCache` out of Serde snapshots and invalidate it only on original video ticks
or changes affecting rendered pixels. `Session::render` routes the frame to the
native dynamic-texture or headless CPU target. Never expand each pixel/run into
geometry in the application host. Games still have no GPU/window/device handles.

Set the optional `ArcadeGame::COVER` to embedded Basalt PNG bytes. The collection
registers and caches these textures automatically and fits the complete portrait
cover into its card. Preserve source credits and update the artwork provenance
manifest when importing or changing a cover.
