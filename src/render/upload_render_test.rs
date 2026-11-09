//! Offscreen: what the 3D renderer uploads and creates per frame, once a
//! scene has settled (`RenderStats::uploads`).
use super::buffer_render_test::Rig;
use super::traffic;
use crate::*;

/// A rig with `n` × `n` cubes on the ground, seen from above, with sun
/// shadows; `None` without a GPU.
fn scene(n: i32) -> Option<(Rig, Vec<Entity>)> {
    let mut rig = Rig::new()?;
    let mesh = rig.mesh(
        &MeshData::cube(0.4),
        MaterialDesc {
            color: Color::WHITE,
            ..Default::default()
        },
    );
    let mut cubes = Vec::new();
    for x in 0..n {
        for z in 0..n {
            let at = vec3(x as f32 - n as f32 / 2.0, 0.0, z as f32 - n as f32 / 2.0);
            cubes.push(rig.ctx.world.spawn((Transform3D::at(at), Model::new(mesh))));
        }
    }
    rig.ctx.camera3d.position = vec3(0.0, n as f32, n as f32);
    rig.ctx.camera3d.look_at(Vec3::ZERO);
    Some((rig, cubes))
}

/// Draws `frames` frames after `settle` warm-up ones, calling `change`
/// before each; the most of each count over the measured frames.
fn steady(rig: &mut Rig, settle: usize, frames: usize, mut change: impl FnMut(&mut Rig, usize)) -> GpuUploads {
    for i in 0..settle {
        change(rig, i);
        rig.draw();
    }
    traffic::take();
    let mut most = GpuUploads::default();
    for i in settle..settle + frames {
        change(rig, i);
        rig.draw();
        let c = traffic::take();
        most.instances = most.instances.max(c.instances);
        most.uniforms = most.uniforms.max(c.uniforms);
        most.lights = most.lights.max(c.lights);
        most.static_batches = most.static_batches.max(c.static_batches);
        most.other = most.other.max(c.other);
        most.materials = most.materials.max(c.materials);
        most.meshes = most.meshes.max(c.meshes);
        most.skins = most.skins.max(c.skins);
        most.textures = most.textures.max(c.textures);
        most.buffers_created = most.buffers_created.max(c.buffers_created);
        most.buffer_bytes_created = most.buffer_bytes_created.max(c.buffer_bytes_created);
    }
    most
}

#[test]
fn a_still_scene_creates_no_buffers_and_uploads_no_instances() {
    let Some((mut rig, _)) = scene(16) else { return };
    let c = steady(&mut rig, 4, 6, |_, _| {});
    eprintln!("still scene, 256 models: {c:?} ({} bytes)", c.bytes());
    assert_eq!(c.buffers_created, 0, "{c:?}");
    assert_eq!(c.instances, 0, "{c:?}");
}

#[test]
fn a_moving_model_uploads_only_its_part_of_the_instances() {
    let Some((mut rig, cubes)) = scene(16) else { return };
    let moving = cubes[100];
    let c = steady(&mut rig, 4, 6, |rig, i| {
        rig.ctx.world.get_mut::<Transform3D>(moving).unwrap().position.y = (i % 2) as f32 * 0.3;
    });
    eprintln!("one model moving of 256: {c:?} ({} bytes)", c.bytes());
    assert_eq!(c.buffers_created, 0, "{c:?}");
    // Every model's record is 80 bytes; all of them would be 20 KiB per view.
    // The model keeps its place in each view that draws it (the camera and
    // the shadow maps), so only its own records are written.
    assert!(c.instances > 0 && c.instances <= 8 * 80, "{c:?}");
}

/// Step `step` of a run of scene edits: a model moved, two despawned, then
/// enough spawned to outgrow the instance buffer, then one moved back.
fn edit(rig: &mut Rig, cubes: &mut Vec<Entity>, step: usize) {
    match step {
        0 => rig.ctx.world.get_mut::<Transform3D>(cubes[3]).unwrap().position.y = 1.5,
        1 => {
            for e in cubes.drain(10..12) {
                rig.ctx.world.despawn(e);
            }
        }
        2 => {
            let model = *rig.ctx.world.get::<Model>(cubes[0]).unwrap();
            for i in 0..200 {
                let at = vec3((i % 20) as f32 - 10.0, 1.0 + (i / 20) as f32 * 0.5, -10.0);
                cubes.push(rig.ctx.world.spawn((Transform3D::at(at), model)));
            }
        }
        _ => rig.ctx.world.get_mut::<Transform3D>(cubes[3]).unwrap().position.y = 0.0,
    }
}

