//! # Ponko
//!
//! A fast 2D engine for games with huge numbers of things on screen.
//!
//! ```no_run
//! use ponko::*;
//!
//! fn main() {
//!     Game::new("My Game", 800, 600).run(|ctx| {
//!         let crab = ctx.load_texture("crab.png");
//!         let mut pos = vec2(400.0, 300.0);
//!
//!         move |ctx: &mut Ctx| {
//!             if ctx.key_down(Key::Right) { pos.x += 200.0 * ctx.dt(); }
//!             if ctx.key_down(Key::Left)  { pos.x -= 200.0 * ctx.dt(); }
//!             if ctx.key_pressed(Key::Escape) { ctx.quit(); }
//!             ctx.draw(&crab, pos);
//!         }
//!     });
//! }
//! ```
//!
//! Everything you need comes in with `use ponko::*;`.
//!
//! ## Features
//!
//! - `window` (default): a window and GPU rendering (winit + wgpu). Without
//!   it the engine is simulation only: `Game::run_headless` works, drawing
//!   calls do nothing, `Game::run` panics. For simulation servers and CI,
//!   with a much smaller build.
//! - `audio` (default): sound through kira; without it sounds are silent.
//! - `inspector` (default): the in-game inspector overlay (egui), opened
//!   with the backquote key. On in debug builds; see `Game::inspector`.
//! - `gamepad` (default): gamepads through gilrs; without it
//!   `Ctx::gamepads` is always empty. On Linux it needs libudev headers.
//! - `profiling`: per-system scopes for `puffin_viewer`. (Not needed for
//!   [`profile_scope!`], [`Ctx::perf_report`] or [`CountingAllocator`], which
//!   are always built in and cost one atomic load while profiling is off.)
//! - `hotpatch`: change system code while the game runs, with
//!   `dx serve --hot-patch --features hotpatch`.
//! - `physics`: 2D physics with rapier (`Game::physics`); `physics-determinism`
//!   for bit-identical results across platforms.
//! - `physics3d`: 3D physics with rapier (`Game::physics3d`): bodies,
//!   colliders (heightfield terrain too), queries, a character controller and
//!   raycast-wheel vehicles.
//! - `steam`: lockstep and byte messages over Steam's relays and lobbies
//!   (`Net::steam_host`), with the `steamworks` crate re-exported.

#![warn(missing_docs)]
// Without a window, the code that only the window loop calls is unused.
#![cfg_attr(not(feature = "window"), allow(dead_code))]

mod actions;
mod alloc_count;
mod assets;
mod attach;
mod audio;
mod camera;
#[cfg(any(feature = "physics", feature = "physics3d"))]
mod collision;
mod color;
mod control;
mod ctx;
mod ecs;
#[cfg(feature = "inspector")]
mod editor;
mod events;
mod flow;
mod game;
#[cfg(feature = "gamepad")]
mod gamepad;
mod hot_reload;
mod input;
pub use actions::{
    Action, ActionId, ActionMap, ActionRef, Axis2Binding, AxisBinding, ButtonBinding, InputEvent, InputScenario,
    UnknownAction,
};
#[cfg(feature = "inspector")]
mod inspector;
mod logs;
#[cfg(feature = "navmesh")]
mod nav3d;
mod net;
#[cfg(feature = "navmesh")]
pub use nav3d::{NavAgent3D, NavBakeError, NavBakeSettings, NavMesh3D, NavObstacle3D, NavSource, step_navmesh3d};
mod perf_report;
#[cfg(feature = "physics")]
mod physics;
#[cfg(feature = "physics3d")]
mod physics3d;
mod profiler;
#[cfg(any(all(target_arch = "wasm32", feature = "window"), test))]
mod query;
mod random;
mod reflect;
mod render;
mod scene;
mod schema;
mod scope;
mod sheet;
mod spatial;
mod stream;
mod text;
mod texture;
#[cfg(feature = "3d")]
mod three;
mod tilemap;
mod time;
mod ui;
#[cfg(all(target_arch = "wasm32", feature = "window"))]
mod web;

