//! Skinning on the GPU: each skinned model drawn this frame gets a vertex
//! buffer of its own (kept between frames), which a compute shader fills
//! with its posed corners before any pass draws. Every pass then draws it
//! like a plain mesh, so material shaders need nothing extra. Morph targets
//! (blend shapes) are added in the same pass, before skinning; meshes with
//! morph targets but no joints go through it with one identity matrix.
//! Where the GPU can't run compute shaders (WebGL2), the CPU poses them
//! instead.

use crate::render::traffic::{self, Counted, MAX_WRITES, NewBuffer, Synced, Upload};

use super::Vertex;
use crate::three::SkinFrame;

/// Corners per workgroup (`@workgroup_size` in skin.wgsl).
const GROUP: u32 = 64;
const SHADER: &str = include_str!("skin.wgsl");

/// What a skinned mesh keeps on the GPU besides its rest-pose vertices.
pub(super) struct GpuSkin {
    /// Per corner: 4 joints as u16 pairs, then 4 weights.
    influences: wgpu::Buffer,
    /// Its morph targets' offsets, target after target: per corner the
    /// position's (3 floats), then the normal's (3). A dummy without any.
    deltas: wgpu::Buffer,
    count: u32,
    /// How many morph targets it has.
    targets: u32,
    /// Without compute shaders: the rest pose, influences and offsets, on
    /// the CPU.
    cpu: Option<(Vec<Vertex>, Vec<Influence>, Vec<Delta>)>,
}

/// One corner's offset in one morph target (6 words in skin.wgsl).
#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Delta {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Influence {
    joints: [u16; 4],
    weights: [f32; 4],
}

/// Where a skinned model's posed corners go.
struct Slot {
    buffer: wgpu::Buffer,
    /// Corners it holds.
    capacity: u32,
    /// Its bind group, with the mesh and the `joints`/`weights` generation
    /// it was made for.
    group: Option<(u32, u64, wgpu::BindGroup)>,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    first: u32,
    count: u32,
    morph_first: u32,
    morph_count: u32,
}

pub(super) struct Skinning {
    /// `None` without compute shaders.
    pipeline: Option<(wgpu::ComputePipeline, wgpu::BindGroupLayout)>,
    /// This frame's skin matrices, written a changed matrix at a time (a
    /// skeleton that holds its pose costs nothing).
    joints: Synced,
    /// This frame's morph weights, written a changed vec4 at a time.
    weights: Synced,
    /// Bumped whenever `joints` or `weights` is made anew (bind groups name
    /// them).
    generation: u64,
    /// One `Params` per slot, `stride` bytes apart; only changed ones are
    /// written.
    params: Synced,
    stride: u64,
    // Keep collection indices stable even when earlier models are culled.
    // Empty slots have no GPU allocation until a pass first draws them.
    slots: Vec<Option<Slot>>,
    /// This frame, per slot: the skinned mesh, and whether any pass draws it.
    pub draws: Vec<(u32, bool)>,
    scratch: Vec<Vertex>,
    params_scratch: Vec<u8>,
    matrix_scratch: Vec<[f32; 16]>,
}

impl Skinning {
    /// Whether meshes are posed by the compute shader (else on the CPU).
    #[cfg(test)]
    pub fn uses_compute(&self) -> bool {
        self.pipeline.is_some()
    }

