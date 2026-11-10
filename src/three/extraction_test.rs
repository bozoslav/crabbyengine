//! The 3D frame collection across frames. What it keeps between frames must
//! draw exactly what a collection that remembers nothing draws, whatever
//! changed in between, and must leave alone what didn't change.

use std::sync::Arc;

use glam::{Quat, Vec2, Vec3};

use super::collect::{Frame3D, Instance3D};
use super::retained::SERIAL;
use super::skin::MeshSkin;
use super::{
    Assets3D, Camera3D, Lighting, Material, Mesh, Model, ModelMaterial, ReceivesDecals, ShadowMesh, Skeleton,
    SkinFrame, SkinOf, Transform3D,
};
use crate::{Color, Entity, World};

const MESHES: u32 = 6;
const SPACING: f32 = 3.0;

/// A world of models on the ground, seen by a camera that looks down along
/// the grid: models to its sides, behind it and far away are out of view.
struct Scene {
    world: World,
    assets: Assets3D,
    camera: Camera3D,
    lighting: Lighting,
    frame: Frame3D,
    models: Vec<Entity>,
}

/// What a frame draws: per view (the camera, then each shadow view) and
/// mesh, its instances in a fixed order (a frame may draw the instances of
/// one mesh in any order).
type Drawn = Vec<(usize, u32, Vec<[u32; 20]>)>;

fn drawn(frame: &Frame3D) -> Drawn {
    let mut out = Vec::new();
    let mut view = |view: usize, runs: &[(u32, u32, u32)]| {
        for &(mesh, start, count) in runs {
            let instances = &frame.instances[start as usize..(start + count) as usize];
            let mut instances: Vec<[u32; 20]> = instances.iter().map(|&i| bytemuck::cast(i)).collect();
            instances.sort_unstable();
            out.push((view, mesh, instances));
        }
    };
    view(0, &frame.runs);
    for (v, runs) in frame.shadow_runs.iter().enumerate() {
        view(1 + v, runs);
    }
    out
}

fn place(i: usize, side: usize) -> Vec3 {
    let (x, z) = ((i % side) as f32, (i / side) as f32);
    Vec3::new(
        (x - side as f32 * 0.5) * SPACING,
        0.0,
        (z - side as f32 * 0.5) * SPACING,
    )
}

fn mesh(i: usize) -> Mesh {
    Mesh {
        id: (i as u32).wrapping_mul(2_654_435_761) % MESHES,
    }
}

impl Scene {
    fn new(models: usize) -> Self {
        let side = (models as f32).sqrt().ceil() as usize;
        let mut world = World::new();
        let models = (0..models)
            .map(|i| world.spawn((Transform3D::at(place(i, side)), Model::new(mesh(i)))))
            .collect();
        let mut camera = Camera3D::new(Vec2::new(1280.0, 720.0));
        camera.position = Vec3::new(0.0, 12.0, 30.0);
        camera.look_at(Vec3::ZERO);
        camera.far = 70.0;
        Self {
            world,
            assets: Assets3D {
                bounds: vec![(Vec3::ZERO, 1.0); MESHES as usize],
                ..Default::default()
            },
            camera,
            // Every cascade is drawn every frame, as in a first frame.
            lighting: Lighting {
                shadow_refresh: 1,
                ..Default::default()
            },
            frame: Frame3D::default(),
            models,
        }
    }

    fn collect(&mut self) -> Drawn {
        let skins = SkinFrame::default();
        self.frame
            .collect(&mut self.world, &self.assets, &skins, &self.camera, &self.lighting);
        drawn(&self.frame)
    }

    /// A frame that remembers nothing, of the same world, collected the
    /// other way (on all cores if this test runs on one, and the reverse).
    fn fresh(&mut self) -> Frame3D {
        let mut fresh = Frame3D::default();
        let skins = SkinFrame::default();
        let mine = SERIAL.get();
        SERIAL.set(mine.map(|serial| !serial));
        fresh.collect(&mut self.world, &self.assets, &skins, &self.camera, &self.lighting);
        SERIAL.set(mine);
        fresh
    }

