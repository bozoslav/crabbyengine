# Crabby Engine

Crabby Engine is a 2D game engine in Rust, built for games with huge numbers of
things in them: colonies, ecosystems, crowds, factories, swarms. The engine
keeps two goals in view:

- **Simulate a lot.** Millions of entities updated every tick on all CPU cores,
  separate from how many you draw. A sim can run without a window at all.
- **Stay easy to change.** Everything the engine does by itself is a plain
  system you can remove or replace. One obvious way to do each thing.

```rust
use crabbyengine::*;

struct Vel(Vec2);

fn movement(ctx: &mut Ctx) {
    let dt = ctx.dt();
    ctx.world
        .query::<(&mut Transform2D, &Vel)>()
        .par_for_each(|(t, v)| t.position += v.0 * dt);
}

fn main() {
    Game::new("Crabs", 1280, 720)
        .sim_system(movement)
        .run(|ctx| {
            let crab = ctx.load_texture("crab.png");
            for i in 0..100_000 {
                let pos = vec2((i % 1280) as f32, (i / 1280) as f32 * 8.0);
                ctx.world.spawn((Transform2D::at(pos), Sprite::new(crab), Vel(vec2(20.0, 0.0))));
            }
        });
}
```

## How this book is laid out

- **Guide**: one chapter per idea, in the order you'll need them.
- **Reference**: benchmark numbers, the generated API docs, and how to
  contribute.

Every page is searchable: press `S` or click the magnifying glass.

## Status

The engine is young and its API still changes between milestones. Code in
this book is kept in sync with the `main` branch.
