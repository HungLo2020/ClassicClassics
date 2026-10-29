# Quasar v0.2.17 native rendering findings

Audited tag: `v0.2.17`, commit `59cf3c34ddb0a59ea91c330b8c93920a86da7c93`.
The Classic Classics tilemap/palette/priority migration passes the retained
software-reference comparisons. This report records the original failures.
The batching fix shipped in v0.2.18 and is now pinned by Classic Classics;
[the integration report](quasar-tilemaps.md) records native verification.
The separate frame-target issue still uses the application workaround.

## 1. Masking a window FrameTarget

`quasar-render-2d/src/gpu.rs`, `RenderState2D::mask_target`, calls
`gal.pass_target_color_attachment(original)` and passes that handle to
`RenderTargetDesc::color_views`. For an acquired window frame, this is a semantic
`FrameTarget`, not a `TextureView`. Native masked presentation fails with:

```text
Handle/WrongHandleType: expected TextureView handle, got FrameTarget
```

Owned offscreen targets work. Classic Classics has application-owned composition
in `src/native_renderer.rs`: Quasar frame/state APIs render to a reusable owned
GPU color target, then `CopyTextureToFrameTarget` presents it. No CPU readback or
framebuffer upload is involved. Upstream's convenience window presenter should
provide equivalent native masked-target composition so this glue can be removed.

## 2. One GPU batch for every mask-writing tile

In the same source file, the draw-batch merge condition contains:

```rust
&& q.pixels.write_mask == 0
```

This separates even consecutive tiles with identical texture, palette, filter and
pixel state. Two contiguous terrain passes (color, then occupancy) still produce
about 960 individual mask draws instead of one occupancy batch.

Steady-state debug profiling at 1280x720 in the local nested X11 session on an
NVIDIA RTX 2070, with other compilation/test workloads stopped, reported:

| Game | Quads | Batches | Scene preparation p95 | Presentation p95 | Recurring texture uploads |
|---|---:|---:|---:|---:|---:|
| Micro Mages | 2036 | 964 | 0.779 ms | 170.961 ms | 0 bytes |
| Jackal | 2078 | 1045 | 0.920 ms | 217.629 ms | 0 bytes |

These are local diagnostic results, not a hardware-independent benchmark. They
show that native presentation remains too slow despite cached atlases and fast
application scene preparation. Reproduce without original setup/cinematics:

```sh
cargo run --locked --example profile_tiled_native -- micromages
cargo run --locked --example profile_tiled_native -- jackal
```

The one-line batching fix removes that guard, permitting merging when the
existing comparisons already establish identical texture/palette/filter/pixel
state. Quasar v0.2.18 includes the fix and GPU conformance tests.

For identical compare/reference/replace rules, passing writes preserve the read
comparison and write the same reference bits; failing comparisons write nothing.
Thus identical mask operations are idempotent and their comparisons remain
invariant. The renderer already relies on ordered primitives for ordinary alpha
blending. Priority sprite color/claim steps have different pixel states and
remain separate; the change does not merge across those steps.

Validation should compare CPU/GPU pixels for overlapping and disjoint mask
writers, transparent/excluded indices, clipping, read/write mask overlap and
priority helpers, and measure batch reduction and native frame timing. Classic
Classics' existing campaign/camera/live-edit and 64-OAM-rotation comparisons must
continue to pass.

## Disposable-patch experiment

The proposed batching change was tested in a copy under ignored `target/`;
neither the real Quasar repository nor Classic Classics' Cargo dependency pin
was changed. Separate steady-state native runs, with compilation/tests stopped,
reported:

| Game | Patched batches | Scene p95 | Presentation p95 | Intervals over 25 ms |
|---|---:|---:|---:|---:|
| Micro Mages | 5 | 0.698 ms | 18.216 ms | 0 / 80 |
| Jackal | 28 | 0.804 ms | 19.357 ms | 0 / 80 |

Both had zero recurring texture uploads and zero dynamic framebuffer textures.
The owned-GPU-target presentation adapter remained enabled for the separate
native-frame-target issue. These results validated the batching diagnosis before the fix was released.

The disposable patched render crate also passed all 24 of its library tests,
including the ignored native GPU conformance tests for indexed palettes, masks,
priority, dynamic palettes and reverse prepared-frame submission.

With explicit one-time authorization, the batching fix was subsequently applied
to the local Quasar repository, together with a regression test for overlapping
mask writers and batch count. All 25 renderer library tests, including native
GPU conformance, passed there; formatting and renderer Clippy also passed. Those
changes were later published upstream in v0.2.18, with additional ordered-color
and transparent-coverage regression tests. Classic Classics now pins that release
without local overrides. Native verification confirms the batching improvement;
see the integration report linked above. The application-side owned-target
workaround remains necessary for the separate masked native presentation issue.
