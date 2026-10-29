//! Window composition using Quasar's frame/render APIs. In v0.2.17–v0.2.18 the
//! convenience Renderer2D cannot attach its private stencil to a semantic
//! swapchain FrameTarget. Masked canvases therefore use an owned color target
//! followed by GAL's GPU texture-to-frame copy (no readback/CPU framebuffer).
use quasar_frame::{CompletionPolicy, FrameSurface, OffscreenTarget};
use quasar_render_2d::{
    Canvas, DisplayOptions, DynamicTexture, LoadOp, PresentOutcome, RenderState2D, RenderStats,
    Textures,
};
use quasar_window::winit::window::Window;
use std::sync::Arc;
use vulkanic_gal::{resources::*, *};

struct ColorTarget {
    color: Handle,
    view: Handle,
    target: Handle,
    size: Extent3d,
    format: TextureFormat,
    initialized: bool,
}
impl ColorTarget {
    fn new(gal: &mut VulkanicGal, size: Extent3d, format: TextureFormat) -> GalResult<Self> {
        let color = gal.create_texture(TextureDesc {
            label: "arcade-masked-color".into(),
            dimension: TextureDimension::D2,
            format,
            extent: size,
            mip_levels: 1,
            array_layers: 1,
            usages: vec![TextureUsage::ColorAttachment, TextureUsage::TransferSrc],
        })?;
        let view = gal.create_texture_view(TextureViewDesc {
            label: "arcade-masked-view".into(),
            texture: color,
            format,
            base_mip: 0,
            mip_count: 1,
            base_layer: 0,
            layer_count: 1,
        })?;
        let target = gal.create_render_target(RenderTargetDesc {
            label: "arcade-masked-target".into(),
            color_views: vec![view],
            depth_stencil_view: None,
            extent: size,
        })?;
        Ok(Self {
            color,
            view,
            target,
            size,
            format,
            initialized: false,
        })
    }
    fn release(self, gal: &mut VulkanicGal) -> GalResult<()> {
        for handle in [self.target, self.view, self.color] {
            gal.retire(handle)?;
        }
        Ok(())
    }
}
fn barrier(resource: Handle, before: TextureUsageState, after: TextureUsageState) -> CommandOp {
    CommandOp::Barrier(ResourceBarrier {
        resource,
        subresources: None,
        before,
        after,
        src_queue: QueueClass::Graphics,
        dst_queue: QueueClass::Graphics,
    })
}
pub struct NativeRenderer {
    gal: VulkanicGal,
    state: RenderState2D,
    frame: FrameSurface,
    options: DisplayOptions,
    color: Option<ColorTarget>,
}
impl NativeRenderer {
    pub fn for_window(
        window: Arc<Window>,
        size: (u32, u32),
        options: DisplayOptions,
    ) -> GalResult<Self> {
        options.validate().map_err(GalError::invalid_argument)?;
        let mut gal = VulkanicGal::create_windowed_with(
            GalOptions::default(),
            "Classic Classics",
            window,
            FrameSurfaceDesc {
                label: "arcade-surface".into(),
                extent: Extent3d {
                    width: size.0,
                    height: size.1,
                    depth: 1,
                },
                color_format: TextureFormat::Bgra8Unorm,
                present_mode: PresentMode::AutoVsync,
                max_frames_in_flight: 3,
            },
        )?;
        let state = RenderState2D::new(&mut gal)?;
        Ok(Self {
            gal,
            state,
            frame: FrameSurface::new(size.0, size.1),
            options,
            color: None,
        })
    }
    pub fn options(&self) -> DisplayOptions {
        self.options
    }
    pub fn set_options(&mut self, options: DisplayOptions) -> GalResult<()> {
        options.validate().map_err(GalError::invalid_argument)?;
        self.options = options;
        Ok(())
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.frame.resize(width, height);
    }
    pub fn stats(&self) -> RenderStats {
        self.state.stats()
    }
    pub fn dynamic_texture_count(&self) -> usize {
        self.state.dynamic_texture_count()
    }
    pub fn create_dynamic_texture(
        &mut self,
        textures: &mut Textures,
        width: u32,
        height: u32,
    ) -> GalResult<DynamicTexture> {
        self.state
            .create_dynamic_texture(&mut self.gal, textures, width, height)
    }
    pub fn update_dynamic_texture(
        &mut self,
        texture: DynamicTexture,
        rgba: &[u8],
    ) -> GalResult<()> {
        self.state.update_dynamic_texture(texture, rgba)
    }
    pub fn destroy_dynamic_texture(
        &mut self,
        textures: &mut Textures,
        texture: DynamicTexture,
    ) -> GalResult<()> {
        self.state
            .destroy_dynamic_texture(&mut self.gal, textures, texture)
    }
    pub fn present(&mut self, canvas: &Canvas, textures: &Textures) -> GalResult<PresentOutcome> {
        if self.frame.needs_resize() {
            self.state.release_targets(&mut self.gal)?;
            if let Some(color) = self.color.take() {
                color.release(&mut self.gal)?;
            }
        }
        let Self {
            gal,
            state,
            frame,
            options,
            color,
        } = self;
        let result = frame.render(gal, CompletionPolicy::Wait, |gal, target| {
            if !canvas.uses_pixel_masks() {
                return Ok(vec![
                    state
                        .prepare(gal, canvas, textures, target, *options, LoadOp::Clear)?
                        .commands,
                ]);
            }
            let size = gal.pass_target_extent(target)?;
            let format = gal.pass_target_color_format(target)?;
            if color
                .as_ref()
                .is_none_or(|old| old.size != size || old.format != format)
            {
                state.release_targets(gal)?;
                if let Some(old) = color.take() {
                    old.release(gal)?;
                }
                *color = Some(ColorTarget::new(gal, size, format)?);
            }
            let owned = color.as_mut().unwrap();
            let prepared =
                state.prepare(gal, canvas, textures, owned.target, *options, LoadOp::Clear)?;
            let mut commands = prepared.commands;
            commands.operations.insert(
                0,
                barrier(
                    owned.color,
                    if owned.initialized {
                        TextureUsageState::TransferSrc
                    } else {
                        TextureUsageState::Undefined
                    },
                    TextureUsageState::ColorAttachment,
                ),
            );
            commands.operations.extend([
                barrier(
                    owned.color,
                    TextureUsageState::ColorAttachment,
                    TextureUsageState::TransferSrc,
                ),
                CommandOp::CopyTextureToFrameTarget {
                    src: owned.color,
                    dst: target,
                    extent: size,
                },
            ]);
            owned.initialized = true;
            Ok(vec![commands])
        });
        match result {
            Ok(Some(_)) => Ok(
                if matches!(
                    frame.last_present_status(),
                    Some(FramePresentStatus::Presented | FramePresentStatus::Suboptimal)
                ) {
                    PresentOutcome::Presented
                } else {
                    PresentOutcome::Deferred
                },
            ),
            Ok(None) => {
                state.release_targets(gal)?;
                Ok(PresentOutcome::Deferred)
            }
            Err(error) => {
                let _ = state.release_targets(gal);
                Err(error)
            }
        }
    }
    pub fn render_to_image(&mut self, canvas: &Canvas, textures: &Textures) -> GalResult<Vec<u8>> {
        let mut target = OffscreenTarget::new(&mut self.gal, canvas.width(), canvas.height())?;
        let result = target.render(&mut self.gal, |gal, target| {
            Ok(vec![
                self.state
                    .prepare(
                        gal,
                        canvas,
                        textures,
                        target,
                        DisplayOptions::default(),
                        LoadOp::Clear,
                    )?
                    .commands,
            ])
        });
        self.state.release_targets(&mut self.gal)?;
        target.release(&mut self.gal)?;
        result.map(|report| report.rgba)
    }
}
impl Drop for NativeRenderer {
    fn drop(&mut self) {
        let _ = self.frame.finish(&mut self.gal);
        let _ = self.state.release(&mut self.gal);
        if let Some(color) = self.color.take() {
            let _ = color.release(&mut self.gal);
        }
        let _ = self.frame.release_targets(&mut self.gal);
        let _ = self.gal.shutdown_frame_surface();
    }
}