    pub fn new(device: &wgpu::Device, compute: bool) -> Self {
        let pipeline = compute.then(|| {
            let storage = |binding, read_only| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            };
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("3d skinning layout"),
                entries: &[
                    storage(0, true),
                    storage(1, true),
                    storage(2, true),
                    storage(3, false),
                    storage(5, true),
                    storage(6, true),
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: wgpu::BufferSize::new(size_of::<Params>() as u64),
                        },
                        count: None,
                    },
                ],
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("3d skinning pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("3d skinning"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("3d skinning"),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some("skin"),
                compilation_options: Default::default(),
                cache: None,
            });
            (pipeline, layout)
        });
        let stride = u64::from(device.limits().min_uniform_buffer_offset_alignment).max(size_of::<Params>() as u64);
        let storage = wgpu::BufferUsages::STORAGE;
        Self {
            pipeline,
            joints: Synced::new(
                device,
                "3d skin matrices",
                storage,
                64 * 64,
                Upload::Skins,
                (64, 4, MAX_WRITES),
            ),
            weights: Synced::new(
                device,
                "3d morph weights",
                storage,
                64 * 4,
                Upload::Skins,
                (16, 8, MAX_WRITES),
            ),
            generation: 0,
            params: Synced::new(
                device,
                "3d skinning params",
                wgpu::BufferUsages::UNIFORM,
                4 * stride as usize,
                Upload::Skins,
                (16, 0, MAX_WRITES),
            ),
            stride,
            slots: Vec::new(),
            draws: Vec::new(),
            scratch: Vec::new(),
            params_scratch: Vec::new(),
            matrix_scratch: Vec::new(),
        }
    }

    /// Usage the rest-pose vertex buffer of a skinned mesh needs.
    pub fn rest_usage(&self) -> wgpu::BufferUsages {
        if self.pipeline.is_some() {
            wgpu::BufferUsages::STORAGE
        } else {
            wgpu::BufferUsages::empty()
        }
    }

    /// The GPU side of a skinned mesh of `vertices`, or of one with morph
    /// targets: `deltas` holds `vertices.len()` offsets per target, target
    /// after target. Without `joints`, every corner follows matrix 0.
    pub fn create(
        &self,
        device: &wgpu::Device,
        vertices: &[Vertex],
        joints: &[[u16; 4]],
        weights: &[[f32; 4]],
        deltas: Vec<Delta>,
    ) -> GpuSkin {
        let influences: Vec<Influence> = if joints.is_empty() {
            let still = Influence {
                joints: [0; 4],
                weights: [1.0, 0.0, 0.0, 0.0],
            };
            vec![still; vertices.len()]
        } else {
            joints
                .iter()
                .zip(weights)
                .map(|(&joints, &weights)| Influence { joints, weights })
                .collect()
        };
        let buffer = device
            .uploading(Upload::Skins)
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3d skin influences"),
                contents: bytemuck::cast_slice(&influences),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let targets = deltas.len().checked_div(vertices.len()).unwrap_or(0) as u32;
        let dummy = [Delta::default()];
        let delta_buffer = device
            .uploading(Upload::Skins)
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3d morph targets"),
                contents: bytemuck::cast_slice(if deltas.is_empty() { &dummy[..] } else { &deltas[..] }),
                usage: wgpu::BufferUsages::STORAGE,
            });
        GpuSkin {
            influences: buffer,
            deltas: delta_buffer,
            count: vertices.len() as u32,
            targets,
            cpu: self.pipeline.is_none().then(|| (vertices.to_vec(), influences, deltas)),
        }
    }

    /// Gets this frame's skinned models ready: their buffers, skin matrices
    /// and (on the CPU path) posed corners. `mesh(id)` gives a skinned mesh's
    /// rest-pose vertices and skin; `used(k)` whether a pass draws model `k`.
    pub fn prepare<'m>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        skins: &SkinFrame,
        mesh: impl Fn(u32) -> Option<(&'m wgpu::Buffer, &'m GpuSkin)>,
        used: impl Fn(usize) -> bool,
    ) {
        self.draws.clear();
        self.draws
            .extend(skins.draws.iter().enumerate().map(|(k, d)| (d.mesh, used(k))));
        if self.draws.iter().all(|d| !d.1) {
            return;
        }
        // Bind groups name the joints and weights: new ones need new groups.
        let matrices = skins.matrices.len().max(1);
        if self.joints.reserve(device, matrices * 64) {
            self.generation += 1;
        }
        if self.weights.reserve(device, skins.weights.len() * 4) {
            self.generation += 1;
        }
        if self.pipeline.is_some() && !skins.matrices.is_empty() {
            self.matrix_scratch.clear();
            self.matrix_scratch
                .extend(skins.matrices.iter().map(glam::Mat4::to_cols_array));
            self.joints.write(queue, 0, bytemuck::cast_slice(&self.matrix_scratch));
        }
        if self.pipeline.is_some() && !skins.weights.is_empty() {
            self.weights.write(queue, 0, bytemuck::cast_slice(&skins.weights));
        }
        if self.params.reserve(device, self.draws.len() * self.stride as usize) {
            for slot in self.slots.iter_mut().flatten() {
                slot.group = None;
            }
        }
        // Past the most the device holds (reported), skins stay at rest.
        let fit = self.params.fits() / self.stride as usize;
        for d in self.draws.iter_mut().skip(fit) {
            d.1 = false;
        }
        if self.slots.len() < skins.draws.len() {
            self.slots.resize_with(skins.draws.len(), || None);
        }
        self.params_scratch.clear();
        self.params_scratch.resize(self.stride as usize * self.draws.len(), 0);
        for (k, d) in skins.draws.iter().enumerate() {
            let (count, targets) = mesh(d.mesh).map_or((0, 0), |(_, s)| (s.count, s.targets));
            let p = Params {
                first: d.first,
                count,
                morph_first: d.morph_first,
                morph_count: d.morph_count.min(targets),
            };
            let at = k * self.stride as usize;
            self.params_scratch[at..at + size_of::<Params>()].copy_from_slice(bytemuck::bytes_of(&p));
            if !self.draws[k].1 {
                continue;
            }
            let usage = self.rest_usage();
            let slot = self.slots[k].get_or_insert_with(|| Slot {
                buffer: slot_buffer(device, count.max(1), usage),
                capacity: count.max(1),
                group: None,
            });
            if slot.capacity < count {
                slot.capacity = count.next_power_of_two();
                traffic::reallocated();
                slot.buffer = slot_buffer(device, slot.capacity, usage);
                slot.group = None;
            }
            let Some((rest, skin)) = mesh(d.mesh) else {
                continue;
            };
            match (&self.pipeline, &skin.cpu) {
                (Some((_, layout)), _) => {
                    let fresh = matches!(&slot.group, Some((m, g, _)) if *m == d.mesh && *g == self.generation);
                    if !fresh {
                        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("3d skinning"),
                            layout,
                            entries: &[
                                entry(0, rest),
                                entry(1, &skin.influences),
                                entry(2, self.joints.buffer()),
                                entry(3, &slot.buffer),
                                entry(5, self.weights.buffer()),
                                entry(6, &skin.deltas),
                                wgpu::BindGroupEntry {
                                    binding: 4,
                                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                        buffer: self.params.buffer(),
                                        offset: 0,
                                        size: wgpu::BufferSize::new(size_of::<Params>() as u64),
                                    }),
                                },
                            ],
                        });
                        slot.group = Some((d.mesh, self.generation, group));
                    }
                }
                (None, Some((vertices, influences, deltas))) => {
                    let skin_matrices = &skins.matrices[d.first as usize..];
                    let morph = skins
                        .weights
                        .get(d.morph_first as usize..(d.morph_first + p.morph_count) as usize)
                        .unwrap_or(&[]);
                    let corners = vertices.len();
                    self.scratch.clear();
                    self.scratch
                        .extend(vertices.iter().zip(influences).enumerate().map(|(c, (v, i))| {
                            let (mut p, mut n) = (glam::Vec3::from(v.position), glam::Vec3::from(v.normal));
                            for (t, &w) in morph.iter().enumerate() {
                                if w != 0.0 {
                                    let delta = &deltas[t * corners + c];
                                    p += w * glam::Vec3::from(delta.position);
                                    n += w * glam::Vec3::from(delta.normal);
                                }
                            }
                            let m = crate::three::blend_skin(skin_matrices, i.joints, i.weights);
                            let p = m.transform_point3(p);
                            let n = m.transform_vector3(n).normalize_or(glam::Vec3::Y);
                            Vertex {
                                position: p.to_array(),
                                normal: n.to_array(),
                                ..*v
                            }
                        }));
                    queue
                        .uploading(Upload::Skins)
                        .write_buffer(&slot.buffer, 0, bytemuck::cast_slice(&self.scratch));
                }
                (None, None) => {}
            }
        }
        if self.pipeline.is_some() {
            self.params.write(queue, 0, &self.params_scratch);
        }
    }

    /// Poses this frame's skinned models (compute path only).
    pub fn record(&self, encoder: &mut wgpu::CommandEncoder, timestamps: Option<wgpu::ComputePassTimestampWrites<'_>>) {
        let Some((pipeline, _)) = &self.pipeline else {
            return;
        };
        if self.draws.iter().all(|d| !d.1) {
            return;
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("3d skinning"),
            timestamp_writes: timestamps,
        });
        pass.set_pipeline(pipeline);
        for (k, &(mesh, used)) in self.draws.iter().enumerate() {
            let Some(slot) = self.slots.get(k).and_then(Option::as_ref) else {
                continue;
            };
            let Some((m, _, group)) = slot.group.as_ref() else {
                continue;
            };
            if !used || *m != mesh {
                continue;
            }
            pass.set_bind_group(0, group, &[(k as u64 * self.stride) as u32]);
            pass.dispatch_workgroups(slot.capacity.div_ceil(GROUP), 1, 1);
        }
    }

    /// See `Renderer3D::check_mirrors`.
    #[cfg(test)]
    pub fn check_mirrors(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        for buffer in [&self.joints, &self.weights, &self.params] {
            buffer.check(device, queue);
        }
    }

    /// Whether this frame poses anything on the GPU.
    pub fn has_work(&self) -> bool {
        self.pipeline.is_some() && self.draws.iter().any(|d| d.1)
    }

    /// Skinned model `k`'s posed vertices, if it's drawn this frame.
    pub fn vertices(&self, k: usize) -> Option<&wgpu::Buffer> {
        let &(_, used) = self.draws.get(k)?;
        used.then(|| self.slots.get(k).and_then(Option::as_ref).map(|s| &s.buffer))
            .flatten()
    }
}