    /// Collects a frame and checks it draws what a frame built from nothing
    /// draws. Returns how many instances the camera draws.
    #[track_caller]
    fn check(&mut self, what: &str) -> usize {
        let kept = self.collect();
        let fresh = self.fresh();
        let (now, new) = (&self.frame, &fresh);
        assert_eq!(
            (now.visible, now.drawn, now.cascades, now.light_views),
            (new.visible, new.drawn, new.cascades, new.light_views),
            "{what}: counts"
        );
        assert!(
            kept == drawn(&fresh),
            "{what}: not what a frame built from nothing draws"
        );
        now.visible
    }

    /// Where the camera draws the model at `x`, `z` (its place in the
    /// instance array).
    fn slot_at(&self, at: Vec3) -> Option<usize> {
        let drawn = |&(_, start, count): &(u32, u32, u32)| start as usize..(start + count) as usize;
        (self.frame.runs.iter().flat_map(drawn)).find(|&i| Vec3::from_slice(&self.frame.instances[i].model[3]) == at)
    }
}

/// The slots of `after` whose instance isn't what `before` had there.
fn rewritten(before: &[Instance3D], after: &[Instance3D]) -> usize {
    let same = |i: usize| (before.get(i)).is_some_and(|b| bytemuck::bytes_of(b) == bytemuck::bytes_of(&after[i]));
    (0..after.len()).filter(|&i| !same(i)).count()
}

/// A small deterministic random number generator.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }

    fn below(&mut self, n: usize) -> usize {
        self.next() as usize % n
    }

    fn unit(&mut self) -> f32 {
        self.next() as f32 / (1u32 << 31) as f32
    }
}

fn a_still_scene_is_left_as_it_was() {
    let mut scene = Scene::new(1500);
    let first = scene.collect();
    assert!(
        scene.frame.visible > 100 && scene.frame.visible < 1500,
        "some in view, some not"
    );
    assert!(scene.frame.drawn > scene.frame.visible, "and some cast shadows");
    let (instances, runs) = (scene.frame.instances.clone(), scene.frame.runs.clone());
    for _ in 0..3 {
        assert!(scene.collect() == first);
        assert_eq!(
            rewritten(&instances, &scene.frame.instances),
            0,
            "no instance is written again"
        );
        assert_eq!(scene.frame.runs, runs);
    }
    scene.check("still");
}

fn a_first_frame_is_packed_by_view_then_mesh_like_a_sorted_one() {
    let mut scene = Scene::new(1500);
    scene.collect();
    let frame = &scene.frame;
    let mut next = 0;
    let views = std::iter::once(&frame.runs).chain(&frame.shadow_runs);
    for &(_, start, count) in views.flatten() {
        assert_eq!(start, next, "runs follow each other without holes");
        next = start + count;
    }
    assert_eq!(next as usize, frame.instances.len());
    assert!(frame.runs.windows(2).all(|w| w[0].0 < w[1].0), "in mesh order");
}

fn a_moving_camera_keeps_what_it_still_sees_in_place() {
    let mut scene = Scene::new(2500);
    scene.collect();
    let mut shares = Vec::new();
    for step in 1..=60 {
        let before = scene.frame.instances.clone();
        let seen = scene.frame.drawn;
        scene.camera.position.x = step as f32 * 0.2;
        scene.camera.look_at(Vec3::new(step as f32 * 0.3, 0.0, 0.0));
        scene.check("camera");
        shares.push(rewritten(&before, &scene.frame.instances) * 100 / seen);
    }
    assert_eq!(scene.frame.repacks(), 1, "only the first frame was packed");
    // A frame that sorted its models would shift most of them every frame.
    // Here a block is written again when it first outgrows its place, and
    // the array once when it is packed again; other frames write the few
    // models that entered or left.
    shares.sort_unstable();
    assert!(
        shares[shares.len() / 2] <= 5,
        "half the frames write under a twentieth: {shares:?}"
    );
    assert!(shares[shares.len() * 3 / 4] <= 20, "and most under a fifth: {shares:?}");
}

