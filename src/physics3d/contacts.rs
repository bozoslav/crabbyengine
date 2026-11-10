//! Contact modification: one-way surfaces, conveyor belts and a custom hook
//! that sees (and changes) a body's contacts before the solver does.

use rapier3d::prelude as rp;

use super::{Physics3D, from_rp, rot_from_rp, to_rp};
use crate::{Entity, Vec3, World};

/// Changes how a body's contacts behave. Put it on an entity with a
/// [`Collider3D`](crate::d3::Collider) (the one-way side and the conveyor
/// velocity are in that body's own frame, so they turn with it):
///
/// - [`one_way`](Self::one_way): things pass through from every side but
///   one, like a platform you jump up through and then stand on.
/// - [`surface_velocity`](Self::surface_velocity): the surface moves what
///   touches it without moving itself, like a conveyor belt or an escalator.
/// - [`hook`](Self::hook): its contacts go to the
///   [`Physics3D::set_contact_hook`] closure.
///
/// Bodies with this component pay a little: their contacts are rebuilt every
/// step instead of reused.
/// ```
/// use ponko::prelude::*;
/// let mut world = World::new();
/// // A platform you can jump up through, and a belt carrying things 2 m/s along +X.
/// world.spawn((
///     ponko::d3::Transform::default(),
///     ponko::d3::RigidBody::fixed(),
///     ponko::d3::Collider::cuboid(vec3(4.0, 0.2, 4.0)),
///     ponko::d3::ContactModifier::one_way(Vec3::Y),
/// ));
/// world.spawn((
///     ponko::d3::Transform::at(vec3(10.0, 0.0, 0.0)),
///     ponko::d3::RigidBody::fixed(),
///     ponko::d3::Collider::cuboid(vec3(8.0, 0.2, 1.0)),
///     ponko::d3::ContactModifier::conveyor(vec3(2.0, 0.0, 0.0)),
/// ));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactModifier3D {
    /// When set, contacts count only where the other body touches the side
    /// this (local) direction points to: from anywhere else it passes
    /// through. A body already partly inside keeps passing until it is out.
    pub one_way: Option<Vec3>,
    /// How far from `one_way` a contact's direction may lean and still
    /// count, in radians. Default 1.2 (about 70°), so a steep slope of the
    /// surface still holds.
    pub one_way_angle: f32,
    /// How fast the surface moves the things on it, in its own frame (m/s);
    /// the part along the surface counts. Zero for none.
    pub surface_velocity: Vec3,
    /// Whether this body's contacts go to the custom
    /// [`Physics3D::set_contact_hook`] closure.
    pub hook: bool,
}

impl Default for ContactModifier3D {
    fn default() -> Self {
        Self {
            one_way: None,
            one_way_angle: 1.2,
            surface_velocity: Vec3::ZERO,
            hook: false,
        }
    }
}

impl ContactModifier3D {
    /// Solid only from the side `up` (local) points to.
    /// ```
    /// # use ponko::prelude::*;
    /// assert_eq!(ponko::d3::ContactModifier::one_way(Vec3::Y).one_way, Some(Vec3::Y));
    /// ```
    #[must_use]
    pub fn one_way(up: Vec3) -> Self {
        Self {
            one_way: Some(up),
            ..Self::default()
        }
    }

    /// A surface moving what touches it at `velocity` (local, m/s).
    /// ```
    /// # use ponko::prelude::*;
    /// assert_eq!(ponko::d3::ContactModifier::conveyor(Vec3::X).surface_velocity, Vec3::X);
    /// ```
    #[must_use]
    pub fn conveyor(velocity: Vec3) -> Self {
        Self {
            surface_velocity: velocity,
            ..Self::default()
        }
    }

    /// Contacts handed to the [`Physics3D::set_contact_hook`] closure.
    /// ```
    /// # use ponko::prelude::*;
    /// assert!(ponko::d3::ContactModifier::hook().hook);
    /// ```
    #[must_use]
    pub fn hook() -> Self {
        Self {
            hook: true,
            ..Self::default()
        }
    }
}

/// The custom contact hook: see [`Physics3D::set_contact_hook`].
pub(crate) type ContactHook = Box<dyn Fn(&mut ContactPair3D) + Send + Sync>;

