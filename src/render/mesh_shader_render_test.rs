//! Mesh and task shaders (issue #164): what this machine's adapter reports,
//! and, where it reports them, a prototype that draws meshlets through a
//! mesh-shader pipeline next to the indexed indirect draws and distance
//! levels the engine's instance batches use. Offscreen, no window.
//!
//! Nothing here touches the renderer: the default pipelines never ask for a
//! mesh-shader feature. The measured recommendation is in
//! `crates/ponko_bench/MESH_SHADERS.md`; the benchmark is
//! `mesh_shader_benchmark` in `mesh_shader_bench.rs`.
use super::mesh_shader_proto::{self as proto, Path, Rig, Scene};
use super::render_test_support as support;
use std::cell::RefCell;

/// What one adapter reports about mesh shaders: one row of the support
/// matrix in `MESH_SHADERS.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MeshReport {
    adapter: String,
    backend: String,
    device_type: String,
    mesh_shader: bool,
    mesh_shader_multiview: bool,
    mesh_shader_points: bool,
    passthrough_shaders: bool,
    subgroup: bool,
    timestamp_query: bool,
    indirect_first_instance: bool,
    multi_draw_indirect: bool,
    /// The limits the adapter reports, zero where it has none.
    task_workgroups_per_dimension: u32,
    task_workgroup_total: u32,
    task_invocations: u32,
    task_payload_bytes: u32,
    mesh_workgroups_per_dimension: u32,
    mesh_workgroup_total: u32,
    mesh_invocations: u32,
    mesh_output_vertices: u32,
    mesh_output_primitives: u32,
    mesh_output_layers: u32,
}

impl MeshReport {
    fn of(adapter: &wgpu::Adapter) -> Self {
        let info = adapter.get_info();
        let features = adapter.features();
        let limits = adapter.limits();
        Self {
            adapter: info.name,
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
            mesh_shader: features.contains(wgpu::Features::EXPERIMENTAL_MESH_SHADER),
            mesh_shader_multiview: features.contains(wgpu::Features::EXPERIMENTAL_MESH_SHADER_MULTIVIEW),
            mesh_shader_points: features.contains(wgpu::Features::EXPERIMENTAL_MESH_SHADER_POINTS),
            passthrough_shaders: features.contains(wgpu::Features::PASSTHROUGH_SHADERS),
            subgroup: features.contains(wgpu::Features::SUBGROUP),
            timestamp_query: features.contains(wgpu::Features::TIMESTAMP_QUERY),
            indirect_first_instance: features.contains(wgpu::Features::INDIRECT_FIRST_INSTANCE),
            multi_draw_indirect: features.contains(wgpu::Features::MULTI_DRAW_INDIRECT_COUNT),
            task_workgroups_per_dimension: limits.max_task_workgroups_per_dimension,
            task_workgroup_total: limits.max_task_workgroup_total_count,
            task_invocations: limits.max_task_invocations_per_workgroup,
            task_payload_bytes: limits.max_task_payload_size,
            mesh_workgroups_per_dimension: limits.max_mesh_workgroups_per_dimension,
            mesh_workgroup_total: limits.max_mesh_workgroup_total_count,
            mesh_invocations: limits.max_mesh_invocations_per_workgroup,
            mesh_output_vertices: limits.max_mesh_output_vertices,
            mesh_output_primitives: limits.max_mesh_output_primitives,
            mesh_output_layers: limits.max_mesh_output_layers,
        }
    }

    /// What must hold for any adapter, whatever it reports: the dependent
    /// features need the base one, and a mesh-shader adapter states limits
    /// a pipeline can be built against.
    fn contradictions(&self) -> Vec<&'static str> {
        let mut found = Vec::new();
        if self.mesh_shader_multiview && !self.mesh_shader {
            found.push("multiview mesh shaders without mesh shaders");
        }
        if self.mesh_shader_points && !self.mesh_shader {
            found.push("point outputs without mesh shaders");
        }
        if self.mesh_shader {
            if self.mesh_invocations == 0 {
                found.push("mesh shaders with no mesh invocations");
            }
            if self.mesh_output_vertices == 0 || self.mesh_output_primitives == 0 {
                found.push("mesh shaders with no output vertices or primitives");
            }
            if self.mesh_workgroups_per_dimension == 0 || self.mesh_workgroup_total == 0 {
                found.push("mesh shaders with no workgroups to dispatch");
            }
        }
        found
    }

    /// One line, the same fields as a row of the matrix.
    fn line(&self) -> String {
        format!(
            "mesh-shader probe: {} ({}, {}): mesh_shader={} multiview={} points={} passthrough={} subgroup={} \
             timestamps={} indirect_first_instance={} multi_draw_count={} | task: groups/dim {} total {} threads {} \
             payload {} B | mesh: groups/dim {} total {} threads {} vertices {} primitives {} layers {}",
            self.adapter,
            self.backend,
            self.device_type,
            self.mesh_shader,
            self.mesh_shader_multiview,
            self.mesh_shader_points,
            self.passthrough_shaders,
            self.subgroup,
            self.timestamp_query,
            self.indirect_first_instance,
            self.multi_draw_indirect,
            self.task_workgroups_per_dimension,
            self.task_workgroup_total,
            self.task_invocations,
            self.task_payload_bytes,
            self.mesh_workgroups_per_dimension,
            self.mesh_workgroup_total,
            self.mesh_invocations,
            self.mesh_output_vertices,
            self.mesh_output_primitives,
            self.mesh_output_layers,
        )
    }
}

