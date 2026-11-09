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
    assert!(c.instances > 0 && c.instances <= 4 * 4096, "{c:?}");
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
