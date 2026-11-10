//! The engine's built-in systems, as plain functions.
//!
//! What lives here: every step the engine runs by itself, or adds when you
//! turn a feature on (`Game::physics`, `Game::spatial_grid`), is an ordinary
//! system `fn(&mut Ctx)`. A game doesn't normally call these: [`Game`] adds
//! them in the right [`Phase`]. They are here for the cases where you place
//! one yourself: take it out with [`Game::remove`] and its
//! [`Systems`](crate::Systems) id, then add the function again where you
//! want it, or add one of the opt-in systems.
//!
//! | Function | Id in [`Systems`](crate::Systems) | Added by | Phase |
//! |---|---|---|---|
//! | [`rebuild_spatial_grid`] | `REBUILD_SPATIAL_GRID` | `Game::spatial_grid` | `Input` |
//! | [`step_physics`] | `PHYSICS` | `Game::physics` | `Physics` |
//! | [`step_physics3d`] | `PHYSICS3D` | `Game::physics3d` | `Physics` |
//! | [`interpolate_physics3d`] | `INTERPOLATE_PHYSICS3D` | `Game::physics3d` | `Frame` |
//! | [`update_animators`] | `UPDATE_ANIMATORS` | always (`3d`) | end of `Animate` |
//! | [`update_particles`] | `UPDATE_PARTICLES` | always (`3d`) | end of `Animate` |
//! | [`update_audio3d`] | `UPDATE_AUDIO3D` | always (`3d`) | end of `Draw` |
//! | [`animate_sprites`] | `ANIMATE_SPRITES` | always | `Draw` |
//! | [`draw_tilemaps`] | `DRAW_TILEMAPS` | always | `Draw` |
//! | [`draw_sprites`] | `DRAW_SPRITES` | always | `Draw` |
//! | [`follow_attachments`] | `FOLLOW_ATTACHMENTS` | you | your choice |
//! | [`step_navmesh3d`] | `STEP_NAVMESH3D` | you | your choice |
//! | [`apply_net_poses`] | | you | your choice |
//!
//! ```
//! use ponko::{Game, Phase, Systems, systems};
//!
//! // Draw sprites before the tilemaps instead of after them.
//! let game = Game::new("Order", 800, 600)
//!     .remove(Systems::DRAW_TILEMAPS)
//!     .add(Phase::Draw, systems::draw_tilemaps);
//! # let _ = game;
//! ```
//!
//! [`Game`]: crate::Game
//! [`Game::remove`]: crate::Game::remove
//! [`Phase`]: crate::Phase

#[cfg(feature = "3d")]
use crate::Ctx;

pub use crate::attach::follow_attachments;
pub use crate::ecs::draw_sprites;
#[cfg(feature = "navmesh")]
pub use crate::nav3d::step_navmesh3d;
#[cfg(feature = "3d")]
pub use crate::net::apply_net_poses;
#[cfg(feature = "physics")]
pub use crate::physics::step_physics;
#[cfg(feature = "physics3d")]
pub use crate::physics3d::{interpolate_physics3d, step_physics3d};
pub use crate::sheet::animate_sprites;
pub use crate::spatial::rebuild_spatial_grid;
pub use crate::tilemap::draw_tilemaps;

/// Advances every [`Animator`](crate::anim::Animator) that isn't in manual mode
/// by this frame's time and poses its [`Skeleton`](crate::anim::Skeleton); with
/// `physics3d`, ragdolls pose theirs first. The game runs it at the end of
/// [`Phase::Animate`](crate::Phase::Animate)
/// ([`Systems::UPDATE_ANIMATORS`](crate::Systems::UPDATE_ANIMATORS)), so late
/// systems, draw systems and the renderer see this frame's pose.
#[cfg(feature = "3d")]
pub fn update_animators(ctx: &mut Ctx) {
    ctx.update_animators();
}

/// Advances every [`ParticleEmitter3D`](crate::d3::ParticleEmitter) and
/// [`Trail3D`](crate::d3::Trail) by this frame's time. The game runs it at
/// the end of [`Phase::Animate`](crate::Phase::Animate), after the animators
/// ([`Systems::UPDATE_PARTICLES`](crate::Systems::UPDATE_PARTICLES)).
#[cfg(feature = "3d")]
pub fn update_particles(ctx: &mut Ctx) {
    ctx.update_particles();
}

/// Moves every positional sound ([`AudioSource3D`](crate::d3::AudioSource))
/// to this frame's listener and emitters. The game runs it at the end of
/// [`Phase::Draw`](crate::Phase::Draw), after the drawing systems
/// ([`Systems::UPDATE_AUDIO3D`](crate::Systems::UPDATE_AUDIO3D)), so sound
/// follows where everything ended up this frame.
#[cfg(feature = "3d")]
pub fn update_audio3d(ctx: &mut Ctx) {
    crate::audio::update_audio3d(ctx);
}

/// The old crate-root spellings of the built-in systems, kept there until
/// the next major release. They re-export the functions above (the same
/// items, as before the move), so `use ponko::*;` combines with
/// `use ponko::systems::*;` and with the owning modules' globs. rustc does
/// not warn through a re-export; the naming chapter lists them.
#[doc(hidden)]
pub mod at_root {
    #[deprecated(note = "use `ponko::systems::animate_sprites`, or `Systems::ANIMATE_SPRITES` to remove it")]
    pub use super::animate_sprites;
    #[cfg(feature = "3d")]
    #[deprecated(note = "use `ponko::systems::apply_net_poses`")]
    pub use super::apply_net_poses;
    #[deprecated(note = "use `ponko::systems::draw_sprites`, or `Systems::DRAW_SPRITES` to remove it")]
    pub use super::draw_sprites;
    #[deprecated(note = "use `ponko::systems::draw_tilemaps`, or `Systems::DRAW_TILEMAPS` to remove it")]
    pub use super::draw_tilemaps;
    #[deprecated(note = "use `ponko::systems::follow_attachments`, or `Systems::FOLLOW_ATTACHMENTS` to remove it")]
    pub use super::follow_attachments;
    #[cfg(feature = "physics3d")]
    #[deprecated(
        note = "use `ponko::systems::interpolate_physics3d`, or `Systems::INTERPOLATE_PHYSICS3D` to remove it"
    )]
    pub use super::interpolate_physics3d;
    #[deprecated(
        note = "use `ponko::systems::rebuild_spatial_grid`, or `Systems::REBUILD_SPATIAL_GRID` to remove it"
    )]
    pub use super::rebuild_spatial_grid;
    #[cfg(feature = "navmesh")]
    #[deprecated(note = "use `ponko::systems::step_navmesh3d`, or `Systems::STEP_NAVMESH3D` to remove it")]
    pub use super::step_navmesh3d;
    #[cfg(feature = "physics")]
    #[deprecated(note = "use `ponko::systems::step_physics`, or `Systems::PHYSICS` to remove it")]
    pub use super::step_physics;
    #[cfg(feature = "physics3d")]
    #[deprecated(note = "use `ponko::systems::step_physics3d`, or `Systems::PHYSICS3D` to remove it")]
    pub use super::step_physics3d;
}