fn a_moved_model_rewrites_its_own_instances_only() {
    let mut scene = Scene::new(1500);
    scene.lighting.shadows = false;
    scene.collect();
    let moved = (scene.models.iter().copied())
        .find(|&e| scene.world.get::<Transform3D>(e).unwrap().position.distance(Vec3::ZERO) < 3.0)
        .unwrap();
    let from = scene.world.get::<Transform3D>(moved).unwrap().position;
    let slot = scene.slot_at(from).expect("the model in the middle is drawn");
    for step in 1..=5 {
        let before = scene.frame.instances.clone();
        let to = from + Vec3::Y * step as f32 * 0.1;
        scene.world.get_mut::<Transform3D>(moved).unwrap().position = to;
        scene.check("moved");
        assert_eq!(rewritten(&before, &scene.frame.instances), 1);
        assert_eq!(scene.slot_at(to), Some(slot), "it stays where it was drawn");
    }
}

fn every_kind_of_edit_draws_what_a_frame_built_from_nothing_draws() {
    let mut scene = Scene::new(900);
    let seen = scene.check("first");
    let near = |scene: &Scene, k: usize| {
        let mut close: Vec<Entity> = scene.models.clone();
        let at = |e: &Entity| scene.world.get::<Transform3D>(*e).unwrap().position.length_squared();
        close.sort_by(|a, b| at(a).total_cmp(&at(b)));
        close[k]
    };
    let (a, b, c) = (near(&scene, 0), near(&scene, 1), near(&scene, 2));

    // Transforms.
    let t = scene.world.get_mut::<Transform3D>(a).unwrap();
    t.position.y = 2.0;
    t.rotation = Quat::from_rotation_y(0.7);
    t.scale = Vec3::splat(1.5);
    assert_eq!(scene.check("transform"), seen);
    scene.world.get_mut::<Transform3D>(a).unwrap().position.z = 500.0;
    assert_eq!(scene.check("moved out of view"), seen - 1);
    scene.world.get_mut::<Transform3D>(a).unwrap().position.z = 0.0;
    assert_eq!(scene.check("and back"), seen);

    // The model's mesh and tint.
    let mesh_of = |scene: &Scene, e| scene.world.get::<Model>(e).unwrap().mesh.id;
    let other = Mesh {
        id: (mesh_of(&scene, a) + 1) % MESHES,
    };
    scene.world.get_mut::<Model>(a).unwrap().mesh = other;
    scene.check("mesh");
    scene.world.get_mut::<Model>(a).unwrap().color = Color::rgb(0.2, 0.4, 0.6);
    scene.check("tint");

    // Components that change how it is drawn, added and removed.
    scene.world.insert(a, ShadowMesh(None));
    let casting = scene.frame.drawn;
    scene.check("no shadow");
    assert!(scene.frame.drawn < casting, "it casts no shadow now");
    scene.world.insert(a, ShadowMesh(Some(Mesh { id: other.id ^ 1 })));
    scene.check("a shadow mesh of its own");
    scene.world.remove::<ShadowMesh>(a);
    scene.check("its own shadow again");
    assert_eq!(scene.frame.drawn, casting);
    scene.world.insert(a, ReceivesDecals(0b101));
    scene.check("decal layers");
    scene.world.remove::<ReceivesDecals>(a);
    scene.check("every decal again");
    scene.world.insert(a, ModelMaterial(Material { id: 3 }));
    scene.world.insert(b, ModelMaterial(Material { id: 3 }));
    scene.check("a material override");
    scene.world.insert(c, ModelMaterial(Material { id: 1 }));
    scene.check("another override, numbered before it");
    scene.world.remove::<ModelMaterial>(c);
    scene.world.remove::<ModelMaterial>(a);
    scene.check("overrides removed");

    // Models that leave the plain path, and come back.
    scene.world.insert(b, SkinOf(a));
    assert_eq!(scene.check("a skin of another entity"), seen - 1);
    scene.world.remove::<SkinOf>(b);
    assert_eq!(scene.check("plain again"), seen);
    let model = scene.world.remove::<Model>(c).unwrap();
    assert_eq!(scene.check("model removed"), seen - 1);
    scene.world.insert(c, model);
    assert_eq!(scene.check("model added"), seen);
    let transform = scene.world.remove::<Transform3D>(c).unwrap();
    assert_eq!(scene.check("transform removed"), seen - 1);
    scene.world.insert(c, transform);
    assert_eq!(scene.check("transform added"), seen);

    // Entities that go, and new ones that take their index.
    assert!(scene.world.despawn(b));
    assert_eq!(scene.check("despawn"), seen - 1);
    let new = scene
        .world
        .spawn((Transform3D::at(Vec3::new(0.5, 3.0, 0.5)), Model::new(other)));
    assert_eq!(new.index(), b.index(), "the new entity has the old one's index");
    assert_eq!(scene.check("spawn"), seen);
    assert!(scene.world.despawn(new));
    let again = scene
        .world
        .spawn((Transform3D::at(Vec3::new(0.5, 4.0, 0.5)), Model::new(Mesh { id: 0 })));
    assert_eq!(scene.check("despawn and spawn in one frame"), seen);
    assert!(scene.world.despawn(again));
    assert!(scene.world.despawn(a));
    assert_eq!(scene.check("two despawns"), seen - 2);

    // The camera and the lights.
    scene.camera.position = Vec3::new(20.0, 8.0, -10.0);
    scene.camera.look_at(Vec3::new(-5.0, 0.0, 5.0));
    scene.check("camera moved");
    scene.camera.fov_y *= 0.5;
    scene.check("camera zoomed");
    scene.lighting.sun_direction = Vec3::new(0.3, -1.0, 0.6).normalize();
    scene.check("sun moved");
    scene.lighting.shadow_cascades = 2;
    scene.check("fewer cascades");
    scene.lighting.shadows = false;
    scene.check("no shadows");
    assert_eq!(scene.frame.drawn, scene.frame.visible);
    scene.lighting.shadows = true;
    scene.lighting.shadow_cascades = 4;
    scene.check("shadows again");

    // Everything gone, and a world again.
    for e in std::mem::take(&mut scene.models) {
        scene.world.despawn(e);
    }
    assert_eq!(scene.check("empty"), 0);
    assert!(scene.frame.instances.is_empty() && scene.frame.runs.is_empty());
    scene.world.spawn((Transform3D::default(), Model::new(Mesh { id: 2 })));
    assert_eq!(scene.check("one model"), 1);
}

