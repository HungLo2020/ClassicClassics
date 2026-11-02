# Quasar tilemaps and indexed rendering

Classic Classics pins all Quasar dependencies to **v0.2.20**, commit
`17fdff06aeb8e28fb0419dd06af2ce2a698c13f3`. The pinned release uses native wgpu
for presentation and retains the preloaded tilesets, incremental tile edits,
indexed images/palette bindings and per-pixel priority APIs. Original asset
files, simulations, collision rules, video frequencies, save records and global
controls stay compatible.

## Game rendering

Micro Mages gameplay and stage-clear scenes build native Quasar draw lists.
Its original four-shade atlas is encoded into background/sprite banks of one
`IndexedImage`; level palettes become small reusable lookup textures. Shade zero
retains its separate universal-background/transparent-sprite meaning. The tilemap
writes a background occupancy mask so players entering doors remain behind the
correct background pixels. The existing actor/frame selection and pixel-aligned
camera are shared with the retained software renderer.

Micro Mages' intro, setup, loading and outcome screens still use the original
software presentation path. They do not contain live gameplay tilemaps. Missile
Command and River Raid also retain their existing single-texture presentation.

Jackal's world, actors, HUD and cinematic atlases submit Quasar sprites directly.
Its terrain GIDs retain the exported atlas palette-bank offsets; background
atlases use indexed sampling and explicit occupancy independently of color.
Palette-cycle compositions are cached per source/phase instead of regenerated
on every redraw. Source water/graphics-upload timing and OAM decisions remain
in game code. Sprite parts are submitted highest-priority first through
`priority_sprite`: a sprite hidden by scenery still blocks lower-priority
sprites. Background color and occupancy are submitted as two contiguous passes
so ordinary terrain does not require a stencil-state change for each tile.

`LoadedTilesets::from_textures` receives caller-owned texture identities derived
from already-loaded assets. It does not reopen PNGs. Each game creates a
`quasar_tilemap::Runtime` for the active level. Before drawing, visible terrain
changes are collected into one `Runtime::set_tiles` batch; no runtime rebuild is
performed for destruction. Offscreen cells synchronize when they become visible.
The game remains the authoritative source for terrain and collision; the tilemap
is a presentation view, not a second mutable simulation.

Quasar owns map validation, atlas selection, visible-cell traversal and drawing.
Jackal rounds submitted cell origins exactly as its original camera did,
including fractional-camera ties. Game-specific boss background patches remain
ordered atlas sprites and update the same occupancy mask.

## Host and resource ownership

`ArcadeGame::draw_frame` optionally returns a headless `DrawFrame`: a Canvas,
shared immutable images and control help. `DrawCache` retains presentation state
and reuses its frame between original video ticks and while paused; it is skipped
by Serde and rebuilt when cloning a simulation. `Session::render` prefers this
path over the legacy `pixel_frame` adapter. Existing software-renderer APIs stay
available for independent reference comparisons.

The host imports image allocations once per stable game texture identity,
remaps source and palette bindings, and fits quads into the shared viewport.
There is no gameplay framebuffer upload in the tilemap path. Returning to the
catalog removes imported texture identities; switching games or restarting
prunes replaced resources. Entering a tiled scene destroys the obsolete native
software-frame dynamic texture. Quasar performs normal GPU upload and retirement.

## Native masked presentation

Quasar v0.2.20 uses native wgpu render passes and supports masked canvases
directly through `quasar_render_2d::Renderer2D`. The application no longer owns
a custom `RenderState2D`/`FrameSurface` composition: `Renderer2D::for_window`
acquires the surface, `present` applies palette and pixel-priority masks in the
native pass, and `render_to_image` provides GPU readback for screenshots. The
previous v0.2.17–v0.2.18 workaround was removed with `src/native_renderer.rs`.

The headless reference renderer still implements the same mask rules in software,
so draw-list tests do not require a GPU. `WGPU_BACKEND` selects the native
backend; Linux uses Vulkan 1.3 with dynamic rendering, synchronization2 and
timeline semaphores.

## Validation

`tests/quasar_tilemaps.rs` compares native-resolution Quasar reference pixels
with the retained software renderers across all 27 Micro Mages levels, six Jackal
stages, fractional cameras, live edits, palette animation and cinematic samples.
Jackal additionally compares background masking and sprite composition for all
64 OAM rotations. The production-session test checks atlas/tile quad submission,
stable texture identities on repeated draws and cleanup on catalog return.

The original game regression suites, no-I/O asset/restart checks, headless host,
native gameplay/settings/window smoke tests and relocated Basalt package checks
continue to exercise the shared pipeline. Native `--smoke-frames` reports CPU
scene preparation, presentation and upload counters; the desktop host now uses
`quasar-app` for window, event and fixed-step scheduling. Keep performance
measurement separate from concurrent compile/test workloads.

The following measurements were recorded on the older VulkanicGAL v0.2.18
renderer and are retained as historical context. Native profiling found that
v0.2.17 split every mask-writing tile into a separate draw batch; that fix
remained in v0.2.18. Isolated 1280×720 native gameplay runs on the local RTX 2070
measured:

| Game | Batches | Scene p95 | Presentation p95 | Intervals over 25 ms |
|---|---:|---:|---:|---:|
| Micro Mages | 5 | 0.591 ms | 16.990 ms | 0 / 80 |
| Jackal | 38 | 0.663 ms | 18.135 ms | 0 / 80 |

Both used zero recurring texture uploads and zero dynamic framebuffer textures.
These are local debug-build measurements, not a guarantee on other hardware.
The v0.2.20 migration removes the old native frame-target workaround; mask and
palette rendering is verified through the current wgpu presenter and the shared
reference-pixel suites.
See [the investigation](quasar-v0.2.17-rendering-issues.md) for the original
170–218 ms presentation measurements and diagnosis.