pub use alloc_count::{AllocCounts, CountingAllocator, alloc_counts, note_allocation};
pub use assets::{AssetSource, MemorySource};
pub use attach::{Attached, follow_attachments};
pub use audio::{AudioBus, AudioEffects, Playing, Sound, SoundParams};
#[cfg(feature = "3d")]
pub use audio::{AudioListener3D, AudioRolloff, AudioSettings3D, AudioSource3D};
pub use camera::{Camera2D, Rect, camera_controls};
#[cfg(any(feature = "physics", feature = "physics3d"))]
pub use collision::{BodyKind, CollisionStarted, CollisionStopped};
pub use color::Color;
pub use ctx::{Ctx, SystemTiming, Turns};
pub use ecs::{Sprite, Transform2D, draw_sprites};
pub use events::Events;
#[doc(hidden)]
pub use flow::RawFlowView;
pub use flow::{FlowField, NavGrid};
pub use game::{Game, Phase, SetupResult};
pub use input::{Gamepad, GamepadAxis, GamepadButton, GamepadStick, Key, MAX_GAMEPADS, MouseButton};
pub use net::{
    Authority, AuthorityCheck, ClockSync, Interest, InterestChange, InterestChangeKind, InterestConfig, InterestEntity,
    InterestEntry, InterestTier, InterestViewer, Interpolate, LatestWins, MAX_INTEREST_TIERS, Pose, SnapshotBuffer,
    SnapshotConfig, SnapshotPush, SnapshotSample, SnapshotState, SnapshotStream, UpdateStamp, seq_diff, seq_newer,
};
pub use net::{
    Desync, Lane, MAX_INPUT_BYTES, MAX_MESSAGE_BYTES, MAX_PLAYERS, MessageEvent, MessageStats, Net, NetMessages,
    NetState, NetStats, PeerId,
};
#[cfg(feature = "3d")]
pub use net::{NetPose, apply_net_poses};
#[cfg(feature = "steam")]
pub use net::{SteamLobby, SteamLobbyVisibility, steam_lobby_from_args, steam_wait_for_invite};
pub use perf_report::{
    AllocReport, Budget, BudgetViolation, Contributor, FrameTimes, PassReport, PerfReport, PerfWarning, Regression,
    RenderReport, ScopeReport, SystemReport, WarningKind, WorstFrame,
};
#[cfg(feature = "physics")]
pub use physics::{Collider, MAX_POLYGON_POINTS, Physics, RayHit, RigidBody, Shape, Velocity, step_physics};
#[cfg(feature = "physics3d")]
pub use physics3d::{
    Character3D, Collider3D, ColliderFocus, CollisionGroups, CollisionMesh, CollisionMeshData, ContactModifier3D,
    ContactPair3D, FrictionCurve, Hit3D, Joint3D, JointAxis3D, JointDrive3D, JointMotion3D, PHYSICS3D_HZ, Physics3D,
    QueryFilter3D, Ragdoll3D, RagdollBone, RigidBody3D, Shape3D, TerrainCollider, Vehicle3D, Velocity3D, Wheel3D,
    interpolate_physics3d, step_physics3d,
};
pub use profiler::{FrameStats, GpuPass};
pub use random::{DEFAULT_SEED, Random, Rng};
pub use reflect::{Field, FieldType, ReflectError, WorldReflect};
pub use render::RenderStats;
pub use schema::ValueSchema;
pub use scope::{ProfileScope, ScopeGuard, ScopeTiming, scope_named};
pub use sheet::{Animation, SpriteSheet, animate_sprites};
#[doc(hidden)]
pub use spatial::RawGridView;
pub use spatial::{GridEntry, SPATIAL_SORT_EVERY, SpatialGrid, rebuild_spatial_grid, spatial_key};
/// The Steamworks SDK (re-exported `steamworks` crate, feature `steam`):
/// create the [`steamworks::Client`] for [`Net::steam_host`] here, and use
/// it for achievements, friends and the rest of Steam.
#[cfg(feature = "steam")]
pub use steamworks;
#[cfg(feature = "3d")]
pub use stream::CellStreamer;
pub use stream::Job;
/// Flex/grid/absolute layout types used by retained UI.
/// ```
/// use ponko::ui_layout::Style;
/// let style: Style = Style::default();
/// ```
pub use taffy as ui_layout;
pub use text::{Font, TextParams};
pub use texture::{DrawParams, Texture, TextureFilter};
#[cfg(feature = "3d")]
pub use three::{
    AmbientOcclusion, AnimationClip, AnimationCondition, AnimationEvent, AnimationGraph, AnimationLayer,
    AnimationParameter, AnimationState, AnimationTrack, AnimationTransition, Animator, AntiAliasing, AttachToJoint,
    Blend, Camera3D, Compute, ComputePass, Decal3D, Effect, FarTerrain, GlobalIllumination, GpuBuffer, GraphicsQuality,
    Heightfield, InstanceBatch, InstanceBatchOptions, Interpolation, Joint, JointAnimation, JointMask,
    LIGHT_SHADOW_ATLAS_SIZE, LIGHT_SHADOW_SIZE, LayerBlend, Lighting, Line3D, LodGroup, LodLevel, LodPick, LodRenderer,
    LodSettings, LodShadow, MAX_DECALS, MAX_DECALS_PER_CLUSTER, MAX_GI_PROBES, MAX_GI_PROBES_PER_FRAME,
    MAX_LIGHT_SHADOW_VIEWS, MAX_RENDER_SCALE, MIN_RENDER_SCALE, Material, MaterialDesc, Mesh, MeshCost, MeshData,
    Model, MorphAnimation, MorphFrame, MorphTarget, MorphWeights, Motion, MotionChild, ParticleBlend3D, ParticleCurve,
    ParticleEmitter3D, ParticleGradient, ParticleSettings3D, ParticleShape3D, ParticleSpace3D, PointLight3D, Post,
    QualityLevel, ReceivesDecals, ReflectionProbe3D, RootMotion, ScreenEffect, Shader, ShaderDiagnostic, ShaderKind,
    ShaderOutcome, ShaderStatus, ShadowMesh, Skeleton, SkinOf, Sky, SpotLight3D, TERRAIN_LAYERS, TERRAIN_LODS,
    TargetCamera, TerrainMaterialDesc, TerrainTile, TerrainTileData, Text3D, TextAlign3D, TextureArray, TextureKind,
    Tonemap, Trail3D, Transform3D, look_at_joint, two_bone_ik,
};
pub use tilemap::{TileMap, draw_tilemaps};
#[cfg(feature = "3d")]
pub use ui::UiPanel3D;
pub use ui::{Ui, UiBuilder, UiEvent, UiId, UiInput, UiLayout, UiPaint, UiStyle, UiTextAlign, UiTheme, UiWidget};

