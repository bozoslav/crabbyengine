//! Persistent buffers and view-specific GPU visibility lists.
use super::*;
use crate::three::instances::Batch;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct View {
    planes: [[f32; 4]; 6],
    camera: [f32; 4],
    occlusion: [f32; 4],
}
pub(crate) struct BatchView {
    pub read: wgpu::BindGroup,
    _data: wgpu::Buffer,
    pub args: wgpu::Buffer,
    uniform: wgpu::Buffer,
    compute: wgpu::BindGroup,
}
pub(crate) struct StaticBatch {
    pub mesh: u32,
    pub vertices: wgpu::Buffer,
    /// Slots: main, four sun cascades, active light views, then target cameras.
    /// Retained at the high water mark; only active slots are dispatched.
    pub views: Vec<BatchView>,
    cells: Vec<crate::three::instances::Cell>,
    pub visible: Vec<Vec<(u32, u32)>>,
    source: Vec<[f32; 4]>,
    gpu_cells: Option<wgpu::Buffer>,
    arguments: Vec<u32>,
    active_views: usize,
    pub options: crate::InstanceBatchOptions,
    pub count: u32,
    pub parts: usize,
}
pub(super) struct Culling {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    occlusion: wgpu::Buffer,
}
impl Culling {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("static culling"),
            entries: &(0..5)
                .map(|i| wgpu::BindGroupLayoutEntry {
                    binding: i,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: if i == 3 {
                            wgpu::BufferBindingType::Uniform
                        } else {
                            wgpu::BufferBindingType::Storage {
                                read_only: i == 1 || i == 4,
                            }
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                })
                .collect::<Vec<_>>(),
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("static culling"),
            source: wgpu::ShaderSource::Wgsl(include_str!("instances.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("static culling"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            })),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let occlusion = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("static occlusion grid"),
            size: ((device.limits().max_texture_dimension_2d as u64 / 8 + 1) * 160 + 32) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            layout,
            pipeline,
            occlusion,
        }
    }
}
impl Renderer3D {
    pub fn set_instance_batch(&mut self, device: &wgpu::Device, id: u32, batch: Option<&Batch>) {
        while self.static_batches.len() <= id as usize {
            self.static_batches.push(None);
        }
        let Some(batch) = batch.filter(|b| !b.instances.is_empty()) else {
            self.static_batches[id as usize] = None;
            return;
        };
        let compute = self.storage_read.is_some()
            && device.limits().max_storage_buffers_per_shader_stage >= 4
            && can_compute(device);
        if compute && self.static_culling.is_none() {
            self.static_culling = Some(Culling::new(device));
        }
        let count = batch.instances.len() as u32;
        assert!(
            !compute || u64::from(count) * 112 + 16 <= device.limits().max_storage_buffer_binding_size,
            "instance batch exceeds device storage limit"
        );
        let mut marker = batch.instances.clone();
        if compute {
            for v in &mut marker {
                v.model[0][3] = -2.0;
            }
        }
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("static instance vertices"),
            contents: bytemuck::cast_slice(&marker),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let views = Vec::new();
        let mut source = Vec::new();
        let mut gpu_cells = None;
        let mut arguments = Vec::new();
        let parts = self.meshes[batch.mesh.id as usize].parts.len();
        if compute {
            source.reserve(1 + 7 * count as usize);
            source.push([count as f32, 0.0, 0.0, 0.0]);
            for (i, s) in batch.instances.iter().zip(&batch.spheres) {
                source.extend_from_slice(&i.model);
                source.push(i.color);
                source.push(*s);
            }
            source.resize(1 + 7 * count as usize, [0.0; 4]);
            let cells: Vec<[f32; 4]> = batch
                .cells
                .iter()
                .flat_map(|c| {
                    [
                        c.min.extend(f32::from_bits(c.start)).to_array(),
                        c.max.extend(f32::from_bits(c.count)).to_array(),
                    ]
                })
                .collect();
            let cells = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("static cells"),
                contents: bytemuck::cast_slice(&cells),
                usage: wgpu::BufferUsages::STORAGE,
            });
            arguments = self.meshes[batch.mesh.id as usize]
                .parts
                .iter()
                .flat_map(|(r, _)| [r.end - r.start, 0, r.start, 0, 0])
                .collect();
            gpu_cells = Some(cells);
        }
        self.static_batches[id as usize] = Some(StaticBatch {
            mesh: batch.mesh.id,
            vertices,
            views,
            cells: batch.cells.clone(),
            visible: Vec::new(),
            source,
            gpu_cells,
            arguments,
            active_views: 0,
            options: batch.options,
            count,
            parts,
        });
    }
    pub(super) fn prepare_static(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &crate::three::Frame3D,
        camera: Vec3,
        occlusion: bool,
    ) {
        if let Some(cull) = &self.static_culling {
            let grid = frame
                .occlusion
                .as_ref()
                .filter(|_| occlusion)
                .and_then(|g| g.base().map(|b| (g, b)));
            let mut header = [0.0f32; 24];
            if let Some((g, (w, h, depth))) = grid {
                header[0..4].copy_from_slice(&[*w as f32, *h as f32, g.camera.distance(camera), 1.0]);
                header[4..20].copy_from_slice(&g.view_proj.to_cols_array());
                queue.write_buffer(&cull.occlusion, 96, bytemuck::cast_slice(depth));
            }
            queue.write_buffer(&cull.occlusion, 0, bytemuck::cast_slice(&header));
        }
        for batch in self.static_batches.iter_mut().flatten() {
            let targets = 1 + MAX_CASCADES + frame.light_views;
            batch.active_views = targets + frame.views.len();
            if batch.gpu_cells.is_none() {
                while batch.visible.len() < batch.active_views {
                    batch.visible.push(Vec::with_capacity(batch.cells.len()));
                }
            }
            if let (Some(culling), Some(storage_read)) = (&self.static_culling, &self.storage_read) {
                while batch.views.len() < batch.active_views {
                    let data = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("static transforms and visibility"),
                        contents: bytemuck::cast_slice(&batch.source),
                        usage: wgpu::BufferUsages::STORAGE,
                    });
                    let args = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("static indirect arguments"),
                        contents: bytemuck::cast_slice(&batch.arguments),
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::INDIRECT
                            | wgpu::BufferUsages::COPY_DST
                            | wgpu::BufferUsages::COPY_SRC,
                    });
                    let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("static cull view"),
                        size: std::mem::size_of::<View>() as u64,
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    let read = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: None,
                        layout: storage_read,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: data.as_entire_binding(),
                        }],
                    });
                    let compute = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: None,
                        layout: &culling.layout,
                        entries: &[
                            &data,
                            batch.gpu_cells.as_ref().unwrap(),
                            &args,
                            &uniform,
                            &culling.occlusion,
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(i, b)| wgpu::BindGroupEntry {
                            binding: i as u32,
                            resource: b.as_entire_binding(),
                        })
                        .collect::<Vec<_>>(),
                    });
                    batch.views.push(BatchView {
                        read,
                        _data: data,
                        args,
                        uniform,
                        compute,
                    });
                }
            }
            for v in 0..batch.active_views {
                let shadow = v > 0 && v < targets;
                if shadow && (!batch.options.cast_shadows || (v <= MAX_CASCADES && v > frame.cascades)) {
                    continue;
                }
                let (matrix, eye) = if v == 0 {
                    (frame.view_proj, camera)
                } else if v <= MAX_CASCADES {
                    (frame.sun_view_proj[v - 1], camera)
                } else if v < targets {
                    let i = v - 1 - MAX_CASCADES;
                    (frame.lights.shadow_views[i], frame.lights.shadow_positions[i])
                } else {
                    let view = &frame.views[v - targets];
                    (view.view_proj, view.camera)
                };
                let r = [matrix.row(0), matrix.row(1), matrix.row(2), matrix.row(3)];
                let planes = [r[3] + r[0], r[3] - r[0], r[3] + r[1], r[3] - r[1], r[2], r[3] - r[2]]
                    .map(|p| p / p.truncate().length().max(f32::MIN_POSITIVE));
                if let Some(view) = batch.views.get(v) {
                    queue.write_buffer(
                        &view.uniform,
                        0,
                        bytemuck::bytes_of(&View {
                            planes: planes.map(|p| p.to_array()),
                            camera: eye.extend(batch.options.max_distance).to_array(),
                            occlusion: [if v == 0 && occlusion { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
                        }),
                    );
                } else {
                    let visible = &mut batch.visible[v];
                    visible.clear();
                    for cell in &batch.cells {
                        let center = (cell.min + cell.max) * 0.5;
                        let radius = (cell.max - cell.min).length() * 0.5;
                        if !(v == 0
                            && occlusion
                            && frame
                                .occlusion
                                .as_ref()
                                .is_some_and(|g| g.hides(cell.min, cell.max, camera)))
                            && center.distance(eye) - radius <= batch.options.max_distance
                            && planes.iter().all(|p| p.truncate().dot(center) + p.w >= -radius)
                        {
                            visible.push((cell.start, cell.count));
                        }
                    }
                }
            }
        }
    }
    pub(super) fn cull_static(&self, encoder: &mut wgpu::CommandEncoder) {
        let Some(cull) = &self.static_culling else {
            return;
        };
        for batch in self.static_batches.iter().flatten() {
            for (v, view) in batch.views.iter().enumerate().take(batch.active_views) {
                if v > 0
                    && v < 1 + MAX_CASCADES + self.light_views
                    && (!batch.options.cast_shadows
                        || (v <= MAX_CASCADES && (v > self.cascades || self.redrawn & (1 << (v - 1)) == 0)))
                {
                    continue;
                }
                for p in 0..batch.parts {
                    encoder.clear_buffer(&view.args, (p * 20 + 4) as u64, Some(4));
                }
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("static instance cull"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&cull.pipeline);
                pass.set_bind_group(0, &view.compute, &[]);
                pass.dispatch_workgroups(
                    (batch.cells.len() as u32).min(256),
                    (batch.cells.len() as u32).div_ceil(256),
                    1,
                );
            }
        }
    }
}
