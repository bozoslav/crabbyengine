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
    let Ok((device, _queue)) =
        pollster::block_on(adapter.request_device(&crate::render::render_test_support::forced(&Default::default())))
    else {
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

/// Material pipeline keys built so far whose module is `source`.
fn keys_of(renderer: &Renderer3D, source: &str) -> Vec<ShaderVariant> {
    let mut variants: Vec<_> = renderer
        .built_pipeline_keys()
        .into_iter()
        .filter(|k| &*k.set.module.source == source)
        .map(|k| k.variant)
        .collect();
    variants.sort_by_key(|v| ShaderVariant::ALL.iter().position(|a| a == v));
    variants
}

#[test]
fn identical_shaders_share_one_compile_and_one_pipeline_set() {
    let Some((device, mut renderer)) = rig() else {
        return;
    };
    let before = renderer.pipeline_stats();
    let a = renderer.create_shader(&device, "a.wgsl", RED).unwrap();
    let b = renderer.create_shader(&device, "b.wgsl", RED).unwrap();
    assert_ne!(a, b, "each shader keeps its own id");
    assert_eq!(pipelines(&renderer, a), pipelines(&renderer, b), "built once, shared");
    let after = renderer.pipeline_stats();
    assert_eq!(after.compiles - before.compiles, 1, "the second compiled nothing");
    assert_eq!(after.shared - before.shared, 1);
    assert_eq!(after.material_shaders - before.material_shaders, 1);
    // The plain one- and double-sided sets, with 4x multisampling's two
    // alpha-to-coverage pipelines each.
    assert_eq!(after.variant_sets - before.variant_sets, 2);
    assert_eq!(after.pipelines - before.pipelines, 20);
    assert_eq!(after.key_bytes - before.key_bytes, RED.len());
}

#[test]
fn a_reload_invalidates_only_the_reloaded_shaders_variants() {
    let Some((device, mut renderer)) = rig() else {
        return;
    };
    let red = renderer.create_shader(&device, "red.wgsl", RED).unwrap();
    let blue = renderer.create_shader(&device, "blue.wgsl", BLUE).unwrap();
    // Each gets a crossfade variant built (as a frame drawing one would).
    let fade = ShaderVariant::new(Sides::One, Instancing::Crossfade);
    let red_fade = renderer.shader_pipeline(red, fade, PipelineRole::Opaque).cloned();
    renderer.shader_pipeline(blue, fade, PipelineRole::Opaque);
    let (red_before, before) = (pipelines(&renderer, red), renderer.pipeline_stats());
    assert_eq!(keys_of(&renderer, BLUE).len(), 3);

    renderer
        .replace_shader(
            &device,
            blue,
            "blue.wgsl",
            RED.replace("1.0, 0.0, 0.0", "0.0, 1.0, 0.0").as_str(),
        )
        .unwrap();
    let after = renderer.pipeline_stats();
    assert_eq!(
        after.invalidated - before.invalidated,
        3,
        "blue's three built sets, nothing else"
    );
    assert!(keys_of(&renderer, BLUE).is_empty(), "the old code's keys are gone");
    assert_eq!(keys_of(&renderer, RED).len(), 3, "red keeps its sets, crossfade too");
    assert_eq!(pipelines(&renderer, red), red_before);
    assert_eq!(
        renderer.shader_pipeline(red, fade, PipelineRole::Opaque).cloned(),
        red_fade,
        "red's lazily built variant survives"
    );
    // The reloaded shader starts with only its plain sets again.
    assert!(!renderer.shader_warm(blue));
}

#[test]
fn reloading_one_of_two_sharing_shaders_leaves_the_other_its_pipelines() {
    let Some((device, mut renderer)) = rig() else {
        return;
    };
    let a = renderer.create_shader(&device, "a.wgsl", RED).unwrap();
    let b = renderer.create_shader(&device, "b.wgsl", RED).unwrap();
    let shared = pipelines(&renderer, b);
    let before = renderer.pipeline_stats();
    renderer.replace_shader(&device, a, "a.wgsl", BLUE).unwrap();
    assert_eq!(pipelines(&renderer, b), shared, "b still draws with the shared set");
    assert_ne!(pipelines(&renderer, a), shared);
    let after = renderer.pipeline_stats();
    assert_eq!(
        after.invalidated, before.invalidated,
        "b holds the set: nothing dropped"
    );
    // Reloading back to code another shader uses compiles nothing.
    renderer.replace_shader(&device, a, "a.wgsl", RED).unwrap();
    assert_eq!(pipelines(&renderer, a), shared);
    let back = renderer.pipeline_stats();
    assert_eq!(back.compiles, after.compiles);
    assert_eq!(back.shared, after.shared + 1);
    assert_eq!(back.invalidated, after.invalidated + 2, "BLUE's plain sets went");
}

#[test]
fn failed_reloads_invalidate_nothing_and_are_timed() {
    let Some((device, mut renderer)) = rig() else {
        return;
    };
    let blue = renderer.create_shader(&device, "blue.wgsl", BLUE).unwrap();
    let before = renderer.pipeline_stats();
    renderer.replace_shader(&device, blue, "blue.wgsl", BROKEN).unwrap_err();
    renderer
        .replace_shader(&device, blue, "blue.wgsl", UNBOUND)
        .unwrap_err();
    let after = renderer.pipeline_stats();
    assert_eq!(after.invalidated, before.invalidated);
    assert_eq!(after.compiles, before.compiles + 2, "failed compiles count");
    assert_eq!(
        after.material_shaders, before.material_shaders,
        "rejected code isn't kept"
    );
    assert!(after.last_compile_ms > 0.0 && after.slowest_compile_ms >= after.last_compile_ms);
}

#[test]
fn freeing_a_shader_lets_go_of_its_pipelines() {
    let Some((device, mut renderer)) = rig() else {
        return;
    };
    let blue = renderer.create_shader(&device, "blue.wgsl", BLUE).unwrap();
    let before = renderer.pipeline_stats();
    renderer.free_shader(blue);
    let after = renderer.pipeline_stats();
    assert_eq!(after.material_shaders, before.material_shaders - 1);
    assert_eq!(after.invalidated, before.invalidated + 2);
}

/// Creating, sharing and reloading material shaders, and building a
/// variant on first use, timed on this adapter (`--nocapture` prints them).
#[test]
fn pipeline_creation_and_reload_stalls_are_measured() {
    let Some((device, mut renderer)) = rig() else {
        return;
    };
    let start = std::time::Instant::now();
    let id = renderer.create_shader(&device, "red.wgsl", RED).unwrap();
    let create = start.elapsed();
    let start = std::time::Instant::now();
    renderer.create_shader(&device, "red again.wgsl", RED).unwrap();
    let shared = start.elapsed();
    let start = std::time::Instant::now();
    renderer.replace_shader(&device, id, "red.wgsl", BLUE).unwrap();
    let reload = start.elapsed();
    let start = std::time::Instant::now();
    renderer.shader_pipeline(
        id,
        ShaderVariant::new(Sides::Both, Instancing::Shared),
        PipelineRole::Opaque,
    );
    let lazy = start.elapsed();
    let s = renderer.pipeline_stats();
    eprintln!(
        "create {:.2} ms, identical create {:.3} ms, reload {:.2} ms, variant on first use {:.2} ms; \
         {} shaders, {} sets, {} pipelines, {} key bytes; builds {} in {:.2} ms (slowest {:.2} ms)",
        create.as_secs_f64() * 1e3,
        shared.as_secs_f64() * 1e3,
        reload.as_secs_f64() * 1e3,
        lazy.as_secs_f64() * 1e3,
        s.material_shaders,
        s.variant_sets,
        s.pipelines,
        s.key_bytes,
        s.builds,
        s.build_ms,
        s.slowest_build_ms,
    );
    assert!(s.builds >= 7 && s.slowest_build_ms > 0.0);
    assert!(shared < create, "sharing skips the compile");
}
