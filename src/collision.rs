//! What 2D and 3D physics share: how a body moves, and collision events.

use serde::{Deserialize, Serialize};

use crate::Entity;

/// How a body moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BodyKind {
    /// Moved by gravity, forces and collisions.
    #[default]
    Dynamic,
    /// Never moves (walls, ground).
    Fixed,
    /// Moved only by you (through its transform or velocity); pushes
    /// dynamic bodies but isn't pushed back (platforms, doors).
    Kinematic,
}

/// Event: two colliders started touching (or overlapping, for sensors).
/// Read with `ctx.events.read::<CollisionStarted>()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)] // C reads these in place as CrabbyCollision.
pub struct CollisionStarted {
    /// One of the two entities.
    pub a: Entity,
    /// The other one.
    pub b: Entity,
    /// One of them is a sensor.
    pub sensor: bool,
}

/// Event: two colliders stopped touching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct CollisionStopped {
    /// One of the two entities.
    pub a: Entity,
    /// The other one.
    pub b: Entity,
    /// One of them is a sensor.
    pub sensor: bool,
}
