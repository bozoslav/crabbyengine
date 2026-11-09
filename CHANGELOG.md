# Changelog

All notable changes to Crabby Engine are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow
[Semantic Versioning](https://semver.org/); [STABILITY.md](STABILITY.md)
says what that covers.

## [Unreleased]

### Render-pass inspector (first stage)

- `perf.get` (MCP `perf_report`) has a `render_inspector` section with a
  versioned schema: the renderer's CPU time split into extract, prepare,
  encode, submit and other; GPU time per frame and per pass; draw, model,
  triangle and occlusion counts; and the active graphics quality. Metrics
  that can't be measured (no timestamp queries, headless, not counted yet)
  are `null` with a reason in `unavailable`, never zero. Upload bytes,
  memory estimates and frustum-culled counts are reserved as `null` for
  later stages of #157.

### Explicit runtime phases (first stage)

- Add `Phase` and `Game::add(phase, system)` for sequential fixed-tick and
  frame scheduling. Within each phase, registration order is preserved.
- `sim_system` and its interval/named forms map to `AfterPhysics`; `system`
  maps to `Frame`, `late_system` to `Late`, and built-in drawing to `Draw`.
  Physics installation always schedules the step in `Physics`. Code that
  previously registered simulation logic before `.physics()` or `.physics3d()`
  to run before physics should use `add(Phase::BeforePhysics, logic)`.
- Plugin composition, stable system IDs, and migrating automatic animation,
  particles and audio to inspectable systems remain follow-ups to #198.

### Added
- `ActionId`: resolve an action name once with `Ctx::action_id` (or
  `action_ids`, or the `actions!` macro, which keeps the names in one
  declaration) and pass the `Copy` id to `action_down`, `action_pressed`,
  `action_released`, `action_axis` and `action_axis2`, which now take an id or
  a name. An unknown name logs an error when it is resolved
  (`try_action_id` returns it). Ids stay valid across `set_actions`. Rust
  only for now.
- `Ctx::load_component_json::<T>(path, entity)` and
  `load_resource_json::<T>(path)` take the name from the type.

### Changed
- The by-name forms of the two JSON loaders (used by the C API) are now
  `load_component_json_named` and `load_resource_json_named`; they are hidden
  from the docs.

## [0.0.1] - 2026-11-08

### Changed
- Rename the Rust packages to Ponko and start a new 0.0.1 release line.
- Publish the Unity importer and benchmark packages alongside the engine.
- Retain existing C/C++ symbols and source-directory paths.


### Added
- Lens post effects: `Post::chromatic_aberration` (color fringes toward
  the edges), `lens_distortion` (barrel or pincushion) and `lens_scale`.
  In C and C++ as `CrabbyPost` fields.
- A playing `AudioSource3D` now follows changes to `settings.effects`
  (low-pass cutoff and reverb send), smoothed, instead of keeping the
  effects it started with.
- Named input actions (behind no feature): bind keyboard, mouse and gamepad
  inputs to names once and query those names in game systems.
  `Action::Button`, `Action::Axis` and `Action::Axis2` take several sources
  each (buttons OR together; axes pick the strongest source), so keyboard and
  controller share one set of names. `ctx.action_down`, `action_pressed`,
  `action_released`, `action_axis` and `action_axis2` read them;
  `ctx.actions()` and `ctx.set_actions` rebind at runtime;
  `ActionMap::to_json`/`from_json` save bindings as JSON; `feed_input`
  feeds input by hand for headless tests and replay. In C
  (`crabby_action_map_new` and friends, `crabby_ctx_action_down` and
  friends) and C++ (`crabby::action_map`, `action_binding`). New
  `action_map` example.
- Frame profiling: `ctx.frame_stats()` (fps, frame-time percentiles, CPU
  split into systems, render and display wait, GPU time), `ctx.gpu_passes()`
  (GPU time per pass: every shadow cascade, prepass, opaque, transparent,
  effects, bloom, post), `render_stats()` now counts models and triangles
  for the camera and the shadow maps separately, `ctx.mesh_costs()` per
  mesh, a performance HUD (`ctx.set_perf_hud`, `CRABBY_HUD=1`), Perfetto
  traces (`ctx.record_trace`, `CRABBY_TRACE`), an offscreen benchmark mode
  (`CRABBY_BENCH=<frames>`, with a JSON and CSV report), and the MCP tools
  `perf_report`, `record_trace` and `perf_hud`. The 3D renderer records its
  frame parts and culls in parallel; `Game::msaa(2)` is an option; on
  tile-based GPUs solid surfaces skip the depth prepass; a `ShadowMesh`
  component gives an entity a simpler shadow caster, a shadow-only shape,
  or none (the Unity importer maps Cast Shadows: Off and Shadows Only to
  it). In C (`crabby_game_msaa`, `crabby_ctx_frame_stats`,
  `crabby_ctx_set_perf_hud`, `crabby_ctx_record_trace`,
  `crabby_ctx_set_shadow_mesh`) and C++ (the same `ctx` methods,
  `game.msaa()`). The `3d` example's 100,000 cubes: 255 → 318 FPS.
- Vehicles (`physics3d` feature): `Vehicle3D` puts a dynamic body on up
  to 8 raycast `Wheel3D`s with spring-damper suspension, slip-based tyre
  grip through `FrictionCurve`s and wheel spin from motor and brake
  torque; the game sets each wheel's torques and steer angle per tick and
  reads back contact, compression, slip, rpm and the wheel's pose for
  drawing. `RigidBody3D::mass` and `::center_of_mass` override a body's
  mass and center of mass; `Physics3D::center_of_mass` reads it. In C
  (`CrabbyVehicle3D`, `crabby_wheel3d`, `crabby_vehicle3d_add_wheel`,
  `crabby_wheel3d_pose`, `crabby_friction_curve_*`,
  `crabby_ctx_body_center_of_mass3d`) and C++ (`crabby::vehicle`,
  `crabby::wheel`, `crabby::wheel_pose`). New `vehicle` example.
- Point and spot lights: `PointLight3D` and `SpotLight3D` components found
  by `Transform3D`, spots aiming along local −Z. Visible lights are binned
  into clusters per camera and a mesh fragment lights only its cluster's
  lights, with the packing WebGL2-friendly (no storage buffers). Sunlight
  and lights share one GGX lighting function; custom shaders call
  `punctual_light` and `direct_light`. 1,000 visible lights cost about
  0.14 ms of CPU and 1 ms of GPU at 720p (M4 Max, shadows off). In C
  (`CrabbyPointLight3D`, `CrabbySpotLight3D`) and C++
  (`crabby::point_light3d`, `spot_light3d`). New `lights` example.
- The Unity importer imports colliders: `Importer::colliders(true)` turns
  enabled box, sphere, capsule and mesh colliders into entities with a
  fixed `RigidBody3D` and a `Collider3D`. Scene files gain
  mesh-collider shapes, built once and shared by `ctx.load_scene`.
- Meshes that cast no shadow: `ctx.set_mesh_shadows(mesh, false)` leaves
  every model using that mesh out of the shadow maps (grass tufts,
  pebbles) while still drawn; `ctx.mesh_shadows(mesh)` reads it. In C
  (`crabby_ctx_set_mesh_shadows`, `crabby_ctx_mesh_shadows`) and C++
  (`ctx.set_mesh_shadows`, `ctx.mesh_shadows`).
- The Unity importer leaves out renderers set to Shadows Only (Unity never
  draws them either) and reports how many.
- 3D physics (`physics3d` feature): `RigidBody3D`, `Collider3D` and
  `Velocity3D` components — sphere, box, capsule, convex hull, triangle
  mesh and heightfield shapes, friction, restitution, sensors and
  collision groups, a fixed 50 Hz step, drawn transforms interpolated
  between steps; setting `Transform3D` teleports the body.
  `Physics3D::heightfield` makes a terrain collider in the exact layout of
  a terrain tile. Queries on `ctx.physics3d`: `cast_ray`, `cast_ray_all`,
  `cast_sphere`, `cast_capsule`, `overlap_sphere`, `overlap_box`,
  `overlap_capsule` (with a `QueryFilter3D`), plus `apply_impulse`.
  `Character3D` is a kinematic walking capsule: slope limit, step height,
  snap to ground, pushes light bodies; set `velocity`, read `grounded`.
  `ctx.set_cursor_locked` grabs the mouse for first-person look and
  `ctx.mouse_motion` reads raw motion. Collision events are shared with
  the 2D physics. In C (`crabby_ctx_cast_ray3d` and friends,
  `crabby_ctx_apply_impulse3d`) and C++ (`dynamic_body_at`, `box_collider`,
  `mesh_collider`, `character`, `filter3d`, `ctx.heightfield_collider()`,
  `game.physics3d()`). New `physics3d` example.
- Distance fog: `Sky::fog_start` and `::fog_end` blend the world into the
  fog from one distance to another, whatever the camera height, so an
  open world fades out before its streamed edges; `Sky::fog_sun_glow`
  (forward scattering of sunlight) makes the haze brighten toward the sun,
  height fog included. The sky itself takes none, so the sun disk stays
  visible. In C and C++ (the four new `Sky` fields), with a Book example
  in both.
- Custom material shaders get `origin` — the model's position — in both
  `vertex` and `surface`, so each instance can vary on its own (how high
  a pixel is above the model's base, a color per model, a sway phase)
  without a material per model.
- A second set of texture coordinates: `MeshData::uvs1` (glTF's
  `TEXCOORD_1`), read in custom shaders from `Vertex.uv1`/`Surface.uv1`;
  the Unity importer writes it for meshes drawn with a game's own
  shaders.
- Materials imported from Unity keep their names:
  `ctx.loaded_materials(name)` finds every material loaded from glTF
  files of that name with the `MaterialDesc` it was made from, so a game
  keeps its colors and textures and swaps in its own shader with
  `ctx.update_material`. `Importer::custom_shader(path)` marks imported
  materials for one of the game's shaders and writes `materials.json`
  with everything they set, textures as PNGs;
  `Importer::vertex_colors(shader, f)` fills a mesh's vertex colors from
  any Unity channel (a second UV set, say) for such shaders. The
  importer is a Rust tool; run it once and keep the files it writes.
- `ponko_unity` imports Unity scenes and prefabs into Crabby scene
  files: a scene's tree in `crabby-world` JSON, a `.glb` per drawn mesh
  with glTF PBR materials (URP Lit, Simple Lit and Unlit, with an ORM map
  packed from Unity's metallic/smoothness/occlusion), PNG textures shared
  across scenes, FBX and OBJ meshes via ufbx, built-in Cube/Plane/Quad.
  `Model` is reflected (scenes save it by its mesh file, and loading a
  scene loads the meshes first); glTF loading shares images between files.
  New `scene_viewer` example.
- `ponko_unity`, a workspace crate (not published yet): reads Unity
  projects as the importer's ground — text serialized files parsed into a
  lossless map, editor/Unity 6 binary files through their type trees, and
  GUIDs mapped to files from the `.meta`s.
- `Heightfield`: the ground's height and facing under a point on the CPU
  (`height_at`, `normal_at`), following the drawn triangles exactly, for
  standing things on terrain; `TerrainTileData::heightfield` keeps one. In
  C (`crabby_heightfield_new`, `crabby_heightfield_at`,
  `crabby_heightfield_free`) and C++ (`crabby::heightfield`).
- Streaming big worlds: `CellStreamer` says which cells of a ground grid
  to load (nearest first, a few per frame) and drop around a point;
  `Job::spawn` runs work on a worker thread and `job.take()` collects it;
  `TerrainTileData` builds a terrain tile on any thread and
  `ctx.upload_terrain_tile` puts it on the GPU; `ctx.free_mesh`,
  `ctx.free_terrain_tile` and `ctx.is_mesh_live` free GPU memory. In C
  (`crabby_cells_*`, `crabby_terrain_tile_data_new`,
  `crabby_ctx_upload_terrain_tile`, `crabby_ctx_free_mesh`,
  `crabby_ctx_free_terrain_tile`, `crabby_ctx_is_mesh_live`) and C++
  (`crabby::cell_streamer`, `crabby::terrain_tile_data`, and the same
  `ctx` methods; C++ uses `std::async` for jobs). New `streaming` example.
- Compute passes: `ctx.create_compute` and `ctx.load_compute` (hot
  reloaded) make a WGSL shader defining `fn compute(c: Cell)`, run once per
  texel of a render target each frame before drawing (ripples, trails in
  snow, wet ground), by adding a `ComputePass` to `ctx.compute_passes`,
  with a material's values and textures. In C
  (`crabby_ctx_create_compute`, `crabby_ctx_load_compute`,
  `crabby_ctx_set_compute_passes`, `crabby_ctx_compute_passes`) and C++
  (`ctx.create_compute`, `ctx.load_compute`, `ctx.set_compute_passes`,
  `ctx.compute_passes()`, `crabby::compute_pass_of`).
- Render targets: `ctx.create_render_target` makes a texture that a
  `TargetCamera` in `ctx.target_cameras` draws the lit 3D world into each
  frame, for materials to read (mirrors, reflections, monitors). An
  optional clip plane tilts the near plane onto a mirror;
  `TargetCamera::reflection` makes the camera for a level one. In C
  (`crabby_ctx_create_render_target`, `crabby_ctx_reflection_camera`,
  `crabby_ctx_set_target_cameras`, `crabby_ctx_target_cameras`) and C++
  (`ctx.create_render_target`, `ctx.reflection_camera`,
  `ctx.set_target_cameras`, `ctx.target_cameras()`,
  `crabby::target_camera_of`).
- 3D screen effects: `ctx.create_effect` and `ctx.load_effect` (hot
  reloaded) make a WGSL shader defining `fn effect(p: Pixel)`, run over the
  lit frame before bloom and tone mapping by adding a `ScreenEffect` to
  `ctx.effects`, with a material's values and textures. In C
  (`crabby_ctx_create_effect`, `crabby_ctx_load_effect`,
  `crabby_ctx_set_effects`, `crabby_ctx_effects`) and C++
  (`ctx.create_effect`, `ctx.load_effect`, `ctx.set_effects`,
  `ctx.effects()`, `crabby::screen_effect_of`).
- Terrain materials: `ctx.create_terrain_material` (`TerrainMaterialDesc`)
  blends up to 8 ground layers by a weight map, with textures lined up
  with the world and optional normal maps. In C
  (`crabby_ctx_create_terrain_material`, `crabby_terrain_material_desc`)
  and C++ (`ctx.create_terrain_material`, `crabby::terrain_material_of`).
- Texture arrays for 3D materials: `ctx.create_texture_array` and
  `ctx.load_texture_array` (`TextureArray`, `TextureKind::Color` or
  `Data`), with mipmaps, read by custom shaders from
  `MaterialDesc::custom_arrays` as `custom_array0` to `custom_array2`. In
  C (`crabby_ctx_create_texture_array`, `crabby_ctx_load_texture_array`)
  and C++ (`ctx.create_texture_array`, `ctx.load_texture_array`).
- Heightfield terrain: `ctx.create_terrain_tile` makes a `TerrainTile`
  from a square grid of heights (NaN cuts a hole), drawn at four levels
  of detail by distance from the camera, with skirts that hide the cracks
  between levels. `MeshData::heightfield` gives the mesh for one level.
  In C (`crabby_ctx_create_terrain_tile`, `crabby_terrain_tile_component`,
  `crabby_mesh_data_heightfield`) and C++ (`ctx.create_terrain_tile`,
  `crabby::terrain_tile`, `mesh_data::heightfield`).
- 3D post-processing, `ctx.post` (`Post`): exposure, bloom, white balance
  (temperature, tint), contrast, saturation, tone mapping (`Tonemap::Aces`,
  `AgX`, `Neutral`, `None`), a color lookup table (`Post::neutral_lut` to
  start one) and vignette. In C (`crabby_ctx_post`, `crabby_ctx_set_post`)
  and C++ (`ctx.post()`, `ctx.set_post`, `crabby::tonemap`).
- Custom 3D material shaders in WGSL: `ctx.load_shader` (hot reloaded)
  and `ctx.create_shader`. A shader defines `vertex` (move vertices)
  and/or `surface` (change each pixel); the engine lights and shadows it.
  `MaterialDesc` gains `shader`, `custom` values, `custom_textures`,
  `normal_map` (with `normal_strength`), `orm_map`, `emissive_map`,
  `blend` (`Blend::Transparent`) and `double_sided`;
  `ctx.update_material` changes a material. In C and C++ too. The
  `shaders` example shows water, wind and a procedural pattern.
- A physical sky and height fog, `ctx.sky` (`Sky`): Rayleigh and Mie
  scattering lit by the sun, a sun disk, stars at night; the sun dims
  and reddens as it sets and the scene's ambient light comes from the
  sky. `Sky::sun_direction_at(hour)` for a day and night cycle. In C
  (`crabby_ctx_sky`, `crabby_ctx_set_sky`, `crabby_sun_direction_at`) and
  C++ (`ctx.sky()`, `ctx.set_sky`, `crabby::sun_direction_at`).
- `Lighting::ambient_intensity` now defaults to 1 (light as bright as the
  sky gives), with the default sky and ground colors halved to match.
- Cascaded sun shadows: `Lighting::shadow_cascades` (1 to 4, default 4)
  shadow maps share `shadow_distance`, sharp near the camera and reaching
  far, blended where they meet. In C and C++ (`shadow_cascades` in the
  lighting struct).
- 3D textures get mipmaps when a material first uses them (made on the
  GPU; sprites still draw the full-size image).
- glTF models load their normal, occlusion, metallic-roughness and
  emissive maps (occlusion packed into the ORM map), vertex colors, alpha
  blending and double-sidedness.
- Vertex colors: `MeshData::colors`, multiplying the material's color and
  readable in shaders. In C (`crabby_mesh_data_set_colors`) and C++
  (`mesh_data::set_colors`).
- Transparent shaders can read the scene behind them: `scene_color`,
  `scene_position` and `scene_distance`, for refraction and water depth.
- `Vec4` and `vec4` are re-exported (feature `3d`).
- The 3D frame is lit in HDR, with a depth prepass (each pixel is lit
  once). `render_stats().gpu_ms` now times the whole frame, 3D and shadow
  passes included.
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

### Changed
- Terrain materials lay their layers on from the sides too on steep
  ground (triplanar), so cliffs no longer stretch them; flat ground costs
  the same.
- 3D materials: `Blend::Opaque` is now fully solid, whatever the alpha;
  the new `Blend::Cutout` (2 in C, `crabby::blend::cutout` in C++) cuts
  out pixels with alpha below 0.5, as `Opaque` did before. glTF materials
  with alpha mode MASK load as `Cutout`. Solid materials write depth and
  shadows without running their shader.
- Shadows: each pixel samples only the cascade it falls in (and the next
  one near the edge), not every cascade, and none when it faces away from
  the sun. Distant cascades draw terrain tiles at a coarser detail level.

### Fixed
- The Unity importer honors `bakeAxisConversion` on FBX meshes: positions
  and normals go through the same axis permutation and the winding turns
  where it mirrors, so such meshes no longer stand the wrong way round in
  imported prefabs.
- A custom `surface` shader that reads `sunlight()` or `sky_radiance()`
  could fail pipeline validation when only the shadow or sky pass drew it;
  those passes now bind everything the frame group declares.
- A 3D body that falls asleep reports a zero `Velocity3D` instead of the
  speed it had just before.
- `vsync(false)` no longer waits for the window's redraw events, which
  macOS sends only once per display refresh.
- Control endpoint `scene.save`, `scene.load`, `prefab.spawn`,
  `prefab.save`, and the MCP tools `save_scene`, `load_scene`,
  `spawn_prefab`, `save_prefab`, so agents can build levels.

## [1.0.0] - 2026-11-03

The first release: a 2D engine for simulations with up to millions of
entities, in Rust, C and C++. Install with `cargo add ponko`; the C
library and headers for Linux, macOS and Windows are attached to the
GitHub release. Everything below is new.

### Simulation
- Own archetype ECS (`ponko_ecs`): entities with generational ids, queries,
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
  and C++ with ponko_ffi's `steam` feature.
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
- `ponko_mcp`: an MCP server for AI agents (launch, inspect, edit, step,
  events, logs, stats, screenshots, save and load).

### Web
- Web builds (wasm32): the same game runs in a browser, drawing with WebGPU
  or WebGL2. Assets listed in `assets/index.json` are fetched before setup,
  sound plays through WebAudio, and the inspector opens with `?inspector=open`.
- `web/build.sh` builds an example or a game's binary into a static site.
- `ponko::Instant`: an `Instant` that also works in browsers.
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
- C API (`ponko_ffi`) with a generated header, and a header-only C++17
  wrapper (`include/crabby.hpp`) covering nearly the whole engine.

[Unreleased]: https://github.com/bozoslav/ponko/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/bozoslav/ponko/releases/tag/v1.0.0
