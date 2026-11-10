# Ponko vs Bevy vs Unity

**Last reviewed: 2026-11-08.** Ponko is the engine previously named Ponko
Engine. This comparison describes the checked-in implementation and the
linked official documentation; it does not claim feature parity or general
performance superiority. Product versions and tool availability change.

Ponko aims to make a capable 2D/3D engine accessible through a small,
code-first public API and integrated runtime tools. Its differentiator is
that combination of choices. Code-first development, ECS, headless runs,
modularity and remote inspection are also available in other engines.

## The programming model

Ponko uses Rust functions or closures taking `&mut Ctx`. The same engine has
a C ABI and C++17 wrapper. Ordinary games start with `Game::new`, optional
builder settings, and `run`; no plugin framework or large visual editor is
required. See [getting started](getting-started.md) for a complete game and
[C and C++](c-cpp.md) for the equivalent native API.

Bevy is also a code-first Rust engine. Its `App`, systems and plugins form
its application model. `DefaultPlugins` already supplies ordinary engine
facilities; developers do not have to assemble those individually. It also
supports selecting or replacing plugins. See the official
[plugin guide](https://bevy.org/learn/quick-start/getting-started/plugins/).

Unity's usual workflow combines C# scripts, components, assets, scenes and
its visual Editor. This is often useful for teams that author scenes and
content visually. Unity also provides an Entities package for data-oriented
work, so its GameObject workflow is not its only programming model. The
[Unity manual](https://docs.unity3d.com/6000.0/Documentation/Manual/index.html)
and [Entities concepts](https://docs.unity3d.com/Packages/com.unity.entities@1.4/manual/concepts-intro.html)
describe those alternatives.

## ECS and parallelism

Ponko stores component columns in archetypes and gives entities generational
IDs. Typed queries, optional components, `with`/`without` filtering, runtime
component registration and parallel query iteration exist today in
`crates/ponko_ecs`. A small fixed-step simulation can be checked without a GPU:

<div class="lang-pair">
<div>

```rust
use ponko::prelude::*;

struct Pos(f32);

fn fall(ctx: &mut Ctx) {
    let dt = ctx.dt();
    ctx.world.query::<&mut Pos>().for_each(|p| p.0 -= 10.0 * dt);
}

#[test]
fn things_fall_ten_units_per_second() {
    let mut world = Game::new("test", 800, 600)
        .sim_system(fall)
        .setup(|ctx| {
            ctx.world.spawn((Pos(100.0),));
        })
        .run_headless(60);
    let y = world.query::<&Pos>().iter().next().unwrap().0;
    assert!((y - 90.0).abs() < 1e-3);
}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:headless}}
```

</div>
</div>

This uses the existing Rust and C++ headless APIs. Parallelizing entities
*within* a query is different from running
independent systems concurrently: Ponko's system loops currently run in
registration order. Arbitrary `&mut Ctx` callbacks remain exclusive.

Bevy's ECS uses typed system parameters to express access, supports table
and sparse-set component storage, and schedules compatible systems in
parallel. Explicit ordering still matters. See
[Bevy ECS](https://docs.rs/bevy_ecs/0.20.1/bevy_ecs/) and its
[system scheduling tutorial](https://bevy.org/learn/quick-start/getting-started/ecs/).
Ponko has declared system access and restricted system views
([#137](https://github.com/ponkoengine/ponko/issues/137), see
[Declared system access](system-access.md)); the optional access-aware
scheduler that uses them is **planned**, following
[benchmarks #132](https://github.com/ponkoengine/ponko/issues/132) and
[barrier contracts #146](https://github.com/ponkoengine/ponko/issues/146),
before [parallel scheduling #138](https://github.com/ponkoengine/ponko/issues/138).
It must preserve the simple existing workflow and cannot safely be achieved
by sharing unrestricted mutable contexts between threads.

Unity Entities supplies ECS systems and archetype/chunk storage; Unity's
[Job System](https://docs.unity3d.com/6000.0/Documentation/Manual/JobSystem.html)
and [Burst](https://docs.unity3d.com/Packages/com.unity.burst@1.8/manual/index.html)
provide parallel and optimized code paths. Comparing Ponko's ECS against
Mono GameObject callbacks does not measure Unity Entities/Jobs/Burst.

## Rendering and customization

Ponko's custom wgpu renderer already draws 2D sprites and 3D meshes with
materials, lighting, shadows and post-processing. WGSL surface shaders,
full-screen effects and compute workloads have existing APIs. Large static
instance batches use GPU culling on supported devices, with device-dependent
fallbacks. See [3D](3d.md), [profiling](profiling.md) and the source modules
listed in [ARCHITECTURE.md](https://github.com/ponkoengine/ponko/blob/main/ARCHITECTURE.md).

Those features do not establish graphics parity. Resource and pipeline
ownership, history management and pass coordination remain tightly coupled
in `src/render/gpu3d.rs`. The roadmap calls for measured subsystem refactoring
[#136](https://github.com/ponkoengine/ponko/issues/136), explicit internal
pass dependencies [#147](https://github.com/ponkoengine/ponko/issues/147),
pipeline caching [#152](https://github.com/ponkoengine/ponko/issues/152) and
capability contracts [#151](https://github.com/ponkoengine/ponko/issues/151).
A general safe custom-pass API [#155](https://github.com/ponkoengine/ponko/issues/155)
is planned; current shader hooks are not full renderer replaceability.

Bevy also uses wgpu. Its renderer exposes extraction, preparation and
rendering infrastructure plus render-graph and material extension points;
see [bevy_render](https://docs.rs/bevy_render/0.20.1/bevy_render/).
Unity provides multiple render pipelines and documented customization,
including [URP](https://docs.unity3d.com/6000.0/Documentation/Manual/urp/urp-introduction.html)
and [HDRP](https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/index.html).
Choose from actual visual requirements, target hardware, tooling and
maintenance cost. A shared graphics API or one favorable scene is not an
overall renderer comparison.

## AI development, inspection and evidence

Ponko's `ponko-mcp` server exposes tools for launching examples, inspecting
reflected entities/resources, modifying scenes, stepping paused simulations,
reading logs, capturing windowed screenshots and requesting performance
reports. The [agent guide](agents.md) documents their real names and inputs.
The optional inspector and control endpoint complement ordinary debugging.
Headless mode cannot produce a rendered screenshot.

For example: launch a seeded reproduction paused, capture a checkpoint,
run a fixed tick count, inspect entity state, change code, rebuild and rerun.
For a visual change, also capture a windowed frame at the same camera and
quality settings. State inspection, successful compilation and screenshots
answer different questions; none substitutes for all the others.

Bevy's official [Remote Protocol](https://docs.rs/bevy_remote/0.20.1/bevy_remote/)
allows inspection and modification of ECS data through JSON-RPC, including
queries and schema discovery. This is real runtime tooling; Ponko does not
have an exclusive claim to remote ECS inspection. BRP and MCP are different
protocols, so client integration must be checked rather than assumed.

Unity provides an official AI Assistant and MCP integration. Its current
[CLI migration guidance](https://docs.unity.com/en-us/unity-cli/replace-mcp-server-unity-cli)
says the in-editor Assistant MCP server is deprecated and replaced by the
CLI's `unity mcp` mode. This is a tool migration, not removal of Unity's
agent support. Follow the current Unity setup and access requirements.

Ponko's structured shader diagnostics
[#153](https://github.com/ponkoengine/ponko/issues/153), schedule/query
inspection [#145](https://github.com/ponkoengine/ponko/issues/145) and expanded
GPU reporting [#157](https://github.com/ponkoengine/ponko/issues/157) are
planned improvements to existing tooling. Reliable autonomous completion
of arbitrary games is a goal to investigate, not a present capability.

## Multiplayer, determinism and servers

Ponko includes UDP lockstep, snapshots/replication and message-oriented
network APIs, with optional Steam integration. These are building blocks;
a shipped game's authority, prediction, recovery and hosting behavior still
need game-specific validation. Headless `Game::run_headless` runs fixed
simulation ticks without rendering. Disabling default features removes the
window/GPU path; enabling only what a server needs is supported today.

Read [networking](networking.md), [headless execution](headless.md) and
[determinism](determinism.md) before choosing a synchronization model.
Reproducibility assumes matching builds, seeds and inputs. Cross-platform
physics determinism requires the appropriate feature; floating-point
transcendentals can differ between platforms. These are not blanket
bit-identical-world guarantees.

Bevy supports headless applications through its plugin selection; networking
is an integration decision that should be evaluated against the chosen
library and release. Its ECS scheduler does not by itself guarantee
cross-platform deterministic gameplay. Unity has both
[desktop headless and Dedicated Server workflows](https://docs.unity3d.com/6000.0/Documentation/Manual/desktop-headless-mode.html)
and an official [multiplayer stack](https://docs.unity.com/en-us/multiplayer)
covering netcode and player connections. Headless operation and multiplayer
are not unique to Ponko.

## Assets, portability and ecosystem

Ponko has direct asset loading, scenes/prefabs, glTF model loading and a
Unity text-scene importer with FBX-related processing. External tools such
as Blender remain appropriate for content creation. Importing supported
scene data does not run arbitrary Unity scripts, packages or shaders.
Texture and mesh residency budgets, asynchronous decoding with a per-frame
upload budget, texture resolution streaming and streamed LOD levels are
described in [Streaming](streaming.md); asset lifecycle
stress diagnostics [#161](https://github.com/ponkoengine/ponko/issues/161)
remain open. Do not infer a complete production asset pipeline from a
successful import of one asset.

Ponko CI covers Linux, macOS and Windows and builds the browser target.
WebGPU/WebGL2 fallback support does not mean that every desktop feature
works on every browser. Mobile and consoles are not currently validated
release targets. [STABILITY.md](https://github.com/ponkoengine/ponko/blob/main/STABILITY.md)
is the compatibility contract; a crate-name/version reset does not authorize
breaking its existing C ABI, saved-world or MCP contracts.

Bevy has a broader public contributor/plugin ecosystem; Unity has a much
longer production history, commercial content/tooling ecosystem and broader
platform workflows. These maturity assessments are practical judgments,
not benchmark results. Check the maintained
[Bevy ecosystem directory](https://bevy.org/assets/),
[Unity platform documentation](https://docs.unity3d.com/6000.0/Documentation/Manual/PlatformSpecific.html)
and required third-party integrations for your actual project.

## Choosing an engine

Choose Ponko when its compact Rust/C++ interface, integrated simulation and
runtime-control workflow fit your project, and you are willing to validate
missing production requirements and contribute to an early engine.

Choose Bevy when you want a Rust ECS-centered ecosystem with automatic
compatible-system scheduling and more existing plugin integrations. Both
engines support code-first development and useful defaults.

Choose Unity when visual content authoring, established production tools,
team experience, commercial integrations or supported deployment targets
are decisive. Its modular packages and high-performance data-oriented path
should be evaluated alongside its familiar GameObject workflow.

Use [the performance chapter](performance.md) as a record of specific
reproducible workloads, not a league table of engines. Match workloads,
resolution, quality, build configuration and target hardware before drawing
conclusions. Consult [the roadmap](https://github.com/ponkoengine/ponko/blob/main/ROADMAP.md)
for unresolved foundations; it is directional and has no promised dates.
