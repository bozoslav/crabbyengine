//! The character controller: a capsule that walks, climbs slopes and
//! stairs, sticks to the ground going downhill and pushes things.

use rapier3d::control::{CharacterAutostep, CharacterCollision, CharacterLength, KinematicCharacterController};
use rapier3d::prelude as rp;
use serde::{Deserialize, Serialize};

use super::{CollisionGroups, LinkKind, Physics3D, from_rp, to_rp};
use crate::{Entity, Vec3, World};

/// A walking character (a player, a person): an upright capsule moved by
/// its [`velocity`](Self::velocity) that slides along walls, climbs slopes
/// up to [`max_slope`](Self::max_slope) and steps up to
/// [`step_height`](Self::step_height), stays on the ground going downhill,
/// falls by gravity and pushes dynamic bodies. Needs a
/// [`Transform3D`](crate::Transform3D), whose position is at its feet.
///
/// Set `velocity`'s X and Z each tick to walk; Y keeps falling speed (set
/// it to jump). After the step, `grounded` says whether it stands on
/// something. Only the position is the physics': turn the transform freely
/// (to face where the player looks).
///
/// ```
/// use crabbyengine::*;
/// let mut player = Character3D::new(1.8, 0.3);
/// player.velocity.x = 4.0; // walk right at 4 m/s
/// if player.grounded {
///     player.velocity.y = 5.0; // jump
/// }
/// assert_eq!(player.max_slope, 45f32.to_radians());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Character3D {
    /// From feet to head, in meters (at least twice the radius).
    pub height: f32,
    /// The capsule's radius.
    pub radius: f32,
    /// The steepest ground it walks up, in radians (default 45°). It slides
    /// down anything steeper.
    pub max_slope: f32,
    /// The highest step it climbs without jumping (default 0.35 m); 0 for none.
    pub step_height: f32,
    /// How far down it follows the ground (down slopes and stairs) instead
    /// of flying off it (default 0.3 m); 0 for never.
    pub snap_distance: f32,
    /// How hard it pushes dynamic bodies it walks into, as a mass in kg
    /// (default 80); 0 for not at all.
    pub push_mass: f32,
    /// 1 falls by the world's gravity, 0 floats.
    pub gravity_scale: f32,
    /// Which colliders it touches.
    pub groups: CollisionGroups,
    /// Meters per second. Yours to set; the step adds gravity and clears
    /// the fall on landing and on hitting a ceiling.
    pub velocity: Vec3,
    /// Stands on something, as of the last step.
    pub grounded: bool,
}

impl Default for Character3D {
    fn default() -> Self {
        Self::new(1.8, 0.3)
    }
}

impl Character3D {
    /// A character `height` tall (feet to head) and `radius` wide, with the
    /// default limits.
    #[must_use]
    pub fn new(height: f32, radius: f32) -> Self {
        Self {
            height,
            radius,
            max_slope: 45f32.to_radians(),
            step_height: 0.35,
            snap_distance: 0.3,
            push_mass: 80.0,
            gravity_scale: 1.0,
            groups: CollisionGroups::ALL,
            velocity: Vec3::ZERO,
            grounded: false,
        }
    }
}

/// What a character's body is made of: change these and it's rebuilt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Capsule {
    radius: f32,
    half_height: f32,
    groups: CollisionGroups,
}

/// The gap kept between a character and the ground or walls.
const SKIN: f32 = 0.01;

impl Capsule {
    pub(super) fn of(c: &Character3D) -> Self {
        let radius = c.radius.max(0.01);
        Self {
            radius,
            half_height: (c.height * 0.5 - radius).max(0.0),
            groups: c.groups,
        }
    }
    fn center(self) -> f32 {
        self.half_height + self.radius
    }
    fn shape(self) -> rp::Capsule {
        rp::Capsule::new_y(self.half_height, self.radius)
    }
    pub(super) fn collider(self, e: Entity) -> rp::Collider {
        rp::ColliderBuilder::new(rp::SharedShape::new(self.shape()))
            .translation(rp::Vector::new(0.0, self.center(), 0.0))
            .collision_groups(self.groups.to_rp())
            .active_events(rp::ActiveEvents::COLLISION_EVENTS)
            .user_data(u128::from(e.to_bits()))
            .build()
    }
}

