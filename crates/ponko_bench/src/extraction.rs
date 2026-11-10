//! 3D frame extraction: what collecting a world's visible models costs each
//! frame (the CPU side of drawing, before any GPU work), and how many
//! instance bytes the frame would upload.
//!
//! One world of `MODELS` models on a grid, drawn by a camera in the middle
//! with the default sun shadows, in these workloads:
//!
//! - `still`: nothing changes between frames.
//! - `camera`: the camera flies and turns; no model changes.
//! - `mixed`: the camera flies and 1% of the models move every frame.
//! - `dynamic`: the camera flies and every model moves every frame.
//! - `edits`: the camera flies; 100 models are despawned and 100 spawned
//!   every frame.
//! - `rebuild`: the first frame of a new collection (nothing remembered):
//!   the worst case.

use std::time::{Duration, Instant};

use ponko::d3::Camera as Camera3D;
use ponko::d3::Transform as Transform3D;
use ponko::render::ExtractionBench;
use ponko::render::Lighting;
use ponko::render::Model;
use ponko::{Entity, Vec3, World, vec3};

use crate::{Bench, report};

const MODELS: usize = 100_000;
const MESHES: u32 = 64;
const SIDE: usize = 400;
const SPACING: f32 = 4.0;
const WARMUP: usize = 30;
const FRAMES: usize = 120;

fn place(i: usize) -> Vec3 {
    let (x, z) = ((i % SIDE) as f32, (i / SIDE) as f32);
    let (wide, deep) = (SIDE as f32, (MODELS / SIDE) as f32);
    vec3((x - wide * 0.5) * SPACING, 0.0, (z - deep * 0.5) * SPACING)
}

struct Scene {
    world: World,
    models: Vec<Entity>,
    extraction: ExtractionBench,
    camera: Camera3D,
    lighting: Lighting,
    frame: usize,
}

fn scene() -> Scene {
    let extraction = ExtractionBench::new(MESHES);
    let mut world = World::new();
    let models = (0..MODELS)
        .map(|i| {
            let mesh = extraction.mesh((i as u32).wrapping_mul(2_654_435_761) % MESHES);
            world.spawn((Transform3D::at(place(i)), Model::new(mesh)))
        })
        .collect();
    let mut camera = ExtractionBench::camera(1920.0, 1080.0);
    camera.far = 400.0;
    Scene {
        world,
        models,
        extraction,
        camera,
        lighting: Lighting::default(),
        frame: 0,
    }
}

impl Scene {
    /// Moves the camera along a slow circle, looking ahead and a little down.
    fn fly(&mut self) {
        let angle = self.frame as f32 * 0.004;
        let at = vec3(angle.cos() * 120.0, 12.0, angle.sin() * 120.0);
        let ahead = vec3((angle + 0.5).cos() * 150.0, 0.0, (angle + 0.5).sin() * 150.0);
        self.camera.position = at;
        self.camera.look_at(ahead);
    }

    /// Bobs every `every`th model up and down.
    fn bob(&mut self, every: usize) {
        let y = (self.frame % 2) as f32 * 0.25;
        for &e in self.models.iter().step_by(every) {
            self.world.get_mut::<Transform3D>(e).unwrap().position.y = y;
        }
    }

    /// Despawns `n` models and spawns as many.
    fn edit(&mut self, n: usize) {
        for k in 0..n {
            let i = (self.frame * 7919 + k * 104_729) % self.models.len();
            self.world.despawn(self.models[i]);
            let mesh = self.extraction.mesh((i as u32 + self.frame as u32) % MESHES);
            self.models[i] = self.world.spawn((Transform3D::at(place(i)), Model::new(mesh)));
        }
    }
}

pub fn run(b: &mut Bench) {
    type Change = fn(&mut Scene);
    let workloads: [(&str, Change); 5] = [
        ("still", |s| {
            if s.frame == 0 {
                s.fly();
            }
        }),
        ("camera", |s| s.fly()),
        ("mixed", |s| {
            s.fly();
            s.bob(100);
        }),
        ("dynamic", |s| {
            s.fly();
            s.bob(1);
        }),
        ("edits", |s| {
            s.fly();
            s.edit(100);
        }),
    ];
    for (name, change) in workloads {
        let name = format!("extract-100k/{name}");
        if b.skips(&name) {
            continue;
        }
        let mut s = scene();
        let mut times = Vec::with_capacity(FRAMES);
        let (mut uploaded, mut sample) = (0, Default::default());
        for i in 0..WARMUP + FRAMES {
            change(&mut s);
            let t = Instant::now();
            sample = s.extraction.collect(&mut s.world, &s.camera, &s.lighting);
            let took = t.elapsed();
            let bytes = s.extraction.sync_uploads();
            if i >= WARMUP {
                times.push(took);
                uploaded += bytes;
            }
            s.frame += 1;
        }
        b.report(report::Record::new(&name, report::Kind::Steady, WARMUP, &mut times));
        println!(
            "    {} visible + {} shadow instances in {} runs; {:.1} KiB of instances uploaded per frame; {:.1} KiB kept",
            sample.visible,
            sample.shadow,
            sample.runs,
            uploaded as f64 / FRAMES as f64 / 1024.0,
            sample.state_bytes as f64 / 1024.0
        );
    }
    // The first frame of a collection that remembers nothing.
    let name = "extract-100k/rebuild";
    if !b.skips(name) {
        let mut s = scene();
        s.fly();
        let mut times: Vec<Duration> = (0..10 + 1)
            .map(|_| {
                s.extraction = ExtractionBench::new(MESHES);
                let t = Instant::now();
                s.extraction.collect(&mut s.world, &s.camera, &s.lighting);
                t.elapsed()
            })
            .skip(1)
            .collect();
        b.report(report::Record::new(name, report::Kind::Fresh, 1, &mut times));
    }
}
