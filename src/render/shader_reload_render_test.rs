//! On a real GPU (offscreen): a material shader reload that fails, in the
//! compiler or when the GPU builds its pipelines, keeps the last good
//! pipelines, and leaves the other shaders alone.
use super::*;

const RED: &str = "fn surface(s: Surface) -> Surface {\n    var out = s;\n    out.albedo = vec3<f32>(1.0, 0.0, 0.0);\n    return out;\n}\n";
const BLUE: &str = "fn surface(s: Surface) -> Surface {\n    var out = s;\n    out.albedo = vec3<f32>(0.0, 0.0, 1.0);\n    return out;\n}\n";
/// A minimal invalid shader: line 3 returns a number, not a `Surface`.
const BROKEN: &str = "// tints everything red\nfn surface(s: Surface) -> Surface {\n    return 1.0;\n}\n";
/// Valid WGSL the GPU can't build pipelines for: a binding no layout has.
const UNBOUND: &str = "@group(3) @binding(7) var<uniform> extra: vec4<f32>;\nfn surface(s: Surface) -> Surface { var out = s; out.albedo *= extra.xyz; return out; }\n";

fn rig() -> Option<(wgpu::Device, Renderer3D)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipped");
        return None;
    };
    eprintln!("shader reload test adapter: {:?}", adapter.get_info());
    let Ok((device, _queue)) = pollster::block_on(adapter.request_device(&Default::default())) else {
        eprintln!("no GPU device: skipped");
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
        label: Some("shader reload test white"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let renderer = Renderer3D::new(&device, &layout, wgpu::TextureFormat::Rgba8Unorm, 0, &white, 4, true);
    Some((device, renderer))
}

/// The pipelines shader `id` draws with now.
fn pipelines(renderer: &Renderer3D, id: u32) -> Vec<wgpu::RenderPipeline> {
    renderer.shaders[id as usize]
        .sides
        .iter()
        .flat_map(|p| {
            [
                p.opaque.clone(),
                p.prepass.clone(),
                p.shadow.clone(),
                p.transparent.clone(),
            ]
        })
        .collect()
}

#[test]
fn failed_reloads_keep_the_last_good_pipelines_and_spare_other_shaders() {
    let Some((device, mut renderer)) = rig() else {
        return;
    };
    let red = renderer.create_shader(&device, "red.wgsl", RED).unwrap();
    let blue = renderer.create_shader(&device, "blue.wgsl", BLUE).unwrap();
    let (red_before, blue_before) = (pipelines(&renderer, red), pipelines(&renderer, blue));

    // The compiler rejects it: where, why, and on which variant.
    let e = renderer.replace_shader(&device, blue, "blue.wgsl", BROKEN).unwrap_err();
    assert_eq!((e.line, e.column, e.in_engine_code), (Some(3), Some(12), false), "{e}");
    assert!(e.variant.starts_with("material shader on this GPU"), "{e}");
    assert_eq!(pipelines(&renderer, blue), blue_before, "the last good version stays");
    assert_eq!(pipelines(&renderer, red), red_before, "other shaders are untouched");

    // The GPU rejects its pipelines: the error comes back, the device lives on.
    let e = renderer
        .replace_shader(&device, blue, "blue.wgsl", UNBOUND)
        .unwrap_err();
    assert!(e.message.contains("rejected"), "{e}");
    assert_eq!(pipelines(&renderer, blue), blue_before);
    assert_eq!(pipelines(&renderer, red), red_before);

    // A fix takes effect, and new shaders still build.
    renderer.replace_shader(&device, blue, "blue.wgsl", RED).unwrap();
    assert_ne!(pipelines(&renderer, blue), blue_before, "the fix replaced them");
    renderer.create_shader(&device, "third.wgsl", BLUE).unwrap();
    assert_eq!(pipelines(&renderer, red), red_before);
}
