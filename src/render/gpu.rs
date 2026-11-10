//! wgpu setup, textures, and the instanced sprite pass.

use crate::render::traffic::{Counted, NewBuffer, Upload};
use std::sync::Arc;

use glam::Mat4;
use winit::window::Window;

use super::batch::{self, DrawList, PanelDraw, SpriteInstance};
use super::timer::GpuTimer;
use super::{FrameStatus, RenderStats, Screenshot};
use crate::color::Color;

pub(super) struct GpuTexture {
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) texture: wgpu::Texture,
    /// Sampled with hard edges (pixel art).
    pub(super) nearest: bool,
    /// Whether it has mipmaps (made once a 3D material uses it), and
    /// whether they average data (normals, ORM) rather than colors.
    #[cfg(feature = "3d")]
    pub(super) mips: Option<bool>,
}

pub(crate) struct Renderer {
    window: Arc<Window>,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// The format we draw in: the surface's, or its sRGB view where the
    /// surface itself can't be sRGB (browser canvases).
    view_format: wgpu::TextureFormat,
    /// The extra view formats image textures declare: their linear
    /// (non-sRGB) twin, or none where the device can't reinterpret texture
    /// formats (WebGL2), so creating them doesn't fail validation there.
    image_view_formats: &'static [wgpu::TextureFormat],

    pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    /// Window-pixel projection for screen-space drawing (UI, text).
    ui_camera_buffer: wgpu::Buffer,
    ui_camera_bind_group: wgpu::BindGroup,
    /// For world-space UI panels: the sprite shader, its layout and the
    /// cameras' layout, a pipeline drawing into render targets (made on
    /// first use), and one camera per panel.
    sprite_shader: wgpu::ShaderModule,
    sprite_layout: wgpu::PipelineLayout,
    camera_layout: wgpu::BindGroupLayout,
    panel_pipeline: Option<wgpu::RenderPipeline>,
    panel_cameras: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    nearest_sampler: wgpu::Sampler,
    textures: Vec<GpuTexture>,
    /// A 1x1 white texture freed textures' slots show (made on first use).
    blank: Option<wgpu::Texture>,
    /// Ids of freed textures (their slots show `blank`).
    freed_textures: std::collections::HashSet<u32>,

    instance_buffer: wgpu::Buffer,
    /// Capacity of `instance_buffer`, in sprites.
    instance_capacity: usize,

    timer: Option<GpuTimer>,
    /// What this GPU can do and the fallbacks used (fixed once opened).
    capabilities: super::GpuCapabilities,
    /// Draw frames the window can't take into `hidden_target` instead of
    /// skipping them (benchmarks).
    draw_hidden: bool,
    /// Draw every frame into `hidden_target`, never to the window: the
    /// display's refresh rate then neither caps nor stalls a benchmark.
    only_hidden: bool,
    hidden_target: Option<wgpu::Texture>,
    /// Offscreen frames still on the GPU, oldest first: no more than a
    /// window's swapchain would allow, so the CPU can't run ahead.
    hidden_in_flight: std::collections::VecDeque<wgpu::SubmissionIndex>,
    stats: RenderStats,
    /// The 3D pass, made when the game first creates a mesh or material.
    #[cfg(feature = "3d")]
    three: Option<super::gpu3d::Renderer3D>,
    /// Samples per pixel of the 3D frame.
    #[cfg(feature = "3d")]
    msaa: u32,
    /// The GPU draws in tiles (see `Renderer3D::solid_prepass`).
    #[cfg(feature = "3d")]
    tiled: bool,
    #[cfg(feature = "3d")]
    mips: Option<super::mips::MipMaker>,
    /// Time spent collecting and uploading this frame's 3D models.
    #[cfg(feature = "3d")]
    prepare_3d_ms: f32,
    /// Of `prepare_3d_ms`, the time collecting models.
    #[cfg(feature = "3d")]
    collect_ms: f32,
    /// The last drawn frame's `prepare_3d_ms`, for `detail`.
    #[cfg(feature = "3d")]
    last_prepare_3d_ms: f32,
    /// How long the last frame waited for the window's next image.
    display_wait_ms: f32,
    /// The last captured frame, waiting to be picked up.
    screenshot: Option<Screenshot>,

    /// The inspector's drawing for the next frame, and egui's renderer
    /// (made the first time the inspector opens).
    #[cfg(feature = "inspector")]
    overlay: Option<crate::inspector::Overlay>,
    #[cfg(feature = "inspector")]
    egui: Option<egui_wgpu::Renderer>,
    /// egui textures to free once the frame that last used them is submitted.
    #[cfg(feature = "inspector")]
    egui_free: Vec<egui::TextureId>,
}

/// The sprite pipeline, drawing into `format`.
fn sprite_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let instance_attributes = wgpu::vertex_attr_array![
        0 => Float32x2, // pos
        1 => Float32x2, // size
        2 => Float16x2, // pivot
        3 => Float32,   // rotation
        4 => Unorm8x4,  // color
        5 => Unorm16x4, // uv: u0, v0, width, height
    ];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sprite pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<SpriteInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &instance_attributes,
            })],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// What one sprite pass draws with.
struct SpritePass<'a> {
    viewport: [u32; 2],
    scale: f32,
    pipeline: &'a wgpu::RenderPipeline,
    instances: &'a wgpu::Buffer,
    textures: &'a [GpuTexture],
    /// Camera, batches, and where the layer's instances start in `instances`.
    layers: &'a [(&'a wgpu::BindGroup, &'a [batch::Batch], u32)],
}

/// An offscreen copy of one frame on its way back to the CPU.
struct Capture {
    texture: wgpu::Texture,
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_row: u32,
    bgra: bool,
}

