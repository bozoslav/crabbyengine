//! # Crabby Engine
//!
//! A fast, beginner-friendly 2D game engine. Fast inside, simple outside.
//!
//! ```no_run
//! use crabbyengine::*;
//!
//! fn main() {
//!     Game::new("My Game", 800, 600).run(|_ctx| {
//!         let mut pos = vec2(400.0, 300.0);
//!
//!         move |ctx: &mut Ctx| {
//!             if ctx.key_down(Key::Right) { pos.x += 200.0 * ctx.dt(); }
//!             if ctx.key_down(Key::Left)  { pos.x -= 200.0 * ctx.dt(); }
//!             if ctx.key_pressed(Key::Escape) { ctx.quit(); }
//!         }
//!     });
//! }
//! ```
//!
//! Everything you need comes in with `use crabbyengine::*;`.

mod ctx;
mod game;
mod input;
mod time;

pub use ctx::Ctx;
pub use game::Game;
pub use input::{Key, MouseButton};

/// 2D vector used for positions, sizes and velocities (re-exported from `glam`).
pub use glam::{Vec2, vec2};
