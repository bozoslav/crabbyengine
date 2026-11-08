# Ponko

Ponko is a Rust game engine with an archetype ECS, 2D and 3D rendering,
physics, audio, networking, a runtime inspector, and C/C++ bindings.

Formerly Crabby Engine. Ponko starts a new release line at **0.0.1**;
APIs may change while the engine develops.

```toml
[dependencies]
ponko = "0.0.1"
```

```rust,no_run
use ponko::*;

fn main() {
    Game::new("Ponko", 800, 600).run(|_| move |_ctx: &mut Ctx| {});
}
```

Packages: `ponko`, `ponko_ecs`, `ponko_ffi`, `ponko_mcp`,
`ponko_unity`, and `ponko_bench`. The MCP executable is `ponko-mcp`.
The existing C/C++ headers and `crabby_*` symbols remain available.
Source directories retain their existing names during the transition.

Website: <https://ponko.dev/> (being set up).
API documentation: <https://docs.rs/ponko>.

Licensed under MIT OR Apache-2.0.
