//! 3D command recording: what turning a prepared frame's passes into GPU
//! command buffers costs (`Renderer3D::draw`), offscreen, without a window.
//! Needs a GPU and the `render` feature:
//! `cargo run --release -p ponko_bench --features render -- render/record`.
//!
//! Two frames of 4,096 models of 64 meshes at 1280x720 with 4x multisampling:
//!
//! - `plain`: sun shadows, depth prepass, opaque, bloom and post.
//! - `effects`: the same with an atmosphere, temporal antialiasing, ambient
//!   occlusion and screen-space reflections (more passes inside each part).
//!
//! Only recording is timed; preparing, submitting and the GPU's work are not.
//! See `RECORDING.md` for numbers.

use ponko::render::{RecordBench, RecordScene};

use crate::{Bench, report};

const WARMUP: usize = 60;
const FRAMES: usize = 600;

pub fn run(b: &mut Bench) {
    for (name, effects) in [("plain", false), ("effects", true)] {
        let name = format!("render/record {name}");
        if b.skips(&name) {
            continue;
        }
        let scene = RecordScene {
            size: (1280, 720),
            models: 4096,
            meshes: 64,
            effects,
        };
        let Some(mut bench) = RecordBench::new(scene) else {
            println!("{name:<32} skipped: no GPU");
            continue;
        };
        let mut times = Vec::with_capacity(FRAMES);
        let mut buffers = 0;
        for i in 0..WARMUP + FRAMES {
            let (took, count) = bench.frame();
            if i >= WARMUP {
                times.push(took);
                buffers = count;
            }
        }
        b.report(report::Record::new(&name, report::Kind::Steady, WARMUP, &mut times));
        println!("    {buffers} command buffers per frame, on {}", bench.adapter());
    }
}
