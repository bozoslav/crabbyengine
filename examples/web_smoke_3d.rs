//! The 3D browser smoke-test fixture: `web_smoke` for the 3D renderer
//! (`web/smoke/fixtures.json`, built with `--features 3d`).
//!
//! It loads one glTF model (`models/house.glb`, fetched over HTTP on the web
//! and parsed in the browser), draws it with a sun with shadows, a few
//! spheres with different materials and a ground plane, and orbits the camera
//! from the fixed-step simulation. The state it publishes (see
//! `common/web_smoke.rs`) adds the triangles and models drawn last frame, so a
//! scene that loaded but draws nothing is caught:
//!
//! `{"fixture":"web_smoke_3d","ticks":120,"frames":118,"clicks":1,"size":[800,600],"triangles":1234,"models_drawn":8}`
//!
//! The failure cases are `web_smoke`'s. On the desktop it opens a window
//! (`cargo run --example web_smoke_3d --features 3d`).

use ponko::*;
use serde_json::json;
use std::cell::Cell;
use std::rc::Rc;

#[path = "common/web_smoke.rs"]
mod smoke;

fn main() {
    let ticks = Rc::new(Cell::new(0u64));
    let sim_ticks = ticks.clone();
    Game::new("Ponko: 3D web smoke", 800, 600)
        .fixed_hz(60)
        .sim_system(move |_ctx| sim_ticks.set(sim_ticks.get() + 1))
        .run(move |ctx| {
            let mut smoke = smoke::Smoke::new("web_smoke_3d");
            ctx.gfx.clear(Color::hex(0x9CC4E4));
            ctx.lighting.sun_direction = vec3(-0.8, -0.6, -0.4);
            let ground_material = ctx.gfx.create_material(MaterialDesc {
                roughness: 0.95,
                ..MaterialDesc::color(Color::hex(0x5E8C4A))
            });
            let ground = ctx.gfx.create_mesh(&MeshData::plane(60.0), ground_material);
            ctx.world.spawn((Transform3D::default(), Model::new(ground)));

            // The fetched asset: a model with its own materials and textures.
            let house = ctx.gfx.load_mesh("models/house.glb");
            ctx.world
                .spawn((Transform3D::at(vec3(0.0, 0.0, -3.0)), Model::new(house)));

            let white = ctx.gfx.default_material();
            let ball = ctx.gfx.create_mesh(&MeshData::sphere(0.45, 24), white);
            for i in 0..6 {
                let material = ctx.gfx.create_material(MaterialDesc {
                    roughness: 0.1 + i as f32 * 0.17,
                    metallic: (i % 2) as f32,
                    ..MaterialDesc::color(Color::hex(0xD8453B))
                });
                ctx.world.spawn((
                    Transform3D::at(vec3(i as f32 * 1.1 - 2.75, 0.45, 2.0)),
                    Model::new(ball),
                    ModelMaterial(material),
                ));
            }

            move |ctx: &mut Ctx| {
                // The camera orbits on the simulation clock, so a stalled
                // simulation also freezes the picture.
                let angle = ticks.get() as f32 / 240.0;
                ctx.camera3d.position = vec3(angle.sin() * 12.0, 6.0, angle.cos() * 12.0);
                ctx.camera3d.look_at(vec3(0.0, 1.0, 0.0));
                smoke.frame(
                    ctx,
                    ticks.get(),
                    json!({ "triangles": ctx.perf.render_stats().triangles, "models_drawn": ctx.gfx.models_drawn() }),
                );
            }
        });
}