/// The ECS: entities, components, queries, resources, commands.
pub use ponko_ecs::{Commands, Component, ComponentId, Entity, Query, World};

/// The full ECS crate, for advanced use (C path, chunks, archetypes).
pub use ponko_ecs as ecs_core;

/// 3D and 4D vectors, rotation and 4x4 matrix, for 3D (re-exported from `glam`).
#[cfg(feature = "3d")]
pub use glam::{Mat4, Vec4, vec4};
/// Rotation and 3D vector (re-exported from `glam`); also used by the
/// replication helpers ([`Pose`], [`Interest`]), which need no `3d` feature.
pub use glam::{Quat, Vec3, vec3};
/// Whole-number 2D vector, for tile coordinates (re-exported from `glam`).
pub use glam::{UVec2, uvec2};
/// 2D vector used for positions, sizes and velocities (re-exported from `glam`).
pub use glam::{Vec2, vec2};

/// JSON values used by reflection (re-exported from `serde_json`).
pub use serde_json::{self, Value, json};

/// A point in time for measuring how long something took, like
/// `std::time::Instant` (which it is, on desktop), but also working in web
/// builds, where `std::time::Instant::now()` panics. Re-exported from
/// `web-time`.
///
/// ```
/// # use ponko::*;
/// let nav = NavGrid::new(256, 256, 8.0);
/// let mut field = FlowField::new();
/// let start = Instant::now();
/// field.compute(&nav, &[vec2(40.0, 40.0)]);
/// println!("flow field in {:.2} ms", start.elapsed().as_secs_f64() * 1000.0);
/// ```
pub use web_time::Instant;

mod environment;
