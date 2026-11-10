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

/// Advances every [`Animator`](crate::Animator) that isn't in manual mode
/// by this frame's time and poses its [`Skeleton`](crate::Skeleton); with
/// `physics3d`, ragdolls pose theirs first. The game runs it at the end of
/// [`Phase::Animate`](crate::Phase::Animate)
/// ([`Systems::UPDATE_ANIMATORS`](crate::Systems::UPDATE_ANIMATORS)), so late
/// systems, draw systems and the renderer see this frame's pose.
#[cfg(feature = "3d")]
pub fn update_animators(ctx: &mut Ctx) {
    ctx.update_animators();
}

/// Advances every [`ParticleEmitter3D`](crate::ParticleEmitter3D) and
/// [`Trail3D`](crate::Trail3D) by this frame's time. The game runs it at
/// the end of [`Phase::Animate`](crate::Phase::Animate), after the animators
/// ([`Systems::UPDATE_PARTICLES`](crate::Systems::UPDATE_PARTICLES)).
#[cfg(feature = "3d")]
pub fn update_particles(ctx: &mut Ctx) {
    ctx.update_particles();
}

/// Moves every positional sound ([`AudioSource3D`](crate::AudioSource3D))
/// to this frame's listener and emitters. The game runs it at the end of
/// [`Phase::Draw`](crate::Phase::Draw), after the drawing systems
/// ([`Systems::UPDATE_AUDIO3D`](crate::Systems::UPDATE_AUDIO3D)), so sound
/// follows where everything ended up this frame.
#[cfg(feature = "3d")]
pub fn update_audio3d(ctx: &mut Ctx) {
    crate::audio::update_audio3d(ctx);
}

/// The old crate-root spellings of the built-in systems, re-exported there
/// until the next major release. Each forwards to the function above and
/// counts as the same built-in (same [`Systems`](crate::Systems) id).
#[doc(hidden)]
pub mod at_root {
    use crate::Ctx;

    macro_rules! moved {
        ($($(#[$cfg:meta])? $name:ident => $note:literal;)*) => {$(
            $(#[$cfg])?
            #[doc = concat!("Moved to [`ponko::systems::", stringify!($name), "`](crate::systems::", stringify!($name), ").")]
            #[deprecated(note = $note)]
            pub fn $name(ctx: &mut Ctx) {
                super::$name(ctx)
            }
        )*};
    }

    moved! {
        follow_attachments => "use `ponko::systems::follow_attachments`, or `Systems::FOLLOW_ATTACHMENTS` to remove it";
        draw_sprites => "use `ponko::systems::draw_sprites`, or `Systems::DRAW_SPRITES` to remove it";
        animate_sprites => "use `ponko::systems::animate_sprites`, or `Systems::ANIMATE_SPRITES` to remove it";
        draw_tilemaps => "use `ponko::systems::draw_tilemaps`, or `Systems::DRAW_TILEMAPS` to remove it";
        rebuild_spatial_grid => "use `ponko::systems::rebuild_spatial_grid`, or `Systems::REBUILD_SPATIAL_GRID` to remove it";
        #[cfg(feature = "physics")]
        step_physics => "use `ponko::systems::step_physics`, or `Systems::PHYSICS` to remove it";
        #[cfg(feature = "physics3d")]
        step_physics3d => "use `ponko::systems::step_physics3d`, or `Systems::PHYSICS3D` to remove it";
        #[cfg(feature = "physics3d")]
        interpolate_physics3d => "use `ponko::systems::interpolate_physics3d`, or `Systems::INTERPOLATE_PHYSICS3D` to remove it";
        #[cfg(feature = "navmesh")]
        step_navmesh3d => "use `ponko::systems::step_navmesh3d`, or `Systems::STEP_NAVMESH3D` to remove it";
        #[cfg(feature = "3d")]
        apply_net_poses => "use `ponko::systems::apply_net_poses`";
    }
}
