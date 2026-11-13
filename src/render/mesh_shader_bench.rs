//! Offscreen benchmark of the mesh-shader prototype against indexed
//! indirect draws with distance levels (issue #164). No window is opened.
//! Run explicitly, in release:
//! `cargo test --release --lib --features 3d mesh_shader_benchmark -- --ignored --nocapture`
//! It prints one table per scene; the results and how to read them are in
//! `crates/ponko_bench/MESH_SHADERS.md`. It skips where the adapter reports
//! no mesh shaders.
use super::mesh_shader_proto::{FrameTime, Path, Rig, Scene};
use super::static_batch_render_test::{BatchRig, RigSetup};
use crate::prelude::*;
use crate::render::{InstanceBatchOptions, InstanceLevel, MaterialDesc, MeshData};

/// Frames drawn back to back and timed as one: the submit and wait latency
/// (about a millisecond) is shared out over them.
const BATCH: u32 = 4;
const WARM_UP_BATCHES: u32 = 3;
const BATCHES: u32 = 12;
/// Each row is measured this many times; the spread between the rounds'
/// medians is the noise (other work on the machine shows up in it).
const ROUNDS: usize = 5;

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

/// The medians of one round, per frame.
#[derive(Clone, Copy, Default)]
struct Medians {
    cpu: f64,
    gpu: f64,
    cull: Option<f64>,
}

fn round(rig: &mut Rig, path: Path) -> Medians {
    for b in 0..WARM_UP_BATCHES {
        rig.batch(path, b * BATCH, BATCH);
    }
    let times: Vec<FrameTime> = (0..BATCHES).map(|b| rig.batch(path, b * BATCH, BATCH)).collect();
    let of = |f: &dyn Fn(&FrameTime) -> Option<f64>| {
        let mut v: Vec<f64> = times.iter().filter_map(f).collect();
        (!v.is_empty()).then(|| median(&mut v))
    };
    Medians {
        cpu: of(&|t| Some(t.cpu_ms)).unwrap(),
        gpu: of(&|t| Some(t.gpu_ms)).unwrap(),
        cull: of(&|t| t.cull_ms),
    }
}

/// Best (lowest) of the rounds' medians and the worst's excess, as `best (+spread%)`.
fn spread(values: &[f64]) -> String {
    let best = values.iter().copied().fold(f64::MAX, f64::min);
    let worst = values.iter().copied().fold(0.0, f64::max);
    format!("{best:.3} (+{:.0}%)", (worst / best - 1.0) * 100.0)
}

fn mib(bytes: u64) -> String {
    format!("{:.2}", bytes as f64 / (1 << 20) as f64)
}

fn scenarios() -> Vec<(&'static str, Scene)> {
    let default = Scene::default();
    vec![
        ("field: 65,536 spheres, 3 levels (48/24/12 segments)", default.clone()),
        (
            "field: 65,536 spheres, nearest level only (48 segments)",
            Scene {
                bands: (f32::INFINITY, f32::INFINITY),
                far: 45.0,
                ..default.clone()
            },
        ),
        (
            "dense: 16,384 spheres, 3 levels (96/48/24 segments)",
            Scene {
                columns: 128,
                rows: 128,
                spacing: 2.0,
                lod0_segments: 96,
                bands: (25.0, 60.0),
                far: 100.0,
                ..default.clone()
            },
        ),
        (
            "light: 4,096 spheres, 3 levels (48/24/12 segments)",
            Scene {
                columns: 64,
                rows: 64,
                ..default
            },
        ),
    ]
}

/// The medians of the engine's own instance-batch path drawing the scene's
/// spheres: the frame (from before `prepare` until the GPU is done), the
/// CPU steps (prepare, record, submit), and the GPU passes it timestamps.
struct Anchor {
    frame_ms: f64,
    cpu_ms: f64,
    gpu_ms: Option<f64>,
    cull_ms: Option<f64>,
}

