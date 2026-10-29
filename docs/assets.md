# Shared assets

`classicclassics-assets` wraps the pinned Quasar `quasar-assets` API. Quasar owns
read-only sources, validated virtual paths, typed loaders, handles and cache
accounting. This project supplies the Basalt layout policy and PNG/audio loaders.
Game crates retain their level schemas, palette interpretation and validation.

## Ownership and loading

Each `Session` owns `GameAssets`, which owns one Quasar `AssetServer`. Selecting a
file-backed game mounts its directory as `micromages/` or `jackal/`. The game's
fallible `ArcadeGame::load` receives that virtual directory and prepares its
immutable content. Gameplay and restart reuse an `Arc` to that content; restart
does not rediscover paths, read files or decode images. Dropping the game releases
its content. There are no process-global game asset caches.

The native audio host prepares only the selected game's clips and Ogg stream
sources through the same server, before gameplay advances. Playback uses typed
Quasar handles. Ogg decoder threads stream from compressed memory, so playback
does not reopen physical files. Leaving or changing games drops the previous
audio handles and stops its music. A missing output device still permits silent
play. Headless runs do not prepare audio for an absent device.

Preparation is synchronous at game boundaries. The desktop resets its fixed-step
clock after preparation, including failed launches; loading time is never treated
as missed simulation time. Async loading/progress UI is a future extension if
larger bundles require it; this implementation adds no custom worker runtime.

## Filesystem and embedded sources

Physical locations are resolved once per game mount, with this precedence:

1. `BASALT_APP_DIR/games/<game>`
2. `BASALT_GAME_DIR/app/games/<game>`
3. `<executable's app directory>/games/<game>` for an installed `app/bin` executable
4. `<game crate>/assets` for Cargo development builds

Empty overrides are ignored. An explicit installed location never falls back to
the development checkout when files are absent. Mounted sources reject traversal
and symlinks escaping their root. Runtime asset access never writes into `app/`.

Catalog PNGs are compiled into the executable and mounted as a Quasar
`MemorySource` at `catalog/`. They use the same PNG loader as disk-backed images.
Missile Command and River Raid retain compiled graphics tables; procedural vector
games generate their graphics. None need artificial asset files or directories.

Existing JSON content files and release directory layouts stay compatible. Game
registration already specifies the namespace and development root, and existing
JSON manifests describe content; a second TOML manifest would duplicate those.
Display and controls preferences remain TOML and use the settings system.

## Cache and failure behavior

The Quasar cache has a 64 MiB / 256-entry budget. Quasar budgets are soft while
handles are held: active audio may exceed the cache budget, and eviction never
invalidates live handles. This is a cache budget, not a whole-process memory cap.
JSON bytes are read through Quasar and interpreted/validated by game loaders;
PNG images, decoded clips and compressed stream sources are typed cached assets.
The cache is cleared on return to the catalog or a failed launch. GPU textures
remain host-owned. Tiled scenes retain atlas/palette identities until catalog
return; software presentation uses the existing dynamic-texture lifecycle.
See [Quasar tilemaps](quasar-tilemaps.md).

Missing/invalid content returns to the catalog with the game/file error. Enter
retries; Backspace dismisses the dialog so another game can be selected. Failed
preparations clear cached partial results, allowing repaired content to reload.
CLI headless launches report loading failures with a nonzero exit status.

Original loader entry points still accept a filesystem `Path` for standalone
simulation/reference fixtures; these create a temporary Quasar mount and use the
same pipeline. `Default` remains a valid-checkout fixture convenience, while the
launcher exclusively uses the fallible injected loader. Immutable assets stay
out of serialized game snapshots. Standalone snapshot deserialization reloads
fixture content; persistent player records keep their existing schema.

## Adding content

For a file-backed game, add `classicclassics-assets.workspace = true`, declare
`ArcadeGame::ASSET_MANIFEST_DIR = Some(env!("CARGO_MANIFEST_DIR"))`, and implement
`ArcadeGame::load` using the supplied `Directory`. Use `Directory::join().json()`
for game-specific data, `image()` for ordinary PNGs, or a typed loader for other
formats. Keep filesystem discovery and native devices out of game simulation.
Ensure the release packager includes the game's external content.

Tests cover memory/file sources, cache sharing and eviction, invalid paths,
missing/corrupt content, Basalt relocation, recovery in the launcher, and rendering
and restarting both sprite games with all subsequent asset reads forbidden.
Original pixel and simulation reference suites continue to check fidelity.
