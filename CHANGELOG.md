# Changelog

All notable changes to Crabby Engine are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow
[Semantic Versioning](https://semver.org/); [STABILITY.md](STABILITY.md)
says what that covers.

## [Unreleased]

### Added
- Polygon colliders: `Collider::polygon` (3 to 8 corners, convex), in C
  (`crabby_set_polygon_collider`, `crabby_collider_points`) and C++
  (`crabby::polygon_collider`).
- Scenes and prefabs: `ctx.load_scene`/`save_scene` keep a whole world in a
  file under `assets/`; `ctx.spawn_prefab`/`save_prefab` keep one entity's
  components and spawn copies where you say. In C (`crabby_ctx_load_scene`
  and friends) and C++ (`ctx.load_scene` and friends).
- `Sprite` is reflected in every game, its texture written as the image's
  name in `assets/` (`"texture": "crab.png"`); setting or loading one loads
  the image. `Color` and `Rect` implement serde's traits.
- The inspector is an editor while the game is paused: click to select,
  drag to move, pan and zoom, Delete, Ctrl+D duplicate, Ctrl+Z undo, add
  components, place prefabs from `assets/prefabs/`, save and load scenes,
  and Play / Stop (Stop puts the world back as it was at Play).
- 3D, behind the `3d` feature: `Transform3D` and `Model` components on
  the same ECS, meshes (`MeshData::cube`/`plane`/`sphere`, your own
  triangles, or glTF with `ctx.load_mesh`), physically based materials
  (`ctx.create_material`), `ctx.camera3d` with `screen_ray` picking, and
  `ctx.lighting`: a sun with soft shadows that follow the camera, plus sky
  light. Instanced by mesh, culled to the view, 4x multisampled, drawn
  under the 2D layer. 100,000 spinning cubes hold 120 fps. In C (`crabby_ctx_create_mesh`
  and friends; `CrabbyTransform3D` and `CrabbyModel` are plain query
  columns) and C++ (`crabby::transform3d_at`, `crabby::mesh_data`,
  `ctx.create_material`, ...). The `3d` example and the "3D" book chapter.
- The MCP `launch` tool takes `features`, for games behind Cargo features.
- `Game::fullscreen(true)` fills the screen (C: `crabby_game_fullscreen`,
  C++: `game.fullscreen()`).
- The `3d` example's benchmark mode (`BENCH_SECONDS`), and the same scene
  in Unity 6 under `bench/unity`: 100k spinning cubes run at 402 FPS in
  Crabby and 54 in Unity.

### Fixed
- `vsync(false)` no longer waits for the window's redraw events, which
  macOS sends only once per display refresh.
- Control endpoint `scene.save`, `scene.load`, `prefab.spawn`,
  `prefab.save`, and the MCP tools `save_scene`, `load_scene`,
  `spawn_prefab`, `save_prefab`, so agents can build levels.

## [1.0.0] - 2026-11-03

The first release: a 2D engine for simulations with up to millions of
entities, in Rust, C and C++. Install with `cargo add crabbyengine`; the C
library and headers for Linux, macOS and Windows are attached to the
GitHub release. Everything below is new.

### Simulation
- Own archetype ECS (`crabby_ecs`): entities with generational ids, queries,
  `par_for_each` over archetype chunks on all cores, commands, resources,
  and components registered at runtime (for C and C++).
- Fixed-rate simulation clock: pause, step one tick, 1–64× speed, systems
  that run every N ticks (`sim_system_every`, `ctx.turns`).
- Spatial grids with radius, rectangle and nearest queries; spatial sorting
  so neighbors on the map are neighbors in memory.
- Flow-field pathfinding (`NavGrid`, `FlowField`).
- Events between systems; attachments (things that follow other things).
- 2D physics with rapier (`physics` feature): `RigidBody`, `Collider` and
  `Velocity` components synced with `Transform2D`, `CollisionStarted` /
  `CollisionStopped` events, ray casts, overlap queries and impulses
  (`ctx.physics`), and `physics-determinism` for bit-identical results across
  platforms. In C (`crabby_game_physics`, `crabby_ctx_cast_ray`, ...) and C++.
- Deterministic lockstep multiplayer for 2–8 players (`Game::lockstep`,
  `Net::host` / `Net::join`, `ctx.net`): plain UDP, input delay, lost
  packets recovered (a peer that stops keeps resending for up to a second
  until the others have its last inputs), and desync checks that name the
  components that differ. `Net::loopback` for tests. In C and C++ too.
- Lockstep over Steam (feature `steam`): `Net::steam_host` / `steam_join`,
  friends-only lobbies, overlay invites, Steam's relays (no port
  forwarding). The Steam library is copied next to examples and tests. In C
  and C++ with crabby_ffi's `steam` feature.