/// The contacts between two bodies this step, as the custom contact hook
/// sees them (see [`Physics3D::set_contact_hook`]). Changes affect this step
/// only; the hook runs again the next.
pub struct ContactPair3D<'a, 'b> {
    /// One body's entity.
    pub a: Entity,
    /// The other body's entity.
    pub b: Entity,
    manifold: &'a mut rp::ModifiableManifold<'b>,
}

impl ContactPair3D<'_, '_> {
    /// The contact direction in the world, pointing from `a` toward `b`.
    pub fn normal(&self) -> Vec3 {
        from_rp(*self.manifold.normal)
    }

    /// How many contact points there are (0 once ignored).
    pub fn point_count(&self) -> usize {
        self.manifold.solver_contacts.len()
    }

    /// Contact point `i` in the world (on `a`'s surface), and how far the two
    /// bodies are apart there (negative: overlapping).
    pub fn point(&self, i: usize) -> Option<(Vec3, f32)> {
        self.manifold
            .solver_contacts
            .get(i)
            .map(|c| (from_rp(c.anchor1), c.dist))
    }

    /// Drops every contact this step: the bodies pass through each other.
    pub fn ignore(&mut self) {
        self.manifold.solver_contacts.clear();
    }

    /// Drops contact point `i`.
    pub fn remove_point(&mut self, i: usize) {
        if i < self.manifold.solver_contacts.len() {
            self.manifold.solver_contacts.swap_remove(i);
        }
    }

    /// The friction used for this pair (by default from both colliders).
    pub fn friction(&self) -> f32 {
        *self.manifold.friction
    }

    /// Changes the friction for this pair, this step.
    pub fn set_friction(&mut self, friction: f32) {
        *self.manifold.friction = friction.max(0.0);
    }

    /// The bounciness used for this pair.
    pub fn restitution(&self) -> f32 {
        *self.manifold.restitution
    }

    /// Changes the bounciness for this pair, this step.
    pub fn set_restitution(&mut self, restitution: f32) {
        *self.manifold.restitution = restitution.max(0.0);
    }

    /// Makes `b` slide along `a`'s surface at `velocity` (world, m/s;
    /// the part along the surface counts), as on a conveyor belt.
    pub fn set_surface_velocity(&mut self, velocity: Vec3) {
        let n = *self.manifold.normal;
        let v = to_rp(velocity);
        let tangent = v - n * v.dot(n);
        for c in self.manifold.solver_contacts.iter_mut() {
            c.tangent_velocity = tangent;
        }
    }
}

/// What the step's hooks need: the modifiers by entity index, and the
/// custom closure.
pub(crate) struct Hooks<'a> {
    pub table: &'a [Option<(Entity, ContactModifier3D)>],
    pub custom: Option<&'a (dyn Fn(&mut ContactPair3D) + Send + Sync)>,
}

impl Hooks<'_> {
    fn modifier(&self, colliders: &rp::ColliderSet, h: rp::ColliderHandle) -> Option<(Entity, ContactModifier3D)> {
        let e = Entity::from_bits(colliders.get(h)?.user_data as u64);
        self.table
            .get(e.index() as usize)
            .copied()
            .flatten()
            .filter(|&(owner, _)| owner == e)
    }
}

impl rp::PhysicsHooks for Hooks<'_> {
    fn modify_solver_contacts(&self, context: &mut rp::ContactModificationContext) {
        let colliders = context.colliders;
        let (c1, c2) = (context.collider1, context.collider2);
        let m1 = self.modifier(colliders, c1);
        let m2 = self.modifier(colliders, c2);
        if let Some((_, m)) = m1 {
            one_way_and_conveyor(context, &m, colliders, c1, true);
        }
        if let Some((_, m)) = m2 {
            one_way_and_conveyor(context, &m, colliders, c2, false);
        }
        let hooked = m1.is_some_and(|(_, m)| m.hook) || m2.is_some_and(|(_, m)| m.hook);
        if let (true, Some(hook)) = (hooked, self.custom) {
            let entity = |h| colliders.get(h).map(|c| Entity::from_bits(c.user_data as u64));
            let (Some(a), Some(b)) = (entity(c1), entity(c2)) else {
                return;
            };
            if let Some(manifold) = context.rigid_mut() {
                hook(&mut ContactPair3D { a, b, manifold });
            }
        }
    }
}

