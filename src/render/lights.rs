//! Camera-local light textures, grown only when a new high-water mark is reached.
use crate::three::lights::{LIST_WIDTH, LightGrid};

pub(super) struct LightTextures {
    pub records: wgpu::Texture,
    pub indices: wgpu::Texture,
    pub records_view: wgpu::TextureView,
    pub indices_view: wgpu::TextureView,
}
impl LightTextures {
    pub fn new(device: &wgpu::Device) -> Self {
        let records = texture(device, "punctual lights", 4, 1, wgpu::TextureFormat::Rgba32Float);
        let indices = texture(
            device,
            "light clusters",
            LIST_WIDTH as u32,
            8,
            wgpu::TextureFormat::R32Uint,
        );
        let records_view = records.create_view(&Default::default());
        let indices_view = indices.create_view(&Default::default());
        Self {
            records,
            indices,
            records_view,
            indices_view,
        }
    }
    /// True when bindings must be rebuilt (growth only).
    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, grid: &LightGrid) -> bool {
        let mut changed = false;
        for (target, view, height) in [
            (
                &mut self.records,
                &mut self.records_view,
                grid.lights.len().max(1) as u32,
            ),
            (
                &mut self.indices,
                &mut self.indices_view,
                (grid.lists.len() / LIST_WIDTH).max(1) as u32,
            ),
        ] {
            if height > target.height() {
                let height = height.next_power_of_two();
                assert!(
                    height <= device.limits().max_texture_dimension_2d,
                    "punctual light texture exceeds adapter capacity"
                );
                *target = texture(device, "grown light data", target.width(), height, target.format());
                *view = target.create_view(&Default::default());
                changed = true;
            }
        }
        write(
            queue,
            &self.records,
            bytemuck::cast_slice(&grid.lights),
            grid.lights.len() as u32,
            64,
        );
        write(
            queue,
            &self.indices,
            bytemuck::cast_slice(&grid.lists),
            (grid.lists.len() / LIST_WIDTH) as u32,
            LIST_WIDTH as u32 * 4,
        );
        changed
    }
}
fn texture(device: &wgpu::Device, label: &str, width: u32, height: u32, format: wgpu::TextureFormat) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}
fn write(queue: &wgpu::Queue, texture: &wgpu::Texture, data: &[u8], height: u32, row: u32) {
    if height == 0 {
        return;
    }
    queue.write_texture(
        texture.as_image_copy(),
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(row),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: texture.width(),
            height,
            depth_or_array_layers: 1,
        },
    );
}
