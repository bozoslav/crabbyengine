//! Morph targets headless: a dent-shaped blend shape found by name and
//! shown by weight.
#![cfg(feature = "3d")]

use crabbyengine::*;

// ANCHOR: morph
// System or setup: everything here runs headless too.
fn morph(ctx: &mut Ctx) {
    // One target, named to find it again: at full weight its third corner
    // sinks 0.4 into the sheet. Normals are left out, so the shading keeps
    // its smooth look; give one offset per corner to bend the light along.
    let mut sheet = MeshData::plane(1.0);
    let mut dent = vec![Vec3::ZERO; sheet.positions.len()];
    dent[2] = vec3(0.0, 0.0, -0.4);
    sheet.morph_targets.push(MorphTarget {
        name: "dent".into(),
        positions: dent,
        normals: Vec::new(),
    });

    let material = ctx.default_material();
    let mesh = ctx.create_mesh(&sheet, material);
    let dent = ctx.morph_target(mesh, "dent").unwrap();
    let mut weights = MorphWeights::default();
    weights.set(dent, 0.5); // halfway dented
    ctx.world.spawn((Transform3D::default(), Model::new(mesh), weights));
}
// ANCHOR_END: morph

#[test]
fn a_named_morph_target_is_weighted_on_a_model() {
    let mut world = Game::new("morph", 1, 1).run_headless(0, morph);
    let mut panels = 0;
    world.query::<(&Model, &MorphWeights)>().for_each(|(_model, weights)| {
        panels += 1;
        assert_eq!(weights.0, [0.5], "halfway toward the target's offsets");
    });
    assert_eq!(panels, 1, "the sheet is the only weighted model");
}
