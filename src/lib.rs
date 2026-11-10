//! # Ponko
//!
//! A fast 2D engine for games with huge numbers of things on screen.
//!
//! ```no_run
//! use ponko::prelude::*;
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
//! Start with `use ponko::prelude::*;`, then add dimension and service modules as needed.
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
//!   `Input::gamepads` is always empty. On Linux it needs libudev headers.
//! - `profiling`: per-system scopes for `puffin_viewer`. (Not needed for
//!   [`profile_scope!`], [`Perf::report`] or [`CountingAllocator`], which
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

// Tests include example files (`use ponko::*;`) to check them as they are.
#[cfg(test)]
extern crate self as ponko;

pub mod anim;
pub mod d2;
#[cfg(feature = "3d")]
pub mod d3;
pub mod nav;
pub mod prelude;
#[cfg(feature = "3d")]
pub mod terrain;

pub mod access;
pub mod actions;
pub mod alloc_count;
mod asset_cache;
pub mod assets;
pub mod attach;
pub mod audio;
pub mod camera;
#[cfg(any(feature = "physics", feature = "physics3d"))]
mod collision;
pub mod color;
mod control;
mod ctx;
mod ctx_compat;
pub mod diagnostics;
mod ecs;
#[cfg(feature = "inspector")]
mod editor;
pub mod error;
pub mod events;
pub mod flow;
pub mod game;
#[cfg(feature = "gamepad")]
mod gamepad;
pub mod gfx;
pub mod handle;
mod hot_reload;
pub mod input;
pub mod inventory;

#[doc(hidden)]
pub use actions::{Action, ActionId, ActionMap, ActionRef, Axis2Binding, AxisBinding, ButtonBinding};

#[cfg(feature = "inspector")]
mod inspector;
mod logs;
#[cfg(feature = "navmesh")]
mod nav3d;
pub mod net;

pub mod perf;
mod perf_report;
#[cfg(feature = "physics")]
pub mod physics;
#[cfg(feature = "physics3d")]
pub mod physics3d;
mod profiler;
#[cfg(any(all(target_arch = "wasm32", feature = "window"), test))]
mod query;
pub mod random;
pub mod reflect;
pub mod render;
pub mod residency;
pub mod scenario;
mod scene;
mod schema;
pub mod scope;
pub mod sheet;
pub mod spatial;
pub mod stream;
pub mod streaming;
pub mod system_view;
pub mod systems;
pub mod text;
pub mod texture;
#[cfg(feature = "3d")]
mod three;
pub mod tilemap;
mod time;
pub mod ui;
#[cfg(all(target_arch = "wasm32", feature = "window"))]
mod web;

#[doc(hidden)]
pub use assets::{AssetSource, Assets, MemorySource};
#[doc(hidden)]
pub use audio::{AudioBus, Playing, Sound, SoundParams};

#[doc(hidden)]
pub use camera::Rect;
#[cfg(any(feature = "physics", feature = "physics3d"))]
#[doc(hidden)]
pub use collision::{BodyKind, CollisionStarted, CollisionStopped};
#[doc(hidden)]
pub use color::Color;
#[doc(hidden)]
pub use ctx::Ctx;

#[doc(hidden)]
pub use error::{Error, ErrorKind};
#[doc(hidden)]
pub use events::Events;
#[doc(hidden)]
pub use flow::RawFlowView;

#[doc(hidden)]
pub use game::{ConfiguredGame, Game, Phase, Plugin, SetupResult, SystemId, Systems};
#[doc(hidden)]
pub use handle::Handle;
#[doc(hidden)]
pub use input::{Key, MouseButton};
#[doc(hidden)]
pub use net::Interpolate;
#[doc(hidden)]
pub use random::{Random, Rng};
#[doc(hidden)]
pub use reflect::{FieldType, WorldReflect};

#[doc(hidden)]
pub use schema::ValueSchema;
#[doc(hidden)]
pub use sheet::SpriteSheet;
#[doc(hidden)]
pub use spatial::RawGridView;
/// The Steamworks SDK (re-exported `steamworks` crate, feature `steam`):
/// create the [`steamworks::Client`] for [`Net::steam_host`] here, and use
/// it for achievements, friends and the rest of Steam.
#[cfg(feature = "steam")]
#[doc(hidden)]
pub use steamworks;
#[allow(deprecated)]
#[doc(hidden)]
pub use systems::at_root::*;
/// Flex/grid/absolute layout types used by retained UI.
/// ```
/// use ponko::ui::layout::Style;
/// let style: Style = Style::default();
/// ```
#[doc(hidden)]
pub use taffy as ui_layout;
#[doc(hidden)]
pub use text::{Font, TextParams};
#[doc(hidden)]
pub use texture::{DrawParams, Texture, TextureFilter};

#[doc(hidden)]
pub use time::{Time, Turns};

/// The ECS: entities, components, queries, resources, commands.
#[doc(hidden)]
pub use ponko_ecs::{
    Added, Changed, Commands, Component, Entity, Or, Query, QueryFilter, Tick, ViewQuery, With, Without, World,
};

/// The whole ECS crate ([`ponko_ecs`]), for advanced use: component ids,
/// read-only world views, the dynamic and C path ([`World::raw`]), chunks
/// and archetypes. Games normally need only the ECS types in the prelude.
pub mod ecs_core {
    #[doc(no_inline)]
    pub use ponko_ecs::*;
}

/// 3D and 4D vectors, rotation and 4x4 matrix, for 3D (re-exported from `glam`).
#[cfg(feature = "3d")]
#[doc(hidden)]
pub use glam::{Mat4, Vec4, vec4};
/// Rotation and 3D vector (re-exported from `glam`); also used by the
/// replication helpers ([`Pose`], [`Interest`]), which need no `3d` feature.
#[doc(hidden)]
pub use glam::{Quat, Vec3, vec3};
/// Whole-number 2D vector, for tile coordinates (re-exported from `glam`).
#[doc(hidden)]
pub use glam::{UVec2, uvec2};
/// 2D vector used for positions, sizes and velocities (re-exported from `glam`).
#[doc(hidden)]
pub use glam::{Vec2, vec2};

/// JSON values used by reflection (re-exported from `serde_json`).
#[doc(hidden)]
pub use serde_json::{self, Value, json};

mod environment;

/// Prefab placement in either dimension.
#[doc(hidden)]
pub use scene::Placement;

mod compat;
#[doc(hidden)]
#[allow(deprecated)]
#[doc(hidden)]
pub use compat::*;
