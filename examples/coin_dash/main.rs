//! Coin Dash: a small, complete game built only on the public `Game`/`Ctx`
//! API, with no asset files. Collect every coin before the clock runs out;
//! then restart and go again.
//!
//!   `cargo run --release --example coin_dash`
//!
//! Move: arrow keys, WASD or the left stick. Restart: R, Enter or the gamepad's
//! South button (any time). Quit: Escape.
//!
//! The rules live in `game.rs`, which `tests/coin_dash.rs` also drives without
//! a window (`cargo test --test coin_dash`), so the win/lose/restart loop is
//! checked in CI. This file only opens the window.

mod game;

fn main() {
    game::build().run(game::setup);
}
