//! Offscreen benchmark of static instance batches: the GPU-driven path
//! (compute culling, indirect draws) against the CPU path (cell culling,
//! direct draws) over instance counts, batch counts, occlusion rates and
//! shadow views, with the culling pass's own GPU time next to the frame's.
//! No window is opened. Run explicitly, in release:
//! `cargo test --release --lib --features 3d static_batch_benchmark -- --ignored --nocapture`
//! (`PONKO_GPU_TIER=baseline` or `webgl2` for the other tiers' limits). The
//! results and how to read them are in `crates/ponko_bench/STATIC_BATCHES.md`.
use super::gpu3d::BatchPath;
use super::render_test_support as support;
use super::static_batch_render_test::{BatchRig, FrameCost, RigSetup};
use crate::prelude::*;
use crate::render::{InstanceBatchOptions, InstanceLevel, MaterialDesc, MeshData};
use crate::three::occlusion::OcclusionGrid;

const SIZE: (u32, u32) = (1280, 720);
const WARM_UP: usize = 20;
const FRAMES: usize = 120;
/// Each row is measured this many times; the best medians are kept.
const ROUNDS: usize = 5;
/// Width of a region (one batch), in metres.
const REGION: f32 = 32.0;

/// One row of the benchmark.
#[derive(Clone, Copy)]
struct Scenario {
    name: &'static str,
    /// Batches, laid out as a square of regions.
    batches: usize,
    /// Instances in every batch.
    per_batch: usize,
    /// Sphere segments of the mesh (0: a 12-triangle cube).
    segments: u32,
    /// Fraction of the screen a synthetic occluder hides, 0..=1.
    occluded: f32,
    /// Sun shadows (four cascades).
    shadows: bool,
    /// Distance levels instead of one mesh.
    levels: bool,
    /// Width of a culling cell, in metres.
    cell: f32,
    /// Metres between neighbouring instances; 0 packs a batch into a
    /// region of [`REGION`] metres whatever its size.
    spacing: f32,
    /// The camera stays where it is.
    still: bool,
}

impl Scenario {
    const fn new(name: &'static str, batches: usize, per_batch: usize) -> Self {
        Self {
            name,
            batches,
            per_batch,
            segments: 0,
            occluded: 0.0,
            shadows: false,
            levels: false,
            cell: 8.0,
            spacing: 0.0,
            still: false,
        }
    }
}