fn a_change_to_a_mesh_reaches_the_models_that_use_it() {
    let mut scene = Scene::new(900);
    let seen = scene.check("first");
    let of_mesh = |scene: &Scene, id: u32| {
        (scene.frame.runs.iter())
            .find(|r| r.0 == id)
            .map_or(0, |r| r.2 as usize)
    };
    let ones = of_mesh(&scene, 1);
    assert!(ones > 0);

    // Its bounds grow: models past the edge of the view reach into it.
    scene.assets.bounds[1] = (Vec3::ZERO, 12.0);
    assert!(scene.check("bigger bounds") > seen);
    scene.assets.bounds[1] = (Vec3::new(0.0, 400.0, 0.0), 1.0);
    assert_eq!(scene.check("bounds far above"), seen - ones);
    // It is freed: its models draw nothing until it is a mesh again.
    scene.assets.bounds[1] = (Vec3::ZERO, -1.0);
    assert_eq!(scene.check("freed"), seen - ones);
    scene.assets.bounds[1] = (Vec3::ZERO, 1.0);
    assert_eq!(scene.check("made again"), seen);

    // It stops casting shadows, and casts again.
    let casting = scene.frame.drawn;
    scene.assets.shadowless = vec![false, true];
    scene.check("shadowless");
    assert!(scene.frame.drawn < casting);
    scene.assets.shadowless.clear();
    scene.check("casting again");
    assert_eq!(scene.frame.drawn, casting);

    // It is skinned: only its models without a skeleton are drawn plain.
    let middle = |e: &Entity| scene.world.get::<Transform3D>(*e).unwrap().position.length_squared();
    let posed = (scene.models.iter().copied())
        .filter(|&e| scene.world.get::<Model>(e).unwrap().mesh.id == 1)
        .min_by(|a, b| middle(a).total_cmp(&middle(b)))
        .unwrap();
    scene.world.insert(posed, Skeleton::new(Vec::new()));
    assert_eq!(scene.check("a skeleton on a plain mesh"), seen);
    let skin = MeshSkin {
        joints: Arc::from(Vec::new()),
        slots: Vec::new(),
        inverse_binds: Vec::new(),
        bounds: Vec::new(),
    };
    scene.assets.skins = vec![None, Some(Arc::new(skin))];
    assert_eq!(scene.check("skinned"), seen - 1);
    scene.world.remove::<Skeleton>(posed);
    assert_eq!(scene.check("its skeleton removed"), seen);

    // More meshes are made: models of the old ones aren't touched.
    let before = scene.frame.instances.clone();
    scene.assets.bounds.push((Vec3::ZERO, 2.0));
    scene.check("a new mesh");
    assert_eq!(rewritten(&before, &scene.frame.instances), 0);
    scene
        .world
        .spawn((Transform3D::default(), Model::new(Mesh { id: MESHES })));
    scene.check("and a model of it");
}