impl Capture {
    fn new(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screenshot"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let padded_row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = device.new_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot readback"),
            size: u64::from(padded_row) * u64::from(height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgra = matches!(
            format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        Self {
            texture,
            buffer,
            width,
            height,
            padded_row,
            bgra,
        }
    }

    fn copy(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Waits for the GPU and returns the pixels.
    fn read(self, device: &wgpu::Device) -> Screenshot {
        self.buffer.map_async(wgpu::MapMode::Read, .., |_| {});
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let mut rgba = Vec::with_capacity((self.width * self.height * 4) as usize);
        if let Ok(view) = self.buffer.get_mapped_range(..) {
            for row in view.chunks_exact(self.padded_row as usize) {
                rgba.extend_from_slice(&row[..(self.width * 4) as usize]);
            }
        }
        if self.bgra {
            for px in rgba.as_chunks_mut::<4>().0 {
                px.swap(0, 2);
            }
        }
        Screenshot {
            width: self.width,
            height: self.height,
            rgba,
        }
    }
}

const INITIAL_INSTANCE_CAPACITY: usize = 1024;

impl Renderer {
    /// What drawing the last frame cost.
    pub fn stats(&self) -> RenderStats {
        self.stats
    }

    /// The pipeline cache's counters; all zero before any 3D.
    pub fn pipeline_stats(&self) -> super::PipelineStats {
        #[cfg(feature = "3d")]
        if let Some(three) = &self.three {
            return three.pipeline_stats();
        }
        super::PipelineStats::default()
    }

    /// See `Renderer3D::mesh_costs`; empty before any 3D.
    #[cfg(feature = "3d")]
    pub fn mesh_costs(&self) -> Vec<(u32, usize, usize, usize, usize)> {
        self.three.as_ref().map(|t| t.mesh_costs()).unwrap_or_default()
    }

    /// Draw frames that the window can't take offscreen instead of skipping
    /// them, so benchmarks measure every frame; with `always`, draw every
    /// frame offscreen.
    pub fn set_draw_hidden(&mut self, on: bool, always: bool) {
        self.draw_hidden = on || always;
        self.only_hidden = always;
    }

    /// A view of the offscreen stand-in for the window, made at its size.
    fn hidden_view(&mut self) -> wgpu::TextureView {
        let (width, height) = (self.config.width, self.config.height);
        let fits = self
            .hidden_target
            .as_ref()
            .is_some_and(|t| t.width() == width && t.height() == height);
        if !fits {
            self.hidden_target = Some(self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("offscreen frame"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.view_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }));
        }
        self.hidden_target
            .as_ref()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
            .expect("made above")
    }

    /// The GPU time of each pass, smoothed (empty without timestamp queries).
    pub fn gpu_passes(&self) -> &[crate::profiler::GpuPass] {
        self.timer.as_ref().map_or(&[], |t| &t.passes)
    }

    /// What the render-pass inspector needs beyond [`RenderStats`].
    pub fn detail(&self) -> super::RenderDetail {
        super::RenderDetail {
            gpu_timestamps: self.timer.is_some(),
            gpu_frames_timed: self.timer.as_ref().map_or(0, |t| t.frames_read),
            #[cfg(feature = "3d")]
            prepare_ms: self.last_prepare_3d_ms,
            #[cfg(not(feature = "3d"))]
            prepare_ms: 0.0,
        }
    }

    /// What this GPU can do, and what the renderer does where it can't.
    pub fn capabilities(&self) -> &super::GpuCapabilities {
        &self.capabilities
    }

    /// The latest GPU frame's passes and how many frames the timer has read.
    pub fn gpu_frame(&self) -> Option<(&[crate::profiler::PassSpan], u64)> {
        self.timer.as_ref().map(|t| (&t.last_frame[..], t.frames_read))
    }

    /// The last captured frame, if one is waiting.
    pub fn take_screenshot(&mut self) -> Option<Screenshot> {
        self.screenshot.take()
    }

    /// Opens the GPU and sets up drawing to `window`, waiting for the GPU
    /// (desktop; the web build calls [`Self::new_async`]).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(window: Arc<Window>, display: winit::event_loop::OwnedDisplayHandle, vsync: bool, msaa: u32) -> Self {
        pollster::block_on(Self::new_async(window, display, vsync, msaa))
    }

    pub async fn new_async(
        window: Arc<Window>,
        display: winit::event_loop::OwnedDisplayHandle,
        vsync: bool,
        msaa: u32,
    ) -> Self {
        let descriptor = wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(display));
        // In a browser, use WebGPU where it works and WebGL2 elsewhere.
        #[cfg(target_arch = "wasm32")]
        let instance = wgpu::util::new_instance_with_webgpu_detection(descriptor).await;
        #[cfg(not(target_arch = "wasm32"))]
        let instance = wgpu::Instance::new(descriptor);
        let surface = instance
            .create_surface(window.clone())
            .unwrap_or_else(|e| panic!("ponko: could not create a GPU surface: {e}"));

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .unwrap_or_else(|e| panic!("ponko: no compatible GPU found: {e}"));

