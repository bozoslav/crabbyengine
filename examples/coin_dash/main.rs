//! Coin Dash: a small, complete game built only on the public `Game`/`Ctx`
//! API, with no asset files. A title screen, rounds (collect every coin before
//! the clock runs out), a pause, a result screen and a saved best time.
//!
//!   `cargo run --release --example coin_dash`
//!
//! Move: arrow keys, WASD or the left stick. Enter / Space / South: start,
//! resume, play again. R / West: restart the round. Esc / P / Start: pause
//! (quit on the title screen). Q / East: back to the title.
//!
//! The best time and most coins are saved in the player's data folder (or the
//! file named by `COIN_DASH_SAVE`); a missing or damaged file just starts the
//! progress over.
//!
//! The rules live in `game.rs`, which `tests/coin_dash.rs` also drives without
//! a window (`cargo test --test coin_dash`), so the whole loop (title, play,
//! pause, result, saving) is checked in CI. This file only opens the window.

mod game;

fn main() {
    game::build().run(game::setup);
}