/// Memory the controller reuses every step.
#[derive(Default)]
pub(super) struct Scratch {
    hits: Vec<CharacterCollision>,
}

pub(super) fn move_characters(physics: &mut Physics3D, world: &mut World, dt: f32) {
    if dt <= 0.0 {
        return;
    }
    let gravity = physics.gravity;
    let up = Vec3::Y;
    world.query::<(Entity, &mut Character3D)>().for_each(|(e, c)| {
        let Some(link) = physics
            .links
            .get(e.index() as usize)
            .and_then(Option::as_ref)
            .filter(|l| l.entity == e)
        else {
            return;
        };
        let LinkKind::Character(capsule) = link.kind else {
            return;
        };
        let body = link.body;
        let Some(feet) = physics.world.bodies.get(body).map(|b| from_rp(b.translation())) else {
            return;
        };

        let controller = KinematicCharacterController {
            up: to_rp(up),
            offset: CharacterLength::Absolute(SKIN),
            slide: true,
            autostep: (c.step_height > 0.0).then_some(CharacterAutostep {
                max_height: CharacterLength::Absolute(c.step_height),
                min_width: CharacterLength::Absolute(capsule.radius),
                include_dynamic_bodies: false,
            }),
            max_slope_climb_angle: c.max_slope,
            min_slope_slide_angle: c.max_slope,
            snap_to_ground: (c.snap_distance > 0.0).then_some(CharacterLength::Absolute(c.snap_distance)),
            ..KinematicCharacterController::default()
        };

        let mut velocity = c.velocity;
        if !velocity.is_finite() {
            velocity = Vec3::ZERO;
        }
        if c.grounded && velocity.dot(up) <= 0.0 {
            // Standing: no fall to push into the ground (snapping keeps it
            // on the ground going downhill); walking off an edge drops it.
            velocity -= up * velocity.dot(up);
        } else {
            velocity += gravity * c.gravity_scale * dt;
        }
        let desired = velocity * dt;
        let shape = capsule.shape();
        let center = rp::Pose::from_translation(to_rp(feet + up * capsule.center()));
        let filter = rp::QueryFilter::default()
            .exclude_rigid_body(body)
            .exclude_sensors()
            .groups(capsule.groups.to_rp());
        let hits = &mut physics.character.hits;
        hits.clear();
        let moved = {
            let queries = physics.world.query_pipeline_with_filter(filter);
            controller.move_shape(dt, &queries, &shape, &center, to_rp(desired), |hit| hits.push(hit))
        };
        let moved_by = from_rp(moved.translation);

        // Push the dynamic bodies it walked into (rapier's impulses only
        // allocate when there are some).
        if c.push_mass > 0.0 {
            let bodies = &physics.world.bodies;
            let colliders = &physics.world.colliders;
            hits.retain(|h| {
                colliders
                    .get(h.handle)
                    .and_then(|co| co.parent())
                    .and_then(|b| bodies.get(b))
                    .is_some_and(|b| b.is_dynamic())
            });
            if !hits.is_empty() {
                let w = &mut physics.world;
                let mut queries = w.broad_phase.as_query_pipeline_mut(
                    w.narrow_phase.query_dispatcher(),
                    &mut w.bodies,
                    &mut w.colliders,
                    filter,
                );
                controller.solve_character_collision_impulses(dt, &mut queries, &shape, c.push_mass, hits.iter());
            }
        }

        let vertical = velocity.dot(up);
        if moved.grounded && vertical < 0.0 {
            // Landed: stop falling.
            velocity -= up * vertical;
        } else if vertical > 0.0 && moved_by.dot(up) < desired.dot(up) * 0.5 {
            // Hit a ceiling: stop rising.
            velocity -= up * vertical;
        }
        c.velocity = velocity;
        c.grounded = moved.grounded;
        if let Some(b) = physics.world.bodies.get_mut(body) {
            b.set_next_kinematic_translation(to_rp(feet + moved_by));
        }
    });
}
