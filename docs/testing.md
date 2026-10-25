# Testing

## Fast core suite

```sh
cargo test --locked --no-default-features
```

This build excludes the window, GPU, gamepad and native audio backends. It runs
the menu, all three simulations, vector draw-list validation, reference
rasterisation, controls validation, CLI parsing and generated-audio tests. The
suite covers:

- deterministic autoplay for each game (finite state, scoring, collisions);
- Pong serve/wall/paddle/score flow and mode switching;
- Lunar Lander pad generation, thrust fuel use, successful touchdown and crash;
- Asteroids splitting, ship damage, wrap-around distance and level advance;
- settings navigation, every display/audio row, reset, close, leave-to-catalog
  and quit outcomes;
- menu launch/back navigation, settings freeze and state serialization;
- full draw-list validation through `Canvas::validate` and a 960×600 headless
  rasterisation.

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

## Native smoke test

```sh
cargo run -- --game pong --autoplay --no-save \
  --smoke-frames 30 --exercise-window
```

The exercise requests a resize, enters and exits fullscreen, switches to
linear+integer presentation, submits a zero-sized surface (which must defer),
restores the size and presents the remaining frames. Startup fullscreen and the
settings overlay are also covered by the smoke commands:

```sh
cargo run -- --no-save --fullscreen --smoke-frames 10
cargo run -- --no-save --game lunar --settings --smoke-frames 10
``` It prints
`presented_frames`, the exercise stage reached and `RenderStats`. The same run
can target Lunar Lander or Asteroids by changing `--game`.

A machine without a Vulkan 1.3 driver cannot run this step. That is expected;
the headless suite and reference renderer still verify all simulation and draw
list behaviour.

## Release verification

```sh
python3 DevUtils/package.py
python3 DevUtils/verify_package.py dist/ClassicClassics-*-linux-x64.zip
```

The verifier checks the asset name, `app/`-only root, absence of user data,
required artwork/runtime files, the executable bit, the cover size and the
catalog entry's schema, id, repository and launch path.

## CI

`.github/workflows/ci.yml` runs formatting, Clippy with `-D warnings`, the full
test suite, the core-only test suite and a headless render smoke test on
Ubuntu 24.04. `.github/workflows/release.yml` repeats the build on a version tag,
packages and verifies the ZIP, uploads it and publishes the GitHub Release.