fn random_edits_always_draw_what_a_frame_built_from_nothing_draws() {
    // Enough models that they are compared on several cores.
    let mut scene = Scene::new(3000);
    let side = 55;
    let mut rng = Rng(7);
    scene.check("first");
    let mut bare: Vec<(Entity, Model)> = Vec::new();
    for frame in 0..160 {
        for _ in 0..rng.below(40) {
            let i = rng.below(scene.models.len());
            let e = scene.models[i];
            match rng.below(14) {
                0..=3 => {
                    let t = scene.world.get_mut::<Transform3D>(e).unwrap();
                    t.position += Vec3::new(rng.unit() - 0.5, rng.unit() * 0.2, rng.unit() - 0.5) * 6.0;
                }
                4 => {
                    let t = scene.world.get_mut::<Transform3D>(e).unwrap();
                    t.rotation = Quat::from_rotation_y(rng.unit() * 6.0);
                    t.scale = Vec3::splat(0.5 + rng.unit() * 2.0);
                }
                5 => {
                    if let Some(m) = scene.world.get_mut::<Model>(e) {
                        m.mesh = mesh(rng.next() as usize);
                    }
                }
                6 => {
                    if let Some(m) = scene.world.get_mut::<Model>(e) {
                        m.color = Color::rgb(rng.unit(), rng.unit(), 1.0);
                    }
                }
                7 => {
                    scene.world.despawn(e);
                    bare.retain(|b| b.0 != e);
                    let at = place(rng.below(side * side), side);
                    scene.models[i] = scene.world.spawn((Transform3D::at(at), Model::new(mesh(i + frame))));
                }
                8 => match rng.below(3) {
                    0 => drop(scene.world.remove::<ShadowMesh>(e)),
                    1 => drop(scene.world.insert(e, ShadowMesh(None))),
                    _ => drop(scene.world.insert(e, ShadowMesh(Some(mesh(i + 1))))),
                },
                9 => match rng.below(3) {
                    0 => drop(scene.world.remove::<ModelMaterial>(e)),
                    k => drop(scene.world.insert(e, ModelMaterial(Material { id: k as u32 }))),
                },
                10 => match rng.below(2) {
                    0 => drop(scene.world.remove::<ReceivesDecals>(e)),
                    _ => drop(scene.world.insert(e, ReceivesDecals(rng.next() & 0xFF))),
                },
                11 => match rng.below(2) {
                    0 => drop(scene.world.remove::<SkinOf>(e)),
                    _ => drop(scene.world.insert(e, SkinOf(e))),
                },
                12 => {
                    if let Some(model) = scene.world.remove::<Model>(e) {
                        bare.push((e, model));
                    }
                }
                _ => {
                    if let Some((e, model)) = bare.pop() {
                        scene.world.insert(e, model);
                    }
                }
            }
        }
        // Some frames the camera flies, some it stays.
        if frame % 3 != 0 {
            let angle = frame as f32 * 0.05;
            scene.camera.position = Vec3::new(angle.cos() * 25.0, 10.0 + angle.sin() * 4.0, angle.sin() * 25.0);
            scene.camera.look_at(Vec3::new((angle * 0.5).sin() * 10.0, 0.0, 0.0));
        }
        match frame % 40 {
            11 => scene.lighting.sun_direction = Vec3::new(rng.unit() - 0.5, -1.0, rng.unit() - 0.5).normalize(),
            17 => scene.lighting.shadows = !scene.lighting.shadows,
            23 => scene.lighting.shadow_cascades = 1 + rng.below(4) as u32,
            29 => scene.assets.bounds[rng.below(MESHES as usize)] = (Vec3::ZERO, [-1.0, 1.0, 3.0][rng.below(3)]),
            31 => scene.assets.shadowless = (0..MESHES).map(|_| rng.below(4) == 0).collect(),
            // A wave of new models, more than are worked out one by one.
            37 => {
                for k in 0..700 {
                    let at = place(rng.below(side * side), side) + Vec3::Y;
                    scene
                        .models
                        .push(scene.world.spawn((Transform3D::at(at), Model::new(mesh(k)))));
                }
            }
            38 => {
                for e in scene.models.split_off(scene.models.len() - 700) {
                    scene.world.despawn(e);
                    bare.retain(|b| b.0 != e);
                }
            }
            _ => {}
        }
        scene.check(&format!("frame {frame}"));
    }
}

