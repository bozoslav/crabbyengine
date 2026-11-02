//! Camera, shapes, text and UI in one small scene.
//!
//!   `cargo run --release --example ui`
//!
//! - Right or middle mouse drag: pan. Scroll: zoom toward the mouse.
//! - Left click in the world: drop a crab there (not when clicking the UI).
//! - Buttons: follow the orbiting crab, reset the camera.

use crabbyengine::*;

struct Scene {
    crab: Texture,
    follow: bool,
    dropped: usize,
}

// ANCHOR: scene
fn scene(ctx: &mut Ctx) {
    let t = ctx.time() as f32;

    // The world: a grid of tiles, a few shapes, a crab in orbit.
    for y in 0..20 {
        for x in 0..20 {
            let shade = if (x + y) % 2 == 0 { 0.16 } else { 0.19 };
            let tile = Rect::new(x as f32 * 64.0, y as f32 * 64.0, 64.0, 64.0);
            ctx.draw_rect(tile, Color::rgb(shade, shade + 0.03, shade));
        }
    }
    let center = vec2(640.0, 640.0);
    ctx.draw_circle(center, 180.0, Color::hex(0x3a86ff).with_alpha(0.15));
    let orbit = center + Vec2::from_angle(t * 0.6) * 180.0;
    ctx.draw_line(center, orbit, 3.0, Color::hex(0xffbe0b));
    let crab = ctx.world.resource::<Scene>().crab;
    ctx.draw_ex(
        &crab,
        orbit,
        DrawParams {
            rotation: t,
            ..Default::default()
        },
    );

    // A label that sticks to the crab: world position -> window pixels.
    let above = ctx.camera.world_to_screen(orbit) + vec2(-28.0, -40.0);
    ctx.draw_text("orbiter", above, 16.0, Color::hex(0xffbe0b));

    if ctx.world.resource::<Scene>().follow {
        ctx.camera.look_at(orbit);
    }
    if ctx.mouse_pressed(MouseButton::Left) && !ctx.mouse_over_ui() {
        let at = ctx.mouse_world();
        ctx.world.spawn((Transform2D::at(at), Sprite::new(crab)));
        ctx.world.resource_mut::<Scene>().dropped += 1;
    }
}
// ANCHOR_END: scene

// ANCHOR: hud
fn hud(ctx: &mut Ctx) {
    let panel = Rect::new(12.0, 12.0, 230.0, 172.0);
    ctx.ui_panel(panel);
    let mut at = panel.min + ctx.ui_style.padding;
    let (zoom, dropped) = (ctx.camera.zoom, ctx.world.resource::<Scene>().dropped);
    let lines = format!("zoom {zoom:.2}x\ncrabs dropped: {dropped}\n{:.0} FPS", ctx.fps());
    at.y += ctx.ui_label(at, &lines).y + 8.0;

    let follow = ctx.world.resource::<Scene>().follow;
    let label = if follow { "Stop following" } else { "Follow orbiter" };
    if ctx.ui_button(Rect::new(at.x, at.y, 210.0, 30.0), label) {
        ctx.world.resource_mut::<Scene>().follow = !follow;
    }
    at.y += 38.0;
    if ctx.ui_button(Rect::new(at.x, at.y, 210.0, 30.0), "Reset camera") {
        ctx.camera.zoom = 1.0;
        ctx.camera.position = Vec2::ZERO;
        ctx.world.resource_mut::<Scene>().follow = false;
    }
}
// ANCHOR_END: hud

fn main() {
    Game::new("Crabby Engine: camera, shapes, text, UI", 1280, 720)
        .system(camera_controls)
        .system(scene)
        .system(hud)
        .run(|ctx| {
            let crab = ctx.load_texture("crab.png");
            ctx.world.insert_resource(Scene {
                crab,
                follow: false,
                dropped: 0,
            });
        });
}
