# Ponko

Ponko is a general-purpose 2D/3D game engine in Rust, with C and C++ bindings.
It aims to combine serious game-engine capabilities, measured performance,
clear code-first APIs and practical AI-assisted development. Large simulations
are a strength, alongside ordinary 2D and 3D games.

You do not need a large visual editor. Start with `Game::new()`, setup code and
systems; use external asset tools and the optional runtime inspector when they
help. Modular internals should support an integrated experience: common things
work immediately, advanced things can be customized, and neither requires
understanding the whole engine.

Read the [project vision](https://github.com/ponkoengine/ponko/blob/main/PROJECT_VISION.md),
[architecture](https://github.com/ponkoengine/ponko/blob/main/ARCHITECTURE.md),
and [roadmap](https://github.com/ponkoengine/ponko/blob/main/ROADMAP.md) for direction,
current limitations and planned work. The [Bevy and Unity comparison](comparison.md)
explains the tradeoffs; Ponko does not claim feature parity or overall superiority.

## Start small, scale when needed

A small game can keep state in a closure. This example loads `assets/crab.png`,
moves it with the arrow keys, and quits on Escape:

<div class="lang-pair">
<div>

```rust
use ponko::prelude::*;

fn main() {
    Game::new("My Game", 800, 600).run(|ctx| {
        let crab = ctx.load_texture("crab.png");
        let mut pos = vec2(400.0, 300.0);
        move |ctx: &mut Ctx| {
            if ctx.key_down(Key::Right) { pos.x += 200.0 * ctx.dt(); }
            if ctx.key_down(Key::Left) { pos.x -= 200.0 * ctx.dt(); }
            if ctx.key_pressed(Key::Escape) { ctx.quit(); }
            ctx.draw(&crab, pos);
        }
    });
}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:first_game_full}}
```

</div>
</div>

The C++ wrapper provides `ponko::` through `ponko.hpp`; legacy spellings remain supported during
the rename. Continue with [Getting started](getting-started.md) for installation
and the paired API examples.

Grow into [entities and systems](ecs.md), fixed-step simulation and parallel
queries as needed. A [headless run](headless.md) advances simulation without a
window or GPU. Configure a minimal build with Cargo features when those
dependencies should also be omitted. Parallel query iteration exists today;
automatic dependency-aware parallel system scheduling is planned.

## Current tools and future goals

Ponko includes a wgpu 2D/3D renderer, physics integrations, scenes/prefabs,
audio, UI, networking and native bindings. Feature availability and platform
limits vary; use the relevant chapters and roadmap before relying on a complete
workflow. Renderer restructuring and stronger ECS scheduling are gradual
improvements to existing systems, not replacements already delivered.

[AI-agent tools](agents.md) can launch a game, inspect reflected state, step
exact ticks, read logs and measure performance. For visual checks, launch a
windowed instance and request a screenshot. A useful loop is: edit, launch,
run a fixed scenario, inspect state and timings, correct, and repeat with
evidence. Broader automated acceptance workflows and reliable autonomous
creation of complete games remain goals.

## How this book is laid out

- **Guide:** practical chapters for game code, assets, rendering, simulation and tools.
- **Reference:** benchmarks, public APIs, compatibility and contribution guidance.

Every page is searchable: press `S` or click the magnifying glass. Use
[performance measurements](performance.md) to understand specific workloads;
benchmark results do not establish a general engine ranking.

## Status and compatibility

Formerly Ponko, Ponko currently has package version 0.0.1. Source folders
retain `ponko_*` names. The guide follows `main`; installed releases may differ.
[STABILITY.md](https://github.com/ponkoengine/ponko/blob/main/STABILITY.md)
remains authoritative for published API/ABI, MSRV, saved-world, MCP and
determinism commitments. The rename does not weaken those promises.

Help improve the engine through
[Contributing](https://github.com/ponkoengine/ponko/blob/main/CONTRIBUTING.md).
