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

mod assets;
mod camera;
mod color;
mod control;
mod ctx;
mod ecs;
mod game;
mod input;
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

pub use camera::{Camera2D, Rect, camera_controls};
pub use color::Color;
pub use ctx::{Ctx, SystemTiming, Turns};
pub use ecs::{Sprite, Transform2D, draw_sprites};
pub use game::{Game, SetupResult};
pub use input::{Key, MouseButton};
pub use random::{DEFAULT_SEED, Random, Rng};
pub use reflect::{ReflectError, WorldReflect};
pub use render::RenderStats;
pub use sheet::{Animation, SpriteSheet, animate_sprites};
pub use spatial::{GridEntry, SPATIAL_SORT_EVERY, SpatialGrid, rebuild_spatial_grid, spatial_key};
pub use text::{Font, TextParams};
pub use texture::{DrawParams, Texture, TextureFilter};
pub use tilemap::{TileMap, draw_tilemaps};
pub use ui::UiStyle;

/// The ECS: entities, components, queries, resources, commands.
pub use crabby_ecs::{Commands, Component, Entity, Query, World};

/// The full ECS crate, for advanced use (C path, chunks, archetypes).
pub use crabby_ecs as ecs_core;

/// Whole-number 2D vector, for tile coordinates (re-exported from `glam`).
pub use glam::{UVec2, uvec2};
/// 2D vector used for positions, sizes and velocities (re-exported from `glam`).
pub use glam::{Vec2, vec2};

/// JSON values used by reflection (re-exported from `serde_json`).
pub use serde_json::{self, Value, json};