#[test]
fn the_adapter_reports_which_mesh_shader_features_it_has() {
    let report = RefCell::new(None);
    // The device this opens asks for nothing: the probe reads what the
    // adapter offers, it does not use it.
    let Some(_gpu) = support::gpu_with(|adapter| {
        *report.borrow_mut() = Some(MeshReport::of(adapter));
        Ok(wgpu::DeviceDescriptor::default())
    }) else {
        return;
    };
    let report = report.into_inner().expect("the closure ran");
    println!("{}", report.line());
    assert_eq!(report.contradictions(), Vec::<&str>::new(), "{}", report.line());
}

#[test]
fn a_report_that_contradicts_itself_is_caught() {
    // The negative control of the check above.
    let broken = MeshReport {
        adapter: String::new(),
        backend: String::new(),
        device_type: String::new(),
        mesh_shader: false,
        mesh_shader_multiview: true,
        mesh_shader_points: false,
        passthrough_shaders: false,
        subgroup: false,
        timestamp_query: false,
        indirect_first_instance: false,
        multi_draw_indirect: false,
        task_workgroups_per_dimension: 0,
        task_workgroup_total: 0,
        task_invocations: 0,
        task_payload_bytes: 0,
        mesh_workgroups_per_dimension: 0,
        mesh_workgroup_total: 0,
        mesh_invocations: 0,
        mesh_output_vertices: 0,
        mesh_output_primitives: 0,
        mesh_output_layers: 0,
    };
    assert_eq!(broken.contradictions().len(), 1);
    let empty_limits = MeshReport {
        mesh_shader: true,
        mesh_shader_multiview: false,
        ..broken
    };
    assert_eq!(empty_limits.contradictions().len(), 3);
}

/// A rig on this adapter, or `None` (logged) where the adapter has no mesh
/// shaders, or can't compile the prototype's.
fn rig(scene: &Scene) -> Option<Rig> {
    Rig::new(scene)
}

/// A small scene: a few hundred spheres in three levels of detail, so every
/// path has something to draw and something to cull.
fn small_scene() -> Scene {
    Scene {
        size: (192, 108),
        columns: 24,
        rows: 24,
        spacing: 1.4,
        lod0_segments: 24,
        bands: (16.0, 26.0),
        far: 60.0,
        ..Scene::default()
    }
}

#[test]
fn meshlets_hold_every_triangle_once_within_the_pipeline_limits() {
    // 12 to 96 are the benchmark's levels; 6 and 64 are odd sizes.
    for segments in [6, 12, 24, 48, 64, 96] {
        let data = crate::render::MeshData::sphere(0.5, segments);
        let meshlets = proto::build_meshlets(&data);
        eprintln!(
            "{segments} segments: {} triangles in {} meshlets, {:.1} triangles each on average (limit {})",
            meshlets.triangle_total(),
            meshlets.table.len(),
            meshlets.average_triangles(),
            proto::MESHLET_TRIANGLES
        );
        assert_eq!(
            meshlets.triangle_total(),
            data.indices.len() / 3,
            "{segments} segments: no triangle is lost or doubled"
        );
        for m in &meshlets.table {
            assert!(m.vertex_count <= proto::MESHLET_VERTICES, "{segments} segments");
            assert!(m.triangle_count <= proto::MESHLET_TRIANGLES, "{segments} segments");
            assert!(m.vertex_count > 0 && m.triangle_count > 0);
        }
        // Rebuilding every triangle's corner positions from the meshlet's
        // own lists gives the mesh back.
        let mut rebuilt = meshlets.corner_positions(&data);
        let mut original: Vec<[u32; 9]> = data
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let mut c = [0u32; 9];
                for (k, i) in t.iter().enumerate() {
                    for (a, v) in data.positions[*i as usize].to_array().iter().enumerate() {
                        c[k * 3 + a] = v.to_bits();
                    }
                }
                c
            })
            .collect();
        rebuilt.sort_unstable();
        original.sort_unstable();
        assert_eq!(rebuilt, original, "{segments} segments");
    }
}

