---
name: classicclassics-quasar
description: Work on Classic Classics, a Quasar-powered arcade collection. Use when adding or changing games, input bindings, vector draw lists, persistence, packaging or the Basalt catalog entry.
---

# Classic Classics / Quasar workflow

Use this skill when modifying the Classic Classics repository.

## Boundaries

- Keep simulations and draw lists in the library. Do not put window, GPU,
  gamepad or audio-device calls in game modules.
- Use `classicclassics-assets` / `quasar-assets` for read-only asset sources, typed
  decoding and caching. Games receive mounts; avoid global caches or path discovery.
- Use Quasar crates for infrastructure: `quasar-core` timing,
  `quasar-window`, `quasar-input`, `quasar-render-2d`, `quasar-audio`,
  `quasar-storage`, `quasar-config`.
- Never write to `app/` at runtime.
- User progress goes to `data/save/`; machine settings to `data/config/`;
  portable settings to `data/config/shared/`; rebuildable files to
  `data/cache/`. Persist JSON through `quasar_storage::write_json`; TOML preferences use the
  collection's atomic `config_file` adapter (Quasar has no TOML writer).
- Do not modify other projects or repositories.

## Change checklist

1. Add or change pure simulation state in `crates/classicclassics-<game>/src/lib.rs`.
   Implement `classicclassics_core::ArcadeGame` and register new games in
   `crates/classicclassics-collection/src/games.rs`. Keep each game independent
   of other games and the collection.
2. Add actions to `crates/classicclassics-collection/src/controls.rs` with defaults,
   validation and HUD hints. Shared remapping uses Quasar UI in `controls_screen.rs`;
   the host applies `Session::controls` with `sync_controls`. Never bind physical
   keys in a game crate.
3. Emit shared `Sound` events. Synthesis and native playback live in
   `crates/classicclassics-collection/src/audio.rs`. Imported audio is prepared
   through shared asset handles at game boundaries; never load/decode during ticks
   or audio callbacks. See `docs/assets.md`.
4. Verify the draw list is deterministic and validates with
   `Canvas::validate`.
5. Keep game record schemas and update rules in each game crate. The collection
   session aggregates records and handles persistence; preserve existing saves.
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