        // `PONKO_GPU_TIER` opens the device as a weaker GPU would be.
        let forced_tier = super::GpuTier::forced();
        if let Some(tier) = forced_tier {
            eprintln!("ponko: PONKO_GPU_TIER forces the {tier} GPU tier");
        }
        let required_features = adapter.features()
            & (wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ponko device"),
                required_features: forced_tier.map_or(required_features, |t| t.device_features(required_features)),
                // The defaults ask for more than WebGL2 has; we need little.
                required_limits: if let Some(tier) = forced_tier {
                    // The tier's limits, but a surface as large as the window.
                    tier.window_limits(wgpu::Limits {
                        max_sampled_textures_per_shader_stage: adapter
                            .limits()
                            .max_sampled_textures_per_shader_stage
                            .clamp(16, 24),
                        ..wgpu::Limits::default()
                    })
                } else if cfg!(target_arch = "wasm32") {
                    adapter.limits()
                } else {
                    // One more sampled texture than the default 16, where
                    // the adapter has it: projected decals' images.
                    wgpu::Limits {
                        max_sampled_textures_per_shader_stage: adapter
                            .limits()
                            .max_sampled_textures_per_shader_stage
                            .clamp(16, 24),
                        ..wgpu::Limits::default()
                    }
                },
                ..Default::default()
            })
            .await
            .unwrap_or_else(|e| panic!("ponko: could not open the GPU device: {e}"));

        // The 3D frame's samples per pixel: the game's choice where this GPU
        // can multisample both 3D targets that much, else 4 (always works).
        let requested_msaa = msaa;
        #[cfg(feature = "3d")]
        let msaa = if super::gpu3d::browser_gl_has_no_multisampled_textures(adapter.get_info().backend) {
            // WebGL2 can't make multisampled textures through wgpu (glow's
            // browser binding panics): draw the 3D frame unsmoothed, and
            // leave edges to FXAA or SMAA.
            1
        } else if super::gpu3d::multisamples(&adapter, msaa) {
            msaa
        } else {
            eprintln!("ponko: this GPU can't do {msaa}x multisampling here; using 4x");
            4
        };
        let mut capabilities = super::capabilities::of_device(&adapter.get_info(), &device, (requested_msaa, msaa));
        capabilities.forced_tier = forced_tier;
        // Apple's GPUs (Metal) draw in tiles and drop hidden solid fragments
        // themselves: there solid surfaces skip the depth prepass.
        #[cfg(feature = "3d")]
        let tiled = adapter.get_info().backend == wgpu::Backend::Metal;

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("ponko: the GPU can't draw to this window");
        // Prefer an sRGB target so blending happens in linear light.
        let caps = surface.get_capabilities(&adapter);
        if let Some(srgb) = caps.formats.iter().copied().find(|f| f.is_srgb()) {
            config.format = srgb;
        }
        // Browser canvases are never sRGB themselves; draw through an sRGB view.
        let view_format = config.format.add_srgb_suffix();
        if view_format != config.format {
            config.view_formats.push(view_format);
        }
        config.present_mode = if vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        config.desired_maximum_frame_latency = 2;
        surface.configure(&device, &config);

        // --- pipeline ---
        let shader = device.create_shader_module(wgpu::include_wgsl!("sprite.wgsl"));

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite pipeline layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&texture_layout)],
            immediate_size: 0,
        });

        let pipeline = sprite_pipeline(&device, &layout, &shader, view_format);

        let camera = |label| {
            let buffer = device.new_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size_of::<Mat4>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &camera_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
            (buffer, bind_group)
        };
        let (camera_buffer, camera_bind_group) = camera("camera");
        let (ui_camera_buffer, ui_camera_bind_group) = camera("ui camera");

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pixel art sampler"),
            ..Default::default() // nearest
        });
        let instance_buffer = Self::create_instance_buffer(&device, INITIAL_INSTANCE_CAPACITY);
        let timer = GpuTimer::new(&device, &queue);
        let image_view_formats: &'static [wgpu::TextureFormat] = if adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::VIEW_FORMATS)
        {
            &[wgpu::TextureFormat::Rgba8Unorm]
        } else {
            &[]
        };

        Self {
            window,
            instance,
            surface,
            device,
            queue,
            config,
            view_format,
            image_view_formats,
            pipeline,
            camera_buffer,
            camera_bind_group,
            ui_camera_buffer,
            ui_camera_bind_group,
            sprite_shader: shader,
            sprite_layout: layout,
            camera_layout,
            panel_pipeline: None,
            panel_cameras: Vec::new(),
            texture_layout,
            sampler,
            nearest_sampler,
            textures: Vec::new(),
            blank: None,
            freed_textures: Default::default(),
            instance_buffer,
            instance_capacity: INITIAL_INSTANCE_CAPACITY,
            timer,
            capabilities,
            draw_hidden: false,
            only_hidden: false,
            hidden_target: None,
            hidden_in_flight: std::collections::VecDeque::with_capacity(4),
            stats: RenderStats::default(),
            #[cfg(feature = "3d")]
            three: None,
            #[cfg(feature = "3d")]
            msaa,
            #[cfg(feature = "3d")]
            tiled,
            #[cfg(feature = "3d")]
            mips: None,
            display_wait_ms: 0.0,
            #[cfg(feature = "3d")]
            prepare_3d_ms: 0.0,
            #[cfg(feature = "3d")]
            collect_ms: 0.0,
            #[cfg(feature = "3d")]
            last_prepare_3d_ms: 0.0,
            screenshot: None,
            #[cfg(feature = "inspector")]
            overlay: None,
            #[cfg(feature = "inspector")]
            egui: None,
            #[cfg(feature = "inspector")]
            egui_free: Vec::new(),
        }
    }

    fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.new_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite instances"),
            size: (capacity * size_of::<SpriteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// The 3D pass, made on first use (with its white texture).
    #[cfg(feature = "3d")]
    fn three(&mut self) -> &mut super::gpu3d::Renderer3D {
        if self.three.is_none() {
            let white = self.create_texture(1, 1, &[255; 4]);
            let white_array = self.upload_texture_array(1, 1, &[&[255; 4]], false);
            let renderer = super::gpu3d::Renderer3D::new(
                &self.device,
                &self.texture_layout,
                self.view_format,
                white,
                &white_array,
                self.msaa,
                !self.tiled,
            );
            // No view formats (WebGL2): sprite textures can't be read through a linear view.
            self.three = Some(if self.image_view_formats.is_empty() {
                renderer.without_linear_views()
            } else {
                renderer
            });
        }
        self.three.as_mut().expect("just made")
    }

    #[cfg(feature = "3d")]
    pub fn create_material(&mut self, desc: &crate::three::GpuMaterialDesc) -> u32 {
        self.material_mips(desc);
        let device = self.device.clone();
        self.three();
        let three = self.three.as_mut().expect("just made");
        three.create_material(&device, desc, &self.textures)
    }

    #[cfg(feature = "3d")]
    pub fn update_material(&mut self, id: u32, desc: &crate::three::GpuMaterialDesc) {
        self.material_mips(desc);
        if let Some(three) = &mut self.three {
            three.update_material(&self.device, &self.queue, id, desc, &self.textures);
        }
    }

    /// Builds the 3D pipeline variants frames would build when first drawn
    /// (see `Renderer3D::warm_pipelines`).
    #[cfg(feature = "3d")]
    pub fn warm_pipelines(&mut self) {
        self.three().warm_pipelines();
    }

    /// Whether projected decals can be drawn (see `gpu3d::supports_decals`).
    #[cfg(feature = "3d")]
    pub fn supports_decals(&self) -> bool {
        super::gpu3d::supports_decals(&self.device)
    }

    /// Whether global illumination can be drawn (see `gpu3d::supports_gi`).
    #[cfg(feature = "3d")]
    pub fn supports_gi(&self) -> bool {
        super::gpu3d::supports_gi(&self.device)
    }

    /// The largest texture array this GPU takes: width and height, and layers.
    #[cfg(feature = "3d")]
    pub fn max_texture_array_size(&self) -> (u32, u32) {
        let limits = self.device.limits();
        (limits.max_texture_dimension_2d, limits.max_texture_array_layers)
    }

    /// Uploads a texture array (RGBA8, one image per layer, sRGB unless
    /// `linear`) with mipmaps; returns its id among the 3D arrays.
    #[cfg(feature = "3d")]
    pub fn create_texture_array(&mut self, width: u32, height: u32, layers: &[&[u8]], linear: bool) -> u32 {
        let texture = self.upload_texture_array(width, height, layers, linear);
        self.three().add_texture_array(&texture)
    }

    #[cfg(feature = "3d")]
    fn upload_texture_array(&mut self, width: u32, height: u32, layers: &[&[u8]], linear: bool) -> wgpu::Texture {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: layers.len() as u32,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("3d texture array"),
            size,
            mip_level_count: super::mips::levels(width, height),
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if linear {
                wgpu::TextureFormat::Rgba8Unorm
            } else {
                wgpu::TextureFormat::Rgba8UnormSrgb
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        for (i, rgba) in layers.iter().enumerate() {
            self.queue.uploading(Upload::Textures).write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    depth_or_array_layers: 1,
                    ..size
                },
            );
        }
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mips = self
            .mips
            .get_or_insert_with(|| super::mips::MipMaker::new(&self.device));
        mips.generate(&self.device, &mut encoder, &texture, linear);
        self.queue.submit([encoder.finish()]);
        texture
    }

    /// Mipmaps for the textures a material reads.
    #[cfg(feature = "3d")]
    fn material_mips(&mut self, desc: &crate::three::GpuMaterialDesc) {
        // Normal and ORM maps (the 2nd and 3rd) hold data, not colors.
        let linear = [false, true, true, false, false, false];
        for (map, linear) in desc.textures.iter().zip(linear) {
            if let Some(id) = *map {
                self.ensure_mips(id, linear);
            }
        }
    }

    #[cfg(feature = "3d")]
    pub fn create_gpu_buffer(&mut self, id: u32, len: u32, data: &[crate::Vec4]) {
        let device = self.device.clone();
        self.three().create_gpu_buffer(&device, id, len, data);
    }
    #[cfg(feature = "3d")]
    pub fn free_gpu_buffer(&mut self, id: u32) {
        self.three().free_gpu_buffer(id);
    }
    #[cfg(feature = "3d")]
    pub fn write_gpu_buffer(&mut self, id: u32, offset: u32, data: &[crate::Vec4]) {
        let queue = self.queue.clone();
        self.three().write_gpu_buffer(&queue, id, offset, data);
    }
    #[cfg(feature = "3d")]
    pub fn set_instance_batch(&mut self, id: u32, batch: Option<&crate::three::instances::Batch>) {
        let device = self.device.clone();
        self.three().set_instance_batch(&device, id, batch);
    }
    #[cfg(feature = "3d")]
    pub fn set_mesh_gpu_buffer(&mut self, mesh: u32, b: u32, count: u32) {
        self.three().set_mesh_gpu_buffer(mesh, b, count);
    }
    /// Compiles a compute pass; its id, or the compiler's message.
    #[cfg(feature = "3d")]
    pub fn create_compute(&mut self, name: &str, source: &str) -> Result<u32, Box<crate::ShaderDiagnostic>> {
        let device = self.device.clone();
        self.three().create_compute(&device, name, source)
    }

    /// Recompiles compute pass `id` in place (hot reload).
    #[cfg(feature = "3d")]
    pub fn replace_compute(&mut self, id: u32, name: &str, source: &str) -> Result<(), Box<crate::ShaderDiagnostic>> {
        let device = self.device.clone();
        self.three().replace_compute(&device, id, name, source)
    }

    /// Compiles a screen effect; its id, or the compiler's message.
    #[cfg(feature = "3d")]
    pub fn create_effect(&mut self, name: &str, source: &str) -> Result<u32, Box<crate::ShaderDiagnostic>> {
        let device = self.device.clone();
        self.three().create_effect(&device, name, source)
    }

    /// Recompiles screen effect `id` (hot reload).
    #[cfg(feature = "3d")]
    pub fn replace_effect(&mut self, id: u32, name: &str, source: &str) -> Result<(), Box<crate::ShaderDiagnostic>> {
        let device = self.device.clone();
        self.three().replace_effect(&device, id, name, source)
    }

    /// Shaders whose pipelines the GPU rejected after they were put in use
    /// (a browser's WebGPU answers late): the previous ones are back.
    #[cfg(feature = "3d")]
    pub fn poll_shader_checks(&mut self, out: &mut Vec<super::gpu3d::LateShaderError>) {
        let device = self.device.clone();
        if let Some(three) = self.three.as_mut() {
            three.poll_shader_checks(&device, out);
        }
    }

    /// Compiles a material shader; its index, or the compiler's message.
    #[cfg(feature = "3d")]
    pub fn create_shader(&mut self, name: &str, source: &str) -> Result<u32, Box<crate::ShaderDiagnostic>> {
        let device = self.device.clone();
        self.three().create_shader(&device, name, source)
    }

    #[cfg(feature = "3d")]
    pub fn replace_shader(&mut self, id: u32, name: &str, source: &str) -> Result<(), Box<crate::ShaderDiagnostic>> {
        let device = self.device.clone();
        self.three().replace_shader(&device, id, name, source)
    }

    #[cfg(feature = "3d")]
    /// Makes 3D mesh `id`: a freed mesh's id, or the next new one.
    #[cfg(feature = "3d")]
    pub fn create_mesh(
        &mut self,
        id: u32,
        data: &crate::MeshData,
        parts: &[(std::ops::Range<u32>, crate::Material)],
        skinned: bool,
    ) {
        let device = self.device.clone();
        self.three().create_mesh_at(&device, id, data, parts, skinned);
    }

    /// Frees 3D mesh `id`'s GPU memory.
    #[cfg(feature = "3d")]
    pub fn free_mesh(&mut self, id: u32) {
        self.three().free_mesh(id);
    }

    /// Uploads the frame's 3D models, camera and light for the next `render`.
    /// `collect_ms`: how long collecting the models took, counted in `cpu_ms`.
    /// The newest depth grid for occlusion culling, if one came back.
    #[cfg(feature = "3d")]
    pub fn take_occlusion(&mut self) -> Option<crate::three::occlusion::OcclusionGrid> {
        self.three.as_mut()?.take_occlusion()
    }

    #[cfg(feature = "3d")]
    pub fn prepare_3d(&mut self, scene: &crate::three::Scene3D<'_>, collect_ms: f32) {
        let start = web_time::Instant::now();
        let size = (self.config.width, self.config.height);
        // A lookup table must be a strip of n squares of n x n.
        let lut = scene.post.lut.filter(|t| {
            let n = t.height();
            (2..=64).contains(&n)
                && t.width() == n * n
                && (t.id as usize) < self.textures.len()
                && !self.freed_textures.contains(&t.id)
        });
        if let Some(three) = &mut self.three {
            let lut = lut.map(|t| (t.id, t.height()));
            let input = super::gpu3d::Prepare3D { scene, lut, size };
            three.prepare(&self.device, &self.queue, &input);
        }
        self.prepare_3d_ms = collect_ms + start.elapsed().as_secs_f32() * 1000.0;
        self.collect_ms = collect_ms;
    }

    pub fn display_wait_ms(&self) -> f32 {
        self.display_wait_ms
    }

    /// What this renderer holds on the GPU, counted from its own tables
    /// (`Ctx::asset_inventory`).
    pub fn residency(&self) -> Option<crate::GpuResidency> {
        #[cfg_attr(not(feature = "3d"), allow(unused_mut))]
        let mut r = crate::GpuResidency {
            textures: self.textures.len(),
            texture_bytes: self.textures.iter().map(|t| texture_bytes(&t.texture)).sum(),
            ..Default::default()
        };
        #[cfg(feature = "3d")]
        if let Some(three) = &self.three {
            (r.meshes, r.mesh_bytes, r.gpu_buffers, r.gpu_buffer_bytes) = three.residency();
        }
        Some(r)
    }

    pub fn max_texture_size(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }

    /// A texture target cameras draw into (linear HDR); returns its id.
    #[cfg(feature = "3d")]
    pub fn create_render_target(&mut self, width: u32, height: u32) -> u32 {
        // Made first: making the 3D renderer adds textures of its own, and
        // the id below must be this one's.
        self.three();
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: super::gpu3d::HDR_FORMAT,
            // Drawn into by target cameras, copied into by compute passes.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let bind_group = self.sprite_bind_group(&texture, false);
        let id = self.textures.len() as u32;
        self.three().add_render_target(id, &texture);
        self.textures.push(GpuTexture {
            bind_group,
            texture,
            nearest: false,
            // Drawn fresh each frame: no mipmaps to make.
            mips: Some(false),
        });
        id
    }

    /// Uploads RGBA8 (sRGB) pixels and returns the texture's id.
    pub fn create_texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> u32 {
        let texture = self.upload_texture(width, height, rgba);
        let bind_group = self.sprite_bind_group(&texture, false);
        self.textures.push(GpuTexture {
            bind_group,
            texture,
            nearest: false,
            #[cfg(feature = "3d")]
            mips: None,
        });
        (self.textures.len() - 1) as u32
    }

    /// Replaces texture `id` with `rgba` at a new size (resolution
    /// streaming): the same id, sampling and mipmap kind, so sprites and
    /// materials showing it see the new pixels at once.
    pub fn replace_texture(&mut self, id: u32, width: u32, height: u32, rgba: &[u8]) {
        let Some(old) = self.textures.get(id as usize) else {
            return;
        };
        let nearest = old.nearest;
        #[cfg(feature = "3d")]
        let mips = old.mips;
        let texture = self.upload_texture(width, height, rgba);
        let bind_group = self.sprite_bind_group(&texture, nearest);
        self.textures[id as usize] = GpuTexture {
            bind_group,
            texture,
            nearest,
            #[cfg(feature = "3d")]
            mips: None,
        };
        #[cfg(feature = "3d")]
        {
            if let Some(linear) = mips {
                self.ensure_mips(id, linear);
            }
            if let Some(three) = &mut self.three {
                three.texture_changed(id);
                three.texture_replaced(&self.device, &self.textures, id);
            }
        }
    }

    /// Frees texture `id`: its slot shows a 1x1 white texture from now on,
    /// so sprites and materials still showing it draw white, and target
    /// cameras and compute passes let go of it if it was a render target.
    pub fn free_texture(&mut self, id: u32) {
        if id as usize >= self.textures.len() {
            return;
        }
        let blank = match &self.blank {
            Some(blank) => blank.clone(),
            None => {
                let blank = self.upload_texture(1, 1, &[255; 4]);
                self.blank = Some(blank.clone());
                blank
            }
        };
        let bind_group = self.sprite_bind_group(&blank, false);
        self.textures[id as usize] = GpuTexture {
            bind_group,
            texture: blank,
            nearest: false,
            // 1x1: no mipmaps to make.
            #[cfg(feature = "3d")]
            mips: Some(false),
        };
        self.freed_textures.insert(id);
        #[cfg(feature = "3d")]
        if let Some(three) = &mut self.three {
            three.free_render_target(id);
            three.rebind_materials(&self.device, &self.queue, &self.textures, Some(id), None);
        }
    }

    /// Material `id` was freed (see `Renderer3D::free_material`).
    #[cfg(feature = "3d")]
    pub fn free_material(&mut self, id: u32) {
        if let Some(three) = &mut self.three {
            three.free_material(&self.device, &self.queue, id, &self.textures);
        }
    }

    /// Material shader `id` was freed (see `Renderer3D::free_shader`).
    #[cfg(feature = "3d")]
    pub fn free_shader(&mut self, id: u32) {
        if let Some(three) = &mut self.three {
            three.free_shader(id);
        }
    }

    /// Compute pass `id` was freed (see `Renderer3D::free_compute`).
    #[cfg(feature = "3d")]
    pub fn free_compute(&mut self, id: u32) {
        if let Some(three) = &mut self.three {
            three.free_compute(id);
        }
    }

    /// Texture array `id` was freed (see `Renderer3D::free_texture_array`).
    #[cfg(feature = "3d")]
    pub fn free_texture_array(&mut self, id: u32) {
        if let Some(three) = &mut self.three {
            three.free_texture_array(&self.device, &self.queue, id, &self.textures);
        }
    }

    /// An RGBA8 (sRGB) texture of `rgba`, not yet in `textures`.
    fn upload_texture(&self, width: u32, height: u32, rgba: &[u8]) -> wgpu::Texture {
        debug_assert_eq!(rgba.len(), (width * height * 4) as usize);
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // Copied from when a 3D material first uses it and it gets mipmaps.
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
            // Materials read normal maps (data, not colors) without sRGB decoding.
            view_formats: self.image_view_formats,
        });
        self.queue.uploading(Upload::Textures).write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        texture
    }

    /// The bind group sprites draw a texture with: its full-size level only,
    /// so mipmaps made for 3D don't change how sprites look.
    fn sprite_bind_group(&self, texture: &wgpu::Texture, nearest: bool) -> wgpu::BindGroup {
        // Its own format: sRGB for images, HDR for render targets.
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            mip_level_count: Some(1),
            ..Default::default()
        });
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite texture"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(if nearest {
                        &self.nearest_sampler
                    } else {
                        &self.sampler
                    }),
                },
            ],
        })
    }

    /// Gives texture `id` mipmaps, so 3D materials far away or at a slant
    /// read it smoothly instead of shimmering. `linear`: it holds data
    /// (normals, ORM), averaged as stored rather than as light.
    #[cfg(feature = "3d")]
    fn ensure_mips(&mut self, id: u32, linear: bool) {
        let Some(tex) = self.textures.get(id as usize) else {
            return;
        };
        if tex.mips.is_some() {
            return;
        }
        let (width, height) = (tex.texture.width(), tex.texture.height());
        let levels = super::mips::levels(width, height);
        // Without view formats (WebGL2) a data texture can't be read through
        // a linear view of its sRGB texture: it is remade as a linear one.
        let remake_as_data = linear && self.image_view_formats.is_empty();
        if levels == 1 && !remake_as_data {
            self.textures[id as usize].mips = Some(linear);
            return;
        }
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mips = self
            .mips
            .get_or_insert_with(|| super::mips::MipMaker::new(&self.device));
        let texture = if remake_as_data {
            mips.data_texture(&self.device, &mut encoder, &self.textures[id as usize].texture, levels)
        } else {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("material texture"),
                size,
                mip_level_count: levels,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: self.image_view_formats,
            });
            encoder.copy_texture_to_texture(
                self.textures[id as usize].texture.as_image_copy(),
                texture.as_image_copy(),
                size,
            );
            mips.generate(&self.device, &mut encoder, &texture, linear);
            texture
        };
        self.queue.submit([encoder.finish()]);
        let bind_group = self.sprite_bind_group(&texture, self.textures[id as usize].nearest);
        let tex = &mut self.textures[id as usize];
        tex.texture = texture;
        tex.bind_group = bind_group;
        tex.mips = Some(linear);
    }

    /// Remakes texture `id`'s mipmaps after its pixels changed.
    fn refresh_mips(&mut self, id: u32) {
        // Called after every change to a texture's pixels: cut-out shadows
        // drawn with it are drawn again.
        #[cfg(feature = "3d")]
        if let Some(three) = &mut self.three {
            three.texture_changed(id);
        }
        #[cfg(feature = "3d")]
        if let Some(linear) = self.textures[id as usize].mips
            && let Some(mips) = &self.mips
        {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            mips.generate(&self.device, &mut encoder, &self.textures[id as usize].texture, linear);
            self.queue.submit([encoder.finish()]);
        }
        #[cfg(not(feature = "3d"))]
        let _ = id;
    }

    /// Replaces every pixel of texture `id` (same size as created).
    pub fn update_texture(&mut self, id: u32, width: u32, height: u32, rgba: &[u8]) {
        debug_assert_eq!(rgba.len(), (width * height * 4) as usize);
        self.queue.uploading(Upload::Textures).write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.textures[id as usize].texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.refresh_mips(id);
    }

    /// Switches texture `id` between smooth and hard-edged sampling.
    pub fn set_filter(&mut self, id: u32, nearest: bool) {
        let bind_group = self.sprite_bind_group(&self.textures[id as usize].texture, nearest);
        let tex = &mut self.textures[id as usize];
        tex.bind_group = bind_group;
        tex.nearest = nearest;
    }

    /// Replaces a `width` × `height` block of texture `id` at `(x, y)`.
    pub fn update_texture_region(&mut self, id: u32, at: [u32; 2], size: [u32; 2], rgba: &[u8]) {
        debug_assert_eq!(rgba.len(), (size[0] * size[1] * 4) as usize);
        self.queue.uploading(Upload::Textures).write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.textures[id as usize].texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: at[0],
                    y: at[1],
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size[0]),
                rows_per_image: Some(size[1]),
            },
            wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
        );
        self.refresh_mips(id);
    }

    /// Call when the window's physical size changes.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return; // minimized
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Draws everything in `list` and presents. `list` is left as-is; the caller clears it.
    ///
    /// With `capture`, the frame is also read back into `self.screenshot`
    /// (this waits for the GPU, so only do it when asked).
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        world: &mut DrawList,
        ui: &mut DrawList,
        panels: &mut [PanelDraw],
        clear: Color,
        world_view: Mat4,
        ui_view: Mat4,
        capture: bool,
    ) -> FrameStatus {
        // `None` when the window can't be drawn to right now; a requested
        // screenshot is still drawn (offscreen), so it never waits on the
        // window being visible.
        let acquire_start = web_time::Instant::now();
        let acquired = if self.only_hidden {
            wgpu::CurrentSurfaceTexture::Occluded
        } else {
            self.surface.get_current_texture()
        };
        self.display_wait_ms = acquire_start.elapsed().as_secs_f32() * 1000.0;
        let (frame, status) = match acquired {
            wgpu::CurrentSurfaceTexture::Success(t) => (Some(t), FrameStatus::Presented),
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                // Draw this one, reconfigure for the next.
                self.surface.configure(&self.device, &self.config);
                (Some(t), FrameStatus::Presented)
            }
            wgpu::CurrentSurfaceTexture::Occluded => (None, FrameStatus::Occluded),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Validation => {
                (None, FrameStatus::Skipped)
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                (None, FrameStatus::Skipped)
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .instance
                    .create_surface(self.window.clone())
                    .unwrap_or_else(|e| panic!("ponko: lost the GPU surface: {e}"));
                self.surface.configure(&self.device, &self.config);
                (None, FrameStatus::Skipped)
            }
        };
        if frame.is_none() && !capture && !self.draw_hidden {
            return status;
        }
        let hidden = frame.is_none() && self.draw_hidden;
        let status = if hidden { FrameStatus::Offscreen } else { status };
        let mut hidden_view = hidden.then(|| self.hidden_view());
        if hidden {
            // As a swapchain would: wait while two frames are on the GPU.
            let wait = web_time::Instant::now();
            while self.hidden_in_flight.len() >= 2 {
                if let Some(oldest) = self.hidden_in_flight.pop_front() {
                    let _ = self.device.poll(wgpu::PollType::Wait {
                        submission_index: Some(oldest),
                        timeout: Some(std::time::Duration::from_secs(1)),
                    });
                }
            }
            self.display_wait_ms += wait.elapsed().as_secs_f32() * 1000.0;
        }

        let cpu_start = web_time::Instant::now();
        if let Some(timer) = &mut self.timer {
            timer.collect(&self.device);
            timer.begin_frame();
        }
        let (world, world_batches) = world.finish();
        let (ui, ui_batches) = ui.finish();
        let panels: Vec<_> = panels
            .iter_mut()
            .filter(|p| (p.texture as usize) < self.textures.len())
            .map(|p| (p.texture, p.size, p.list.finish()))
            .collect();
        let panel_sprites: usize = panels.iter().map(|(_, _, (sprites, _))| sprites.len()).sum();
        let total = world.len() + ui.len() + panel_sprites;
        self.stats.sprites = total;
        self.stats.draw_calls = world_batches.len()
            + ui_batches.len()
            + panels.iter().map(|(_, _, (_, batches))| batches.len()).sum::<usize>();
        #[cfg(feature = "3d")]
        if let Some(three) = &self.three {
            self.stats.draw_calls += three.draw_calls();
            let [(instances, triangles), (shadow_instances, shadow_triangles)] = three.counts();
            self.stats.instances = instances;
            self.stats.triangles = triangles;
            self.stats.shadow_instances = shadow_instances;
            self.stats.shadow_triangles = shadow_triangles;
            self.stats.occluded = three.occluded;
            self.stats.pipelines_built = three.take_pipelines_built();
            self.stats.collect_ms = self.collect_ms;
        }

        if total > self.instance_capacity {
            self.instance_capacity = total.next_power_of_two();
            self.instance_buffer = Self::create_instance_buffer(&self.device, self.instance_capacity);
        }
        if !world.is_empty() {
            self.queue
                .uploading(Upload::Sprites)
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(world));
        }
        if !ui.is_empty() {
            let offset = std::mem::size_of_val(world) as u64;
            self.queue
                .uploading(Upload::Sprites)
                .write_buffer(&self.instance_buffer, offset, bytemuck::cast_slice(ui));
        }
        // World-space UI panels' sprites go after the screen's, each panel
        // with a camera of its own pixels.
        let mut panel_base = (world.len() + ui.len()) as u32;
        let mut panel_starts = Vec::with_capacity(panels.len());
        for (i, (_, size, (sprites, _))) in panels.iter().enumerate() {
            if !sprites.is_empty() {
                let offset = u64::from(panel_base) * size_of::<SpriteInstance>() as u64;
                self.queue.uploading(Upload::Sprites).write_buffer(
                    &self.instance_buffer,
                    offset,
                    bytemuck::cast_slice(sprites),
                );
            }
            if i == self.panel_cameras.len() {
                self.panel_cameras.push(self.make_camera("ui panel camera"));
            }
            let view = crate::ctx::screen_projection(glam::Vec2::new(size[0] as f32, size[1] as f32));
            self.queue.uploading(Upload::Uniforms).write_buffer(
                &self.panel_cameras[i].0,
                0,
                bytemuck::cast_slice(&view.to_cols_array()),
            );
            panel_starts.push(panel_base);
            panel_base += sprites.len() as u32;
        }
        if !panels.is_empty() && self.panel_pipeline.is_none() {
            // Panels draw into render targets, which only 3D makes.
            #[cfg(feature = "3d")]
            let format = super::gpu3d::HDR_FORMAT;
            #[cfg(not(feature = "3d"))]
            let format = self.view_format;
            self.panel_pipeline = Some(sprite_pipeline(
                &self.device,
                &self.sprite_layout,
                &self.sprite_shader,
                format,
            ));
        }
        self.queue.uploading(Upload::Uniforms).write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&world_view.to_cols_array()),
        );
        self.queue.uploading(Upload::Uniforms).write_buffer(
            &self.ui_camera_buffer,
            0,
            bytemuck::cast_slice(&ui_view.to_cols_array()),
        );
        // World sprites first, then the UI on top.
        let layers = [
            (&self.camera_bind_group, world_batches, 0),
            (&self.ui_camera_bind_group, ui_batches, world.len() as u32),
        ];
        let pass = SpritePass {
            viewport: [self.config.width, self.config.height],
            scale: self.window.scale_factor() as f32,
            pipeline: &self.pipeline,
            instances: &self.instance_buffer,
            textures: &self.textures,
            layers: &layers,
        };
        // Panels are drawn first, in buffers of their own: the 3D frame
        // samples them.
        let panel_commands = self
            .panel_pipeline
            .as_ref()
            .filter(|_| !panels.is_empty())
            .map(|pipeline| {
                let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ui panels"),
                });
                for (i, (texture, size, (_, batches))) in panels.iter().enumerate() {
                    let view = self.textures[*texture as usize]
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let layers = [(&self.panel_cameras[i].1, *batches, panel_starts[i])];
                    let pass = SpritePass {
                        viewport: *size,
                        scale: 1.0,
                        pipeline,
                        instances: &self.instance_buffer,
                        textures: &self.textures,
                        layers: &layers,
                    };
                    Self::sprite_pass(&mut encoder, &view, Some(Color::TRANSPARENT), None, &pass);
                }
                encoder.finish()
            });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        let view = match &frame {
            Some(frame) => Some(frame.texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.view_format),
                ..Default::default()
            })),
            None => hidden_view.take(),
        };
        // The 3D passes come in command buffers of their own (recorded in
        // parallel), submitted before `encoder`'s sprites go on top.
        let mut buffers_3d = Vec::new();
        let record_start = web_time::Instant::now();
        if let Some(view) = &view {
            let timer = self.timer.as_ref();
            let sprite_clear = self.draw_3d(&mut buffers_3d, view, clear, timer);
            Self::sprite_pass(
                &mut encoder,
                view,
                sprite_clear,
                timer.and_then(|t| t.pass("sprites")),
                &pass,
            );
        }
        // A screenshot draws the same sprites a second time, into a texture
        // that can be copied (the window's surface can't, on some GPUs).
        let capture = capture.then(|| {
            let target = Capture::new(&self.device, self.config.width, self.config.height, self.view_format);
            let target_view = target.texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sprite_clear = self.draw_3d(&mut buffers_3d, &target_view, clear, None);
            Self::sprite_pass(&mut encoder, &target_view, sprite_clear, None, &pass);
            target.copy(&mut encoder);
            target
        });
        // The inspector goes on the window only, not into screenshots.
        #[cfg(feature = "inspector")]
        let overlay_commands = match &view {
            Some(view) => self.draw_overlay(&mut encoder, view),
            None => Vec::new(),
        };
        #[cfg(not(feature = "inspector"))]
        let overlay_commands: [wgpu::CommandBuffer; 0] = [];
        if let Some(timer) = &mut self.timer {
            timer.resolve(&mut encoder);
        }
        let finished = encoder.finish();
        self.stats.record_ms = record_start.elapsed().as_secs_f32() * 1000.0;
        let submit_start = web_time::Instant::now();
        let submitted = self.queue.submit(
            panel_commands
                .into_iter()
                .chain(overlay_commands)
                .chain(buffers_3d)
                .chain([finished]),
        );
        self.stats.submit_ms = submit_start.elapsed().as_secs_f32() * 1000.0;
        if hidden {
            self.hidden_in_flight.push_back(submitted);
        }
        #[cfg(feature = "3d")]
        if let Some(three) = &mut self.three {
            three.submitted();
        }
        if let Some(timer) = &mut self.timer {
            timer.submitted();
            self.stats.gpu_ms = timer.smoothed_ms;
        }
        self.stats.cpu_ms = cpu_start.elapsed().as_secs_f32() * 1000.0;
        self.stats.uploads = super::traffic::take();
        #[cfg(feature = "3d")]
        {
            self.last_prepare_3d_ms = std::mem::take(&mut self.prepare_3d_ms);
            self.stats.cpu_ms += self.last_prepare_3d_ms;
        }
        if let Some(frame) = frame {
            self.window.pre_present_notify();
            self.queue.present(frame);
        }
        if let Some(capture) = capture {
            self.screenshot = Some(capture.read(&self.device));
        }
        status
    }

    /// Hands the renderer the inspector's drawing for the next frame. A
    /// drawing that wasn't shown yet (the window was hidden) is replaced,
    /// but its texture changes are kept.
    #[cfg(feature = "inspector")]
    pub fn set_overlay(&mut self, mut overlay: crate::inspector::Overlay) {
        if let Some(older) = self.overlay.take() {
            overlay.merge_older(older);
        }
        self.overlay = Some(overlay);
    }

    /// Records the inspector's pass on top of the frame. Returns egui's
    /// extra command buffers, to submit before the frame's.
    #[cfg(feature = "inspector")]
    fn draw_overlay(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(overlay) = self.overlay.take() else {
            return Vec::new();
        };
        let renderer = self.egui.get_or_insert_with(|| {
            let options = egui_wgpu::RendererOptions {
                msaa_samples: 1,
                depth_stencil_format: None,
                dithering: true,
                predictable_texture_filtering: false,
            };
            egui_wgpu::Renderer::new(&self.device, self.view_format, options)
        });
        for id in self.egui_free.drain(..) {
            renderer.free_texture(&id);
        }
        for (id, deltas) in &overlay.textures.set {
            for delta in deltas {
                renderer.update_texture(&self.device, &self.queue, *id, delta);
            }
        }
        self.egui_free.extend(overlay.textures.free.iter().copied());
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: overlay.pixels_per_point,
        };
        let commands = renderer.update_buffers(&self.device, &self.queue, encoder, &overlay.primitives, &screen);
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("inspector"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            })
            .forget_lifetime();
        renderer.render(&mut pass, &overlay.primitives, &screen);
        commands
    }

    /// Draws the frame's 3D models (clearing to `clear`) when there are
    /// any. Returns the color the sprite pass should clear to: `None` when
    /// it draws over the 3D models.
    #[cfg_attr(not(feature = "3d"), allow(clippy::unused_self))]
    /// Adds the 3D frame's command buffers to `buffers`; the clear color
    /// the sprite pass still needs (none if 3D drew the background).
    fn draw_3d(
        &self,
        buffers: &mut Vec<wgpu::CommandBuffer>,
        view: &wgpu::TextureView,
        clear: Color,
        timer: Option<&GpuTimer>,
    ) -> Option<Color> {
        #[cfg(feature = "3d")]
        if let Some(three) = self.three.as_ref().filter(|t| t.has_draws()) {
            buffers.extend(three.draw(&self.device, view, clear.to_linear_wgpu(), &self.textures, timer));
            return None;
        }
        let _ = (buffers, view, timer);
        Some(clear)
    }

    /// A camera uniform and its bind group for the sprite pipeline.
    fn make_camera(&self, label: &str) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = self.device.new_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size_of::<Mat4>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        (buffer, bind_group)
    }

    fn sprite_pass(
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        clear: Option<Color>,
        timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>,
        p: &SpritePass<'_>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sprites"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: clear.map_or(wgpu::LoadOp::Load, |c| wgpu::LoadOp::Clear(c.to_linear_wgpu())),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if p.layers.iter().all(|(_, batches, _)| batches.is_empty()) {
            return;
        }
        pass.set_pipeline(p.pipeline);
        pass.set_vertex_buffer(0, p.instances.slice(..));
        for (camera, batches, base) in p.layers {
            pass.set_bind_group(0, *camera, &[]);
            for batch in *batches {
                let [width, height] = p.viewport;
                let (x, y, w, h) = batch.clip.map_or((0, 0, width, height), |r| {
                    let x = (r.min.x * p.scale).floor().clamp(0.0, width as f32) as u32;
                    let y = (r.min.y * p.scale).floor().clamp(0.0, height as f32) as u32;
                    let right = (r.max.x * p.scale).ceil().clamp(x as f32, width as f32) as u32;
                    let bottom = (r.max.y * p.scale).ceil().clamp(y as f32, height as f32) as u32;
                    (x, y, right - x, bottom - y)
                });
                if w == 0 || h == 0 {
                    continue;
                }
                pass.set_scissor_rect(x, y, w, h);
                // `Texture::NONE` (no texture) draws nothing.
                let Some(texture) = p.textures.get(batch.texture as usize) else {
                    continue;
                };
                pass.set_bind_group(1, &texture.bind_group, &[]);
                pass.draw(0..6, base + batch.start..base + batch.end);
            }
        }
    }
}

/// Bytes `texture` takes on the GPU, every mipmap level and layer.
fn texture_bytes(texture: &wgpu::Texture) -> u64 {
    let block = u64::from(texture.format().block_copy_size(None).unwrap_or(4));
    let layers = u64::from(texture.depth_or_array_layers());
    let samples = u64::from(texture.sample_count());
    (0..texture.mip_level_count())
        .map(|level| {
            let w = u64::from((texture.width() >> level).max(1));
            let h = u64::from((texture.height() >> level).max(1));
            w * h * block
        })
        .sum::<u64>()
        * layers
        * samples
}
