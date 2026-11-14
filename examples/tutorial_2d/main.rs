//! Crab Cove: the game the guide's "Tutorial: a first 2D game" builds.
//!
//!   `cargo run --example tutorial_2d --features physics`
//!
//! Arrow keys or A/D walk, Space, Up or W jumps (a gamepad's left stick and
//! South button too). Collect the coins and touch the flag; Escape opens the
//! title screen. The best score is kept in the player's data folder.
//!
//! The game is in `game.rs` (in a project made with `ponko new`, that is
//! `src/lib.rs`, and this file is `src/main.rs`). `tests/tutorial_2d.rs`
//! plays it headlessly. The web build:
//! `web/build.sh --features physics tutorial_2d`.

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
