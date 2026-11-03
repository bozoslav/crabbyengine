//! wgpu setup, textures, and the instanced sprite pass.

use std::sync::Arc;

use glam::Mat4;
use winit::window::Window;

use super::batch::{self, DrawList, SpriteInstance};
use super::timer::GpuTimer;
use super::{FrameStatus, RenderStats, Screenshot};
use crate::color::Color;

struct GpuTexture {
    bind_group: wgpu::BindGroup,
    texture: wgpu::Texture,
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

    pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    /// Window-pixel projection for screen-space drawing (UI, text).
    ui_camera_buffer: wgpu::Buffer,
    ui_camera_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    nearest_sampler: wgpu::Sampler,
    textures: Vec<GpuTexture>,

    instance_buffer: wgpu::Buffer,
    /// Capacity of `instance_buffer`, in sprites.
    instance_capacity: usize,

    timer: Option<GpuTimer>,
    stats: RenderStats,
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

/// What one sprite pass draws with.
struct SpritePass<'a> {
    pipeline: &'a wgpu::RenderPipeline,
    instances: &'a wgpu::Buffer,
    textures: &'a [GpuTexture],
    /// Camera, batches, and where the layer's instances start in `instances`.
    layers: [(&'a wgpu::BindGroup, &'a [batch::Batch], u32); 2],
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
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
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

    /// The last captured frame, if one is waiting.
    pub fn take_screenshot(&mut self) -> Option<Screenshot> {
        self.screenshot.take()
    }

