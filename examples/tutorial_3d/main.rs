//! Lantern Yard: the game the guide's "Tutorial: a first 3D game" builds.
//!
//!   `cargo run --example tutorial_3d --features physics3d`
//!
//! WASD or the arrow keys walk, Q and E turn, Space jumps (a gamepad's
//! sticks and South button too). Walk up to every lantern to light it.
//!
//! The game is in `game.rs` (in a project made with `ponko new --3d`, that is
//! `src/lib.rs`, and this file is `src/main.rs`). `tests/tutorial_3d.rs`
//! walks it headlessly. The web build:
//! `web/build.sh --features physics3d tutorial_3d`.

mod game;

fn main() {
    // `HEADLESS=1` runs it without a window, as the `ponko new` template does.
    let headless = std::env::var("HEADLESS").is_ok_and(|v| v != "0");
    if headless {
        let world = game::game().configured().run_headless(600);
        println!("ran 600 ticks headless: {} entities", world.len());
    } else {
        game::game().configured().run();
    }
}
