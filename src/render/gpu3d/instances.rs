//! Shared persistent instance records and sparse view/level visibility lists.
use super::*;
use crate::render::InstanceLevel;
use crate::three::instances::{Batch, Cell};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct View {
    planes: [[f32; 4]; 6],
    camera: [f32; 4],
    occlusion: [f32; 4],
    band: [f32; 4],
}
pub(crate) struct BatchView {
    pub read: wgpu::BindGroup,
    _visibility: wgpu::Buffer,
    pub args: wgpu::Buffer,
    uniform: wgpu::Buffer,
    /// What `uniform` holds: a view that didn't move isn't uploaded again.
    uniform_mirror: crate::render::traffic::Mirror,
    compute: wgpu::BindGroup,
    slot: usize,
    level: usize,
    active: bool,
}
pub(crate) struct StaticBatch {
    pub levels: Vec<InstanceLevel>,
    /// Shared storage records and cells on compute devices; instance vertices
    /// on the CPU fallback. Compute draws bind one zero-stride marker instead.
    pub vertices: wgpu::Buffer,
    pub views: Vec<BatchView>,
    cells: Vec<Cell>,
    min: Vec3,
    max: Vec3,
    pub visible: Vec<Vec<(u32, u32)>>,
    pub(super) compute: bool,
    arguments: Vec<Vec<u32>>,
    pub options: crate::render::InstanceBatchOptions,
    legacy_distance: bool,
    pub count: u32,
}
impl StaticBatch {
    pub(super) fn view(&self, slot: usize, level: usize) -> Option<&BatchView> {
        self.views
            .iter()
            .find(|v| v.slot == slot && v.level == level && v.active)
    }
    pub(super) fn casts(&self) -> bool {
        self.options.cast_shadows && self.levels.iter().any(|l| l.shadows)
    }
    pub(super) fn shadow_distance(&self) -> bool {
        self.casts()
            && if self.legacy_distance {
                self.options.max_distance.is_finite()
            } else {
                self.levels
                    .iter()
                    .any(|l| l.shadows && (l.min_distance > 0.0 || l.max_distance.is_finite()))
            }
    }
}
pub(super) struct Culling {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    occlusion: wgpu::Buffer,
    /// What `occlusion` holds: its header, and the depth grid after it
    /// (only what changed is uploaded).
    header_mirror: crate::render::traffic::Mirror,
    grid_mirror: crate::render::traffic::Mirror,
    /// One attribute marker, shared by all batch draws with zero instance stride.
    pub(super) marker: wgpu::Buffer,
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
                                read_only: i == 0 || i == 4,
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
        let occlusion = device.new_buffer(&wgpu::BufferDescriptor {
            label: Some("static occlusion grid"),
            size: ((device.limits().max_texture_dimension_2d as u64 / 8 + 1) * 160 + 32) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | crate::render::traffic::MIRRORED,
            mapped_at_creation: false,
        });
        let mut marker = Instance3D::default();
        marker.model[0][3] = -2.0;
        let marker = device
            .uploading(Upload::StaticBatches)
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("static instance marker"),
                contents: bytemuck::bytes_of(&marker),
                usage: wgpu::BufferUsages::VERTEX,
            });
        Self {
            layout,
            pipeline,
            occlusion,
            header_mirror: Default::default(),
            grid_mirror: Default::default(),
            marker,
        }
    }
}
fn planes(matrix: Mat4) -> [crate::Vec4; 6] {
    let r = [matrix.row(0), matrix.row(1), matrix.row(2), matrix.row(3)];
    [r[3] + r[0], r[3] - r[0], r[3] + r[1], r[3] - r[1], r[2], r[3] - r[2]]
        .map(|p| p / p.truncate().length().max(f32::MIN_POSITIVE))
}
fn intersects(min: Vec3, max: Vec3, planes: &[crate::Vec4; 6], eye: Vec3, near: f32, far: f32) -> bool {
    let center = (min + max) * 0.5;
    let radius = (max - min).length() * 0.5;
    let distance = center.distance(eye);
    distance - radius <= far
        && distance + radius >= near
        && planes.iter().all(|p| p.truncate().dot(center) + p.w >= -radius)
}
impl Renderer3D {
    pub fn set_instance_batch(&mut self, device: &wgpu::Device, id: u32, batch: Option<&Batch>) {
        self.shadow_cache.invalidate();
        while self.static_batches.len() <= id as usize {
            self.static_batches.push(None);
        }
        let Some(batch) = batch.filter(|b| !b.instances.is_empty()) else {
            self.static_batches[id as usize] = None;
            return;
        };
        let compute = self.storage_read.is_some() && GpuLimits::of(device).compute_culling();
        if compute && self.static_culling.is_none() {
            self.static_culling = Some(Culling::new(device));
        }
        let count = batch.instances.len() as u32;
        assert!(
            !compute
                || u64::from(count) * 96 + 16 + batch.cells.len() as u64 * 32
                    <= device.limits().max_storage_buffer_binding_size,
            "instance batch exceeds device storage limit"
        );
        let vertices = if compute {
            // Pack immutable cells after the source records: four storage
            // bindings still suffice on downlevel compute devices.
            let mut source = Vec::with_capacity(1 + 6 * count as usize + 2 * batch.cells.len());
            source.push([count as f32, batch.cells.len() as f32, 0.0, 0.0]);
            for (i, sphere) in batch.instances.iter().zip(&batch.spheres) {
                source.extend_from_slice(&i.model);
                source.push(i.color);
                source.push(*sphere);
            }
            for cell in &batch.cells {
                source.push(cell.min.extend(f32::from_bits(cell.start)).to_array());
                source.push(cell.max.extend(f32::from_bits(cell.count)).to_array());
            }
            device
                .uploading(Upload::StaticBatches)
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("shared static records and cells"),
                    contents: bytemuck::cast_slice(&source),
                    usage: wgpu::BufferUsages::STORAGE,
                })
        } else {
            device
                .uploading(Upload::StaticBatches)
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("static instance vertices"),
                    contents: bytemuck::cast_slice(&batch.instances),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        };
        let arguments = batch
            .levels
            .iter()
            .map(|l| {
                self.meshes[l.mesh.id as usize]
                    .parts
                    .iter()
                    .flat_map(|(r, _)| [r.end - r.start, 0, r.start, 0, 0])
                    .collect()
            })
            .collect();
        self.static_batches[id as usize] = Some(StaticBatch {
            levels: batch.levels.clone(),
            vertices,
            views: Vec::new(),
            cells: batch.cells.clone(),
            min: batch.min,
            max: batch.max,
            visible: Vec::new(),
            compute,
            arguments,
            options: batch.options,
            legacy_distance: batch.legacy_distance,
            count,
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
        if self.static_shadowless != frame.static_shadowless {
            self.shadow_cache.invalidate();
            self.static_shadowless.clear();
            self.static_shadowless.extend_from_slice(&frame.static_shadowless);
        }
        if let Some(cull) = &mut self.static_culling {
            let grid = frame
                .occlusion
                .as_ref()
                .filter(|_| occlusion)
                .and_then(|g| g.base().map(|b| (g, b)));
            let mut header = [0.0f32; 24];
            if let Some((g, (w, h, depth))) = grid {
                header[0..4].copy_from_slice(&[*w as f32, *h as f32, g.camera.distance(camera), 1.0]);
                header[4..20].copy_from_slice(&g.view_proj.to_cols_array());
                let depth = bytemuck::cast_slice(depth);
                cull.grid_mirror
                    .upload(queue, Upload::StaticBatches, &cull.occlusion, 96, depth);
            }
            let header = bytemuck::cast_slice(&header);
            cull.header_mirror
                .upload(queue, Upload::StaticBatches, &cull.occlusion, 0, header);
        }
        for batch in self.static_batches.iter_mut().flatten() {
            for view in &mut batch.views {
                view.active = false;
            }
            for ranges in &mut batch.visible {
                ranges.clear();
            }
            let targets = 1 + MAX_CASCADES + frame.light_views;
            let slots = targets + frame.views.len();
            if !batch.compute {
                while batch.visible.len() < slots * batch.levels.len() {
                    batch.visible.push(Vec::new());
                }
            }
            for slot in 0..slots {
                let shadow = slot > 0 && slot < targets;
                if shadow && (!batch.casts() || (slot <= MAX_CASCADES && slot > frame.cascades)) {
                    continue;
                }
                let (matrix, eye) = if slot == 0 {
                    (frame.view_proj, camera)
                } else if slot <= MAX_CASCADES {
                    (frame.sun_view_proj[slot - 1], camera)
                } else if slot < targets {
                    let i = slot - 1 - MAX_CASCADES;
                    (frame.lights.shadow_views[i], frame.lights.shadow_positions[i])
                } else {
                    let v = &frame.views[slot - targets];
                    (v.view_proj, v.camera)
                };
                let planes = planes(matrix);
                // Whole-batch rejection precedes uniforms, dispatches and draws.
                let enabled = |l: &InstanceLevel| {
                    if shadow {
                        l.shadows && !frame.static_shadowless.contains(&l.mesh.id)
                    } else {
                        l.camera
                    }
                };
                let far = if batch.legacy_distance {
                    batch.options.max_distance
                } else {
                    batch
                        .levels
                        .iter()
                        .filter(|l| enabled(l))
                        .map(|l| l.max_distance)
                        .fold(0.0, f32::max)
                };
                if !batch.levels.iter().any(enabled) || !intersects(batch.min, batch.max, &planes, eye, 0.0, far) {
                    continue;
                }
                for (level, l) in batch.levels.iter().enumerate() {
                    if !enabled(l) {
                        continue;
                    }
                    let (near, far) = if batch.legacy_distance {
                        (0.0, batch.options.max_distance)
                    } else {
                        (l.min_distance, l.max_distance)
                    };
                    if !intersects(batch.min, batch.max, &planes, eye, near, far) {
                        continue;
                    }
                    if let (Some(culling), Some(storage_read), true) =
                        (&self.static_culling, &self.storage_read, batch.compute)
                    {
                        let index = match batch.views.iter().position(|v| v.slot == slot && v.level == level) {
                            Some(index) => index,
                            None => {
                                let mut indices = vec![0u32; count_visibility(batch.count)];
                                indices[0] = 1;
                                let visibility = device.uploading(Upload::StaticBatches).create_buffer_init(
                                    &wgpu::util::BufferInitDescriptor {
                                        label: Some("static visibility"),
                                        contents: bytemuck::cast_slice(&indices),
                                        usage: wgpu::BufferUsages::STORAGE,
                                    },
                                );
                                let args = device.uploading(Upload::StaticBatches).create_buffer_init(
                                    &wgpu::util::BufferInitDescriptor {
                                        label: Some("static indirect arguments"),
                                        contents: bytemuck::cast_slice(&batch.arguments[level]),
                                        usage: wgpu::BufferUsages::STORAGE
                                            | wgpu::BufferUsages::INDIRECT
                                            | wgpu::BufferUsages::COPY_DST
                                            | wgpu::BufferUsages::COPY_SRC,
                                    },
                                );
                                let uniform = device.new_buffer(&wgpu::BufferDescriptor {
                                    label: Some("static cull view"),
                                    size: size_of::<View>() as u64,
                                    usage: wgpu::BufferUsages::UNIFORM
                                        | wgpu::BufferUsages::COPY_DST
                                        | crate::render::traffic::MIRRORED,
                                    mapped_at_creation: false,
                                });
                                let read = device.create_bind_group(&wgpu::BindGroupDescriptor {
                                    label: None,
                                    layout: storage_read,
                                    entries: &[
                                        wgpu::BindGroupEntry {
                                            binding: 0,
                                            resource: batch.vertices.as_entire_binding(),
                                        },
                                        wgpu::BindGroupEntry {
                                            binding: 1,
                                            resource: visibility.as_entire_binding(),
                                        },
                                    ],
                                });
                                let compute = device.create_bind_group(&wgpu::BindGroupDescriptor {
                                    label: None,
                                    layout: &culling.layout,
                                    entries: [&batch.vertices, &visibility, &args, &uniform, &culling.occlusion]
                                        .into_iter()
                                        .enumerate()
                                        .map(|(i, b)| wgpu::BindGroupEntry {
                                            binding: i as u32,
                                            resource: b.as_entire_binding(),
                                        })
                                        .collect::<Vec<_>>()
                                        .as_slice(),
                                });
                                batch.views.push(BatchView {
                                    read,
                                    _visibility: visibility,
                                    args,
                                    uniform,
                                    uniform_mirror: Default::default(),
                                    compute,
                                    slot,
                                    level,
                                    active: false,
                                });
                                batch.views.len() - 1
                            }
                        };
                        let view = &mut batch.views[index];
                        view.active = true;
                        let settings = View {
                            planes: planes.map(|p| p.to_array()),
                            camera: eye.extend(far).to_array(),
                            occlusion: [if slot == 0 && occlusion { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
                            band: [near, far, if batch.legacy_distance { 1.0 } else { 0.0 }, 0.0],
                        };
                        let settings = bytemuck::bytes_of(&settings);
                        view.uniform_mirror
                            .upload(queue, Upload::StaticBatches, &view.uniform, 0, settings);
                    } else {
                        let visible = &mut batch.visible[slot * batch.levels.len() + level];
                        if visible.capacity() < batch.cells.len() {
                            visible.reserve(batch.cells.len());
                        }
                        for cell in &batch.cells {
                            let distance = ((cell.min + cell.max) * 0.5).distance(eye);
                            // A cell uses its centre, selecting the first matching level.
                            // This intentionally disables crossfades on the fallback.
                            let selected = batch.legacy_distance
                                || batch
                                    .levels
                                    .iter()
                                    .position(|other| enabled(other) && other.contains(distance))
                                    == Some(level);
                            if selected
                                && intersects(
                                    cell.min,
                                    cell.max,
                                    &planes,
                                    eye,
                                    0.0,
                                    if batch.legacy_distance { far } else { f32::INFINITY },
                                )
                                && !(slot == 0
                                    && occlusion
                                    && frame
                                        .occlusion
                                        .as_ref()
                                        .is_some_and(|g| g.hides(cell.min, cell.max, camera)))
                            {
                                visible.push((cell.start, cell.count));
                            }
                        }
                    }
                }
            }
        }
    }
    pub(super) fn cull_static<'t>(&self, encoder: &mut wgpu::CommandEncoder, time: &PassTimer<'_, 't>) {
        let Some(cull) = &self.static_culling else {
            return;
        };
        let active = |view: &BatchView| {
            view.active
                && !(view.slot > 0
                    && view.slot < 1 + MAX_CASCADES + self.light_views
                    && ((view.slot <= MAX_CASCADES && self.redrawn & (1 << (view.slot - 1)) == 0)
                        || (view.slot > MAX_CASCADES
                            && self.shadow_cache.light_kept & (1 << (view.slot - 1 - MAX_CASCADES)) != 0)))
        };
        for batch in self.static_batches.iter().flatten() {
            for view in batch.views.iter().filter(|v| active(v)) {
                for p in 0..batch.arguments[view.level].len() / 5 {
                    encoder.clear_buffer(&view.args, (p * 20 + 4) as u64, Some(4));
                }
            }
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("static instance cull"),
            timestamp_writes: time("static instance cull").map(|t| wgpu::ComputePassTimestampWrites {
                query_set: t.query_set,
                beginning_of_pass_write_index: t.beginning_of_pass_write_index,
                end_of_pass_write_index: t.end_of_pass_write_index,
            }),
        });
        pass.set_pipeline(&cull.pipeline);
        for batch in self.static_batches.iter().flatten() {
            for view in batch.views.iter().filter(|v| active(v)) {
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
fn count_visibility(count: u32) -> usize {
    count as usize + 1
}

#[cfg(test)]
impl Renderer3D {
    /// The static batches' and their culling's part of `check_mirrors`.
    pub(super) fn check_batch_mirrors(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        for batch in self.static_batches.iter().flatten() {
            for view in &batch.views {
                view.uniform_mirror
                    .check(device, queue, &view.uniform, 0, "static batch view");
            }
        }
        if let Some(cull) = &self.static_culling {
            cull.header_mirror
                .check(device, queue, &cull.occlusion, 0, "static culling header");
            cull.grid_mirror
                .check(device, queue, &cull.occlusion, 96, "static culling depth grid");
        }
    }
}
