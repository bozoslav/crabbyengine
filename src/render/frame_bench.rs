//! Offscreen benchmark of a whole 3D frame: what `Renderer3D::prepare` and
//! recording every pass cost on the CPU, and all passes on the GPU, for a
//! scene of ordinary models (no instance batches) with and without the
//! screen-space effects. It is the before/after measurement of renderer
//! refactors that must not change the frame's cost. No window is opened.
//! Run explicitly, in release:
//! `cargo test --release --lib --features 3d renderer_frame_benchmark -- --ignored --nocapture`
//! The results are in `crates/ponko_bench/RENDERER_FRAME.md`.
use super::render_test_support as support;
use super::static_batch_render_test::{BatchRig, RigSetup};
use crate::d3::Transform as Transform3D;
use crate::prelude::*;
use crate::render::{AntiAliasing, MaterialDesc, MeshData, Model};
use crate::three::Blend;

const SIZE: (u32, u32) = (1280, 720);
const WARM_UP: usize = 30;
const FRAMES: usize = 200;
/// Each row is measured this many times; the best medians are kept.
const ROUNDS: usize = 5;
/// Models in the scene, over [`MESHES`] meshes with a material each.
const MODELS: usize = 4000;
const MESHES: usize = 40;

/// One row: the medians of a run, in milliseconds.
#[derive(Clone, Copy)]
struct Row {
    prepare: f64,
    record: f64,
    submit: f64,
    frame: f64,
    gpu: Option<f64>,
    draws: u32,
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

/// Draws the scene with (`effects`) or without the screen-space effects
/// and measures it. `None` without a GPU.
fn run(effects: bool) -> Option<Row> {
    let mut rig = BatchRig::new(RigSetup {
        size: SIZE,
        timestamps: true,
        ..Default::default()
    })?;
    // Cubes and spheres; every eighth material blends, every fifth is cut out.
    let meshes: Vec<_> = (0..MESHES)
        .map(|i| {
            let data = if i % 2 == 0 {
                MeshData::cube(0.4)
            } else {
                MeshData::sphere(0.3, 12)
            };
            let blend = if i % 8 == 7 {
                Blend::Transparent
            } else if i % 5 == 4 {
                Blend::Cutout
            } else {
                Blend::Opaque
            };
            let shade = 0.3 + 0.7 * (i as f32 / MESHES as f32);
            rig.mesh(
                &data,
                MaterialDesc {
                    color: Color::rgba(
                        shade,
                        1.0 - shade * 0.5,
                        0.6,
                        if blend == Blend::Transparent { 0.5 } else { 1.0 },
                    ),
                    blend,
                    ..Default::default()
                },
            )
        })
        .collect();
    let side = (MODELS as f32).sqrt().ceil() as usize;
    let extent = side as f32 * 1.5;
    for i in 0..MODELS {
        let at = vec3(
            (i % side) as f32 * 1.5 - extent / 2.0,
            ((i * 7) % 5) as f32 * 0.4,
            -((i / side) as f32) * 1.5,
        );
        rig.ctx
            .world
            .spawn((Transform3D::at(at), Model::new(meshes[i % MESHES])));
    }
    rig.ctx.camera3d.far = extent * 2.0;
    rig.ctx.lighting.shadows = true;
    rig.ctx.lighting.shadow_distance = extent * 0.5;
    if effects {
        let post = &mut rig.ctx.post;
        post.anti_aliasing = AntiAliasing::Taa;
        post.bloom = 0.3;
        post.ambient_occlusion.enabled = true;
        post.ssr = 0.5;
    }
    // The camera flies ahead every frame, as a game's does: uniforms,
    // sorted transparents and shadow maps change.
    let start = vec3(-extent / 2.0 - 2.0, 6.0, 2.0);
    let ahead = (vec3(0.0, 0.0, -extent / 2.0) - start).normalize();
    let step = |rig: &mut BatchRig, frame: usize| {
        rig.ctx.camera3d.position = start + ahead * (frame as f32 * 0.02);
        rig.ctx.camera3d.look_at(rig.ctx.camera3d.position + ahead);
        rig.frame()
    };
    for frame in 0..WARM_UP {
        step(&mut rig, frame);
    }
    let (mut prepare, mut record, mut submit, mut frame) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut gpu = Vec::new();
    let mut draws = 0;
    let mut read = rig.timer.as_ref().map_or(0, |t| t.frames_read);
    for i in 0..FRAMES {
        let cost = step(&mut rig, WARM_UP + i);
        prepare.push(cost.prepare_ms);
        record.push(cost.record_ms);
        submit.push(cost.submit_ms);
        frame.push(cost.frame_ms);
        draws = cost.changes.draws;
        if let Some(timer) = rig.timer.as_ref().filter(|t| t.frames_read != read) {
            read = timer.frames_read;
            gpu.push(timer.last_frame.iter().map(|p| f64::from(p.ms)).sum::<f64>());
        }
    }
    Some(Row {
        prepare: median(&mut prepare),
        record: median(&mut record),
        submit: median(&mut submit),
        frame: median(&mut frame),
        gpu: (!gpu.is_empty()).then(|| median(&mut gpu)),
        draws,
    })
}

#[test]
#[ignore = "requires a GPU; run explicitly in release for numbers"]
fn renderer_frame_benchmark() {
    let Some(probe) = BatchRig::new(RigSetup::default()) else {
        return;
    };
    println!(
        "adapter: {}; target {}x{} Rgba8UnormSrgb, 4x MSAA, depth prepass, four shadow cascades; {MODELS} models of {MESHES} meshes; {WARM_UP} warm-up + {FRAMES} measured frames of a camera flying 0.02 m a frame, best of {ROUNDS} runs' medians",
        support::describe_adapter(&probe.gpu.info),
        SIZE.0,
        SIZE.1,
    );
    drop(probe);
    println!("| scene | prepare ms | record ms | submit ms | CPU ms | GPU ms | frame ms | model draws |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|");
    for (name, effects) in [
        ("models, shadows, tone mapping", false),
        ("the same with TAA, bloom, GTAO and reflections", true),
    ] {
        let mut best: Option<Row> = None;
        for _ in 0..ROUNDS {
            let Some(row) = run(effects) else {
                return;
            };
            best = Some(match best {
                None => row,
                Some(b) => Row {
                    prepare: b.prepare.min(row.prepare),
                    record: b.record.min(row.record),
                    submit: b.submit.min(row.submit),
                    frame: b.frame.min(row.frame),
                    gpu: b.gpu.zip(row.gpu).map(|(a, b)| a.min(b)),
                    draws: row.draws,
                },
            });
        }
        let row = best.unwrap();
        println!(
            "| {name} | {:.3} | {:.3} | {:.3} | {:.3} | {} | {:.3} | {} |",
            row.prepare,
            row.record,
            row.submit,
            row.prepare + row.record + row.submit,
            row.gpu.map_or("n/a".to_owned(), |g| format!("{g:.3}")),
            row.frame,
            row.draws,
        );
    }
}