#[test]
fn edited_scenes_draw_the_same_as_ones_uploaded_whole() {
    let Some((mut rig, mut cubes)) = scene(16) else { return };
    let before = rig.draw();
    rig.draw();
    let mut grew = 0;
    for step in 0..4 {
        edit(&mut rig, &mut cubes, step);
        rig.draw();
        grew += traffic::take().buffers_created;
    }
    let edited = rig.draw();
    assert!(grew > 0, "the spawns outgrew the instance buffer");
    assert_ne!(edited, before, "the edits show");
    // The same scene, drawn once from scratch.
    let Some((mut fresh, mut fresh_cubes)) = scene(16) else {
        return;
    };
    for step in 0..4 {
        edit(&mut fresh, &mut fresh_cubes, step);
    }
    let whole = fresh.draw();
    let differ = edited.iter().zip(&whole).filter(|(a, b)| a != b).count();
    assert_eq!(differ, 0, "{differ} bytes differ");
}

#[test]
fn moved_and_removed_lights_draw_the_same_as_ones_uploaded_whole() {
    let light = |color| PointLight3D {
        color,
        intensity: 6.0,
        range: 6.0,
        ..Default::default()
    };
    let lit = |rig: &mut Rig| {
        rig.ctx.lighting.sun_intensity = 0.0;
        let red = rig
            .ctx
            .world
            .spawn((Transform3D::at(vec3(-3.0, 1.0, 0.0)), light(Color::RED)));
        let blue = rig
            .ctx
            .world
            .spawn((Transform3D::at(vec3(3.0, 1.0, 0.0)), light(Color::BLUE)));
        (red, blue)
    };
    let edit = |rig: &mut Rig, (red, blue): (Entity, Entity), step: usize| match step {
        0 => rig.ctx.world.get_mut::<Transform3D>(red).unwrap().position.x = 0.0,
        _ => {
            rig.ctx.world.despawn(blue);
        }
    };
    let Some((mut rig, _)) = scene(8) else { return };
    let lights = lit(&mut rig);
    let before = rig.draw();
    rig.draw();
    traffic::take();
    rig.draw();
    assert_eq!(traffic::take().lights, 0, "unchanged lights aren't uploaded again");
    for step in 0..2 {
        edit(&mut rig, lights, step);
        rig.draw();
        assert!(traffic::take().lights > 0, "changed lights are");
    }
    let edited = rig.draw();
    assert_ne!(edited, before, "the edits show");
    let Some((mut fresh, _)) = scene(8) else { return };
    let fresh_lights = lit(&mut fresh);
    for step in 0..2 {
        edit(&mut fresh, fresh_lights, step);
    }
    let whole = fresh.draw();
    let differ = edited.iter().zip(&whole).filter(|(a, b)| a != b).count();
    assert_eq!(differ, 0, "{differ} bytes differ");
}

/// Flies the camera along the cubes: every frame some leave its view and
/// others enter.
fn pan(rig: &mut Rig, step: usize) {
    let x = step as f32 * 0.15 - 4.0;
    rig.ctx.camera3d.position = vec3(x, 6.0, 9.0);
    rig.ctx.camera3d.look_at(vec3(x, 0.0, 0.0));
}

#[test]
fn a_moving_camera_uploads_the_models_that_entered_or_left_not_the_scene() {
    let Some((mut rig, _)) = scene(40) else { return };
    // Every cascade is drawn every frame, so its models are uploaded too.
    rig.ctx.lighting.shadow_refresh = 1;
    rig.ctx.camera3d.far = 30.0;
    let mut whole = usize::MAX;
    let c = steady(&mut rig, 12, 24, |rig, i| {
        pan(rig, i);
        if i > 0 {
            whole = whole.min(rig.models_drawn() * 80);
        }
    });
    eprintln!("camera panning over 1600 models: {c:?}; the views draw at least {whole} bytes of instances");
    assert!(whole >= 400 * 80, "a few hundred models are in view: {whole}");
    assert_eq!(c.buffers_created, 0, "{c:?}");
    // The models that stay in view keep their records' places, so the
    // frame that uploads the most still uploads a small part of what the
    // views draw. (A frame that sorted its models shifted most of them, and
    // uploaded nearly all.)
    assert!(c.instances > 0 && c.instances * 8 <= whole as u64, "{c:?} of {whole}");
}

