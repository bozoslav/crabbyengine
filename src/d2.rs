//! Two-dimensional game types. Pair with `ponko::prelude::*`.
//!
//! `use ponko::d2::*;` brings in the types whose 3D twin has the same name
//! in [`d3`](crate::d3): the transform, the camera, the physics (feature
//! `physics`), the particle emitter, and the sprite. This module is their
//! one path, under these names, so it is the name rustdoc and the guide
//! use. The rest of 2D lives in its topic module: [`tilemap`](crate::tilemap),
//! [`sheet`](crate::sheet) (sprite-sheet animation), [`attach`](crate::attach),
//! [`spatial`](crate::spatial), [`flow`](crate::flow),
//! [`particles`](crate::particles) (curves and gradients shared with 3D),
//! [`collision`](crate::collision) (shared with 3D) and
//! [`render`](crate::render) (sprite materials).
//!
//! ## Physics
//!
//! 2D physics with rapier (feature `physics`): rigid bodies and colliders
//! as components, stepped inside the simulation clock. [`Transform`] stays
//! the source of truth: set it and the body teleports; dynamic bodies write
//! their new position back after every step. Physics is opt-in per entity,
//! so the million-entity paths (spatial grid, flow fields) stay as fast as
//! before. Turn it on with [`Game::physics`](crate::game::Game::physics);
//! the step is [`systems::step_physics`](crate::systems::step_physics).
#[doc(inline)]
pub use crate::camera2d::Camera2D as Camera;
#[doc(inline)]
pub use crate::ecs::{Sprite, Transform2D as Transform};
#[doc(inline)]
pub use crate::particles2d::{
    ParticleEmitter2D as ParticleEmitter, ParticleSettings2D as ParticleSettings, ParticleShape2D as ParticleShape,
};
#[cfg(feature = "physics")]
#[doc(inline)]
pub use crate::physics::{Collider, Physics, QueryFilter, RayHit as Hit, RigidBody, Shape, Velocity};
