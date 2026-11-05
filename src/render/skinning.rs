//! Skinning on the GPU: each skinned model drawn this frame gets a vertex
//! buffer of its own (kept between frames), which a compute shader fills
//! with its posed corners before any pass draws. Every pass then draws it
//! like a plain mesh, so material shaders need nothing extra. Where the GPU
//! can't run compute shaders (WebGL2), the CPU poses them instead.

use wgpu::util::DeviceExt;

use super::Vertex;
use crate::three::SkinFrame;

/// Corners per workgroup (`@workgroup_size` in skin.wgsl).
const GROUP: u32 = 64;
const SHADER: &str = include_str!("skin.wgsl");

/// What a skinned mesh keeps on the GPU besides its rest-pose vertices.
pub(super) struct GpuSkin {
    /// Per corner: 4 joints as u16 pairs, then 4 weights.
    influences: wgpu::Buffer,
    count: u32,
    /// Without compute shaders: the rest pose and influences, on the CPU.
    cpu: Option<(Vec<Vertex>, Vec<Influence>)>,
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
    /// Its bind group, with the mesh and `joints` generation it was made for.
    group: Option<(u32, u64, wgpu::BindGroup)>,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    first: u32,
    count: u32,
    pad: [u32; 2],
}

pub(super) struct Skinning {
    /// `None` without compute shaders.
    pipeline: Option<(wgpu::ComputePipeline, wgpu::BindGroupLayout)>,
    /// This frame's skin matrices, and how many fit.
    joints: wgpu::Buffer,
    joint_capacity: usize,
    /// Bumped whenever `joints` is made anew (bind groups name it).
    generation: u64,
    /// One `Params` per slot, `stride` bytes apart.
    params: wgpu::Buffer,
    param_capacity: usize,
    stride: u64,
    slots: Vec<Slot>,
    /// This frame, per slot: the skinned mesh, and whether any pass draws it.
    pub draws: Vec<(u32, bool)>,
    scratch: Vec<Vertex>,
    params_scratch: Vec<u8>,
    matrix_scratch: Vec<[f32; 16]>,
}