/// A rig with cubes on the ground under a low camera, a second mesh (a
/// bigger blue cube, or a ball) and a material to edit them with; `None`
/// without a GPU.
fn flight(ball: bool) -> Option<(Rig, Vec<Entity>, Mesh, Material)> {
    let (mut rig, cubes) = scene(24)?;
    let data = if ball {
        MeshData::sphere(0.3, 12)
    } else {
        MeshData::cube(0.6)
    };
    let other = rig.mesh(
        &data,
        MaterialDesc {
            color: Color::rgb(0.2, 0.5, 1.0),
            ..Default::default()
        },
    );
    let red = rig.ctx.create_material(MaterialDesc {
        color: Color::RED,
        ..Default::default()
    });
    // Every cascade is drawn every frame, as in a renderer's first frame.
    rig.ctx.lighting.shadow_refresh = 1;
    rig.ctx.camera3d.far = 40.0;
    Some((rig, cubes, other, red))
}

/// Step `step` of a flight over the cubes while the scene is edited: the
/// camera moves every step; then models move, are despawned, lose and get
/// components, change mesh and material, and new ones are spawned.
fn fly(rig: &mut Rig, cubes: &mut Vec<Entity>, (other, red): (Mesh, Material), step: usize) {
    pan(rig, step * 4);
    let world = &mut rig.ctx.world;
    match step {
        // Some models move, then all of them.
        2 => {
            for &e in cubes.iter().step_by(3) {
                world.get_mut::<Transform3D>(e).unwrap().position.y = 0.3;
            }
        }
        3 => {
            for (i, &e) in cubes.iter().enumerate() {
                let t = world.get_mut::<Transform3D>(e).unwrap();
                t.position.y = (i % 5) as f32 * 0.1;
                t.rotation = Quat::from_rotation_y(i as f32 * 0.2);
            }
        }
        4 => {
            for e in cubes.drain(280..300) {
                world.despawn(e);
            }
        }
        // Components removed and added.
        5 => {
            for &e in &cubes[300..310] {
                world.remove::<Model>(e);
            }
            for &e in &cubes[310..330] {
                world.insert(e, ShadowMesh(None));
            }
        }
        // Another mesh, another material.
        6 => {
            for &e in &cubes[250..280] {
                world.get_mut::<Model>(e).unwrap().mesh = other;
            }
        }
        7 => {
            for &e in &cubes[200..240] {
                world.insert(e, ModelMaterial(red));
            }
        }
        8 => {
            for i in 0..60 {
                let at = vec3((i % 20) as f32 * 0.9 - 9.0, 1.2, (i / 20) as f32 - 4.0);
                cubes.push(world.spawn((Transform3D::at(at), Model::new(other))));
            }
        }
        9 => {
            for &e in &cubes[200..210] {
                world.remove::<ModelMaterial>(e);
            }
            for &e in &cubes[300..305] {
                world.insert(e, Model::new(other).with_color(Color::GREEN));
            }
            for &e in &cubes[310..320] {
                world.remove::<ShadowMesh>(e);
            }
        }
        _ => {}
    }
}

/// Flies and edits (see [`fly`]), drawing every step, and after each step
/// draws the same scene in a renderer that has drawn nothing yet: the most
/// pixels that differ between the two, over the steps.
fn flown_against_drawn_once(ball: bool) -> Option<usize> {
    let (mut rig, mut cubes, other, red) = flight(ball)?;
    let mut last = rig.draw();
    let mut most = 0;
    for step in 0..12 {
        fly(&mut rig, &mut cubes, (other, red), step);
        let kept = rig.draw();
        assert_ne!(kept, last, "step {step} shows");
        let (mut fresh, mut fresh_cubes, other, red) = flight(ball)?;
        for earlier in 0..=step {
            fly(&mut fresh, &mut fresh_cubes, (other, red), earlier);
        }
        let whole = fresh.draw();
        let differ = (kept.chunks(4).zip(whole.chunks(4))).filter(|(a, b)| a != b).count();
        most = most.max(differ);
        last = kept;
    }
    Some(most)
}

#[test]
fn a_flight_with_edits_draws_the_same_as_the_scene_drawn_once() {
    let Some(differ) = flown_against_drawn_once(false) else {
        return;
    };
    assert_eq!(differ, 0, "{differ} pixels differ");
}

#[test]
fn models_drawn_in_another_order_differ_by_no_more_than_edge_samples() {
    // A view that keeps its models' places draws the models of one mesh in
    // another order than a first frame does. The picture is the same but
    // for what the GPU's rasterizer makes of the order: with balls a few
    // pixels wide, some edge pixels differ by a sample. (A first frame of
    // the same balls spawned in another order differs as much.)
    let Some(differ) = flown_against_drawn_once(true) else {
        return;
    };
    eprintln!("balls, flown against drawn once: at most {differ} of 16384 pixels differ");
    assert!(differ <= 16384 / 100, "{differ} pixels differ");
}
