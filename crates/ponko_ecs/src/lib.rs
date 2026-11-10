//! # ponko_ecs
//!
//! The entity-component-system behind Ponko, built for simulations
//! with millions of entities.
//!
//! - **Entities** are cheap handles ([`Entity`]).
//! - **Components** are plain Rust types, with no derive needed. Anything
//!   `Send + Sync + 'static` works.
//! - **Queries** loop over every entity that has a set of components, as fast
//!   as a hand-written loop over arrays.
//! - **Resources** are global values stored in the world.
//! - **Commands** record changes to apply after a query finishes.
//!
//! ```
//! use ponko_ecs::*;
//!
//! struct Pos(f32, f32);
//! struct Vel(f32, f32);
//! struct Gravity(f32);
//!
//! let mut world = World::new();
//! world.insert_resource(Gravity(-9.8));
//! for i in 0..1000 {
//!     world.spawn((Pos(i as f32, 0.0), Vel(0.0, 0.0)));
//! }
//!
//! world
//!     .query::<&mut Vel>()
//!     .with_resources::<&Gravity>()
//!     .for_each(|v, gravity| v.1 += gravity.0);
//! world.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| {
//!     p.0 += v.0;
//!     p.1 += v.1;
//! });
//! ```
//!
//! Under the hood every component, Rust or not, is a runtime-registered column
//! (name, size, alignment, drop). Rust types are registered on first use;
//! C/C++ code uses [`World::register_component`] and [`World::for_each_chunk`].

#![warn(missing_docs)]

mod raw;
pub use raw::{RawWorld, RawWorldRef};
mod access;
mod borrow;
mod bundle;
mod change;
mod commands;
mod entity;
mod query;
mod storage;
mod view;
mod world;

pub use access::AccessLog;
pub use borrow::{Lookup, QueryLookup, QueryResources, ReadOnlyQueryData, ResourceSet, ResourceTuple};
pub use bundle::{Bundle, Component};
pub use change::{Added, Changed, ChunkQuery, Tick};
pub use commands::Commands;
pub use entity::Entity;
pub use query::{Or, OrTerm, OrTerms, Query, QueryData, QueryFilter, QueryIter, With, Without};
pub use storage::{Archetype, ComponentId, ComponentInfo, DropFn};
pub use view::{ViewAccess, ViewQuery, ViewState, WorldView};
pub use world::{Chunk, World};
