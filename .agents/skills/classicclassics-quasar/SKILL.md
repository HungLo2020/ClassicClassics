---
name: classicclassics-quasar
description: Work on Classic Classics, a Quasar-powered arcade collection. Use when adding or changing games, input bindings, vector draw lists, persistence, packaging or the Basalt catalog entry.
---

# Classic Classics / Quasar workflow

Use this skill when modifying the Classic Classics repository.

## Boundaries

- Keep simulations and draw lists in the library. Do not put window, GPU,
  gamepad or audio-device calls in game modules.
- Use Quasar crates for infrastructure: `quasar-core` timing,
  `quasar-window`, `quasar-input`, `quasar-render-2d`, `quasar-audio`,
  `quasar-storage`, `quasar-config`.
- Never write to `app/` at runtime.
- User progress goes to `data/save/`; machine settings to `data/config/`;
  portable settings to `data/config/shared/`; rebuildable files to
  `data/cache/`. Persist through `quasar_storage::write_json`.
- Do not modify other projects or repositories.

## Change checklist

1. Add or change pure simulation state in `src/games/<game>.rs`.
2. Add actions to `src/controls.rs` with defaults, validation and HUD hints.
3. Emit audio events through `src/audio.rs`; never load audio files.
4. Verify the draw list is deterministic and validates with
   `Canvas::validate`.
5. Update records in `src/session.rs` only when a result changes, not every tick.
6. Add deterministic unit tests and, for visual changes, run the headless
   screenshot command.
7. Run:
   ```sh
   cargo fmt --all -- --check
   cargo clippy --locked --all-targets -- -D warnings
   cargo test --locked
   cargo test --locked --no-default-features
   ```
8. If packaging or metadata changed, run `DevUtils/package.py` and
   `DevUtils/verify_package.py`.

## Basalt rules to remember

- Release ZIP root is exactly `app/`.
- Asset name is `ClassicClassics-<version>-linux-x64.zip`.
- Launch executable is `app/bin/classicclassics`.
- Catalog entry is `ClassicClassics.json` at the repository root.
- No updater, no writes outside the install root, no network at startup.