impl Skinning {
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
        Self {
            pipeline,
            joints: joint_buffer(device, 64),
            joint_capacity: 64,
            generation: 0,
            params: param_buffer(device, stride, 4),
            param_capacity: 4,
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

    /// The GPU side of a skinned mesh of `vertices`.
    pub fn create(
        &self,
        device: &wgpu::Device,
        vertices: &[Vertex],
        joints: &[[u16; 4]],
        weights: &[[f32; 4]],
    ) -> GpuSkin {
        let influences: Vec<Influence> = joints
            .iter()
            .zip(weights)
            .map(|(&joints, &weights)| Influence { joints, weights })
            .collect();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3d skin influences"),
            contents: bytemuck::cast_slice(&influences),
            usage: wgpu::BufferUsages::STORAGE,
        });
        GpuSkin {
            influences: buffer,
            count: vertices.len() as u32,
            cpu: self.pipeline.is_none().then(|| (vertices.to_vec(), influences)),
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
        let matrices = skins.matrices.len().max(1);
        if matrices > self.joint_capacity {
            self.joint_capacity = matrices.next_power_of_two();
            self.joints = joint_buffer(device, self.joint_capacity);
            self.generation += 1;
        }
        if self.pipeline.is_some() && !skins.matrices.is_empty() {
            self.matrix_scratch.clear();
            self.matrix_scratch
                .extend(skins.matrices.iter().map(glam::Mat4::to_cols_array));
            queue.write_buffer(&self.joints, 0, bytemuck::cast_slice(&self.matrix_scratch));
        }
        if self.draws.len() > self.param_capacity {
            self.param_capacity = self.draws.len().next_power_of_two();
            self.params = param_buffer(device, self.stride, self.param_capacity);
            for slot in &mut self.slots {
                slot.group = None;
            }
        }
        self.params_scratch.clear();
        self.params_scratch.resize(self.stride as usize * self.draws.len(), 0);
        for (k, d) in skins.draws.iter().enumerate() {
            let count = mesh(d.mesh).map_or(0, |(_, s)| s.count);
            let p = Params {
                first: d.first,
                count,
                pad: [0; 2],
            };
            let at = k * self.stride as usize;
            self.params_scratch[at..at + size_of::<Params>()].copy_from_slice(bytemuck::bytes_of(&p));
            if !self.draws[k].1 {
                continue;
            }
            if k == self.slots.len() {
                self.slots.push(Slot {
                    buffer: slot_buffer(device, count.max(1), self.rest_usage()),
                    capacity: count.max(1),
                    group: None,
                });
            }
            let usage = self.rest_usage();
            let slot = &mut self.slots[k];
            if slot.capacity < count {
                slot.capacity = count.next_power_of_two();
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
                                entry(2, &self.joints),
                                entry(3, &slot.buffer),
                                wgpu::BindGroupEntry {
                                    binding: 4,
                                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                        buffer: &self.params,
                                        offset: 0,
                                        size: wgpu::BufferSize::new(size_of::<Params>() as u64),
                                    }),
                                },
                            ],
                        });
                        slot.group = Some((d.mesh, self.generation, group));
                    }
                }
                (None, Some((vertices, influences))) => {
                    let skin_matrices = &skins.matrices[d.first as usize..];
                    self.scratch.clear();
                    self.scratch.extend(vertices.iter().zip(influences).map(|(v, i)| {
                        let m = crate::three::blend_skin(skin_matrices, i.joints, i.weights);
                        let p = m.transform_point3(glam::Vec3::from(v.position));
                        let n = m
                            .transform_vector3(glam::Vec3::from(v.normal))
                            .normalize_or(glam::Vec3::Y);
                        Vertex {
                            position: p.to_array(),
                            normal: n.to_array(),
                            ..*v
                        }
                    }));
                    queue.write_buffer(&slot.buffer, 0, bytemuck::cast_slice(&self.scratch));
                }
                (None, None) => {}
            }
        }
        if self.pipeline.is_some() {
            queue.write_buffer(&self.params, 0, &self.params_scratch);
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
            let Some(Some((m, _, group))) = self.slots.get(k).map(|s| s.group.as_ref()) else {
                continue;
            };
            if !used || *m != mesh {
                continue;
            }
            pass.set_bind_group(0, group, &[(k as u64 * self.stride) as u32]);
            pass.dispatch_workgroups(self.slots[k].capacity.div_ceil(GROUP), 1, 1);
        }
    }

    /// Whether this frame poses anything on the GPU.
    pub fn has_work(&self) -> bool {
        self.pipeline.is_some() && self.draws.iter().any(|d| d.1)
    }

    /// Skinned model `k`'s posed vertices, if it's drawn this frame.
    pub fn vertices(&self, k: usize) -> Option<&wgpu::Buffer> {
        let &(_, used) = self.draws.get(k)?;
        used.then(|| self.slots.get(k).map(|s| &s.buffer)).flatten()
    }
}

fn entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn joint_buffer(device: &wgpu::Device, matrices: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("3d skin matrices"),
        size: (matrices * 64) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn param_buffer(device: &wgpu::Device, stride: u64, slots: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("3d skinning params"),
        size: stride * slots as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn slot_buffer(device: &wgpu::Device, corners: u32, storage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
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

    /// Poses three corners on the GPU (when there is one) and on the CPU:
    /// both must agree.
    #[test]
    fn gpu_skinning_matches_the_cpu() {
        use glam::{Mat4, Quat, Vec3};
        use wgpu::util::DeviceExt;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            eprintln!("no GPU adapter: skipped");
            return;
        };
        let Ok((device, queue)) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) else {
            eprintln!("no GPU device: skipped");
            return;
        };
        let skinning = &mut super::Skinning::new(&device, super::super::can_compute(&device));
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
        let skin = skinning.create(&device, &rest, &joints, &weights);
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
            matrix: Mat4::IDENTITY,
            scale: 1.0,
            bounds: (Vec3::ZERO, 1.0),
            first: 1,
        });
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
            let want = m.transform_point3(Vec3::from(rest[i].position));
            assert!(
                Vec3::from(posed[i].position).abs_diff_eq(want, 1e-5),
                "corner {i}: {:?} vs {want}",
                posed[i].position
            );
            let n = Vec3::from(posed[i].normal);
            assert!((n.length() - 1.0).abs() < 1e-5);
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
            size_of::<super::super::Vertex>(),
            44,
            "skin.wgsl reads 11 words per vertex"
        );
    }
}
