# Changelog

All notable changes to Crabby Engine are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions will
follow [Semantic Versioning](https://semver.org/) from the first release on.

## [Unreleased]

Everything so far, ahead of the first release.

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
  packets recovered, and desync checks that name the components that
  differ. `Net::loopback` for tests. In C and C++ too.
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

[Unreleased]: https://github.com/bozoslav/crabbyengine/commits/main
