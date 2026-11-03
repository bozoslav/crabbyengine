//! # Crabby Engine
//!
//! A fast 2D engine for games with huge numbers of things on screen.
//!
//! ```no_run
//! use crabbyengine::*;
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
//! Everything you need comes in with `use crabbyengine::*;`.
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
//! - `profiling`: per-system scopes for `puffin_viewer`.
//! - `hotpatch`: change system code while the game runs, with
//!   `dx serve --hot-patch --features hotpatch`.
//! - `physics`: 2D physics with rapier (`Game::physics`); `physics-determinism`
//!   for bit-identical results across platforms.
//! - `steam`: lockstep multiplayer over Steam's relays and lobbies
//!   (`Net::steam_host`), with the `steamworks` crate re-exported.

// Without a window, the code that only the window loop calls is unused.
#![cfg_attr(not(feature = "window"), allow(dead_code))]

mod assets;
mod attach;
mod audio;
mod camera;
mod color;
mod control;
mod ctx;
mod ecs;
mod events;
mod flow;
mod game;
#[cfg(feature = "gamepad")]
mod gamepad;
mod hot_reload;
mod input;
#[cfg(feature = "inspector")]
mod inspector;
mod logs;
mod net;
#[cfg(feature = "physics")]
mod physics;
#[cfg(any(all(target_arch = "wasm32", feature = "window"), test))]
mod query;
mod random;
mod reflect;
mod render;
mod sheet;
mod spatial;
mod text;
mod texture;
mod tilemap;
mod time;
mod ui;
#[cfg(all(target_arch = "wasm32", feature = "window"))]
mod web;

pub use attach::{Attached, follow_attachments};
pub use audio::{Playing, Sound, SoundParams};
pub use camera::{Camera2D, Rect, camera_controls};
pub use color::Color;
pub use ctx::{Ctx, SystemTiming, Turns};
pub use ecs::{Sprite, Transform2D, draw_sprites};
pub use events::Events;
#[doc(hidden)]
pub use flow::RawFlowView;
pub use flow::{FlowField, NavGrid};
pub use game::{Game, SetupResult};
pub use input::{Gamepad, GamepadAxis, GamepadButton, GamepadStick, Key, MAX_GAMEPADS, MouseButton};
pub use net::{Desync, MAX_INPUT_BYTES, MAX_PLAYERS, Net, NetState, NetStats};
#[cfg(feature = "steam")]
pub use net::{steam_lobby_from_args, steam_wait_for_invite};
#[cfg(feature = "physics")]
pub use physics::{
    BodyKind, Collider, CollisionStarted, CollisionStopped, Physics, RayHit, RigidBody, Shape, Velocity, step_physics,
};
pub use random::{DEFAULT_SEED, Random, Rng};
pub use reflect::{Field, FieldType, ReflectError, WorldReflect};
pub use render::RenderStats;
pub use sheet::{Animation, SpriteSheet, animate_sprites};
#[doc(hidden)]
pub use spatial::RawGridView;
pub use spatial::{GridEntry, SPATIAL_SORT_EVERY, SpatialGrid, rebuild_spatial_grid, spatial_key};
/// The Steamworks SDK (re-exported `steamworks` crate, feature `steam`):
/// create the [`steamworks::Client`] for [`Net::steam_host`] here, and use
/// it for achievements, friends and the rest of Steam.
#[cfg(feature = "steam")]
pub use steamworks;
pub use text::{Font, TextParams};
pub use texture::{DrawParams, Texture, TextureFilter};
pub use tilemap::{TileMap, draw_tilemaps};
pub use ui::UiStyle;

/// The ECS: entities, components, queries, resources, commands.
pub use crabby_ecs::{Commands, Component, ComponentId, Entity, Query, World};

/// The full ECS crate, for advanced use (C path, chunks, archetypes).
pub use crabby_ecs as ecs_core;

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
/// # use crabbyengine::*;
/// let nav = NavGrid::new(256, 256, 8.0);
/// let mut field = FlowField::new();
/// let start = Instant::now();
/// field.compute(&nav, &[vec2(40.0, 40.0)]);
/// println!("flow field in {:.2} ms", start.elapsed().as_secs_f64() * 1000.0);
/// ```
pub use web_time::Instant;