#[test]
fn the_cpu_replica_of_the_culling_counts_what_the_gpu_culled() {
    let Some(mut rig) = rig(&small_scene()) else {
        return;
    };
    for frame in [0, 40] {
        let expected = rig.scene.expected_visible(frame);
        let drawn = rig.indirect_instances(frame);
        assert_eq!(drawn.iter().sum::<u32>(), expected.iter().sum::<u32>(), "frame {frame}");
        assert_eq!(drawn, expected, "per level, frame {frame}");
        assert!(
            expected.iter().all(|&n| n > 0),
            "every level has instances: {expected:?}"
        );
        assert!(
            expected.iter().sum::<u32>() < rig.scene.instances(),
            "the frustum culls something: {expected:?}"
        );
    }
}

#[test]
fn the_mesh_shader_path_draws_the_picture_the_indexed_path_draws() {
    let Some(mut rig) = rig(&small_scene()) else {
        return;
    };
    let tolerance = support::Tolerance {
        channel: 24,
        outliers: 0.004,
        edge: 1,
    };
    for frame in [0, 40] {
        let indexed = rig.image(Path::Indirect, frame);
        let lit = |image: &support::Image| {
            image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[0] > 10 || p[1] > 10 || p[2] > 10)
                .count()
        };
        assert!(lit(&indexed) > 1000, "the scene draws something: {}", lit(&indexed));
        for path in [Path::Mesh, Path::MeshCone, Path::MeshIndirect] {
            let image = rig.image(path, frame);
            let d = support::diff_within(&indexed, &image, tolerance);
            eprintln!("{path:?} against Indirect, frame {frame}: {d}");
            assert!(d.within(tolerance), "{path:?}, frame {frame}: {d}");
        }
        // The same without any culling or distance levels.
        let plain = rig.image(Path::IndirectPlain, frame);
        let plain_mesh = rig.image(Path::MeshPlain, frame);
        let d = support::diff_within(&plain, &plain_mesh, tolerance);
        eprintln!("MeshPlain against IndirectPlain, frame {frame}: {d}");
        assert!(d.within(tolerance), "plain paths, frame {frame}: {d}");
    }
}

#[test]
fn the_comparison_can_tell_paths_apart() {
    // Negative control: a picture of another view differs from this one, so
    // the check above would fail on a path that drew the wrong thing.
    let Some(mut rig) = rig(&small_scene()) else {
        return;
    };
    let tolerance = support::Tolerance {
        channel: 24,
        outliers: 0.004,
        edge: 1,
    };
    let now = rig.image(Path::Indirect, 0);
    let later = rig.image(Path::Indirect, 60);
    let d = support::diff_within(&now, &later, tolerance);
    eprintln!("frame 0 against frame 60: {d}");
    assert!(!d.within(tolerance), "different frames differ: {d}");
}

#[test]
fn the_cone_and_meshlet_culling_drops_triangles_the_plain_path_draws() {
    let Some(mut rig) = rig(&small_scene()) else {
        return;
    };
    let culled = rig.mesh_triangles(Path::Mesh, 0);
    let coned = rig.mesh_triangles(Path::MeshCone, 0);
    let plain = rig.mesh_triangles(Path::MeshPlain, 0);
    eprintln!("triangles through the mesh stage: plain {plain}, culled {culled}, with cones {coned}");
    assert!(
        culled < plain,
        "frustum and distance levels cut work: {culled} against {plain}"
    );
    assert!(coned < culled, "cones cut back faces: {coned} against {culled}");
}

#[test]
fn unchecked_mesh_shaders_draw_what_the_checked_ones_draw() {
    // The unchecked build (`create_shader_module_trusted`, no workgroup
    // memory zeroing and no bounds checks) is the one the mesh-shader docs
    // recommend; the prototype's shaders stay in bounds, so it must draw the
    // same picture.
    let Some(mut rig) = rig(&small_scene()) else {
        return;
    };
    let checked = rig.image(Path::MeshCone, 40);
    rig.set_checked(false);
    let unchecked = rig.image(Path::MeshCone, 40);
    let d = support::diff(&checked, &unchecked, 0);
    eprintln!("unchecked against checked: {d}");
    assert_eq!(d.outliers, 0, "{d}");
}