- Determinism: seeded randomness per (seed, tick, system, entity), the same
  with parallel queries. Save and load the whole world, keeping entity ids.
- Headless runs (`Game::run_headless`) and an optional `window` feature for
  servers and CI.

### Drawing, sound and input
- wgpu renderer: an instanced sprite batcher, parallel instance building,
  GPU timings (`ctx.render_stats()`).
- `Camera2D` (pan, zoom, follow), text, shapes, simple UI.
- Sprite sheets, animation and tilemaps.
- Sound through kira (`audio` feature).
- Gamepads through gilrs (`gamepad` feature, on by default): buttons named
  by position, sticks and triggers, up to 8 pads in fixed slots, plugged in
  and out while the game runs; in C and C++ too.

### Fast iteration
- Hot reload of images, sounds, fonts and JSON data files.
- Hot-patching of game code with `dx serve --hot-patch` (`hotpatch` feature).
- The runtime inspector (`inspector` feature, the `` ` `` key).
- Reflection: components and resources as JSON.
- Faster debug builds (dev opt-level 1).

### Tools and agents
- Control endpoint (`CRABBY_CONTROL=1`): localhost, token-protected.
- `crabby_mcp`: an MCP server for AI agents (launch, inspect, edit, step,
  events, logs, stats, screenshots, save and load).

### Web
- Web builds (wasm32): the same game runs in a browser, drawing with WebGPU
  or WebGL2. Assets listed in `assets/index.json` are fetched before setup,
  sound plays through WebAudio, and the inspector opens with `?inspector=open`.
- `web/build.sh` builds an example or a game's binary into a static site.
- `crabbyengine::Instant`: an `Instant` that also works in browsers.
- The docs site publishes playable builds of the battle, flow and ants demos.

### API and stability
- [STABILITY.md](STABILITY.md): semver from 1.0, deprecation policy, MSRV
  (Rust 1.95, checked in CI), C ABI versioning, save compatibility and what
  is deterministic where.
- Every public item is documented (`missing_docs` is enforced), with
  `# Panics` / `# Errors` sections where they apply.
- `#[must_use]` on getters, constructors and builders; `Game` and `Query`
  warn when built and never run.
- `#[non_exhaustive]` on `NetState`, `BodyKind`, `Shape`, `FieldType` and
  `ReflectError`, so new variants aren't breaking changes.

### C and C++
- C++: components are found by their type, with nothing to register:
  `world.spawn(transform_at(p), Health{100})`, `world.get<Health>(e)`,
  `world.each([](transform2d& t, const Vel& v) { ... })` (as fast as a
  hand-written loop), `each_par` on all cores, `ctx.send(Died{e})` and
  `ctx.read<Died>()`. `world.reflect<T>(fields)` describes a type for tools
  and saves. Replaces component tokens (`world.component<T>("Name")`,
  `token.with(value)`).
- Resources from C and C++: `world.set_resource(Score{...})`,
  `world.resource<Score>()`, `try_resource`, `remove_resource`, and
  `reflect_resource<T>(fields)` so agents and saves see them (C:
  `crabby_resource_*`, by name). The C++ battle keeps its score in one.
  `ctx.load_json<T>(path)` loads a data file into a resource, live.
- Fixed: `ctx.load_json<T>(path, e)` failed for C++ types in a namespace.
- C API (`crabby_ffi`) with a generated header, and a header-only C++17
  wrapper (`include/crabby.hpp`) covering nearly the whole engine.

[Unreleased]: https://github.com/bozoslav/crabbyengine/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/bozoslav/crabbyengine/releases/tag/v1.0.0
