//! Application post-processing, built on Quasar's public frame and 2D APIs.
use quasar_core::{DisplayOptions, Filter, Viewport};
use quasar_frame::{
    CompletionPolicy, FrameSurface, Gpu, GraphicsResult, OffscreenTarget, RenderTarget,
};
use quasar_render_2d::{
    Canvas, DynamicTexture, LoadOp, PresentOutcome, RenderState2D, RenderStats, Textures,
};
use std::sync::Arc;
use vulkanic_gal as gpu;

pub struct CrtRenderer {
    gpu: Gpu,
    surface: FrameSurface,
    state: RenderState2D,
    options: DisplayOptions,
    post: Option<PostProcess>,
}

struct PostProcess {
    _texture: gpu::Texture,
    format: gpu::TextureFormat,
    target: RenderTarget,
    group: gpu::BindGroup,
    pipeline: gpu::RenderPipeline,
    filter: Filter,
}

impl PostProcess {
    fn new(
        gpu: &Gpu,
        target: &RenderTarget,
        source_size: (u32, u32),
        filter: Filter,
    ) -> GraphicsResult<Self> {
        let texture = quasar_frame::texture_2d(
            &gpu.device,
            "CRT source",
            source_size.0,
            source_size.1,
            gpu::TextureFormat::Rgba8Unorm,
            gpu::TextureUsages::RENDER_ATTACHMENT | gpu::TextureUsages::TEXTURE_BINDING,
        )?;
        let source = RenderTarget::from_texture(&texture);
        let (group, pipeline) = quasar_frame::validation_scope(gpu, || {
            let layout = gpu
                .device
                .create_bind_group_layout(&gpu::BindGroupLayoutDescriptor {
                    label: Some("CRT source layout"),
                    entries: &[
                        gpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: gpu::ShaderStages::FRAGMENT,
                            ty: gpu::BindingType::Texture {
                                sample_type: gpu::TextureSampleType::Float { filterable: true },
                                view_dimension: gpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        gpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: gpu::ShaderStages::FRAGMENT,
                            ty: gpu::BindingType::Sampler(gpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });
            let sampler = gpu.device.create_sampler(&gpu::SamplerDescriptor {
                label: Some("CRT sampler"),
                mag_filter: if filter == Filter::Linear {
                    gpu::FilterMode::Linear
                } else {
                    gpu::FilterMode::Nearest
                },
                min_filter: if filter == Filter::Linear {
                    gpu::FilterMode::Linear
                } else {
                    gpu::FilterMode::Nearest
                },
                ..Default::default()
            });
            let group = gpu.device.create_bind_group(&gpu::BindGroupDescriptor {
                label: Some("CRT source"),
                layout: &layout,
                entries: &[
                    gpu::BindGroupEntry {
                        binding: 0,
                        resource: gpu::BindingResource::TextureView(&source.color),
                    },
                    gpu::BindGroupEntry {
                        binding: 1,
                        resource: gpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });
            let shader = gpu
                .device
                .create_shader_module(gpu::ShaderModuleDescriptor {
                    label: Some("CRT shader"),
                    source: gpu::ShaderSource::Wgsl(include_str!("crt.wgsl").into()),
                });
            let pipeline_layout =
                gpu.device
                    .create_pipeline_layout(&gpu::PipelineLayoutDescriptor {
                        label: Some("CRT pipeline layout"),
                        bind_group_layouts: &[Some(&layout)],
                        immediate_size: 0,
                    });
            let pipeline = gpu
                .device
                .create_render_pipeline(&gpu::RenderPipelineDescriptor {
                    label: Some("CRT pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: gpu::VertexState {
                        module: &shader,
                        entry_point: Some("vertex"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    fragment: Some(gpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fragment"),
                        compilation_options: Default::default(),
                        targets: &[Some(gpu::ColorTargetState {
                            format: target.format,
                            blend: None,
                            write_mask: gpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                });
            (group, pipeline)
        })?;
        Ok(Self {
            _texture: texture,
            format: target.format,
            target: source,
            group,
            pipeline,
            filter,
        })
    }

    fn commands(&self, gpu: &Gpu, target: &RenderTarget, viewport: Viewport) -> gpu::CommandBuffer {
        let mut encoder = gpu.encoder("CRT pass");
        {
            let attachments = [Some(gpu::RenderPassColorAttachment {
                view: &target.color,
                depth_slice: None,
                resolve_target: None,
                ops: gpu::Operations {
                    load: gpu::LoadOp::Clear(gpu::Color::BLACK),
                    store: gpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&gpu::RenderPassDescriptor {
                label: Some("CRT pass"),
                color_attachments: &attachments,
                ..Default::default()
            });
            pass.set_viewport(
                viewport.x as f32,
                viewport.y as f32,
                viewport.width as f32,
                viewport.height as f32,
                0.0,
                1.0,
            );
            pass.set_scissor_rect(viewport.x, viewport.y, viewport.width, viewport.height);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.finish()
    }
}

impl CrtRenderer {
    pub fn for_window<W>(
        window: Arc<W>,
        size: (u32, u32),
        options: DisplayOptions,
    ) -> GraphicsResult<Self>
    where
        W: gpu::DisplayAndWindowHandle + std::fmt::Debug + Send + Sync + 'static,
    {
        let (gpu, surface) = FrameSurface::for_window(window, size.0, size.1)?;
        let state = RenderState2D::new(&gpu)?;
        options.validate().map_err(quasar_frame::error)?;
        Ok(Self {
            gpu,
            surface,
            state,
            options,
            post: None,
        })
    }
    pub fn options(&self) -> DisplayOptions {
        self.options
    }
    pub fn set_options(&mut self, options: DisplayOptions) -> GraphicsResult<()> {
        options.validate().map_err(quasar_frame::error)?;
        self.options = options;
        Ok(())
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface.resize(width, height);
    }
    pub fn stats(&self) -> RenderStats {
        self.state.stats()
    }
    pub fn dynamic_texture_count(&self) -> usize {
        self.state.dynamic_texture_count()
    }
    pub fn create_dynamic_texture(
        &mut self,
        assets: &mut Textures,
        width: u32,
        height: u32,
    ) -> GraphicsResult<DynamicTexture> {
        self.state
            .create_dynamic_texture(&self.gpu, assets, width, height)
    }
    pub fn update_dynamic_texture(
        &mut self,
        texture: DynamicTexture,
        pixels: &[u8],
    ) -> GraphicsResult<()> {
        self.state.update_dynamic_texture(texture, pixels)
    }
    pub fn destroy_dynamic_texture(
        &mut self,
        assets: &mut Textures,
        texture: DynamicTexture,
    ) -> GraphicsResult<()> {
        self.state
            .destroy_dynamic_texture(&self.gpu, assets, texture)
    }
    pub fn present(
        &mut self,
        canvas: &Canvas,
        textures: &Textures,
        crt: bool,
    ) -> GraphicsResult<PresentOutcome> {
        let Some(frame) = self.surface.acquire(&self.gpu)? else {
            return Ok(PresentOutcome::Deferred);
        };
        let commands = if crt {
            if self.post.as_ref().is_none_or(|p| {
                p.target.size() != (canvas.width(), canvas.height())
                    || p.format != frame.target.format
                    || p.filter != self.options.filter
            }) {
                self.post = Some(PostProcess::new(
                    &self.gpu,
                    &frame.target,
                    (canvas.width(), canvas.height()),
                    self.options.filter,
                )?);
            }
            let post = self.post.as_ref().expect("CRT initialized");
            let draw = self.state.prepare(
                &self.gpu,
                canvas,
                textures,
                &post.target,
                self.options,
                LoadOp::Clear,
            )?;
            let viewport = Viewport::calculate(
                (canvas.width(), canvas.height()),
                frame.target.size(),
                self.options,
            )
            .ok_or_else(|| quasar_frame::error("invalid CRT viewport"))?;
            vec![
                draw.commands,
                post.commands(&self.gpu, &frame.target, viewport),
            ]
        } else {
            vec![
                self.state
                    .prepare(
                        &self.gpu,
                        canvas,
                        textures,
                        &frame.target,
                        self.options,
                        LoadOp::Clear,
                    )?
                    .commands,
            ]
        };
        self.surface
            .present(&self.gpu, frame, commands, CompletionPolicy::Wait)?;
        Ok(PresentOutcome::Presented)
    }
    pub fn render_to_image(
        &mut self,
        canvas: &Canvas,
        textures: &Textures,
        crt: bool,
    ) -> GraphicsResult<Vec<u8>> {
        let target = OffscreenTarget::new(&self.gpu, canvas.width(), canvas.height())?;
        let post = if crt {
            Some(PostProcess::new(
                &self.gpu,
                &target.target,
                (canvas.width(), canvas.height()),
                self.options.filter,
            )?)
        } else {
            None
        };
        let result = target.render(&self.gpu, |gpu, target| {
            if let Some(post) = &post {
                let draw = self.state.prepare(
                    gpu,
                    canvas,
                    textures,
                    &post.target,
                    self.options,
                    LoadOp::Clear,
                )?;
                let viewport = Viewport::calculate(
                    (canvas.width(), canvas.height()),
                    target.size(),
                    self.options,
                )
                .ok_or_else(|| quasar_frame::error("invalid CRT viewport"))?;
                Ok(vec![draw.commands, post.commands(gpu, target, viewport)])
            } else {
                Ok(vec![
                    self.state
                        .prepare(gpu, canvas, textures, target, self.options, LoadOp::Clear)?
                        .commands,
                ])
            }
        })?;
        Ok(result.rgba)
    }
}
