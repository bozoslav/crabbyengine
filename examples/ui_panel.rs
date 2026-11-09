//! Retained UI on a screen in the 3D world: `cargo run --features 3d --example ui_panel`.
use ponko::*;

/// A terminal with a button on it.
struct Terminal {
    buy: UiId,
}

fn setup(ctx: &mut Ctx) {
    ctx.camera3d.position = vec3(0.0, 1.6, 1.0);
    ctx.camera3d.look_at(vec3(0.0, 1.5, -2.0));
    let floor_material = ctx.gfx.create_material(MaterialDesc::color(Color::hex(0x6B7B58)));
    let floor = ctx.gfx.create_mesh(&MeshData::plane(12.0), floor_material);
    ctx.world.spawn((Transform3D::default(), Model::new(floor)));

    // ANCHOR: panel_create
    // A 1.2 x 0.8 m terminal, 600 x 400 pixels, with a button.
    let (mut panel, model) = ctx.gfx.create_ui_panel(600, 400, vec2(1.2, 0.8));
    let root = panel.ui.root().layout(UiLayout::fill().padding(20.0).gap(12.0)).id();
    let buy = panel.ui.button(root, "Buy").layout(UiLayout::fixed(200.0, 60.0)).id();
    panel.ui.push_screen(root, false);
    ctx.world
        .spawn((Transform3D::at(vec3(0.0, 1.5, -2.0)), model, panel, Terminal { buy }));
    // ANCHOR_END: panel_create
}

// ANCHOR: panel_events
fn terminal(ctx: &mut Ctx) {
    // Aim with the middle of the screen instead of the mouse (for a locked
    // cursor, or a controller in VR).
    let (eye, forward) = (ctx.camera3d.position, ctx.camera3d.forward());
    ctx.ui.set_pointer_ray(Some((eye, forward)));
    ctx.world
        .query::<(&mut UiPanel3D, &Terminal)>()
        .for_each(|(panel, terminal)| {
            while let Some(event) = panel.ui.next_event() {
                if event == UiEvent::Clicked(terminal.buy) {
                    panel.ui.set_text(terminal.buy, "Bought!");
                }
            }
        });
}
// ANCHOR_END: panel_events

fn main() {
    Game::new("UI panel", 960, 640).system(terminal).run(setup);
}
