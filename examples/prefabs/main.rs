//! Prefabs with children: carts with wheels, a driver and the driver's hat,
//! all from one prefab file that includes another (see `game.rs`).
//!
//!   `cargo run --example prefabs`
//!   `HEADLESS=1 cargo run --example prefabs` (no window: prints the entity counts)

mod game;

fn main() {
    if std::env::var_os("HEADLESS").is_some() {
        let world = game::game().run_headless(game::SCRAP_AT + 1, |ctx| {
            let carts = game::setup(ctx);
            println!("{} carts, {} entities", carts.len(), ctx.world.len());
        });
        println!("after one cart is scrapped: {} entities", world.len());
        return;
    }
    game::game().run(|ctx| {
        game::setup(ctx);
    });
}