fn shadow_only_casters_follow_edits_too() {
    let mut scene = Scene::new(400);
    let casters: Vec<Entity> = (0..30)
        .map(|i| {
            let at = Vec3::new(i as f32 - 15.0, 2.0, 0.0);
            scene.world.spawn((Transform3D::at(at), ShadowMesh(Some(mesh(i)))))
        })
        .collect();
    let seen = scene.check("first");
    let casting = scene.frame.drawn;
    scene.world.get_mut::<Transform3D>(casters[3]).unwrap().position.y = 4.0;
    scene.check("moved");
    scene.world.despawn(casters[5]);
    scene.world.remove::<ShadowMesh>(casters[6]);
    assert_eq!(scene.check("gone"), seen);
    assert!(scene.frame.drawn < casting);
    scene.world.insert(casters[7], Model::new(mesh(2)));
    assert_eq!(scene.check("now a model with a shadow mesh"), seen + 1);
    scene.camera.position.x += 6.0;
    scene.check("camera");
}

fn a_kept_cascade_draws_nothing_and_is_right_when_its_turn_comes() {
    let mut scene = Scene::new(1500);
    scene.lighting.shadow_refresh = 4;
    scene.lighting.shadow_cascades = 4;
    let mut kept = 0;
    for frame in 0..24 {
        // A model moves through the cascades while some are kept.
        let e = scene.models[frame * 31 % scene.models.len()];
        scene.world.get_mut::<Transform3D>(e).unwrap().position.y += 1.5;
        scene.collect();
        // A frame built from nothing draws every cascade, into the same boxes
        // as the ones this frame drew again.
        let fresh = scene.fresh();
        let (now, new) = (drawn(&scene.frame), drawn(&fresh));
        for cascade in 0..scene.frame.cascades {
            let of = |d: &Drawn| d.iter().filter(|r| r.0 == 1 + cascade).cloned().collect::<Vec<_>>();
            if scene.frame.redrawn & (1 << cascade) == 0 {
                assert!(
                    of(&now).is_empty(),
                    "frame {frame}: kept cascade {cascade} draws nothing"
                );
                kept += 1;
            } else if scene.frame.sun_view_proj[cascade] == fresh.sun_view_proj[cascade] {
                assert!(of(&now) == of(&new), "frame {frame}: cascade {cascade}");
            }
        }
        assert!(now.iter().filter(|r| r.0 == 0).eq(new.iter().filter(|r| r.0 == 0)));
    }
    assert!(kept > 10, "the far cascades were kept on most frames");
}