#[test]
fn the_default_renderer_asks_for_no_mesh_shader_feature() {
    // Mesh shaders are a prototype outside the renderer: the engine's own
    // device and passes (and so every WebGL, WebGPU and older-GPU path) must
    // neither request them nor opt in to experimental features, anywhere in
    // the renderer's sources. Only the prototype's own modules may, and the
    // offscreen render tests (`*_render_test.rs`, compiled only for tests),
    // such as the ray-tracing evaluation's probe and prototype.
    const NEEDLES: [&str; 4] = [
        "MESH_SHADER",
        "ExperimentalFeatures",
        "create_mesh_pipeline",
        "draw_mesh_tasks",
    ];
    let prototype = |name: &str| {
        [
            "src/render/mesh_shader_proto.rs",
            "src/render/mesh_shader_bench.rs",
            "src/render/mesh_shader_render_test.rs",
        ]
        .contains(&name)
            || name.ends_with("_render_test.rs")
    };
    let mut files = rust_sources("src/render");
    files.extend(rust_sources("src/three"));
    assert!(
        files.iter().any(|(name, _)| name == "src/render/gpu.rs")
            && files.iter().any(|(name, _)| name.starts_with("src/render/gpu3d/"))
            && files.iter().any(|(name, _)| name.starts_with("src/three/")),
        "the walk finds the renderer's sources, nested folders too: {} files",
        files.len()
    );
    for (name, text) in &files {
        if prototype(name) {
            continue;
        }
        for needle in NEEDLES {
            assert!(!text.contains(needle), "{name} mentions {needle}");
        }
    }
    // The negative control: the prototype the scan leaves out does mention them.
    let (_, proto_text) = files
        .iter()
        .find(|(name, _)| name == "src/render/mesh_shader_proto.rs")
        .expect("the prototype is found");
    assert!(NEEDLES.iter().all(|needle| proto_text.contains(needle)));
}

/// Every Rust file under `folder` (relative to the package), nested folders
/// too, with its path (`/`-separated) and text.
fn rust_sources(folder: &str) -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, root: &std::path::Path, found: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, root, found);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                found.push((name, std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    walk(&root.join(folder), root, &mut found);
    found.sort();
    found
}

#[test]
fn the_mesh_path_draws_the_same_for_any_number_of_instances_per_task_workgroup() {
    let Some(mut rig) = rig(&small_scene()) else {
        return;
    };
    // The reference is the vertex pipeline's picture, so the comparison
    // takes the cross-pipeline tolerance: rasterisation and interpolation
    // may differ by a level between pipeline types on some drivers.
    let tolerance = support::Tolerance {
        channel: 24,
        outliers: 0.004,
        edge: 1,
    };
    let reference = rig.image(Path::Indirect, 40);
    let most = rig.instances_per_task();
    let mut mesh_images = Vec::new();
    for per_task in [1, 5, most] {
        rig.set_instances_per_task(per_task);
        let image = rig.image(Path::Mesh, 40);
        let d = support::diff_within(&reference, &image, tolerance);
        eprintln!("{per_task} instances per task workgroup, against Indirect: {d}");
        assert!(d.within(tolerance), "{per_task} per workgroup: {d}");
        mesh_images.push((per_task, image));
    }
    // Between mesh pipelines on one device, only the grouping changes: the
    // pictures are identical.
    let (_, first) = &mesh_images[0];
    for (per_task, image) in &mesh_images[1..] {
        let d = support::diff(first, image, 0);
        eprintln!("{per_task} instances per task workgroup, against 1: {d}");
        assert_eq!(d.outliers, 0, "{per_task} per workgroup against 1: {d}");
    }
}

#[test]
fn the_indirect_mesh_dispatch_fits_the_scene_and_the_check_can_fail() {
    let Some(rig) = rig(&small_scene()) else {
        return;
    };
    assert_eq!(rig.indirect_mesh_fits([0, 40]), Ok(()));
    // Negative control, on the limits of an Apple GPU: 40 meshlets of 30,000
    // kept instances pass the million workgroups a dispatch may have, and a
    // million and a half instances pass one dimension.
    assert!(proto::dispatch_fits_apple9(40, 20_000));
    assert!(!proto::dispatch_fits_apple9(40, 30_000));
    assert!(!proto::dispatch_fits_apple9(1, 1_500_000));
}
