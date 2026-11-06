//! Global illumination's GPU side (see `three/gi.rs` for the probes).
//!
//! - The **atlas** the material shaders read the probes from (binding 10
//!   of the frame, see `gi.wgsl`): each probe slot's light tile (8x8) in
//!   the top part and its distance tile (16x16) below, 64 to a row.
//! - The **views**: six 16x16 views per probe updated this frame, side by
//!   side (one row per probe), drawn with the material shaders (see
//!   `Renderer3D::record_gi`), lit by a **shadow map** of their own.
//! - The **update** compute pass (`gi_update.wgsl`): turns each probe's
//!   views into its tiles in a scratch image, which is then copied into
//!   the atlas.
use super::lights::LightTextures;
use crate::three::MAX_GI_PROBES_PER_FRAME;
use crate::three::gi::{GI_SHADOW_SIZE, GiFrame};

/// Each probe view's width and height, in pixels.
pub(super) const VIEW_SIZE: u32 = 16;
/// The most probes the views have room for.
const MAX_UPDATES: u32 = MAX_GI_PROBES_PER_FRAME;
/// Probe tiles per row of the atlas.
const COLUMNS: u32 = 64;
/// A probe slot's light and distance tiles' widths.
const LIGHT_TILE: u32 = 8;
const DEPTH_TILE: u32 = 16;
const ATLAS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// One probe to update, as `gi_update.wgsl` reads it.
#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct ProbeUpdate {
    probe: [f32; 4],
    tile: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Updates {
    list: [ProbeUpdate; MAX_UPDATES as usize],
    params: [f32; 4],
}

pub(super) struct Gi {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    uniform: wgpu::Buffer,
    atlas: wgpu::Texture,
    pub atlas_view: wgpu::TextureView,
    /// Probe slots the atlas has room for, and its rows of tiles.
    slots: usize,
    rows: u32,
    /// The probes' views (multisampled), what they resolve into, and
    /// their depth.
    pub views: wgpu::TextureView,
    pub resolve: wgpu::TextureView,
    pub depth: wgpu::TextureView,
    scratch: wgpu::Texture,
    group: wgpu::BindGroup,
    /// Each view's frame (camera and light), and its bind group (made by
    /// the renderer, from its frame sources).
    pub face_buffers: Vec<wgpu::Buffer>,
    pub face_groups: Vec<wgpu::BindGroup>,
    /// The lights near the probes.
    pub lights: LightTextures,
    /// The views' sun shadow map: as a one-layer array (for the frame) and
    /// as the pass's target.
    pub shadow_array: wgpu::TextureView,
    pub shadow: wgpu::TextureView,
    /// This frame's updates: (slot, tile column, tile row).
    updates: Vec<(u32, u32, u32)>,
}