/// Manifold states for one-way surfaces, kept in the manifold's user data.
const UNKNOWN: u32 = 0;
const ALLOWED: u32 = 1;
const FORBIDDEN: u32 = 2;

fn one_way_and_conveyor(
    context: &mut rp::ContactModificationContext,
    m: &ContactModifier3D,
    colliders: &rp::ColliderSet,
    own: rp::ColliderHandle,
    first: bool,
) {
    let Some(rigid) = context.rigid_mut() else {
        return;
    };
    if let Some(up) = m.one_way.and_then(Vec3::try_normalize) {
        // The contact direction out of this collider, in its own frame.
        let local = if first {
            rigid.manifold.local_n1
        } else {
            rigid.manifold.local_n2
        };
        let ok = from_rp(local).dot(up) >= m.one_way_angle.cos();
        match *rigid.user_data {
            UNKNOWN if ok => *rigid.user_data = ALLOWED,
            UNKNOWN => {
                rigid.solver_contacts.clear();
                // A zero normal (touching at a single point) can't tell yet.
                if local.length_squared() > 0.1 {
                    *rigid.user_data = FORBIDDEN;
                }
            }
            FORBIDDEN => {
                if ok && rigid.solver_contacts.iter().all(|c| c.dist > 0.0) {
                    *rigid.user_data = ALLOWED;
                } else {
                    rigid.solver_contacts.clear();
                }
            }
            _ => {
                if rigid.solver_contacts.is_empty() {
                    *rigid.user_data = UNKNOWN;
                }
            }
        }
    }
    if m.surface_velocity != Vec3::ZERO && !rigid.solver_contacts.is_empty() {
        let Some(collider) = colliders.get(own) else {
            return;
        };
        let world = rot_from_rp(collider.position().rotation) * m.surface_velocity;
        let n = *rigid.normal;
        let v = to_rp(world);
        let tangent = v - n * v.dot(n);
        // Tangent velocity is the second body's relative to the first.
        let tangent = if first { tangent } else { -tangent };
        for c in rigid.solver_contacts.iter_mut() {
            c.tangent_velocity = tangent;
        }
    }
}

/// Picks up added, changed and removed modifiers, and switches rapier's
/// contact hook on for exactly the colliders that have one.
pub(crate) fn sync(physics: &mut Physics3D, world: &mut World) {
    // Last step's list moves to `old`; this step's is built in the buffer
    // freed the step before, so nothing allocates once warm.
    let old = std::mem::take(&mut physics.contact_hooked);
    let mut now = std::mem::take(&mut physics.contact_spare);
    now.clear();
    world
        .query::<(Entity, &ContactModifier3D)>()
        .for_each(|(e, m)| now.push((e, *m)));
    for &(e, _) in &old {
        if let Some(slot) = physics.contact_table.get_mut(e.index() as usize) {
            *slot = None;
        }
    }
    for &(e, m) in &now {
        let i = e.index() as usize;
        if physics.contact_table.len() <= i {
            physics.contact_table.resize(i + 1, None);
        }
        physics.contact_table[i] = Some((e, m));
        set_hooks(physics, e, true);
    }
    for &(e, _) in &old {
        let kept = physics
            .contact_table
            .get(e.index() as usize)
            .is_some_and(|slot| slot.is_some_and(|(owner, _)| owner == e));
        if !kept {
            set_hooks(physics, e, false);
        }
    }
    physics.contact_hooked = now;
    physics.contact_spare = old;
}

fn set_hooks(physics: &mut Physics3D, e: Entity, on: bool) {
    let Some(link) = physics.link(e) else {
        return;
    };
    let body = link.body;
    let count = physics.world.bodies.get(body).map_or(0, |b| b.colliders().len());
    for k in 0..count {
        let h = physics.world.bodies[body].colliders()[k];
        let has = physics
            .world
            .colliders
            .get(h)
            .is_some_and(|c| c.active_hooks().contains(rp::ActiveHooks::MODIFY_SOLVER_CONTACTS));
        // `get_mut` marks a collider changed, so only touch ones that differ.
        if has != on
            && let Some(c) = physics.world.colliders.get_mut(h)
        {
            let mut hooks = c.active_hooks();
            hooks.set(rp::ActiveHooks::MODIFY_SOLVER_CONTACTS, on);
            c.set_active_hooks(hooks);
        }
    }
}