/// Draws `scene` through `Renderer3D`'s static instance batches (compute
/// culling, indirect draws, three distance levels), as a game would: a full
/// frame with its shading, depth prepass and 4x multisampling, so it is
/// context for the prototype's numbers and not a like-for-like row.
fn engine_anchor(scene: &Scene) -> Option<Anchor> {
    let mut rig = BatchRig::new(RigSetup {
        size: scene.size,
        compute: true,
        timestamps: true,
        ..Default::default()
    })?;
    rig.keep_order = true;
    let segments = |level: u32| (scene.lod0_segments >> level).max(6);
    let meshes: Vec<_> = (0..3)
        .map(|l| rig.mesh(&MeshData::sphere(0.5, segments(l)), MaterialDesc::default()))
        .collect();
    let transforms: Vec<Mat4> = (0..scene.instances())
        .map(|i| {
            let t = scene.instance(i);
            Mat4::from_scale_rotation_translation(Vec3::splat(t[3]), Quat::IDENTITY, vec3(t[0], t[1], t[2]))
        })
        .collect();
    let edges = [
        0.0,
        scene.bands.0.min(scene.far),
        scene.bands.1.min(scene.far),
        scene.far,
    ];
    let levels: Vec<InstanceLevel> = (0..3)
        .map(|l| InstanceLevel {
            mesh: meshes[l],
            min_distance: edges[l],
            max_distance: edges[l + 1],
            camera: true,
            shadows: false,
        })
        .filter(|l| l.max_distance > l.min_distance)
        .collect();
    let options = InstanceBatchOptions {
        max_distance: scene.far,
        ..Default::default()
    };
    rig.batch_levels(&levels, &transforms, &[], options);
    rig.ctx.scene3d.camera.fov_y = 1.05;
    rig.ctx.scene3d.camera.near = 0.3;
    rig.ctx.scene3d.camera.far = scene.far;
    let step = |rig: &mut BatchRig, frame: u32| {
        let (eye, forward) = scene.eye_and_forward(frame);
        rig.ctx.scene3d.camera.position = eye;
        rig.ctx.scene3d.camera.look_at(eye + forward);
        rig.frame()
    };
    for frame in 0..20 {
        step(&mut rig, frame);
    }
    let (mut frame_ms, mut cpu_ms, mut gpu, mut cull) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut read = rig.timer.as_ref().map_or(0, |t| t.frames_read);
    for frame in 0..60 {
        let cost = step(&mut rig, frame);
        frame_ms.push(cost.frame_ms);
        cpu_ms.push(cost.prepare_ms + cost.record_ms + cost.submit_ms);
        if let Some(timer) = rig.timer.as_ref().filter(|t| t.frames_read != read) {
            read = timer.frames_read;
            let of = |name: Option<&str>| {
                timer
                    .last_frame
                    .iter()
                    .filter(|p| name.is_none_or(|n| p.name == n))
                    .map(|p| f64::from(p.ms))
                    .sum::<f64>()
            };
            cull.push(of(Some("static instance cull")));
            gpu.push(of(None));
        }
    }
    Some(Anchor {
        frame_ms: median(&mut frame_ms),
        cpu_ms: median(&mut cpu_ms),
        gpu_ms: (!gpu.is_empty()).then(|| median(&mut gpu)),
        cull_ms: (!cull.is_empty()).then(|| median(&mut cull)),
    })
}

