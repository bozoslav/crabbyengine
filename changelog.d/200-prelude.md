### Added
- A curated `prelude`, the one glob import a game starts with, and public modules for the rest. The root modules this release adds are `access`, `actions`, `alloc_count`, `anim`, `assets`, `attach`, `audio`, `camera`, `color`, `d2`, `d3`, `diagnostics`, `ecs_core` (now a documented module), `error`, `events`, `flow`, `game`, `gfx`, `handle`, `input`, `inventory`, `nav`, `net`, `perf`, `physics`, `physics3d`, `prelude`, `random`, `reflect` (with `reflect::json`), `render`, `residency`, `scenario`, `scope`, `sheet`, `spatial`, `stream`, `streaming`, `system_view`, `systems`, `terrain`, `text`, `texture`, `tilemap` and `ui` (some only with their feature). New root modules brought in by `use ponko::*;` can clash with a dependency of the same name (`audio`, `input`, `net`, `render`, `ui`, `random`, `events`, ...): a path in code means the Ponko module and `use name::...;` is ambiguous (E0659). Write `::name::...` for the crate, or switch to `use ponko::prelude::*;`. `ponko::nav` names its navigation-mesh types `NavMesh`, `NavAgent` and `NavObstacle`, as in `d3`.
- Associated constants on their types (`d3::Decal::MAX`, `spatial::SpatialGrid::SORT_EVERY`, `render::GraphicsQuality::MIN_RENDER_SCALE`, `terrain::TerrainTile::LODS`, ...), `perf::Instant`, and `world.raw()` / `world.raw_ref()` views with the dynamic component, resource, chunk and archetype methods.

### Changed
- The rustdoc landing page shows only modules; examples and guide code use the prelude and module imports. The root re-exports the prelude's items, as the same items and undeprecated, so `use ponko::*;` combines with `use ponko::prelude::*;`; every other root name re-exports its original item, so the root glob combines with `d2`, `d3` and module globs except where a name really differs (see the naming chapter).
- `Animation` (sprite-sheet animation) is no longer in the prelude, since games often define their own; it is `d2::Animation` (and still at the root).

### Deprecated
- `World`'s dynamic methods (`get_dynamic`, `insert_dynamic`, `spawn_dynamic`, `spawn_at_dynamic`, `remove_dynamic`, `get_resource_dynamic`, `insert_resource_dynamic`, `resource_layout_dynamic`, `remove_resource_dynamic`, `for_each_chunk`, `component_by_name`, `archetypes`): call them through `world.raw()` / `world.raw_ref()`. These warn.
- Root constants that became associated constants: `MAX_DECALS`, `MAX_DECALS_PER_CLUSTER`, `MAX_GI_PROBES`, `MAX_GI_PROBES_PER_FRAME`, `MAX_LIGHT_SHADOW_VIEWS`, `LIGHT_SHADOW_SIZE`, `LIGHT_SHADOW_ATLAS_SIZE`, `MIN_RENDER_SCALE`, `MAX_RENDER_SCALE`, `TERRAIN_LODS`, `TERRAIN_LAYERS`, `SPATIAL_SORT_EVERY`. These warn.
- Root spellings of items that moved to a module. They are re-exports of the original items and carry `#[deprecated]`, but rustc does not warn through a re-export, so this list records them (by new module; the new name follows where it changed):
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
  - `systems`: `animate_sprites`, `apply_net_poses`, `draw_sprites`, `draw_tilemaps`, `follow_attachments`, `interpolate_physics3d`, `rebuild_spatial_grid`, `step_navmesh3d`, `step_physics`, `step_physics3d` (re-exports of the same functions, so the root glob combines with `systems::*`).
  - `streaming`: `AssetState`, `DEFAULT_UPLOAD_BUDGET`, `StreamedAsset`, `UploadStats`.
  - `system_view`: `SystemView`, `ViewEvents`.
  - `terrain`: `FarTerrain`, `Heightfield`, `TerrainMaterialDesc`, `TerrainTile`, `TerrainTileData`, `TextureArray`, `TextureKind`.
  - `ui`: `UiAccessNode`, `UiAccessTree`, `UiAccessValue`, `UiBuilder`, `UiEvent`, `UiImmediate`, `UiInput`, `UiLayout`, `UiPaint`, `UiRole`, `UiStyle`, `UiTextAlign`, `UiTheme`, `UiWidget`.
