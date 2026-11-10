//! The one glob import a game starts with: `use ponko::prelude::*;`.
//!
//! It holds what a typical game touches: the game loop ([`Game`], [`Ctx`],
//! [`Phase`], [`Plugin`], [`Error`]), the ECS ([`World`], [`Entity`],
//! [`Query`], [`Commands`]), math (`Vec2`, `Vec3`, `Quat`, `Mat4` and their
//! constructors), input ([`Key`], [`MouseButton`]), [`Color`], textures,
//! sounds and fonts, [`Rect`], [`Random`], [`Events`], reflection, and the
//! common transforms and cameras under their descriptive names
//! (`Transform2D`, `Camera2D`, `Transform3D`, `Camera3D`).
//!
//! Add `use ponko::d2::*;` or `use ponko::d3::*;` for one dimension's names
//! (`Transform`, `RigidBody`, `Physics`, `Hit`, ...), and import service
//! modules (`ponko::net`, `ponko::perf`, `ponko::render`, `ponko::anim`, ...)
//! where a file needs them. Short generic names a game is likely to define
//! itself (`Pose`, `Material`, `Job`, `Motion`, `Shader`, `Joint`, ...) stay
//! out of the prelude.
//!
//! ```
//! use ponko::prelude::*;
//! use ponko::d2::*;
//!
//! struct Pose; // the game's own; no clash with the prelude
//! let mut world = World::new();
//! let e = world.spawn((Transform::at(vec2(1.0, 2.0)),));
//! assert_eq!(world.get::<Transform2D>(e).unwrap().position, vec2(1.0, 2.0));
//! # let _ = Pose;
//! ```
pub use crate::actions::{Action, ActionId, ActionMap, ActionRef, Axis2Binding, AxisBinding, ButtonBinding};
pub use crate::assets::{AssetSource, Assets, MemorySource};
pub use crate::audio::{AudioBus, Playing, Sound, SoundParams};
pub use crate::camera::Rect;
#[cfg(any(feature = "physics", feature = "physics3d"))]
pub use crate::collision::{BodyKind, CollisionStarted, CollisionStopped};
pub use crate::color::Color;
pub use crate::ctx::Ctx;
#[cfg(feature = "3d")]
pub use crate::d3;
pub use crate::error::{Error, ErrorKind};
pub use crate::events::Events;
pub use crate::game::{ConfiguredGame, Game, Phase, Plugin, SetupResult, SystemId, Systems};
pub use crate::handle::Handle;
pub use crate::input::{Key, MouseButton};
pub use crate::random::{Random, Rng};
pub use crate::reflect::WorldReflect;
pub use crate::scene::Placement;
pub use crate::sheet::SpriteSheet;
pub use crate::text::{Font, TextParams};
pub use crate::texture::{DrawParams, Texture, TextureFilter};
pub use crate::time::{Time, Turns};
pub use crate::{d2, systems};
pub use crate::{json, profile_scope};
#[cfg(feature = "3d")]
pub use glam::{Mat4, Vec4, vec4};
pub use glam::{Quat, UVec2, Vec2, Vec3, uvec2, vec2, vec3};
pub use ponko_ecs::{Added, Changed, Commands, Component, Entity, Or, Query, Tick, With, Without, World};
// Common dimension types retain descriptive spellings in mixed-dimensional code.
pub use crate::actions;
pub use crate::d2::{Camera as Camera2D, Sprite, Transform as Transform2D};
#[cfg(feature = "3d")]
pub use crate::d3::{Camera as Camera3D, Model, Transform as Transform3D};
pub use crate::ui::{Ui, UiId};
