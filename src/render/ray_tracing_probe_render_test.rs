//! What this machine's adapter offers for hardware ray tracing through the
//! wgpu version the engine builds against: one measured row of the support
//! matrix in `crates/ponko_bench/RAY_TRACING.md`. Nothing here touches the
//! engine's renderer; skips without a GPU.
//!
//! Print the row with `cargo test --lib --features 3d ray_tracing_probe --
//! --nocapture`.

/// The ray tracing features wgpu names, with the names the matrix uses.
const RAY_TRACING_FEATURES: [(&str, wgpu::Features); 5] = [
    ("EXPERIMENTAL_RAY_QUERY", wgpu::Features::EXPERIMENTAL_RAY_QUERY),
    (
        "EXPERIMENTAL_RAY_TRACING_PIPELINES",
        wgpu::Features::EXPERIMENTAL_RAY_TRACING_PIPELINES,
    ),
    (
        "EXPERIMENTAL_RAY_HIT_VERTEX_RETURN",
        wgpu::Features::EXPERIMENTAL_RAY_HIT_VERTEX_RETURN,
    ),
    (
        "EXTENDED_ACCELERATION_STRUCTURE_VERTEX_FORMATS",
        wgpu::Features::EXTENDED_ACCELERATION_STRUCTURE_VERTEX_FORMATS,
    ),
    (
        "ACCELERATION_STRUCTURE_BINDING_ARRAY",
        wgpu::Features::ACCELERATION_STRUCTURE_BINDING_ARRAY,
    ),
];

/// What the adapter reports, read inside the device request.
struct Probe {
    info: wgpu::AdapterInfo,
    features: wgpu::Features,
    limits: wgpu::Limits,
}

/// The adapter's report, or `None` without a GPU (the test returns early).
/// The device opened alongside asks for nothing beyond the defaults.
fn probe() -> Option<Probe> {
    let mut seen = None;
    crate::render::render_test_support::gpu_with(|adapter| {
        seen = Some(Probe {
            info: adapter.get_info(),
            features: adapter.features(),
            limits: adapter.limits(),
        });
        Ok(wgpu::DeviceDescriptor::default())
    })?;
    seen
}

#[test]
fn the_adapter_reports_its_ray_tracing_features() {
    let Some(p) = probe() else {
        return;
    };
    let l = &p.limits;
    eprintln!(
        "ray tracing probe: {} ({:?}, {:?})",
        p.info.name, p.info.backend, p.info.device_type
    );
    for (name, flag) in RAY_TRACING_FEATURES {
        eprintln!("  {name}: {}", p.features.contains(flag));
    }
    eprintln!(
        "  limits: max_blas_primitive_count {}, max_blas_geometry_count {}, max_tlas_instance_count {}, \
         max_acceleration_structures_per_shader_stage {}, max_ray_recursion_depth {}",
        l.max_blas_primitive_count,
        l.max_blas_geometry_count,
        l.max_tlas_instance_count,
        l.max_acceleration_structures_per_shader_stage,
        l.max_ray_recursion_depth
    );
    eprintln!(
        "  BLAS vertex formats: {:?}",
        p.features.allowed_vertex_formats_for_blas()
    );

    let ray_query = p.features.contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY);
    // An adapter that has ray queries has room for a scene's acceleration
    // structures.
    if ray_query {
        assert!(
            l.max_blas_primitive_count > 0 && l.max_tlas_instance_count > 0,
            "ray queries without room for acceleration structures: {l:?}"
        );
        assert!(l.max_acceleration_structures_per_shader_stage >= 1);
    }
    // BLASes take plain float positions exactly when the adapter has
    // acceleration structures, which ray queries or ray tracing pipelines
    // bring (Vulkan can report the pipelines without ray queries).
    let structures = p
        .features
        .intersects(wgpu::Features::EXPERIMENTAL_RAY_QUERY | wgpu::Features::EXPERIMENTAL_RAY_TRACING_PIPELINES);
    assert_eq!(
        p.features
            .allowed_vertex_formats_for_blas()
            .contains(&wgpu::VertexFormat::Float32x3),
        structures,
        "Float32x3 BLAS positions with ray query or pipelines, and only then"
    );
    // Only Vulkan, Metal and DX12 implement acceleration structures; the
    // GL backend (and so WebGL2) and the browser's WebGPU never report them.
    if matches!(p.info.backend, wgpu::Backend::Gl | wgpu::Backend::BrowserWebGpu) {
        assert!(!ray_query, "{:?} reports ray queries", p.info.backend);
    }
}

/// Every ray tracing entry point sits behind wgpu's experimental opt-in:
/// requesting one needs `ExperimentalFeatures::enabled()` (an `unsafe`
/// acknowledgement) in the device descriptor, on any adapter. The engine's
/// own device asks for none of them, so nothing changes for anyone today.
#[test]
fn ray_tracing_features_need_wgpu_s_experimental_opt_in() {
    let experimental = wgpu::Features::all_experimental_mask();
    for flag in [
        wgpu::Features::EXPERIMENTAL_RAY_QUERY,
        wgpu::Features::EXPERIMENTAL_RAY_TRACING_PIPELINES,
        wgpu::Features::EXPERIMENTAL_RAY_HIT_VERTEX_RETURN,
    ] {
        assert!(experimental.contains(flag), "{flag:?} is no longer experimental");
    }
    // Both need native wgpu: none is part of WebGPU.
    for (name, flag) in RAY_TRACING_FEATURES {
        assert!(
            !wgpu::Features::all_webgpu_mask().contains(flag),
            "{name} is a WebGPU feature now"
        );
    }
    // The default descriptor (what the engine's device starts from) leaves
    // the opt-in off and asks for no acceleration structures.
    let d = wgpu::DeviceDescriptor::default();
    assert!(!d.experimental_features.is_enabled());
    assert_eq!(d.required_limits.max_tlas_instance_count, 0);
    assert_eq!(d.required_limits.max_blas_primitive_count, 0);
}