fn moved_models_are_listed_with_where_they_were() {
    let mut scene = Scene::new(600);
    scene.lighting.shadows = false;
    scene.frame.motion.enabled = true;
    scene.collect();
    scene.collect();
    assert!(scene.frame.motion.instances.is_empty(), "nothing moved");
    let e = (scene.models.iter().copied())
        .find(|&e| scene.world.get::<Transform3D>(e).unwrap().position.distance(Vec3::ZERO) < 3.0)
        .unwrap();
    let from = scene.world.get::<Transform3D>(e).unwrap().position;
    scene.world.get_mut::<Transform3D>(e).unwrap().position.y = 1.0;
    scene.camera.position.x += 0.5; // a moving camera alone moves no model
    scene.collect();
    let moved = &scene.frame.motion.instances;
    assert_eq!(moved.len(), 1);
    assert_eq!(Vec3::from_slice(&moved[0].previous[3]), from);
    assert_eq!(Vec3::from_slice(&moved[0].instance.model[3]), from.with_y(1.0));
    scene.collect();
    assert!(scene.frame.motion.instances.is_empty(), "it stopped");
}

fn what_is_kept_per_model_is_bounded_and_stops_growing() {
    let models = 20_000;
    let mut scene = Scene::new(models);
    scene.collect();
    let first = scene.frame.extraction_bytes();
    // About 210 bytes a model, and the instances drawn with room to grow.
    let budget = models * 260 + scene.frame.drawn * 2 * size_of::<Instance3D>() + 64 * 1024;
    assert!(first <= budget, "{first} bytes kept for {models} models");
    let mut most = first;
    for round in 0..3 {
        for step in 0..60 {
            let angle = step as f32 * 0.1;
            scene.camera.position = Vec3::new(angle.cos() * 30.0, 12.0, angle.sin() * 30.0);
            scene.camera.look_at(Vec3::ZERO);
            scene.collect();
            most = most.max(scene.frame.extraction_bytes());
        }
        // The same flight again takes no more than the first time.
        if round == 0 {
            assert!(most <= budget * 2, "{most} bytes kept after flying around");
        }
    }
    let settled = most;
    for step in 0..60 {
        let angle = step as f32 * 0.1;
        scene.camera.position = Vec3::new(angle.cos() * 30.0, 12.0, angle.sin() * 30.0);
        scene.camera.look_at(Vec3::ZERO);
        scene.collect();
        assert!(scene.frame.extraction_bytes() <= settled, "it stopped growing");
    }
}

/// Runs every test twice: with the models collected in one pass on this
/// thread (as with one core, or in a browser), and compared on all cores.
macro_rules! both_ways {
    ($($name:ident),* $(,)?) => {
        mod on_one_core {
            $(#[test]
            fn $name() {
                super::SERIAL.set(Some(true));
                super::$name();
            })*
        }
        mod on_several_cores {
            $(#[test]
            fn $name() {
                super::SERIAL.set(Some(false));
                super::$name();
            })*
        }
    };
}

both_ways!(
    a_still_scene_is_left_as_it_was,
    a_first_frame_is_packed_by_view_then_mesh_like_a_sorted_one,
    a_moving_camera_keeps_what_it_still_sees_in_place,
    a_moved_model_rewrites_its_own_instances_only,
    every_kind_of_edit_draws_what_a_frame_built_from_nothing_draws,
    a_change_to_a_mesh_reaches_the_models_that_use_it,
    random_edits_always_draw_what_a_frame_built_from_nothing_draws,
    shadow_only_casters_follow_edits_too,
    a_kept_cascade_draws_nothing_and_is_right_when_its_turn_comes,
    moved_models_are_listed_with_where_they_were,
    what_is_kept_per_model_is_bounded_and_stops_growing,
);