fn entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn slot_buffer(device: &wgpu::Device, corners: u32, storage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.new_buffer(&wgpu::BufferDescriptor {
        label: Some("3d posed vertices"),
        size: u64::from(corners) * size_of::<Vertex>() as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC | storage,
        mapped_at_creation: false,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_skinning_shader_is_valid() {
        super::super::check_shader("skin.wgsl", super::SHADER).unwrap();
    }

    /// Shapes (two morph targets) and poses three corners on the GPU (when
    /// there is one), with the compute shader and with the CPU fallback:
    /// both must agree with the CPU's math.
    #[test]
    fn gpu_skinning_matches_the_cpu() {
        posed_corners_match(true);
        posed_corners_match(false);
    }

    fn posed_corners_match(compute: bool) {
        use glam::{Mat4, Quat, Vec3};
        use wgpu::util::DeviceExt;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            crate::render::render_test_support::no_gpu("no GPU adapter");
            return;
        };
        let Ok((device, queue)) = pollster::block_on(adapter.request_device(
            &crate::render::render_test_support::forced(&wgpu::DeviceDescriptor::default()),
        )) else {
            crate::render::render_test_support::no_gpu("no GPU device");
            return;
        };
        // The compute pass binds six storage buffers (the renderer's own check).
        if compute && !crate::render::render_test_support::needs(&device, crate::GpuFeature::GpuSkinning) {
            return;
        }
        let skinning = &mut super::Skinning::new(&device, compute);
        let vertex = |p: [f32; 3]| super::Vertex {
            position: p,
            normal: [0.0, 0.0, 1.0],
            uv: [0.25, 0.5],
            color: [1, 2, 3, 4],
            uv1: [7.0, 8.0],
        };
        let rest = [
            vertex([0.0, 0.0, 0.0]),
            vertex([1.0, 0.0, 0.0]),
            vertex([0.0, 1.0, 0.0]),
        ];
        let joints = [[0u16, 0, 0, 0], [0, 1, 0, 0], [1, 0, 0, 0]];
        let weights = [[1.0f32, 0.0, 0.0, 0.0], [0.5, 0.5, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]];
        // Target 0 lifts corner 1, target 1 pushes corner 2 along x and
        // tilts its normal.
        let delta = |p: [f32; 3], n: [f32; 3]| super::Delta { position: p, normal: n };
        let none = delta([0.0; 3], [0.0; 3]);
        let deltas = vec![
            none,
            delta([0.0, 2.0, 0.0], [0.0; 3]),
            none,
            none,
            none,
            delta([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        ];
        let skin = skinning.create(&device, &rest, &joints, &weights, deltas.clone());
        let rest_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&rest),
            usage: wgpu::BufferUsages::VERTEX | skinning.rest_usage(),
        });
        let mut frame = crate::three::SkinFrame::default();
        // Another model's matrix first, so `first` matters.
        frame.matrices = vec![
            Mat4::from_scale(Vec3::splat(9.0)),
            Mat4::from_translation(Vec3::new(0.0, 0.0, 2.0)),
            Mat4::from_rotation_translation(Quat::from_rotation_z(1.0), Vec3::X),
        ];
        frame.draws.push(crate::three::SkinDraw {
            mesh: 0,
            instance: Default::default(),
            entity: 0,
            matrix: Mat4::IDENTITY,
            scale: 1.0,
            bounds: (Vec3::ZERO, 1.0),
            first: 1,
            morph_first: 1,
            morph_count: 2,
            material: u32::MAX,
            parts: (0, 0),
        });
        // Another model's weight first, so `morph_first` matters.
        frame.weights = vec![9.0, 0.5, 0.25];
        skinning.prepare(&device, &queue, &frame, |_| Some((&rest_buffer, &skin)), |_| true);
        let mut encoder = device.create_command_encoder(&Default::default());
        skinning.record(&mut encoder, None);
        let size = (rest.len() * size_of::<super::Vertex>()) as u64;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(skinning.vertices(0).unwrap(), 0, &readback, 0, size);
        queue.submit([encoder.finish()]);
        readback.slice(..).map_async(wgpu::MapMode::Read, |r| r.unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let posed: Vec<super::Vertex> = bytemuck::cast_slice(&readback.slice(..).get_mapped_range().unwrap()).to_vec();
        for i in 0..3 {
            let m = crate::three::blend_skin(&frame.matrices[1..], joints[i], weights[i]);
            let shaped = Vec3::from(rest[i].position)
                + 0.5 * Vec3::from(deltas[i].position)
                + 0.25 * Vec3::from(deltas[3 + i].position);
            let want = m.transform_point3(shaped);
            assert!(
                Vec3::from(posed[i].position).abs_diff_eq(want, 1e-5),
                "compute {compute}, corner {i}: {:?} vs {want}",
                posed[i].position
            );
            let n = Vec3::from(posed[i].normal);
            let shaped_n = Vec3::from(rest[i].normal) + 0.25 * Vec3::from(deltas[3 + i].normal);
            let want_n = m.transform_vector3(shaped_n).normalize();
            assert!(n.abs_diff_eq(want_n, 1e-5), "corner {i}'s normal: {n} vs {want_n}");
            assert_eq!(
                (posed[i].uv, posed[i].color, posed[i].uv1),
                (rest[i].uv, rest[i].color, rest[i].uv1)
            );
        }
    }

    #[test]
    fn influences_are_six_words() {
        assert_eq!(size_of::<super::Influence>(), 24, "skin.wgsl reads 6 words per corner");
        assert_eq!(
            size_of::<super::Delta>(),
            24,
            "skin.wgsl reads 6 words per morph offset"
        );
        assert_eq!(
            size_of::<super::super::Vertex>(),
            44,
            "skin.wgsl reads 11 words per vertex"
        );
    }
}