impl Gi {
    /// `frame_layout`: the material shaders' frame bindings, which the
    /// update reads the sky from.
    pub fn new(device: &wgpu::Device, frame_layout: &wgpu::BindGroupLayout, samples: u32, uniform_size: u64) -> Self {
        let texture = |entry: wgpu::BindingType, binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: entry,
            count: None,
        };
        let float = |multisampled| wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d gi update layout"),
            entries: &[
                texture(
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    0,
                ),
                texture(float(false), 1),
                texture(
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: true,
                    },
                    2,
                ),
                texture(float(false), 3),
                texture(
                    wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: ATLAS_FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    4,
                ),
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("3d gi update"),
            source: wgpu::ShaderSource::Wgsl(super::gpu3d::gi_update_source().into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d gi update"),
            bind_group_layouts: &[Some(frame_layout), None, None, Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("3d gi update"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("cs_update"),
            compilation_options: Default::default(),
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3d gi updates"),
            size: size_of::<Updates>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let image = |label, size: (u32, u32), samples, format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let views_size = (6 * VIEW_SIZE, MAX_UPDATES * VIEW_SIZE);
        let attachment = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let sampled = wgpu::TextureUsages::TEXTURE_BINDING;
        let views = image("3d gi views", views_size, samples, super::gpu3d::HDR_FORMAT, attachment);
        let resolve = image(
            "3d gi views resolved",
            views_size,
            1,
            super::gpu3d::HDR_FORMAT,
            attachment | sampled,
        );
        let depth = image(
            "3d gi views depth",
            views_size,
            samples,
            DEPTH_FORMAT,
            attachment | sampled,
        );
        let scratch = image(
            "3d gi scratch",
            (LIGHT_TILE + DEPTH_TILE, MAX_UPDATES * DEPTH_TILE),
            1,
            ATLAS_FORMAT,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        );
        let shadow_map = image(
            "3d gi shadow map",
            (GI_SHADOW_SIZE, GI_SHADOW_SIZE),
            1,
            DEPTH_FORMAT,
            attachment | sampled,
        );
        let shadow_array = shadow_map.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let face_buffers = (0..MAX_UPDATES * 6)
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("3d gi view frame"),
                    size: uniform_size,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        let (atlas, atlas_view) = Self::atlas(device, 1);
        let views = views.create_view(&Default::default());
        let resolve = resolve.create_view(&Default::default());
        let depth = depth.create_view(&Default::default());
        let group = Self::group(device, &layout, &uniform, &resolve, &depth, &atlas_view, &scratch);
        Self {
            layout,
            pipeline,
            uniform,
            atlas,
            atlas_view,
            slots: 0,
            rows: 1,
            views,
            resolve,
            depth,
            scratch,
            group,
            face_buffers,
            face_groups: Vec::new(),
            lights: LightTextures::new(device),
            shadow_array,
            shadow: shadow_map.create_view(&Default::default()),
            updates: Vec::new(),
        }
    }

    /// An atlas with `rows` rows of probe tiles, zero (no probe) in each.
    fn atlas(device: &wgpu::Device, rows: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("3d gi probes"),
            size: wgpu::Extent3d {
                width: COLUMNS * DEPTH_TILE,
                height: rows * (LIGHT_TILE + DEPTH_TILE),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: ATLAS_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = atlas.create_view(&Default::default());
        (atlas, view)
    }

    fn group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniform: &wgpu::Buffer,
        resolve: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        atlas: &wgpu::TextureView,
        scratch: &wgpu::Texture,
    ) -> wgpu::BindGroup {
        let scratch = scratch.create_view(&Default::default());
        let view = |binding, view| wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        };
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d gi update"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                view(1, resolve),
                view(2, depth),
                view(3, atlas),
                view(4, &scratch),
            ],
        })
    }

    /// Rows of tiles the atlas has.
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Makes the atlas fit `frame`'s grid (true when it's a new one: the
    /// frame bind groups must take it), and writes this frame's updates.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, frame: &GiFrame) -> bool {
        let slots = frame.grid.total();
        let remade = slots != self.slots;
        if remade {
            self.slots = slots;
            self.rows = (slots as u32).div_ceil(COLUMNS).max(1);
            (self.atlas, self.atlas_view) = Self::atlas(device, self.rows);
            self.group = Self::group(
                device,
                &self.layout,
                &self.uniform,
                &self.resolve,
                &self.depth,
                &self.atlas_view,
                &self.scratch,
            );
        }
        self.updates.clear();
        let mut updates = Updates {
            list: [ProbeUpdate::default(); MAX_UPDATES as usize],
            params: [frame.near, frame.far, self.rows as f32, VIEW_SIZE as f32],
        };
        for (u, out) in frame.updates.iter().zip(&mut updates.list) {
            let (column, row) = (u.slot % COLUMNS, u.slot / COLUMNS);
            self.updates.push((u.slot, column, row));
            *out = ProbeUpdate {
                probe: u.position.extend(u.code).to_array(),
                tile: [column as f32, row as f32, u.keep, 0.0],
            };
        }
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&updates));
        remade
    }

    /// How many probes are updated this frame.
    pub fn update_count(&self) -> usize {
        self.updates.len()
    }

    /// The update pass, after the views are drawn: each probe's tiles into
    /// the scratch image, then into the atlas.
    pub fn update(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        frame_group: &wgpu::BindGroup,
        timestamps: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) {
        if self.updates.is_empty() {
            return;
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("3d gi update"),
                timestamp_writes: timestamps,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, frame_group, &[]);
            pass.set_bind_group(3, &self.group, &[]);
            pass.dispatch_workgroups(5, 1, self.updates.len() as u32);
        }
        let tile = |x, y| wgpu::TexelCopyTextureInfo {
            texture: &self.atlas,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        };
        let scratch = |x, y| wgpu::TexelCopyTextureInfo {
            texture: &self.scratch,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        };
        let size = |s| wgpu::Extent3d {
            width: s,
            height: s,
            depth_or_array_layers: 1,
        };
        for (k, &(_, column, row)) in self.updates.iter().enumerate() {
            let y = k as u32 * DEPTH_TILE;
            encoder.copy_texture_to_texture(
                scratch(0, y),
                tile(column * LIGHT_TILE, row * LIGHT_TILE),
                size(LIGHT_TILE),
            );
            encoder.copy_texture_to_texture(
                scratch(LIGHT_TILE, y),
                tile(column * DEPTH_TILE, self.rows * LIGHT_TILE + row * DEPTH_TILE),
                size(DEPTH_TILE),
            );
        }
    }
}
