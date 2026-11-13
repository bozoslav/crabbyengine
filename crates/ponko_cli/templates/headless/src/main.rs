//! Runs the simulation for `TICKS` ticks (default 600, ten simulated
//! seconds) and prints a summary. `ponko-mcp`'s `launch` starts it paused,
//! so an agent can step it and look inside.

fn main() {
    let ticks = std::env::var("TICKS").ok().and_then(|t| t.parse().ok()).unwrap_or(600);
    let start = std::time::Instant::now();
    let world = {{crate_name}}::simulation().run_headless(ticks);
    let bounces = world.resource::<{{crate_name}}::Bounces>().0;
    println!(
        "{ticks} ticks in {:.1} ms: {} walkers, {bounces} bounces",
        start.elapsed().as_secs_f64() * 1e3,
        world.len()
    );
}
