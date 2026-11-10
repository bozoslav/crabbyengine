//! Attachments: entities that follow another entity (health bars, turrets,
//! selection rings, carried things).

use glam::Vec2;
use rayon::prelude::*;

use crate::{Ctx, Entity, Transform2D, World};

/// Keeps this entity's [`Transform2D`] position and rotation at an offset
/// from another entity's, updated by [`follow_attachments`]. Its own scale
/// is left alone. When the entity it's attached to is despawned, this one is
/// despawned too. Attachments can be attached to attachments.
///
/// ```
/// use ponko::*;
///
/// struct Tank;
///
/// let mut turret = None;
/// let world = Game::new("turrets", 320, 240)
///     .sim_system(|ctx: &mut Ctx| {
///         ctx.world.query::<(&mut Transform2D, &Tank)>().for_each(|(t, _)| t.position.x += 1.0);
///     })
///     .sim_system(systems::follow_attachments) // after the systems that move things
///     .run_headless(10, |ctx| {
///         let tank = ctx.world.spawn((Transform2D::at(vec2(0.0, 0.0)), Tank));
///         let on_top = Attached::to(tank).offset(vec2(0.0, -4.0));
///         turret = Some(ctx.world.spawn((Transform2D::default(), on_top)));
///     });
/// let at = world.get::<Transform2D>(turret.unwrap()).unwrap().position;
/// assert_eq!(at, vec2(10.0, -4.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(C)] // shared with C/C++ (`CrabbyAttached`)
pub struct Attached {
    /// The entity to follow.
    pub to: Entity,
    /// Where, in the followed entity's space: turned by its rotation and
    /// stretched by its scale (so a flipped sprite mirrors it).
    pub offset: Vec2,
    /// Added to the followed entity's rotation.
    pub rotation: f32,
}

impl Attached {
    /// Attached to `to`, right on top of it.
    #[must_use]
    pub fn to(to: Entity) -> Self {
        Self {
            to,
            offset: Vec2::ZERO,
            rotation: 0.0,
        }
    }

    /// Same attachment, `offset` away.
    #[must_use]
    pub fn offset(self, offset: Vec2) -> Self {
        Self { offset, ..self }
    }

    /// Same attachment, turned by `rotation` radians more.
    #[must_use]
    pub fn rotated(self, rotation: f32) -> Self {
        Self { rotation, ..self }
    }
}

/// Chains deeper than this are treated as broken (a loop, most likely).
const MAX_DEPTH: usize = 64;

/// Scratch kept between ticks so following allocates nothing.
#[derive(Default)]
struct Scratch {
    attached: Vec<(Entity, Attached)>,
    placed: Vec<Option<(Vec2, f32)>>,
}

/// Moves every entity with [`Attached`] to its place next to the entity it
/// follows, on all cores, and despawns those whose entity is gone. Add it as
/// a simulation system after the ones that move things.
pub fn follow_attachments(ctx: &mut Ctx) {
    let mut s = ctx.world.remove_resource::<Scratch>().unwrap_or_default();
    // Same entities, in the same order, as the writing query below.
    ctx.world
        .query::<(Entity, &Attached)>()
        .with::<Transform2D>()
        .par_collect_into(&mut s.attached, |(e, a)| (e, *a));
    if !s.attached.is_empty() {
        let world = &ctx.world;
        s.placed.clear();
        s.attached
            .par_iter()
            .map(|&(_, a)| place(world, a))
            .collect_into_vec(&mut s.placed);
        let placed = &s.placed;
        ctx.world
            .query::<(&mut Transform2D, &Attached)>()
            .par_for_each_indexed(|i, (t, _)| {
                if let Some((position, rotation)) = placed[i] {
                    t.position = position;
                    t.rotation = rotation;
                }
            });
        for (&(e, _), placed) in s.attached.iter().zip(&s.placed) {
            if placed.is_none() {
                ctx.commands.despawn(e);
            }
        }
    }
    ctx.world.insert_resource(s);
}

/// Where `a` puts its entity: from the root of the chain down. `None` if
/// something up the chain is gone (or the chain loops).
fn place(world: &World, a: Attached) -> Option<(Vec2, f32)> {
    // Walk up to the first entity that isn't attached, keeping the links.
    let mut links = [a; MAX_DEPTH];
    let mut depth = 1;
    let mut up = a.to;
    while let Some(&next) = world.get::<Attached>(up) {
        if depth == MAX_DEPTH {
            return None;
        }
        links[depth] = next;
        depth += 1;
        up = next.to;
    }
    let root = world.get::<Transform2D>(up)?;
    if !world.is_alive(up) {
        return None;
    }
    let (mut position, mut rotation) = (root.position, root.rotation);
    let mut scale = root.scale;
    // Back down: each link is placed in its parent's space.
    for i in (0..depth).rev() {
        let link = links[i];
        position += Vec2::from_angle(rotation).rotate(link.offset * scale);
        rotation += link.rotation;
        if i > 0 {
            // The next link hangs off this one, whose own scale applies.
            let this = links[i - 1].to;
            scale = world.get::<Transform2D>(this).map_or(Vec2::ONE, |t| t.scale);
        }
    }
    Some((position, rotation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vec2;

    #[test]
    fn chains_compose_from_the_root_down() {
        let mut w = World::new();
        let root = w.spawn((Transform2D::at(vec2(10.0, 0.0)).rotated(std::f32::consts::FRAC_PI_2),));
        let arm = w.spawn((Transform2D::default(), Attached::to(root).offset(vec2(1.0, 0.0))));
        let hand = Attached::to(arm).offset(vec2(1.0, 0.0));
        let (p, r) = place(&w, hand).unwrap();
        // Root turned a quarter: both offsets point down (+y on screen).
        assert!((p - vec2(10.0, 2.0)).length() < 1e-5, "{p}");
        assert!((r - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    fn loops_and_missing_roots_place_nothing() {
        let mut w = World::new();
        let a = w.spawn((Transform2D::default(),));
        let b = w.spawn((Transform2D::default(), Attached::to(a)));
        w.insert(a, Attached::to(b)); // a <-> b
        assert_eq!(place(&w, Attached::to(a)), None);
        let gone = w.spawn((Transform2D::default(),));
        w.despawn(gone);
        assert_eq!(place(&w, Attached::to(gone)), None);
    }
}
