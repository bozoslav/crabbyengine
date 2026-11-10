# Ponko naming and compatibility

Cargo packages and source directories use `ponko` and `ponko_*`, starting at
version `0.0.1`. This naming change preserves the compatibility commitments in
[STABILITY.md](https://github.com/ponkoengine/ponko/blob/main/STABILITY.md).

## Native code

New C programs include `ponko.h` and use `Ponko*` types, `PONKO_*` constants
and `ponko_*` functions. New C++ programs include `ponko.hpp` and use `ponko::`.
`PONKO_PROFILE_SCOPE` and `PONKO_COUNT_CPP_ALLOCATIONS` provide the profiling
macros. Link against `ponko_ffi` as before.

These headers provide **source aliases**, including for function pointers, over
the existing ABI. The exported binary symbols remain `crabby_*`, the ABI version
remains 2, and layouts and calling conventions are unchanged. Dynamic library
loaders must keep looking up the existing symbols; independently exported
`ponko_*` symbols would be a separate additive ABI project. The `crabby.h` and
`crabby.hpp` headers, `crabby::` namespace, native types, constants and macros
continue to work. Both header spellings may be included in the same translation
unit. Rust users of `ponko_ffi` retain its existing low-level names.

## Environment settings

All engine runtime settings now accept `PONKO_*`: control, start-paused,
instance directory, launch ID, inspector, hot reload, HUD, vsync, benchmark and
trace settings (including their warmup, output and frame-count variants).
The corresponding `CRABBY_*` spelling remains a fallback. **Presence of the
Ponko spelling wins**, even if its value is empty, invalid, or `0`; fallback
happens only when it is absent. Parsing and defaults otherwise stay unchanged.
This also applies to the MCP server's instance-directory setting. MCP launches
set both spellings so older engine executables can still be launched.

For example, `PONKO_CONTROL=0 CRABBY_CONTROL=1` leaves the endpoint disabled.
`PONKO_BENCH=600` is the primary benchmark spelling. The CMake examples accept
`PONKO_LIBRARY_DIR` and `PONKO_INCLUDE_DIR`, with the old cache names as fallbacks.
`Color::PONKO_DARK` is the primary Rust color constant; `Color::CRABBY_DARK`
is an alias with the same value.

## Persistent and external names

These identifiers deliberately retain their old spelling:

- Saved worlds and prefabs use `crabby-world` and `crabby-prefab` with their
  existing versions. Existing saves and importer output remain interoperable.
- glTF extras retain `crabbyMorphFrames` and `crabbyAnimatedTargets`.
- The default instance registry remains `~/.crabby/instances` (or
  `%LOCALAPPDATA%\crabby\instances`), shared with existing games and tools.
  Set `PONKO_INSTANCES_DIR` to select another location. MCP launch logs keep
  their existing home directory, too.
- The web canvas ID remains `crabby` so existing embedding pages work.
- MCP tool names, parameters, protocol versions, control messages and benchmark
  JSON identifiers retain their established contracts. The reference benchmark
  app bundle keeps its existing name so previously documented commands work.
- Historical release notes, license notices and pre-existing README files retain
  their original text. The leak-check configuration path stays unchanged.

Changing persistent identifiers or removing aliases requires a separately
approved migration under the stability policy.

## Inventory and regression check

`naming-inventory.json` at the repository root inventories every tracked file
with case-insensitive legacy references, including generated native headers,
fixtures, metadata, historical text and paths. Each entry records its category,
reason and maximum occurrence count. `scripts/check-naming.py` rejects new
legacy-bearing files or paths and increased legacy counts; shrinking the
inventory is allowed. Review inventory changes alongside naming changes.
Generated aliases are checked separately by `scripts/generate-ponko-header.py
--check`. CI runs both checks. The Pages workflow builds this guide directly
from `book/` after changes reach `main`.

## Dimension modules

Start with `use ponko::prelude::*;`, then `use ponko::d2::*;` or
`use ponko::d3::*;`. Both expose `Transform`, `Camera`, `RigidBody`, `Collider`,
`Velocity`, `Shape`, `Physics`, and `Hit` (physics features required).
`d2` also holds `Sprite`, `TileMap`, `Attached`, `SpriteSheet`, `Animation`
and the 2D grids (`SpatialGrid`, `NavGrid`, `FlowField`); `d3` holds models,
materials, lights, decals, text, particles, terrain, animation, audio sources
and navigation agents. A game with both dimensions in one file imports one
module and qualifies the other (`d2::Transform` next to bare 3D names).
The modules are aliases for existing types: layout, component identity and saved
names are unchanged. Context fields remain `camera`/`camera3d` and
`physics`/`physics3d` so both dimensions can coexist; their types are
`d2::Camera`/`d3::Camera` and `d2::Physics`/`d3::Physics`.

| Retained root spelling | Preferred spelling |
|---|---|
| `Transform2D`, `Camera2D` (prelude names, not deprecated) | `d2::Transform`, `d2::Camera` in 2D-only code |
| `RigidBody`, `Collider`, `Velocity`, `Shape`, `Physics`, `RayHit` | corresponding `d2` name (`Hit` for `RayHit`) |
| `AudioListener3D` | `d3::AudioListener` |
| `AudioSettings3D` | `d3::AudioSettings` |
| `AudioSource3D` | `d3::AudioSource` |
| `Camera3D` (prelude name, not deprecated) | `d3::Camera` in 3D-only code |
| `Character3D` | `d3::Character` |
| `Collider3D` | `d3::Collider` |
| `ContactModifier3D` | `d3::ContactModifier` |
| `ContactPair3D` | `d3::ContactPair` |
| `Decal3D` | `d3::Decal` |
| `Hit3D` | `d3::Hit` |
| `Joint3D` | `d3::Joint` |
| `JointAxis3D` | `d3::JointAxis` |
| `JointDrive3D` | `d3::JointDrive` |
| `JointMotion3D` | `d3::JointMotion` |
| `Line3D` | `d3::Line` |
| `NavAgent3D` | `d3::NavAgent` |
| `NavMesh3D` | `d3::NavMesh` |
| `NavObstacle3D` | `d3::NavObstacle` |
| `ParticleBlend3D` | `d3::ParticleBlend` |
| `ParticleEmitter3D` | `d3::ParticleEmitter` |
| `ParticleSettings3D` | `d3::ParticleSettings` |
| `ParticleShape3D` | `d3::ParticleShape` |
| `ParticleSpace3D` | `d3::ParticleSpace` |
| `Physics3D` | `d3::Physics` |
| `PointLight3D` | `d3::PointLight` |
| `QueryFilter3D` | `d3::QueryFilter` |
| `Ragdoll3D` | `d3::Ragdoll` |
| `ReflectionProbe3D` | `d3::ReflectionProbe` |
| `RigidBody3D` | `d3::RigidBody` |
| `Shape3D` | `d3::Shape` |
| `SpotLight3D` | `d3::SpotLight` |
| `Text3D` | `d3::Text` |
| `TextAlign3D` | `d3::TextAlign` |
| `Trail3D` | `d3::Trail` |
| `Transform3D` (prelude name, not deprecated) | `d3::Transform` in 3D-only code |
| `UiPanel3D` | `d3::UiPanel` |
| `Vehicle3D` | `d3::Vehicle` |
| `Velocity3D` | `d3::Velocity` |
| `Wheel3D` | `d3::Wheel` |

Use `physics.cast_ray_filtered(origin, direction, max_distance, filter)` in
both dimensions and `physics.overlap_ball(center, radius, filter, &mut hits)`
for circles/spheres. Filters default to solid colliders, can include sensors,
and can exclude an entity. Both `Hit` types carry `entity`, `point`, `normal`
and `distance`; vectors have the matching dimension. `apply_impulse` and `mass`
share the same convention. Existing `cast_ray`, `entities_at`,
`entities_in_circle` and `overlap_sphere` calls retain their behavior.
The issue sketch called the common method `cast_ray`; Rust cannot overload
inherent methods by argument count, so the additive filtered spelling preserves
the existing three-argument 2D call and four-argument 3D call.

`Ctx::spawn_prefab(path, placement)` also accepts a `Vec3` or `d3::Transform`.
A 3D transform supplies position, rotation and scale; a 2D position keeps the
prefab's existing rotation and scale. Saved component names remain unchanged.

## Prelude and service imports

`use ponko::prelude::*;` is the default game import. It includes the game loop,
context, ECS, math, input, colors, textures/sounds/fonts, common transforms and
reflection registration. Add `d2::*` or `d3::*` for one dimension, then explicit
service imports as needed. `Pose`, `Material` and `Job` are deliberately absent
from the prelude so games can define them freely.

| Area | Canonical module |
|---|---|
| Networking, replication, interest, Steam | `net` (`net::steam` is the SDK) |
| Frame/per-system reports, budgets, scopes | `perf` |
| Materials, effects, compute, targets, quality | `render` |
| Clips, graphs, skeletal animation, IK | `anim` |
| Heightfields, terrain tiles, terrain streaming | `terrain` |
| Retained UI and layout | `ui`, `ui::layout` |
| Physics and navigation | `physics`, `physics3d`, `nav`, `d2`, `d3` |
| Reflection JSON | `reflect::json` |
| Advanced ECS and native storage | `ecs_core` |

### The root glob and deprecated root spellings

`use ponko::*;` still compiles and keeps every name it had. The root also
re-exports exactly what the prelude holds, as the same items and without
deprecation, so `use ponko::*;` next to `use ponko::prelude::*;` (the
midpoint of a migration) is never ambiguous. The other root names are
re-exports of the *original* items, so `use ponko::*;` next to `use
ponko::d2::*;`, `use ponko::d3::*;` or a service module's glob is ambiguous
only where the two names really are different things:

- `Collider`, `RigidBody`, `Velocity`, `Shape` and `Physics` at the root are
  the 2D ones; `d3` has the 3D ones under the same names.
- `QueryFilter` at the root is the ECS query-filter trait; `d2`, `d3` and
  `physics` have the physics query filters.
- `Joint` at the root is the skeleton joint (`anim::Joint`); `d3::Joint` is
  the physics joint (`Joint3D`).

The way out is the same for each: drop `use ponko::*;` for
`use ponko::prelude::*;`, or import the module rather than its glob
(`use ponko::d3;` then `d3::Collider`), or name the one item
(`use ponko::d3::Collider;`, which wins over both globs).

Canonical modules export the original types, so identity, layouts, saves and
flat C/C++ names do not change. Compatibility exports are hidden from the root
rustdoc page; module docs are the discovery path.

The deprecated root spellings below are re-exports. They carry
`#[deprecated]`, but **rustc does not warn through a re-export**, so this list
is where the deprecation is recorded. A type alias would warn, but it is a
different item from the original, which would make the root glob clash with
every module glob, and it would break enum-variant imports
(`use ponko::TextAlign3D::Center;`) and unit or tuple struct values
(`static A: CountingAllocator = CountingAllocator;`). They stay until the next
major release:

- `access`: `AccessConflict`, `AccessReport`, `ReportedPair`, `ReportedSystem`, `Service`, `SystemAccess`.
- `actions`: `InputEvent`, `UnknownAction`.
- `alloc_count`: `AllocCounts`, `CountingAllocator`.
- `anim`: `AnimationClip`, `AnimationCondition`, `AnimationEvent`, `AnimationGraph`, `AnimationLayer`, `AnimationParameter`, `AnimationState`, `AnimationTrack`, `AnimationTransition`, `Animator`, `AttachToJoint`, `Blend`, `Interpolation`, `Joint`, `JointAnimation`, `JointMask`, `LayerBlend`, `look_at_joint`, `MorphAnimation`, `MorphFrame`, `MorphTarget`, `MorphWeights`, `Motion`, `MotionChild`, `RootMotion`, `Skeleton`, `SkinOf`, `two_bone_ik`.
- `audio`: `Audio`, `AudioEffects`, `AudioRolloff`.
- `camera`: `camera_controls`.
- `d2`: `Animation`, `Attached`, `Collider`, `Physics`, `RayHit` (`d2::Hit`), `RigidBody`, `Shape`, `TileMap`, `Velocity`.
- `d3`: `AudioListener3D` (`d3::AudioListener`), `AudioSettings3D` (`d3::AudioSettings`), `AudioSource3D` (`d3::AudioSource`), `Character3D` (`d3::Character`), `Collider3D` (`d3::Collider`), `ContactModifier3D` (`d3::ContactModifier`), `ContactPair3D` (`d3::ContactPair`), `Decal3D` (`d3::Decal`), `Hit3D` (`d3::Hit`), `Joint3D` (`d3::Joint`), `JointAxis3D` (`d3::JointAxis`), `JointDrive3D` (`d3::JointDrive`), `JointMotion3D` (`d3::JointMotion`), `Line3D` (`d3::Line`), `NavAgent3D` (`d3::NavAgent`), `NavMesh3D` (`d3::NavMesh`), `NavObstacle3D` (`d3::NavObstacle`), `ParticleBlend3D` (`d3::ParticleBlend`), `ParticleEmitter3D` (`d3::ParticleEmitter`), `ParticleSettings3D` (`d3::ParticleSettings`), `ParticleShape3D` (`d3::ParticleShape`), `ParticleSpace3D` (`d3::ParticleSpace`), `Physics3D` (`d3::Physics`), `PointLight3D` (`d3::PointLight`), `QueryFilter3D` (`d3::QueryFilter`), `Ragdoll3D` (`d3::Ragdoll`), `ReflectionProbe3D` (`d3::ReflectionProbe`), `RigidBody3D` (`d3::RigidBody`), `Shape3D` (`d3::Shape`), `SpotLight3D` (`d3::SpotLight`), `StreamedLod`, `Text3D` (`d3::Text`), `TextAlign3D` (`d3::TextAlign`), `Trail3D` (`d3::Trail`), `UiPanel3D` (`d3::UiPanel`), `Vehicle3D` (`d3::Vehicle`), `Velocity3D` (`d3::Velocity`), `Wheel3D` (`d3::Wheel`).
- `diagnostics`: `ArchetypeDiagnostics`, `ComponentDiagnostics`, `ECS_DIAGNOSTICS_VERSION`, `EcsDiagnostics`, `MatchCount`, `SystemDiagnostics`.
- `ecs_core`: `ComponentId`, `WorldView`.
- `flow`: `FlowField`, `NavGrid`.
- `gfx`: `Graphics`.
- `handle`: `GfxHandle`, `LiveCounts`.
- `input`: `Gamepad`, `GamepadAxis`, `GamepadButton`, `GamepadStick`, `Input`, `MAX_GAMEPADS`.
- `inventory`: `AssetClass`, `AssetCount`, `AssetGrowth`, `AssetInventory`, `DanglingHandle`, `GpuResidency`.
- `nav`: `NavBakeError`, `NavBakeSettings`, `NavSource`.
- `net`: `Authority`, `AuthorityCheck`, `ClockSync`, `Desync`, `Interest`, `InterestChange`, `InterestChangeKind`, `InterestConfig`, `InterestEntity`, `InterestEntry`, `InterestTier`, `InterestViewer`, `Lane`, `LatestWins`, `MAX_INPUT_BYTES`, `MAX_INTEREST_TIERS`, `MAX_MESSAGE_BYTES`, `MAX_PLAYERS`, `MessageEvent`, `MessageStats`, `Net`, `NetMessages`, `NetPose`, `NetState`, `NetStats`, `PeerId`, `Pose`, `seq_diff`, `seq_newer`, `SnapshotBuffer`, `SnapshotConfig`, `SnapshotPush`, `SnapshotSample`, `SnapshotState`, `SnapshotStream`, `steam_lobby_from_args`, `steam_wait_for_invite`, `SteamLobby`, `SteamLobbyVisibility`, `UpdateStamp`.
- `perf`: `alloc_counts`, `AllocReport`, `Budget`, `BudgetViolation`, `Contributor`, `FrameStats`, `FrameTimes`, `GpuPass`, `Instant`, `note_allocation`, `PassReport`, `Perf`, `PerfReport`, `PerfWarning`, `Regression`, `RenderReport`, `scope_named`, `ScopeReport`, `SystemReport`, `SystemTiming`, `WarningKind`, `WorstFrame`.
- `physics`: `MAX_POLYGON_POINTS`.
- `physics3d`: `ColliderFocus`, `CollisionGroups`, `CollisionMesh`, `CollisionMeshData`, `FrictionCurve`, `PHYSICS3D_HZ`, `RagdollBone`, `TerrainCollider`.
- `random`: `DEFAULT_SEED`.
- `reflect`: `Field`, `ReflectError`.
- `render`: `AmbientOcclusion`, `AntiAliasing`, `Compute`, `ComputePass`, `Effect`, `ExtractionBench`, `ExtractionSample`, `GlobalIllumination`, `GpuAdapter`, `GpuBuffer`, `GpuCapabilities`, `GpuFeature`, `GpuFeatureStatus`, `GpuLimits`, `GpuTier`, `GpuUploads`, `GraphicsQuality`, `InstanceBatch`, `InstanceBatchOptions`, `InstanceLevel`, `Lighting`, `LodGroup`, `LodHysteresis`, `LodLevel`, `LodPick`, `LodRenderer`, `LodSettings`, `LodShadow`, `Material`, `MaterialDesc`, `Mesh`, `MeshCost`, `MeshData`, `ModelMaterial`, `ModelMaterials`, `ParticleCurve`, `ParticleGradient`, `PipelineStats`, `Post`, `QualityLevel`, `ReceivesDecals`, `RenderStats`, `ScreenEffect`, `Shader`, `ShaderDiagnostic`, `ShaderKind`, `ShaderOutcome`, `ShaderStatus`, `ShadowMesh`, `Sky`, `TargetCamera`, `TargetEffect`, `Tonemap`.
- `residency`: `ResidencyKind`, `ResidencyStats`.
- `scenario`: `InputScenario`, `MAX_SCENARIO_STEPS`, `MAX_SCENARIO_TICKS`, `SCENARIO_VERSION`, `ScenarioCheck`, `ScenarioError`, `ScenarioFailure`, `ScenarioReport`, `ScenarioStatus`.
- `scope`: `ProfileScope`, `ScopeGuard`, `ScopeTiming`.
- `spatial`: `GridEntry`, `spatial_key`, `SpatialGrid`.
- `stream`: `CellStreamer`, `Job`.
- `systems`: `animate_sprites`, `apply_net_poses`, `draw_sprites`, `draw_tilemaps`, `follow_attachments`, `interpolate_physics3d`, `rebuild_spatial_grid`, `step_navmesh3d`, `step_physics`, `step_physics3d`.
- `streaming`: `AssetState`, `DEFAULT_UPLOAD_BUDGET`, `StreamedAsset`, `UploadStats`.
- `system_view`: `SystemView`, `ViewEvents`.
- `terrain`: `FarTerrain`, `Heightfield`, `TerrainMaterialDesc`, `TerrainTile`, `TerrainTileData`, `TextureArray`, `TextureKind`.
- `ui`: `UiAccessNode`, `UiAccessTree`, `UiAccessValue`, `UiBuilder`, `UiEvent`, `UiImmediate`, `UiInput`, `UiLayout`, `UiPaint`, `UiRole`, `UiStyle`, `UiTextAlign`, `UiTheme`, `UiWidget`.

What does warn: the root constants that became associated constants
(`MAX_DECALS` → `d3::Decal::MAX`, `MAX_DECALS_PER_CLUSTER`, `MAX_GI_PROBES`,
`MAX_GI_PROBES_PER_FRAME`, `MAX_LIGHT_SHADOW_VIEWS`, `LIGHT_SHADOW_SIZE`,
`LIGHT_SHADOW_ATLAS_SIZE`, `MIN_RENDER_SCALE`, `MAX_RENDER_SCALE`,
`TERRAIN_LODS`, `TERRAIN_LAYERS`, `SPATIAL_SORT_EVERY`), which are constants
of their own, and `World`'s dynamic methods (below). Everything else in the
list above only keeps compiling, silently.

Use `d3::Decal::MAX`, `d3::Decal::MAX_PER_CLUSTER`,
`spatial::SpatialGrid::<()>::SORT_EVERY`, `render::GraphicsQuality::MIN_RENDER_SCALE`
and `MAX_RENDER_SCALE`, `terrain::TerrainTile::LODS` and `LAYERS`,
`d3::PointLight::SHADOW_SIZE`, `SHADOW_ATLAS_SIZE` and `MAX_SHADOW_VIEWS`, and
`render::GlobalIllumination::MAX_PROBES` and `MAX_PROBES_PER_FRAME`; the new
`d3`, `render` and `terrain` modules export only these associated forms.
Constants that belong to one module keep their name there
(`net::MAX_PLAYERS`, `input::MAX_GAMEPADS`, `random::DEFAULT_SEED`,
`physics3d::PHYSICS3D_HZ`, `scenario::SCENARIO_VERSION`,
`streaming::DEFAULT_UPLOAD_BUDGET`). `Instant` is `perf::Instant`.
`Animation` (the sprite-sheet animation) is in `d2`, not the prelude: games
often name their own type `Animation`.

### Root modules and crates of the same name

`use ponko::*;` also brings in the root modules (`audio`, `input`, `net`,
`render`, `ui`, `random`, `events`, ...), which are new in this release. With
the root glob and a dependency of the same name, a path in code
(`net::connect()`) means `ponko::net`, and a `use net::...;` declaration is
an error (E0659: `net` is ambiguous). Write `::net::...` (`use ::net::connect;`)
for the crate, or use `use ponko::prelude::*;`, which holds no such modules
apart from `d2`, `d3`, `systems` and `actions`. The clock module is private,
so the `time` crate is not shadowed; `Instant` is `perf::Instant` (and still
`ponko::Instant`).

`ponko::nav` names the navigation-mesh types as `d3` does (`NavMesh`,
`NavAgent`, `NavObstacle`), so they don't clash with the root's `Mesh`.

The engine's built-in systems (`step_physics3d`, `draw_sprites`,
`animate_sprites`, `rebuild_spatial_grid`, ...) live in `ponko::systems`, and
each has a `Systems` id for `Game::remove` ([Systems](systems.md)). Their root
spellings are deprecated re-exports of the same functions (listed above).

