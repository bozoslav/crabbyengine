//! Opens a window and shows off the game loop and input.
//!
//! Run with: `cargo run --example window`
//!
//! - Arrow keys / WASD: move an (invisible, for now) player; Space prints its position
//! - Click: prints the mouse position
//! - Scroll: changes zoom
//! - Escape: quit
//!
//! Nothing is drawn yet; rendering arrives in M2. Watch the terminal.

use ponko::*;

fn main() {
    Game::new("Ponko: window", 800, 600).run(|ctx| {
        println!("window is {} x {}", ctx.window_size().x, ctx.window_size().y);

        let mut pos = ctx.window_size() / 2.0;
        let mut zoom = 1.0_f32;
        let mut fixed_ticks = 0_u64;

        move |ctx: &mut Ctx| {
            // Per-frame movement, scaled by dt so speed is the same at any frame rate.
            let mut dir = Vec2::ZERO;
            if ctx.key_down(Key::Left) || ctx.key_down(Key::A) {
                dir.x -= 1.0;
            }
            if ctx.key_down(Key::Right) || ctx.key_down(Key::D) {
                dir.x += 1.0;
            }
            if ctx.key_down(Key::Up) || ctx.key_down(Key::W) {
                dir.y -= 1.0;
            }
            if ctx.key_down(Key::Down) || ctx.key_down(Key::S) {
                dir.y += 1.0;
            }
            pos += dir.normalize_or_zero() * 200.0 * ctx.dt();

            // Fixed-rate logic: runs exactly 60 times per second on average.
            for _ in 0..ctx.time.fixed_steps() {
                fixed_ticks += 1;
            }

            if ctx.key_pressed(Key::Space) {
                println!(
                    "pos = ({:.0}, {:.0})  time = {:.2}s  fixed ticks = {}  fps = {:.0}",
                    pos.x,
                    pos.y,
                    ctx.time.elapsed(),
                    fixed_ticks,
                    ctx.time.fps()
                );
            }

            if ctx.input.mouse_pressed(MouseButton::Left) {
                let m = ctx.mouse_pos();
                println!("click at ({:.0}, {:.0})", m.x, m.y);
            }

            if ctx.input.mouse_wheel().y != 0.0 {
                zoom = (zoom * 1.1_f32.powf(ctx.input.mouse_wheel().y)).clamp(0.1, 10.0);
                println!("zoom = {zoom:.2}");
            }

            if ctx.key_pressed(Key::Escape) {
                println!("bye!");
                ctx.quit();
            }
        }
    });
}
