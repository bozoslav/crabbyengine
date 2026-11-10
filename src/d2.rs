//! Two-dimensional game types. Pair with `ponko::prelude::*`.
#[doc(inline)]
pub use crate::attach::Attached;
#[doc(inline)]
pub use crate::camera::Camera2D as Camera;
#[cfg(any(feature = "physics", feature = "physics3d"))]
#[doc(inline)]
pub use crate::collision::{BodyKind, CollisionStarted, CollisionStopped};
#[doc(inline)]
pub use crate::ecs::{Sprite, Transform2D as Transform};
#[doc(inline)]
pub use crate::flow::{FlowField, NavGrid};
#[cfg(feature = "physics")]
#[doc(inline)]
pub use crate::physics::{Collider, Physics, QueryFilter, RayHit as Hit, RigidBody, Shape, Velocity};
#[doc(inline)]
pub use crate::sheet::{Animation, SpriteSheet};
#[doc(inline)]
pub use crate::spatial::SpatialGrid;
#[doc(inline)]
pub use crate::tilemap::TileMap;
