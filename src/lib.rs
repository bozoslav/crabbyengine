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
mod color;
mod ctx;
mod ecs;
mod game;
mod input;
mod render;
mod spatial;
mod texture;
mod time;

pub use color::Color;
pub use ctx::{Ctx, SystemTiming};
pub use ecs::{Sprite, Transform2D, draw_sprites};
pub use game::{Game, SetupResult};
pub use input::{Key, MouseButton};
pub use spatial::{GridEntry, SpatialGrid, rebuild_spatial_grid};
pub use texture::{DrawParams, Texture};

/// The ECS: entities, components, queries, resources, commands.
pub use crabby_ecs::{Commands, Component, Entity, Query, World};

/// The full ECS crate, for advanced use (C path, chunks, archetypes).
pub use crabby_ecs as ecs_core;

/// 2D vector used for positions, sizes and velocities (re-exported from `glam`).
pub use glam::{Vec2, vec2};
