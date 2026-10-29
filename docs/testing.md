# Testing

## Fast core suite

```sh
cargo test --locked --no-default-features
```

This build excludes the window, GPU, gamepad and native audio backends. It runs
the menu, all seven simulations, vector/pixel draw-list validation, reference
rasterisation, controls validation, CLI parsing and generated-audio tests. The
suite covers:

- imported upstream simulation, rendering, campaign and replay regression suites;
- adapter/original state equivalence for movement, multiplayer, grenades and batteries;
- shared Quasar mounts/cache, missing/corrupt asset recovery and gameplay/restart without asset reads;
- imported clock cadence, buffered taps, pause/restart and original audio decoding;

- deterministic autoplay for each game (finite state, scoring, collisions);
- Pong serve/wall/paddle/score flow and mode switching;
- Lunar Lander pad generation, thrust fuel use, successful touchdown and crash;
- Asteroids splitting, ship damage, wrap-around distance and level advance;
- settings navigation, every display/audio row, reset, close, leave-to-catalog
  and quit outcomes;
- menu launch/back navigation, settings freeze and state serialization;
- full draw-list validation through `Canvas::validate` and a 960×600 headless
  rasterisation;
- six-card page bounds, navigation across partial rows and additional pages;
- registered game serialization and deterministic continuation, legacy record schemas;
- TOML round trips, JSON preference migration, invalid-file preservation,
  monitor-resolution fallback and explicit windowed CLI precedence.

All workspace packages are default members. Test a single game independently:

```sh
cargo test --locked -p classicclassics-pong
cargo tree --locked -p classicclassics-pong -e features
```

The game dependency tree must remain free of native window/audio/gamepad backends.

## Headless screenshots

```sh
cargo run --locked --no-default-features -- \
  --headless --ticks 2 --screenshot /tmp/menu.png --state /tmp/menu.json
cargo run --locked --no-default-features -- \
  --headless --game pong --ticks 1200 --autoplay --screenshot /tmp/pong.png
cargo run --locked --no-default-features -- \
  --headless --game lunar --ticks 1200 --autoplay --screenshot /tmp/lunar.png
cargo run --locked --no-default-features -- \
  --headless --game asteroids --ticks 1200 --autoplay --screenshot /tmp/asteroids.png
cargo run --locked --no-default-features -- \
  --headless --game asteroids --settings --ticks 1 --screenshot /tmp/settings.png
```

Additional game aliases are `micromages`, `jackal`, `missile-command` and
`river-raid`. Use 6000 ticks to exercise setup and actual gameplay during autoplay.

The catalog screenshot is 1920×1080; game screenshots remain 960×600.

`--state` writes a JSON snapshot with the selected menu entry, records and the
current game state. `--ticks` counts fixed simulation ticks, not frames.

## Full native suite

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
```

The full build adds `quasar-window`, VulkanicGAL, `quasar-audio`'s CPAL backend
and the `gilrs` gamepad adapter. Unit tests still do not require a GPU; the
windowed smoke test does.

## Shared controls

```sh
cargo run --locked --no-default-features -- --headless --no-save --controls \
  --ticks 1 --screenshot /tmp/controls.png
cargo run --locked -- --no-save --controls --smoke-frames 10
```

Tests cover modal capture isolation/cancellation, keyboard chords and modifier
keys, mouse/gamepad buttons, digital axis pairs, stick orientation and neutral
arming, device-specific cell replacement/clearing, mouse hit testing at different
canvas sizes, category/column navigation, restoring defaults, TOML round trips, preset migration,
and remapped catalog navigation. The host integration test navigates the shared
editor, changes Fire, saves/reloads the preset, and uses it in both Pong and
Asteroids. Native checks should also exercise Escape/arrow/Enter recovery after
clearing navigation, shared key notices and focus-loss cancellation.

## Native smoke test

```sh
cargo run -- --game pong --autoplay --no-save --windowed \
  --smoke-frames 30 --exercise-window
```

The exercise requests a resize, enters and exits fullscreen, switches to
linear+integer presentation, submits a zero-sized surface (which must defer),
restores the size and presents the remaining frames. Startup fullscreen and the
settings overlay are also covered by the smoke commands:

```sh
cargo run -- --no-save --smoke-frames 10
cargo run -- --no-save --windowed --resolution 1280x720 --smoke-frames 10
cargo run -- --no-save --game lunar --settings --smoke-frames 10
``` It prints
`presented_frames`, the exercise stage, actual fullscreen state, resolved
resolution and `RenderStats`. The same run
can target Lunar Lander or Asteroids by changing `--game`.

A machine without a Vulkan 1.3 driver cannot run this step. That is expected;
the headless suite and reference renderer still verify all simulation and draw
list behaviour.

## Release verification

Release publication is manual and separate from normal commits/pushes. Test the
publisher locally against its simulated API (this never publishes):

```sh
python3 -m unittest DevUtils.test_release
```

These checks cover first publication, replacing the same release/tag, version
changes, failed/corrupt uploads preserving the previous asset, draft retries,
stale commits and failed CI.


```sh
python3 DevUtils/package.py
python3 DevUtils/verify_package.py dist/ClassicClassics-*-linux-x64.zip
```

The verifier checks both imported sprite games’ complete asset trees, the asset name, `app/`-only root, absence of user data,
required artwork/runtime files, the executable bit, the cover size and the
catalog entry's schema, id, repository and launch path.

## CI

`.github/workflows/ci.yml` runs formatting, Clippy with `-D warnings`, the full
test suite, the core-only test suite and a headless render smoke test on
Ubuntu 24.04, including simulated release-tool tests. It never publishes.
`.github/workflows/release.yml` runs only by explicit manual dispatch, requires
successful CI for the requested current main SHA, packages/verifies the ZIP and
replaces the single `Latest` release. Use the project release skill to dispatch,
watch and verify both the Actions artifact and the ZIP Basalt downloads.

## Imported-game performance and artwork

`rendering.rs` integration tests guard one-sprite pixel presentation, immutable
frame reuse between video ticks, unchanged simulation snapshots, correct pixel
colors/aspect, and complete Basalt covers on both catalog pages. Cover pixels are
checked against independent source checksums in `docs/artwork-sources.json`.

For repeatable CPU profiling after setup:

```sh
cargo run --locked --no-default-features --example profile_imports
cargo run --locked --release --no-default-features --example profile_imports
```

For native frame pacing (use debug and release, with audio on and off):

```sh
cargo run --locked -- --game micromages --autoplay --no-save --no-audio --smoke-frames 1800
cargo run --locked -- --game micromages --autoplay --no-save --set audio.volume=0.0 --smoke-frames 1800
```

Smoke runs report scene/presentation/interval p95 times, slow intervals, upload
bytes and dynamic texture count. Statistics exclude the first third of frames;
use enough frames to reach gameplay. A game frame should use one dynamic texture
and a few dozen glyph/background quads. Repeat for the other three imports.