    /// Opens the GPU and sets up drawing to `window`, waiting for the GPU
    /// (desktop; the web build calls [`Self::new_async`]).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(window: Arc<Window>, display: winit::event_loop::OwnedDisplayHandle, vsync: bool) -> Self {
        pollster::block_on(Self::new_async(window, display, vsync))
    }

    pub async fn new_async(window: Arc<Window>, display: winit::event_loop::OwnedDisplayHandle, vsync: bool) -> Self {
        let descriptor = wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(display));
        // In a browser, use WebGPU where it works and WebGL2 elsewhere.
        #[cfg(target_arch = "wasm32")]
        let instance = wgpu::util::new_instance_with_webgpu_detection(descriptor).await;
        #[cfg(not(target_arch = "wasm32"))]
        let instance = wgpu::Instance::new(descriptor);
        let surface = instance
            .create_surface(window.clone())
            .unwrap_or_else(|e| panic!("crabbyengine: could not create a GPU surface: {e}"));

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .unwrap_or_else(|e| panic!("crabbyengine: no compatible GPU found: {e}"));

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("crabbyengine device"),
                required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
                // The defaults ask for more than WebGL2 has; we need little.
                required_limits: if cfg!(target_arch = "wasm32") {
                    adapter.limits()
                } else {
                    wgpu::Limits::default()
                },
                ..Default::default()
            })
            .await
            .unwrap_or_else(|e| panic!("crabbyengine: could not open the GPU device: {e}"));

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("crabbyengine: the GPU can't draw to this window");
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

        let instance_attributes = wgpu::vertex_attr_array![
            0 => Float32x2, // pos
            1 => Float32x2, // size
            2 => Float16x2, // pivot
            3 => Float32,   // rotation
            4 => Unorm8x4,  // color
            5 => Unorm16x4, // uv: u0, v0, width, height
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
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
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: view_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let camera = |label| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
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

        Self {
            window,
            instance,
            surface,
            device,
            queue,
            config,
            view_format,
            pipeline,
            camera_buffer,
            camera_bind_group,
            ui_camera_buffer,
            ui_camera_bind_group,
            texture_layout,
            sampler,
            nearest_sampler,
            textures: Vec::new(),
            instance_buffer,
            instance_capacity: INITIAL_INSTANCE_CAPACITY,
            timer,
            stats: RenderStats::default(),
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
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite instances"),
            size: (capacity * size_of::<SpriteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn max_texture_size(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }

    /// Uploads RGBA8 (sRGB) pixels and returns the texture's id.
    pub fn create_texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> u32 {
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
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
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
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite texture"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.textures.push(GpuTexture { bind_group, texture });
        (self.textures.len() - 1) as u32
    }

    /// Replaces every pixel of texture `id` (same size as created).
    pub fn update_texture(&mut self, id: u32, width: u32, height: u32, rgba: &[u8]) {
        debug_assert_eq!(rgba.len(), (width * height * 4) as usize);
        self.queue.write_texture(
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
    }

    /// Switches texture `id` between smooth and hard-edged sampling.
    pub fn set_filter(&mut self, id: u32, nearest: bool) {
        let tex = &mut self.textures[id as usize];
        let view = tex.texture.create_view(&wgpu::TextureViewDescriptor::default());
        tex.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        });
    }

    /// Replaces a `width` × `height` block of texture `id` at `(x, y)`.
    pub fn update_texture_region(&mut self, id: u32, at: [u32; 2], size: [u32; 2], rgba: &[u8]) {
        debug_assert_eq!(rgba.len(), (size[0] * size[1] * 4) as usize);
        self.queue.write_texture(
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
    pub fn render(
        &mut self,
        world: &mut DrawList,
        ui: &mut DrawList,
        clear: Color,
        world_view: Mat4,
        ui_view: Mat4,
        capture: bool,
    ) -> FrameStatus {
        // `None` when the window can't be drawn to right now; a requested
        // screenshot is still drawn (offscreen), so it never waits on the
        // window being visible.
        let (frame, status) = match self.surface.get_current_texture() {
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
                    .unwrap_or_else(|e| panic!("crabbyengine: lost the GPU surface: {e}"));
                self.surface.configure(&self.device, &self.config);
                (None, FrameStatus::Skipped)
            }
        };
        if frame.is_none() && !capture {
            return status;
        }

        let cpu_start = web_time::Instant::now();
        if let Some(timer) = &mut self.timer {
            timer.collect(&self.device);
        }
        let (world, world_batches) = world.finish();
        let (ui, ui_batches) = ui.finish();
        let total = world.len() + ui.len();
        self.stats.sprites = total;
        self.stats.draw_calls = world_batches.len() + ui_batches.len();

        if total > self.instance_capacity {
            self.instance_capacity = total.next_power_of_two();
            self.instance_buffer = Self::create_instance_buffer(&self.device, self.instance_capacity);
        }
        if !world.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(world));
        }
        if !ui.is_empty() {
            let offset = std::mem::size_of_val(world) as u64;
            self.queue
                .write_buffer(&self.instance_buffer, offset, bytemuck::cast_slice(ui));
        }
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&world_view.to_cols_array()),
        );
        self.queue.write_buffer(
            &self.ui_camera_buffer,
            0,
            bytemuck::cast_slice(&ui_view.to_cols_array()),
        );
        // World sprites first, then the UI on top.
        let pass = SpritePass {
            pipeline: &self.pipeline,
            instances: &self.instance_buffer,
            textures: &self.textures,
            layers: [
                (&self.camera_bind_group, world_batches, 0),
                (&self.ui_camera_bind_group, ui_batches, world.len() as u32),
            ],
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        let view = frame.as_ref().map(|frame| {
            frame.texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.view_format),
                ..Default::default()
            })
        });
        if let Some(view) = &view {
            let timestamps = self.timer.as_mut().and_then(GpuTimer::begin);
            Self::sprite_pass(&mut encoder, view, clear, timestamps, &pass);
        }
        // A screenshot draws the same sprites a second time, into a texture
        // that can be copied (the window's surface can't, on some GPUs).
        let capture = capture.then(|| {
            let target = Capture::new(&self.device, self.config.width, self.config.height, self.view_format);
            let target_view = target.texture.create_view(&wgpu::TextureViewDescriptor::default());
            Self::sprite_pass(&mut encoder, &target_view, clear, None, &pass);
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
        if let Some(timer) = &self.timer {
            timer.resolve(&mut encoder);
        }
        self.queue
            .submit(overlay_commands.into_iter().chain([encoder.finish()]));
        if let Some(timer) = &mut self.timer {
            timer.submitted();
            self.stats.gpu_ms = timer.smoothed_ms;
        }
        self.stats.cpu_ms = cpu_start.elapsed().as_secs_f32() * 1000.0;
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

    fn sprite_pass(
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        clear: Color,
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
                    load: wgpu::LoadOp::Clear(clear.to_linear_wgpu()),
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
        for (camera, batches, base) in &p.layers {
            pass.set_bind_group(0, *camera, &[]);
            for batch in *batches {
                pass.set_bind_group(1, &p.textures[batch.texture as usize].bind_group, &[]);
                pass.draw(0..6, base + batch.start..base + batch.end);
            }
        }
    }
}