#[test]
#[ignore = "requires a GPU with mesh shaders; run explicitly in release for numbers"]
fn mesh_shader_benchmark() {
    let mut header_printed = false;
    for (name, scene) in scenarios() {
        let Some(mut rig) = Rig::new(&scene) else {
            return;
        };
        if !header_printed {
            header_printed = true;
            println!(
                "adapter: {}; {}x{} Rgba8Unorm + Depth32Float, no MSAA; {WARM_UP_BATCHES} warm-up and {BATCHES} measured \
                 batches of {BATCH} frames (camera flying {} m a frame); {ROUNDS} rounds, best median shown (+ spread to \
                 the worst round); GPU ms is the batch's wall time from submit to done over its frames; culling pass \
                 timestamps: {}",
                rig.adapter(),
                scene.size.0,
                scene.size.1,
                scene.speed,
                if rig.timestamps() { "yes" } else { "no" },
            );
        }
        let memory = rig.memory();
        println!(
            "\n### {name}\n{} instances, {} meshlets in all ({} in the nearest level), {} instances per task workgroup; \
             {} triangles per meshlet on average by level, nearest first (limit {}); \
             vertex data {} MiB, instances {} MiB, indexed path extra (indices, visible lists, draw arguments) {} MiB, \
             mesh path extra (meshlet table, vertex lists, triangles) {} MiB",
            scene.instances(),
            rig.meshlets(),
            rig.nearest_meshlets(),
            rig.instances_per_task(),
            rig.meshlet_fill().map(|f| format!("{f:.1}")).join(", "),
            super::mesh_shader_proto::MESHLET_TRIANGLES,
            mib(memory.vertices),
            mib(memory.instances),
            mib(memory.indexed_extra),
            mib(memory.mesh_extra),
        );
        let (indexed_build, mesh_build) = rig.build_ms();
        println!(
            "building the shaders and pipelines in this run: culling and indexed {indexed_build:.1} ms (2 pipelines), \
             task and mesh {mesh_build:.1} ms (3 pipelines)"
        );
        println!(
            "| path | triangles in the mesh stage or drawn | GPU ms per frame | of it the culling pass | CPU ms per frame |"
        );
        println!("|---|---|---|---|---|");
        let frames = 0..(WARM_UP_BATCHES + BATCHES) * BATCH;
        let mut paths: Vec<(String, Path, bool, Option<u32>)> = Path::ALL
            .iter()
            .map(|p| (p.name().to_owned(), *p, true, None))
            .collect();
        paths.push(("mesh, unchecked shaders".to_owned(), Path::Mesh, false, None));
        paths.push((
            "mesh, 1 instance per task workgroup".to_owned(),
            Path::Mesh,
            true,
            Some(1),
        ));
        let best_per_task = rig.instances_per_task();
        for (label, path, checked, per_task) in paths {
            rig.set_checked(checked);
            rig.set_instances_per_task(per_task.unwrap_or(best_per_task));
            if path == Path::MeshIndirect
                && let Err(reason) = rig.indirect_mesh_fits(frames.clone().step_by(8))
            {
                println!("| {label} | not run: {reason} | | | |");
                continue;
            }
            let triangles = if path.is_mesh() {
                rig.mesh_triangles(path, BATCH)
            } else {
                rig.indexed_triangles(path, BATCH)
            };
            let rounds: Vec<Medians> = (0..ROUNDS).map(|_| round(&mut rig, path)).collect();
            let col = |f: &dyn Fn(&Medians) -> f64| spread(&rounds.iter().map(f).collect::<Vec<_>>());
            println!(
                "| {label} | {:.2} M | {} | {} | {} |",
                triangles as f64 / 1e6,
                col(&|m| m.gpu),
                if path.culls_on_compute() {
                    col(&|m| m.cull.unwrap_or(0.0))
                } else {
                    "-".to_owned()
                },
                col(&|m| m.cpu),
            );
        }
        rig.set_checked(true);
        rig.set_instances_per_task(best_per_task);
        drop(rig);
        if let Some(a) = engine_anchor(&scene) {
            println!(
                "\nthe engine's own instance batches on this scene (full renderer frame: shading, depth prepass, 4x \
                 multisampling; not like-for-like): frame {:.3} ms (CPU steps {:.3} ms), GPU passes {} ms, of them the \
                 culling pass {} ms",
                a.frame_ms,
                a.cpu_ms,
                a.gpu_ms.map_or("n/a".to_owned(), |v| format!("{v:.3}")),
                a.cull_ms.map_or("n/a".to_owned(), |v| format!("{v:.3}")),
            );
        }
    }
}
