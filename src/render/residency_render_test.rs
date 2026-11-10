//! Headless GPU test of what the 3D renderer reports holding
//! (`Ctx::asset_inventory`'s `gpu`): meshes and GPU buffers made and freed
//! in a loop leave nothing behind, and a mesh's bytes match the estimate
//! the engine makes without a GPU. Skips without a GPU.
use super::gpu3d::*;
use crate::*;

fn renderer() -> Option<(wgpu::Device, Renderer3D)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
        crate::render::render_test_support::no_gpu("no GPU adapter");
        return None;
    };
    eprintln!("residency test adapter: {:?}", adapter.get_info());
    let Ok((device, _queue)) = pollster::block_on(adapter.request_device(&crate::render::render_test_support::forced(
        &wgpu::DeviceDescriptor::default(),
    ))) else {
        crate::render::render_test_support::no_gpu("no GPU device");
        return None;
    };
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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
    let white = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let renderer = Renderer3D::new(&device, &layout, wgpu::TextureFormat::Rgba8Unorm, 0, &white, 4, true);
    Some((device, renderer))
}

#[test]
fn freed_meshes_and_buffers_leave_no_gpu_memory() {
    let Some((device, mut renderer)) = renderer() else {
        return;
    };
    let base = renderer.residency();
    let cube = MeshData::cube(1.0);
    let parts = [(0..cube.indices.len() as u32, Material { id: 0 })];
    let id = renderer.create_mesh(&device, &cube, &parts, false);
    let (meshes, bytes, _, _) = renderer.residency();
    assert_eq!(meshes, base.0 + 1);
    assert_eq!(
        bytes - base.1,
        crate::three::mesh_gpu_bytes(&cube),
        "the engine's estimate matches the buffers made"
    );
    renderer.free_mesh(id);
    for _ in 0..50 {
        // Freed ids are made again in place, as the engine reuses them.
        renderer.create_mesh_at(&device, id, &cube, &parts, false);
        renderer.free_mesh(id);
    }
    assert_eq!(renderer.residency(), base);

    renderer.create_gpu_buffer(&device, 0, 64, &[]);
    let (_, _, buffers, buffer_bytes) = renderer.residency();
    if buffers == base.2 {
        eprintln!("no storage buffers on this GPU: buffer part skipped");
        return;
    }
    assert_eq!((buffers, buffer_bytes), (base.2 + 1, base.3 + 64 * 16));
    for _ in 0..50 {
        renderer.free_gpu_buffer(0);
        renderer.create_gpu_buffer(&device, 0, 64, &[]);
    }
    renderer.free_gpu_buffer(0);
    assert_eq!(renderer.residency(), base);
}
