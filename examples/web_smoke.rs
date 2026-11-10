//! The 2D browser smoke-test fixture: a tiny game that `web/smoke/` builds with
//! `web/build.sh web_smoke` and drives in a headless browser.
//!
//! It loads one asset file (`crab.png`, fetched over HTTP on the web), runs
//! a fixed-step simulation, counts left clicks and draws something on every
//! frame. Each frame it publishes its state for the runner (see
//! `common/web_smoke.rs`):
//!
//! `{"fixture":"web_smoke","ticks":120,"frames":118,"clicks":1,"size":[800,600],"asset":[32,32]}`
//!
//! Failure cases for the runner's self-tests, chosen with the page's query
//! string: `?smoke_case=panic` panics during setup, `?smoke_case=stall`
//! pauses the simulation after a few ticks. `web_smoke_3d` is the same
//! fixture for the 3D renderer.
//!
//! On the desktop it opens a window like any other example
//! (`cargo run --example web_smoke`) and prints its state once a second.

use ponko::*;
use serde_json::json;
use std::cell::Cell;
use std::rc::Rc;

#[path = "common/web_smoke.rs"]
mod smoke;

fn main() {
    let ticks = Rc::new(Cell::new(0u64));
    let sim_ticks = ticks.clone();
    Game::new("Ponko: web smoke", 800, 600)
        .fixed_hz(60)
        .sim_system(move |_ctx| sim_ticks.set(sim_ticks.get() + 1))
        .run(move |ctx| {
            let mut smoke = smoke::Smoke::new("web_smoke");
            // The one fetched asset; a missing file panics with its name.
            let crab = ctx.load_texture("crab.png");
            move |ctx: &mut Ctx| {
                smoke.frame(ctx, ticks.get(), json!({ "asset": [crab.width(), crab.height()] }));
                // The background changes with each click, so a click shows.
                ctx.clear(if smoke.clicks().is_multiple_of(2) {
                    Color::hex(0x1E2A38)
                } else {
                    Color::hex(0x2E4A28)
                });
                let size = ctx.window_size();
                let t = ticks.get() as f32 / 60.0;
                for i in 0..8 {
                    let angle = t + i as f32 * std::f32::consts::TAU / 8.0;
                    ctx.draw_ex(
                        &crab,
                        size / 2.0 + Vec2::from_angle(angle) * size.min_element() * 0.3,
                        DrawParams {
                            scale: Vec2::splat(3.0),
                            ..Default::default()
                        },
                    );
                }
            }
        });
}