Use `world.raw()` for dynamic components and resources (`get_dynamic`,
`insert_dynamic`, `spawn_dynamic`, `remove_dynamic`, the `*_resource_dynamic`
calls), chunks (`for_each_chunk`) and archetypes; `world.raw_ref()` supplies
read-only dynamic inspection. These views borrow the same world and preserve
every pointer/layout requirement of the existing native interface. The same
methods on `World` itself still compile as deprecated forwarders and are hidden
from the typed `World` documentation. The C bindings call the same functions
through `raw()`.

### Native filtered queries and prefab placement

The native headers keep flat names. C uses `PonkoQueryFilter2D` (zero means
solids only), `ponko_ctx_cast_ray_filtered`, and `ponko_ctx_overlap_ball`.
Existing 3D ray/sphere calls already accept `PonkoQueryFilter3D`.
C++ offers `filter2d`/`filter3d` and overloads of `cast_ray_filtered` and
`overlap_ball` with the same argument order in both dimensions. The filter is
explicit, so the vector/filter types select the dimension. Existing ray and
overlap methods retain their behavior.

C prefab placement adds `ponko_ctx_spawn_prefab3d` (position, preserving saved
rotation/scale), `ponko_ctx_spawn_prefab_transform3d` and
`ponko_ctx_spawn_prefab_transform2d` (complete transform). C++ uses the same
flat method names without the `ponko_ctx_` prefix. These are separate names
rather than overloads of the old `spawn_prefab`: otherwise an existing
`spawn_prefab(path, {x, y})` would become ambiguous with a 3D aggregate.
Outputs are validated before spawning; invalid paths and null outputs do not
spawn an entity. The compiled native `dimensions.c` and `dimensions.cpp` tests
exercise filtering, buffer sizing, errors, placement and legacy calls.