/// The medians of one scenario on one path.
struct Row {
    cost: FrameCost,
    /// GPU time of the culling compute pass and of all passes, in ms.
    gpu: Option<(f64, f64)>,
    triangles: u32,
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

/// Builds `scenario` on `path` and measures it. `None` without a GPU.
fn run(scenario: &Scenario, path: BatchPath, compute: bool) -> Option<Row> {
    let mut rig = BatchRig::new(RigSetup {
        size: SIZE,
        compute,
        timestamps: true,
        ..Default::default()
    })?;
    rig.renderer.batching.path = path;
    rig.keep_order = true;
    let data = if scenario.segments == 0 {
        MeshData::cube(0.3)
    } else {
        MeshData::sphere(0.2, scenario.segments)
    };
    let triangles = data.indices.len() as u32 / 3;
    let mesh = rig.mesh(
        &data,
        MaterialDesc {
            color: Color::WHITE,
            ..Default::default()
        },
    );
    let side = (scenario.batches as f32).sqrt().ceil() as usize;
    let row = (scenario.per_batch as f32).sqrt().ceil() as usize;
    let region = if scenario.spacing > 0.0 {
        scenario.spacing * row as f32
    } else {
        REGION
    };
    let spacing = region / row as f32;
    let extent = side as f32 * region;
    // Spread scenes are big: what is drawn ends 150 m from the camera.
    let reach = if scenario.spacing > 0.0 {
        (extent * 0.75).min(150.0)
    } else {
        extent * 0.75
    };
    let region_width = region;
    for region in 0..scenario.batches {
        let origin = vec3(
            (region % side) as f32 * region_width - extent / 2.0,
            0.0,
            -((region / side) as f32) * region_width,
        );
        let transforms: Vec<_> = (0..scenario.per_batch)
            .map(|i| {
                Mat4::from_translation(origin + vec3((i % row) as f32 * spacing, 0.0, -((i / row) as f32) * spacing))
            })
            .collect();
        let options = InstanceBatchOptions {
            cell_size: scenario.cell,
            max_distance: reach,
            ..Default::default()
        };
        if scenario.levels {
            let bands = [
                (0.0, extent * 0.2),
                (extent * 0.2, extent * 0.45),
                (extent * 0.45, extent * 0.75),
            ];
            let levels = bands.map(|(min_distance, max_distance)| InstanceLevel {
                mesh,
                min_distance,
                max_distance,
                camera: true,
                shadows: true,
            });
            rig.batch_levels(&levels, &transforms, &[], options);
        } else {
            rig.batch(mesh, &transforms, options);
        }
    }
    // From a corner, looking along the diagonal: the view takes in a wedge
    // of the field, and the far part is past the maximum distance. The
    // camera flies ahead every frame, as a game's does: view uniforms and
    // visible sets change, and cached shadow maps go stale.
    let start = vec3(-extent / 2.0 - 2.0, 6.0, 2.0);
    let ahead = (vec3(0.0, 0.0, -extent / 2.0) - start).normalize();
    rig.ctx.camera3d.far = extent * 2.0;
    rig.ctx.lighting.shadows = scenario.shadows;
    rig.ctx.lighting.shadow_distance = extent * 0.5;
    rig.ctx.camera3d.occlusion_culling = scenario.occluded > 0.0;
    let step = |rig: &mut BatchRig, frame: usize| {
        // The depth grid is the frame before's, as a readback delivers it:
        // a wall two metres ahead over the left part of the screen.
        if scenario.occluded > 0.0 {
            let view_proj = rig.ctx.camera3d.view_proj();
            let wall = view_proj * (rig.ctx.camera3d.position + ahead * 2.0).extend(1.0);
            let (w, h) = (64u32, 36u32);
            let depth = (0..w * h)
                .map(|i| {
                    if ((i % w) as f32) < scenario.occluded * w as f32 {
                        wall.z / wall.w
                    } else {
                        1.0
                    }
                })
                .collect();
            rig.grid = Some(OcclusionGrid::new(view_proj, rig.ctx.camera3d.position, w, h, depth));
        }
        let frame = if scenario.still { 0 } else { frame };
        rig.ctx.camera3d.position = start + ahead * (frame as f32 * 0.02);
        rig.ctx.camera3d.look_at(rig.ctx.camera3d.position + ahead);
        rig.frame()
    };
    step(&mut rig, 0);
    for frame in 0..WARM_UP {
        step(&mut rig, frame);
    }
    let (mut prepare, mut record, mut submit, mut frame) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let (mut cull, mut gpu) = (Vec::new(), Vec::new());
    let mut cost = FrameCost::default();
    let mut read = rig.timer.as_ref().map_or(0, |t| t.frames_read);
    for i in 0..FRAMES {
        cost = step(&mut rig, WARM_UP + i);
        prepare.push(cost.prepare_ms);
        record.push(cost.record_ms);
        submit.push(cost.submit_ms);
        frame.push(cost.frame_ms);
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
    cost.prepare_ms = median(&mut prepare);
    cost.record_ms = median(&mut record);
    cost.submit_ms = median(&mut submit);
    cost.frame_ms = median(&mut frame);
    Some(Row {
        cost,
        gpu: (!gpu.is_empty()).then(|| (median(&mut cull), median(&mut gpu))),
        triangles,
    })
}

#[test]
#[ignore = "requires a GPU; run explicitly in release for numbers"]
fn static_batch_benchmark() {
    let tier = support::forced_tier();
    let Some(probe) = BatchRig::new(RigSetup::default()) else {
        return;
    };
    let compute = probe.compute_culling();
    println!(
        "adapter: {}; tier: {}; compute culling: {compute}; target {}x{} Rgba8UnormSrgb, 4x MSAA, depth prepass; {WARM_UP} warm-up + {FRAMES} measured frames of a camera flying 0.02 m a frame, best of {ROUNDS} runs' medians",
        support::describe_adapter(&probe.gpu.info),
        tier.map_or("device default (desktop limits)".to_owned(), |t| format!(
            "{t} (forced)"
        )),
        SIZE.0,
        SIZE.1,
    );
    drop(probe);
    let mut scenarios = Vec::new();
    // One batch, growing.
    for per_batch in [256, 4096, 65_536, 131_072, 262_144, 524_288, 1_000_000] {
        scenarios.push(Scenario::new("one batch", 1, per_batch));
    }
    // The crossover: the same 65,536 instances in more and smaller
    // batches (regions), as cubes and as spheres of 224 triangles.
    for batches in [16, 32, 64, 128, 256, 512, 1024, 2048, 4096] {
        scenarios.push(Scenario::new("65,536 cubes in regions", batches, 65536 / batches));
    }
    for batches in [64, 256, 1024] {
        scenarios.push(Scenario {
            segments: 16,
            ..Scenario::new("65,536 spheres in regions", batches, 65536 / batches)
        });
    }
    // Growing scenes of regions of 1,024 instances each.
    for batches in [128, 256, 512, 1024] {
        scenarios.push(Scenario::new("regions of 1,024 cubes", batches, 1024));
    }
    // What culling saves: the spheres behind an occluder.
    for occluded in [0.5, 0.9] {
        scenarios.push(Scenario {
            segments: 16,
            occluded,
            ..Scenario::new("occluded spheres", 64, 1024)
        });
    }
    scenarios.push(Scenario {
        shadows: true,
        ..Scenario::new("sun shadows", 64, 1024)
    });
    scenarios.push(Scenario {
        levels: true,
        ..Scenario::new("three levels", 64, 1024)
    });
    scenarios.push(Scenario {
        levels: true,
        shadows: true,
        ..Scenario::new("three levels, shadows", 64, 1024)
    });
    // The default cell width (32 m) on instances a metre apart, as a
    // scattered scene has them: cubes, and meshes of 2,024 and 5,040
    // triangles, where what a path draws outside the view costs most.
    for (name, segments) in [
        ("spread cubes, 32 m cells", 0),
        ("spread 2,024-triangle meshes, 32 m cells", 46),
        ("spread 5,040-triangle meshes, 32 m cells", 72),
    ] {
        for per_batch in [1024, 4096, 10_000, 32_768, 131_072] {
            scenarios.push(Scenario {
                segments,
                cell: 32.0,
                spacing: 1.0,
                ..Scenario::new(name, 1, per_batch)
            });
        }
    }
    scenarios.push(Scenario {
        levels: true,
        ..Scenario::new("million instance levels", 1, 1_000_000)
    });
    // The CPU path with a camera at rest keeps its lists.
    for per_batch in [65_536, 1_000_000] {
        scenarios.push(Scenario {
            still: true,
            ..Scenario::new("one batch, camera at rest", 1, per_batch)
        });
    }
    println!(
        "| scenario | batches | per batch | tris/mesh | occluded | path | prepare µs | record µs | submit µs | cull GPU µs | GPU ms | frame ms | draws | bind groups | pipelines | vertex+index buffers |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    // `PONKO_STATIC_BENCH=<text>` keeps the scenarios whose name contains it.
    let only = std::env::var("PONKO_STATIC_BENCH").unwrap_or_default();
    for s in scenarios.iter().filter(|s| s.name.contains(&only)) {
        let paths: &[(BatchPath, &str)] = if compute {
            &[
                (BatchPath::Gpu, "gpu"),
                (BatchPath::Cpu, "cpu"),
                (BatchPath::Auto, "auto"),
            ]
        } else {
            &[(BatchPath::Cpu, "cpu")]
        };
        for &(path, label) in paths {
            // The best of `ROUNDS` runs' medians: other work on the
            // machine only ever adds time.
            let Some(mut row) = run(s, path, compute) else { return };
            // The spread scenes draw millions of triangles a frame: fewer runs.
            let rounds = if s.spacing > 0.0 { 3 } else { ROUNDS };
            for _ in 1..rounds {
                let Some(again) = run(s, path, compute) else { return };
                row.cost.prepare_ms = row.cost.prepare_ms.min(again.cost.prepare_ms);
                row.cost.record_ms = row.cost.record_ms.min(again.cost.record_ms);
                row.cost.submit_ms = row.cost.submit_ms.min(again.cost.submit_ms);
                row.cost.frame_ms = row.cost.frame_ms.min(again.cost.frame_ms);
                row.gpu = match (row.gpu, again.gpu) {
                    (Some(a), Some(b)) => Some((a.0.min(b.0), a.1.min(b.1))),
                    (a, b) => a.or(b),
                };
            }
            let c = row.cost.changes;
            let (cull, gpu) = match row.gpu {
                Some((cull, gpu)) => (format!("{:.0}", cull * 1e3), format!("{gpu:.3}")),
                None => ("n/a".into(), "n/a".into()),
            };
            println!(
                "| {} | {} | {} | {} | {:.0}% | {label} | {:.0} | {:.0} | {:.0} | {cull} | {gpu} | {:.3} | {} | {} | {} | {} |",
                s.name,
                s.batches,
                s.per_batch,
                row.triangles,
                s.occluded * 100.0,
                row.cost.prepare_ms * 1e3,
                row.cost.record_ms * 1e3,
                row.cost.submit_ms * 1e3,
                row.cost.frame_ms,
                c.draws,
                c.bind_groups,
                c.pipelines,
                c.vertex_buffers + c.index_buffers,
            );
        }
    }
}
