//! Starts the game in a window, or headless with `HEADLESS=1` (what
//! `ponko-mcp`'s `launch` sets by default; `TICKS` caps the ticks).

use {{crate_name}}::game;

fn main() {
    if std::env::var("HEADLESS").is_ok_and(|v| v != "0") {
        let ticks = std::env::var("TICKS")
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(600);
        let world = game().run_headless(ticks);
        println!("ran {ticks} ticks headless: {} entities", world.len());
    } else {
        game().run();
    }
}
