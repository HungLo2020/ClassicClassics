# Quasar tilemaps and indexed rendering

Classic Classics pins all Quasar dependencies to **v0.2.18**, commit
`5cc7517daa1dee8d18cc0cc8474099fc8cd12075`. The migration uses the release's
preloaded tilesets, incremental tile edits, indexed images/palette bindings and
per-pixel priority APIs. Original asset files, simulations, collision rules,
video frequencies, save records and global controls stay compatible.

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

## Native masked presentation in v0.2.17–v0.2.18

Quasar's `Renderer2D::present` convenience path cannot currently attach its
private stencil to a semantic swapchain `FrameTarget`: `mask_target` obtains a
frame handle where `RenderTargetDesc::color_views` requires a texture view.
Native reproduction reports `WrongHandleType: expected TextureView, got FrameTarget`.
Headless owned-target rendering does not have this limitation.

`src/native_renderer.rs` therefore composes Quasar's public `RenderState2D`,
`FrameSurface` and GAL APIs. Ordinary canvases draw directly to the acquired
frame. Masked canvases draw into one reusable owned color target, then use
`CopyTextureToFrameTarget` on the GPU. This adds no CPU framebuffer/readback,
shader implementation or custom tile renderer. Resizing retires the old target;
Quasar owns the mask attachments, frame transactions and submission tracking.
GPU screenshots use Quasar's `OffscreenTarget`. This application-owned glue can
be simplified when Quasar's convenience window presenter supports masked targets.

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
scene preparation, presentation and upload counters; keep performance measurement
separate from concurrent compile/test workloads.

Native profiling found that v0.2.17 split every mask-writing tile into a
separate draw batch. The fix is included in the pinned v0.2.18 release. Isolated
1280×720 native gameplay runs on the local RTX 2070 measured:

| Game | Batches | Scene p95 | Presentation p95 | Intervals over 25 ms |
|---|---:|---:|---:|---:|
| Micro Mages | 5 | 0.591 ms | 16.990 ms | 0 / 80 |
| Jackal | 38 | 0.663 ms | 18.135 ms | 0 / 80 |

Both used zero recurring texture uploads and zero dynamic framebuffer textures.
These are local debug-build measurements, not a guarantee on other hardware.
All 26 renderer library tests from the published v0.2.18 source passed with
native GPU conformance enabled, including both mask-batching regression tests.
The separate native frame-target workaround above remains necessary.
See [the investigation](quasar-v0.2.17-rendering-issues.md) for the original
170–218 ms presentation measurements and diagnosis.
