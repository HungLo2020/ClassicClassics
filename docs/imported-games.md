# Imported games

MicroMages, Jackal, MissileCommand and RiverRaid are now owned workspace game
crates in Classic Classics. They are copied source imports, not Git dependencies,
submodules, subprocesses or independent applications. Future gameplay development
belongs here; the source repositories have not been changed. Each crate's
`UPSTREAM.md` records the exact GitHub source commit.

| Crate | Original simulation rate | Retained presentation |
|---|---|---|
| `classicclassics-micromages` | 60 Hz | PNG tile/sprite renderer, campaign/password screens, original Ogg music/effects |
| `classicclassics-jackal` | 60.0988 Hz | PNG renderer, six-stage campaign, cinematics, original WAV music/effects |
| `classicclassics-missile-command` | 61.03515625 Hz | Original 256×231 software renderer and arcade rules |
| `classicclassics-river-raid` | 60 Hz | Original 160×210 software renderer, river generator and fuel rules |

## Scope of the adapters

Each `src/adapter.rs` implements `ArcadeGame`, maps logical input to the original
simulation, records high scores, and supplies launcher metadata and artwork.
`VideoClock` advances the original video clock from the shared 120 Hz host;
short button taps are accumulated until a video frame consumes them. Pause and
restart belong to the adapter, so the shared Settings overlay freezes every game.

Original gameplay, collision, level data, enemies and pixel renderers are retained.
Integration changes include snapshot/clone/debug derives, shared Quasar asset
loading, render pixel alignment, and replacing Micro Mages' static animation-name
field with an owned string for deserialization. Level terrain damage is included in snapshots.
Immutable assets are owned by each running game and restored when loading a
standalone fixture snapshot. All mutable simulation state remains serializable without replaying an
input log. Existing standalone device, storage and executable hosts are omitted.

The original software renderers remain to preserve the recovered pixels. Each
adapter caches one immutable Quasar `Image` per original video tick. Redraws
between ticks and paused frames reuse its RGBA allocation. `Session::render`
uploads it through a host-owned `SceneTarget`: a persistent Quasar dynamic texture
on the GPU, or a CPU image in `SceneAssets` for reference screenshots. A game image
is **one sprite**, not thousands of tiny rectangles. Texture storage is reused;
changing game dimensions destroys the old dynamic texture before creating another.
Control hints and shared overlays remain ordinary Quasar draw lists. The old
geometry-only `scene`/`reference_canvas` path exists for simulation reference tests;
production hosts must use `Session::render`.

PNG decoding now uses the shared Quasar loader; game-specific palette conversion
and pixel fixtures preserve the recovered rendering behavior. See [Shared assets](assets.md). Imported modules retain their source
formatting and permit a few legacy Clippy style lints. Development builds optimize
these CPU renderers and the core helpers, while host code remains debuggable.

Micro Mages and Jackal emit asset cue data. When a game is selected, the
collection prepares only that game's short effects and WAV clips through Quasar
asset handles, before its simulation clock advances. Micro Mages' Ogg music uses
Quasar's bounded streaming decoder threads, keeping whole-track decoding off the
simulation thread. Playback never synchronously loads/decodes a missing clip.
Music stops at the catalog and pauses with Settings or the game. Source loop
regions are retained; Micro Mages' fade-out cue currently stops immediately.
Standalone effect-priority rules use the collection's shared bus/voice limits.
The other games' semantic sound events use generated effects.

## Launcher artwork

All four imported games use the same covers as their standalone Basalt packages.
Micro Mages' cover comes from `packaging/artwork/cover.png`, Jackal's from
`packaging/artwork/cover.jpg`, and Missile Command/River Raid from `assets/cover.png`.
Jackal's PNG is a lossless conversion of that JPEG. Source hashes, pixel checksums
and credits are recorded in [artwork-sources.json](artwork-sources.json) and the
sprite games' `assets/basalt/sources.json` files. The original cover pixels are
embedded and decoded once; catalog cards show the entire 3:4 image without cropping
or distortion, alongside the title and score. Six cards still fit at 1920×1080.

## Controls

All controls below are defaults in the global Quasar preset, editable through
Settings → Controls. Existing portable presets gain missing new actions while
keeping their assignments and explicitly disabled actions.

| Game | Movement | Primary | Additional controls |
|---|---|---|---|
| Micro Mages | WASD / D-pad or left stick | LMB / South: spell | Space / North: jump; Enter / South: advance setup; Shift / West: hard mode in setup |
| Jackal | WASD / D-pad or left stick | LMB / South: gun | Shift or RMB / West: grenade; Enter / South: start; Tab / North: select 1–2 players on title |
| Missile Command | Mouse, WASD, D-pad or left stick: aim | LMB / South: center battery | Shift or RMB / West: left battery; Space or MMB / North: right battery; Enter / South: start |
| River Raid | A/D: steer, W/S: speed / D-pad or left stick | LMB / South: fire | Enter / South: start |

P pauses, R restarts, Escape opens Settings, and Backspace returns to the catalog.
The existing Pong/ship controls remain separate from the imported games' movement
so player two's arrows cannot steer player one.

Micro Mages preserves its 1–4 player setup and Jackal preserves 1–2 players. Player
two can use arrows with Numpad 0 (fire), Numpad 1 (jump) and Numpad 2 (grenade).
Up to four controllers use separate Quasar player slots and the same global
gameplay mapping. The first controller is player one, even when a keyboard is in
use. Select the desired player count in the game's original setup screens.

## Assets, records and packaging

The two sprite games retain their complete exported asset trees under their game
crates. Packaging copies them to `app/games/micromages/` and `app/games/jackal/`.
An installed executable resolves those paths using Basalt's environment or its
own `app/bin/` location; it never depends on a source checkout. Cargo builds use
the crate-local asset trees. Runtime writes still go only to the collection's
`data/` directories. The archive verifier checks that every required asset ships.

Each imported game adds its own defaulted record entry to `data/save/records.json`.
The original Pong, Lander and Asteroids record entries retain their schema.
Standalone installations and save/config files are not altered or imported.

The original simulation/rendering regression fixtures and integration suites are
retained. Tests for retired standalone executables/settings hosts are replaced by
collection tests. `classicclassics-collection/tests/imported_games.rs` compares
adapter sessions directly with original simulation sessions, checks multiplayer,
battery edges, pause/restart and Quasar decoding of the original audio assets.
