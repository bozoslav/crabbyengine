//! Where level-of-detail work goes under a moving camera: a field of
//! three-level `LodGroup`s picked on the CPU every frame (entities), against
//! the same groups as one static batch of their instance levels, picked in
//! the GPU's culling pass or cell by cell on the CPU. Offscreen, no window;
//! needs a GPU and the `render` feature:
//!
//! ```sh
//! cargo run --release -p ponko_bench --features render -- render/lod
//! ```
//!
//! Settings: `PONKO_LOD_FRAMES` (measured frames, default 240),
//! `PONKO_LOD_WARMUP` (default 60). `PONKO_GPU_TIER` opens the device as a
//! weaker GPU. The results and how to read them: `LOD_SPLIT.md`.

use std::time::Duration;

use ponko::render::{LodBench, LodFrame, LodPath, LodScene};

use crate::{Bench, report};

/// Groups, a name for them, and how fast the camera flies (m per frame) and
/// a name for that, and whether the sun casts shadows.
const SCENES: [(usize, &str, f32, &str, bool); 6] = [
    (10_000, "10k", 0.5, "drive", false),
    (10_000, "10k", 4.0, "fly", false),
    (100_000, "100k", 0.5, "drive", false),
    (100_000, "100k", 4.0, "fly", false),
    (100_000, "100k", 0.5, "drive-shadows", true),
    (1_000_000, "1m", 0.5, "drive", false),
];

fn number(name: &str, default: usize) -> usize {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => v.trim().parse().unwrap_or_else(|_| {
            eprintln!("{name}={v} is not a whole number");
            std::process::exit(2);
        }),
        _ => default,
    }
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some(values[values.len() / 2])
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

/// Runs the scenes and paths the filter keeps.
pub fn run(b: &mut Bench) {
    let frames = number("PONKO_LOD_FRAMES", 240).max(1);
    let warmup = number("PONKO_LOD_WARMUP", 60);
    let mut header = false;
    for (groups, count, speed, motion, shadows) in SCENES {
        for path in LodPath::ALL {
            let name = format!("render/lod {count} {motion} {}", path.name());
            if b.skips(&name) {
                continue;
            }
            let scene = LodScene {
                size: (1280, 720),
                groups,
                path,
                shadows,
                speed,
            };
            let mut bench = match LodBench::new(scene) {
                Ok(bench) => bench,
                Err(reason) => {
                    println!("{name:<40} skipped: {reason}");
                    return;
                }
            };
            if !header {
                header = true;
                println!(
                    "render/lod: 1280 x 720, 4x MSAA, {warmup} warm-up + {frames} measured frames, on {}{}",
                    bench.adapter(),
                    if bench.timed() { "" } else { " (no GPU timestamps)" }
                );
            }
            for _ in 0..warmup {
                bench.frame();
            }
            let measured: Vec<LodFrame> = (0..frames).map(|_| bench.frame()).collect();
            for step in ["collect", "prepare", "encode", "cpu", "frame"] {
                let mut times: Vec<Duration> = measured
                    .iter()
                    .map(|f| match step {
                        "collect" => f.collect,
                        "prepare" => f.prepare,
                        "encode" => f.encode,
                        "cpu" => f.collect + f.prepare + f.encode + f.submit,
                        _ => f.frame,
                    })
                    .collect();
                b.report(report::Record::new(
                    &format!("{name} {step}"),
                    report::Kind::Steady,
                    warmup,
                    &mut times,
                ));
            }
            let med = |f: fn(&LodFrame) -> Option<f64>| median(measured.iter().filter_map(f).collect());
            let shown = |v: Option<f64>| v.map_or_else(|| "–".to_owned(), |v| format!("{v:.3}"));
            let memory = bench.memory();
            // One row for the write-up's tables (medians, ms; bytes as MiB).
            println!(
                "    | {count} | {motion} | {} ({}) | {:.3} | {:.3} | {:.3} | {:.3} | {} | {} | {:.3} | {:.0} | {:.0} | {:.1} | {:.1} | {:.1} |",
                path.name(),
                bench.route(),
                med(|f| Some(ms(f.collect))).unwrap_or(0.0),
                med(|f| Some(ms(f.prepare))).unwrap_or(0.0),
                med(|f| Some(ms(f.encode))).unwrap_or(0.0),
                med(|f| Some(ms(f.collect + f.prepare + f.encode + f.submit))).unwrap_or(0.0),
                shown(med(|f| f.cull_gpu_ms)),
                shown(med(|f| f.gpu_ms)),
                med(|f| Some(ms(f.frame))).unwrap_or(0.0),
                med(|f| Some(f.instances as f64)).unwrap_or(0.0),
                med(|f| Some((f.instance_bytes + f.batch_bytes) as f64 / 1024.0)).unwrap_or(0.0),
                memory.cpu as f64 / (1 << 20) as f64,
                memory.frame_instances as f64 / (1 << 20) as f64,
                memory.gpu_pools as f64 / (1 << 20) as f64,
            );
        }
    }
}
