//! Mipmaps for 3D material textures, made on the GPU: each level is the
//! one above it, halved with a bilinear sample in the middle of every 2x2
//! block. Color maps are averaged as light (sRGB decoded), data maps
//! (normals, ORM) as stored.

pub(super) const SHADER: &str = r"
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> Out {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: Out;
    out.clip = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    return out;
}

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var linear_filter: sampler;

@fragment
fn fs(in: Out) -> @location(0) vec4<f32> {
    return textureSampleLevel(source, linear_filter, in.uv, 0.0);
}

// The bytes an sRGB view decoded, back: the first level of a data texture
// made from an sRGB one on a device that can't view it as linear.
fn encode(c: f32) -> f32 {
    return select(1.055 * pow(c, 1.0 / 2.4) - 0.055, c * 12.92, c <= 0.0031308);
}

@fragment
fn fs_encode(in: Out) -> @location(0) vec4<f32> {
    let c = textureSampleLevel(source, linear_filter, in.uv, 0.0);
    return vec4<f32>(encode(c.r), encode(c.g), encode(c.b), c.a);
}
";

/// How many mip levels a `width` x `height` texture has, down to 1x1.
pub(super) fn levels(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

pub(super) struct MipMaker {
    layout: wgpu::BindGroupLayout,
    /// For sRGB (color) and linear (data) views.
    srgb: wgpu::RenderPipeline,
    linear: wgpu::RenderPipeline,
    /// Writes a linear (`Rgba8Unorm`) level the bytes of an sRGB one, see `data_texture`.
    encode: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

impl MipMaker {
    pub fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mipmaps"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mipmaps"),
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mipmaps"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |format: wgpu::TextureFormat, entry: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("mipmaps"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(format.into())],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        Self {
            srgb: pipeline(wgpu::TextureFormat::Rgba8UnormSrgb, "fs"),
            linear: pipeline(wgpu::TextureFormat::Rgba8Unorm, "fs"),
            encode: pipeline(wgpu::TextureFormat::Rgba8Unorm, "fs_encode"),
            layout,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("mipmaps"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
        }
    }

    /// A data texture (normals, ORM) with `levels` mip levels from the
    /// single-level sRGB texture `source`, on a device that can't read an
    /// sRGB texture through a linear view (WebGL2): the texture is made
    /// `Rgba8Unorm` with the same bytes, so it is sampled and averaged as
    /// stored, not decoded as light. (The bytes are written by a draw that
    /// undoes the sRGB decoding, not copied: a copy between the two formats
    /// isn't the same bytes on every API.)
    pub fn data_texture(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::Texture,
        levels: u32,
    ) -> wgpu::Texture {
        let size = source.size();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("material data texture"),
            size,
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mipmaps"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source.create_view(&wgpu::TextureViewDescriptor {
                        mip_level_count: Some(1),
                        ..Default::default()
                    })),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let target = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("data texture level 0"),
            base_mip_level: 0,
            mip_level_count: Some(1),
            ..Default::default()
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("data texture"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.encode);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.generate(device, encoder, &texture, true);
        texture
    }

    /// Fills every level of `texture` below the first from the one above,
    /// in each of its layers.
    pub fn generate(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        linear: bool,
    ) {
        let format = if linear {
            wgpu::TextureFormat::Rgba8Unorm
        } else {
            wgpu::TextureFormat::Rgba8UnormSrgb
        };
        let view = |level, layer| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("mip level"),
                format: Some(format),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_mip_level: level,
                mip_level_count: Some(1),
                base_array_layer: layer,
                array_layer_count: Some(1),
                ..Default::default()
            })
        };
        let layers = texture.depth_or_array_layers();
        for (layer, level) in (0..layers).flat_map(|l| (1..texture.mip_level_count()).map(move |m| (l, m))) {
            let source = view(level - 1, layer);
            let target = view(level, layer);
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("mipmaps"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mipmaps"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(if linear { &self.linear } else { &self.srgb });
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An offscreen device, or `None` without a GPU.
    fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
    }

    /// Mip level `level` of an RGBA8 `texture`, rows unpadded.
    fn read_level(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, level: u32) -> Vec<u8> {
        let (width, height) = ((texture.width() >> level).max(1), (texture.height() >> level).max(1));
        let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        buffer.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let mapped = buffer.get_mapped_range(..).unwrap();
        mapped
            .chunks(row as usize)
            .take(height as usize)
            .flat_map(|r| r[..(width * 4) as usize].iter().copied())
            .collect()
    }

    /// The device without view formats: an sRGB texture holding data (a
    /// normal map's flat blue and a hard black and white edge) becomes a
    /// linear one with the same bytes, and its mip levels average the stored
    /// values, as on a device that can view it as linear.
    #[test]
    fn data_textures_keep_their_bytes_without_view_formats() {
        let Some((device, queue)) = gpu() else {
            eprintln!("no GPU: skipped");
            return;
        };
        // 4x4: red alternates 0 and 200 by column; green and blue are a normal map's flat (128, 255).
        let pixels: Vec<u8> = (0..16)
            .flat_map(|i| [if i % 2 == 0 { 0 } else { 200 }, 128, 255, 255])
            .collect();
        let source = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            source.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(16),
                rows_per_image: Some(4),
            },
            source.size(),
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        let texture = MipMaker::new(&device).data_texture(&device, &mut encoder, &source, levels(4, 4));
        queue.submit([encoder.finish()]);

        assert_eq!(
            texture.format(),
            wgpu::TextureFormat::Rgba8Unorm,
            "read without decoding"
        );
        let base = read_level(&device, &queue, &texture, 0);
        for (i, (got, want)) in base.iter().zip(&pixels).enumerate() {
            assert!(got.abs_diff(*want) <= 1, "byte {i}: {got}, stored {want}");
        }
        // Averaged as stored: (0 + 200) / 2 = 100. As light it would be about 146.
        let level1 = read_level(&device, &queue, &texture, 1);
        for texel in level1.chunks(4) {
            assert!(texel[0].abs_diff(100) <= 2, "red {texel:?}");
            assert!(texel[1].abs_diff(128) <= 1 && texel[2].abs_diff(255) <= 1, "{texel:?}");
        }
    }

    #[test]
    fn levels_go_down_to_one_texel() {
        assert_eq!(levels(1, 1), 1);
        assert_eq!(levels(2, 2), 2);
        assert_eq!(levels(256, 256), 9);
        assert_eq!(levels(300, 7), 9);
        assert_eq!(levels(1024, 2048), 12);
    }
}
