//! 3D physics with rapier (feature `physics3d`): rigid bodies, colliders
//! (heightfield terrain and triangle meshes too), a character controller
//! and raycast-wheel vehicles as components, stepped inside the simulation
//! clock.
//!
//! [`Transform3D`] stays the source of truth: set it and the body teleports;
//! moving bodies write their new pose back after every step, and between
//! steps (in windowed games) a pose blended between the last two steps, so
//! motion looks smooth at any frame rate.

use std::sync::Mutex;

use rapier3d::prelude as rp;
use serde::{Deserialize, Serialize};

use crate::collision::{BodyKind, CollisionStarted, CollisionStopped};
use crate::{Ctx, Entity, Quat, Transform3D, Vec3, World};

mod character;
mod files;
mod query;
mod vehicle;

pub use character::Character3D;
pub use query::{Hit3D, QueryFilter3D};
pub use vehicle::{FrictionCurve, Vehicle3D, Wheel3D};

/// The simulation rate [`Game::physics3d`](crate::Game::physics3d) sets,
/// unless the game picked its own with
/// [`Game::fixed_hz`](crate::Game::fixed_hz): 50 steps per second.
pub const PHYSICS3D_HZ: u32 = 50;

/// Makes an entity a 3D physics body. It also needs a [`Collider3D`] and a
/// [`Transform3D`].
///
/// ```
/// use crabbyengine::*;
/// let crate_body = RigidBody3D::dynamic();
/// let ground = RigidBody3D::fixed();
/// assert_eq!(crate_body.gravity_scale, 1.0);
/// assert_eq!(ground.kind, BodyKind::Fixed);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RigidBody3D {
    /// Dynamic, fixed or kinematic.
    pub kind: BodyKind,
    /// 1 normal, 0 floats, negative falls up.
    pub gravity_scale: f32,
    /// Slows movement down over time (air drag); 0 none.
    pub linear_damping: f32,
    /// Slows spinning down over time; 0 none.
    pub angular_damping: f32,
    /// Never rotates.
    pub lock_rotation: bool,
    /// Continuous collision detection: fast bodies (bullets) don't pass
    /// through thin walls. Costs a little more.
    pub ccd: bool,
    /// Its mass in kg, instead of its collider's density × volume (its
    /// inertia scales along). `None` (the default): from the density.
    pub mass: Option<f32>,
    /// Its center of mass in its own frame (relative to its
    /// [`Transform3D`] position), instead of its collider's center. A car
    /// wants it low, so it doesn't tip over in turns. `None` (the
    /// default): the collider's.
    pub center_of_mass: Option<Vec3>,
}

impl Default for RigidBody3D {
    fn default() -> Self {
        Self::dynamic()
    }
}

impl RigidBody3D {
    /// A body moved by gravity and collisions.
    #[must_use]
    pub const fn dynamic() -> Self {
        Self {
            kind: BodyKind::Dynamic,
            gravity_scale: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            lock_rotation: false,
            ccd: false,
            mass: None,
            center_of_mass: None,
        }
    }
    /// A body that never moves (ground, walls, terrain).
    #[must_use]
    pub const fn fixed() -> Self {
        Self {
            kind: BodyKind::Fixed,
            ..Self::dynamic()
        }
    }
    /// A body only you move, through its [`Velocity3D`] or [`Transform3D`]
    /// (platforms, doors). It pushes dynamic bodies and isn't pushed back.
    #[must_use]
    pub const fn kinematic() -> Self {
        Self {
            kind: BodyKind::Kinematic,
            ..Self::dynamic()
        }
    }
    /// Weighing `kg`, whatever its collider's density.
    ///
    /// ```
    /// use crabbyengine::*;
    /// let chassis = RigidBody3D::dynamic().mass(1200.0).center_of_mass(vec3(0.0, -0.4, 0.0));
    /// assert_eq!(chassis.mass, Some(1200.0));
    /// ```
    #[must_use]
    pub const fn mass(self, kg: f32) -> Self {
        Self { mass: Some(kg), ..self }
    }
    /// With its center of mass at `center`, in its own frame.
    #[must_use]
    pub const fn center_of_mass(self, center: Vec3) -> Self {
        Self {
            center_of_mass: Some(center),
            ..self
        }
    }
}

/// A convex hull, triangle mesh or heightfield, built once with
/// [`Physics3D::convex_hull`], [`Physics3D::trimesh`] or
/// [`Physics3D::heightfield`] and shared by any number of
/// [`Collider3D::mesh`] colliders. A handle: copying it doesn't copy the
/// data. Handles are only meaningful in the game that made them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct CollisionMesh {
    index: u32,
    generation: u32,
}

impl CollisionMesh {
    /// The handle as one number (for C).
    #[doc(hidden)]
    #[must_use]
    pub fn to_bits(self) -> u64 {
        u64::from(self.generation) << 32 | u64::from(self.index)
    }
    /// A handle from [`to_bits`](Self::to_bits).
    #[doc(hidden)]
    #[must_use]
    pub fn from_bits(bits: u64) -> Self {
        Self {
            index: bits as u32,
            generation: (bits >> 32) as u32,
        }
    }
}

/// A collider's shape, in world units (meters), centered on the entity
/// (plus [`Collider3D::offset`]) and turned with it. Scale isn't applied.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Shape3D {
    /// A ball.
    Sphere {
        /// Its radius.
        radius: f32,
    },
    /// A box `size` wide, high and deep.
    Cuboid {
        /// Full size along X, Y and Z.
        size: Vec3,
    },
    /// An upright capsule: `height` between the centers of its round ends.
    Capsule {
        /// Distance between the centers of the round ends.
        height: f32,
        /// Radius of the round ends.
        radius: f32,
    },
    /// A convex hull, triangle mesh or heightfield made by [`Physics3D`].
    Mesh(CollisionMesh),
}

/// Which colliders touch which: a collider is a member of the groups in
/// `memberships` (one bit each, 32 groups) and touches colliders whose
/// memberships share a bit with its `filter`. Both sides must agree.
/// Queries use the same test against [`QueryFilter3D::groups`].
///
/// ```
/// use crabbyengine::*;
/// const PLAYER: u32 = 1 << 0;
/// const DEBRIS: u32 = 1 << 1;
/// // Debris hits everything except other debris.
/// let debris = CollisionGroups::new(DEBRIS, !DEBRIS);
/// assert!(!debris.touches(debris));
/// assert!(debris.touches(CollisionGroups::new(PLAYER, u32::MAX)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct CollisionGroups {
    /// The groups this collider is in.
    pub memberships: u32,
    /// The groups it touches.
    pub filter: u32,
}

impl Default for CollisionGroups {
    fn default() -> Self {
        Self::ALL
    }
}

impl CollisionGroups {
    /// In every group, touching every group (the default).
    pub const ALL: Self = Self::new(u32::MAX, u32::MAX);

    /// Member of `memberships`, touching `filter`.
    #[must_use]
    pub const fn new(memberships: u32, filter: u32) -> Self {
        Self { memberships, filter }
    }

    /// Whether colliders with these groups and `other` touch.
    #[must_use]
    pub const fn touches(self, other: Self) -> bool {
        self.memberships & other.filter != 0 && other.memberships & self.filter != 0
    }

    fn to_rp(self) -> rp::InteractionGroups {
        rp::InteractionGroups::new(
            rp::Group::from_bits_retain(self.memberships),
            rp::Group::from_bits_retain(self.filter),
            rp::InteractionTestMode::And,
        )
    }
}

/// The solid shape of a 3D body, its surface (friction, bounciness) and
/// what it touches.
///
/// ```
/// use crabbyengine::*;
/// let ball = Collider3D::sphere(0.5).restitution(0.8);
/// let trigger = Collider3D::cuboid(vec3(4.0, 2.0, 4.0)).sensor();
/// assert!(trigger.sensor);
/// assert_eq!(ball.friction, 0.5);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Collider3D {
    /// Sphere, box, capsule or a mesh.
    pub shape: Shape3D,
    /// Mass per cubic meter (mass = density × volume). Default 1000, water.
    #[serde(default = "water")]
    pub density: f32,
    /// 0 ice, 1 rubber. Default 0.5.
    #[serde(default = "half")]
    pub friction: f32,
    /// Bounciness: 0 none, 1 keeps all its speed. Default 0.
    #[serde(default)]
    pub restitution: f32,
    /// Detects overlaps (sends [`CollisionStarted`]) but doesn't push.
    #[serde(default)]
    pub sensor: bool,
    /// Where the shape sits relative to the entity's position, in its frame.
    #[serde(default)]
    pub offset: Vec3,
    /// Which colliders it touches.
    #[serde(default)]
    pub groups: CollisionGroups,
}

// Collider defaults for fields missing from JSON, as in `Collider3D::of`.
fn water() -> f32 {
    1000.0
}
fn half() -> f32 {
    0.5
}

impl Collider3D {
    fn of(shape: Shape3D) -> Self {
        Self {
            shape,
            density: water(),
            friction: half(),
            restitution: 0.0,
            sensor: false,
            offset: Vec3::ZERO,
            groups: CollisionGroups::ALL,
        }
    }
    /// A ball.
    #[must_use]
    pub fn sphere(radius: f32) -> Self {
        Self::of(Shape3D::Sphere { radius })
    }
    /// A box `size` wide, high and deep.
    #[must_use]
    pub fn cuboid(size: Vec3) -> Self {
        Self::of(Shape3D::Cuboid { size })
    }
    /// An upright capsule: `height` between the centers of its round ends
    /// (so `height + 2 × radius` tall).
    #[must_use]
    pub fn capsule(height: f32, radius: f32) -> Self {
        Self::of(Shape3D::Capsule { height, radius })
    }
    /// A convex hull, triangle mesh or heightfield made by [`Physics3D`].
    ///
    /// ```
    /// use crabbyengine::*;
    /// Game::new("t", 1, 1).physics3d(Vec3::ZERO).run_headless(1, |ctx| {
    ///     let ground = ctx.physics3d.heightfield(&[0.0; 3 * 3], 10.0);
    ///     ctx.world.spawn((Transform3D::default(), RigidBody3D::fixed(), Collider3D::mesh(ground)));
    /// });
    /// ```
    #[must_use]
    pub fn mesh(mesh: CollisionMesh) -> Self {
        Self::of(Shape3D::Mesh(mesh))
    }
    /// With this density.
    #[must_use]
    pub fn density(self, density: f32) -> Self {
        Self { density, ..self }
    }
    /// With this friction.
    #[must_use]
    pub fn friction(self, friction: f32) -> Self {
        Self { friction, ..self }
    }
    /// With this bounciness.
    #[must_use]
    pub fn restitution(self, restitution: f32) -> Self {
        Self { restitution, ..self }
    }
    /// As a sensor: reports overlaps, doesn't collide.
    #[must_use]
    pub fn sensor(self) -> Self {
        Self { sensor: true, ..self }
    }
    /// Shifted from the entity's position.
    #[must_use]
    pub fn offset(self, offset: Vec3) -> Self {
        Self { offset, ..self }
    }
    /// In these collision groups.
    #[must_use]
    pub fn groups(self, groups: CollisionGroups) -> Self {
        Self { groups, ..self }
    }
}

/// A 3D body's speed: meters per second, and radians per second around
/// each axis. Read it after a step; write it to set the speed.
///
/// ```
/// use crabbyengine::*;
/// let thrown = Velocity3D::linear(vec3(0.0, 4.0, -10.0));
/// assert_eq!(thrown.angular, Vec3::ZERO);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[repr(C)] // C sees it as CrabbyVelocity3D.
pub struct Velocity3D {
    /// Meters per second.
    pub linear: Vec3,
    /// Radians per second around X, Y and Z.
    pub angular: Vec3,
}

impl Velocity3D {
    /// Moving at `linear`, not spinning.
    #[must_use]
    pub const fn linear(linear: Vec3) -> Self {
        Self {
            linear,
            angular: Vec3::ZERO,
        }
    }
}

/// A position and rotation, for blending between steps.
#[derive(Clone, Copy, PartialEq)]
struct Pose {
    position: Vec3,
    rotation: Quat,
}

impl Pose {
    fn of(t: &Transform3D) -> Self {
        Self {
            position: t.position,
            rotation: t.rotation,
        }
    }
}

/// What the world last knew of one entity, to notice your changes.
#[derive(Clone, Copy)]
struct Link {
    entity: Entity,
    body: rp::RigidBodyHandle,
    kind: LinkKind,
    velocity: Velocity3D,
    /// The transform as last written or seen: anything else is your change.
    written: Transform3D,
    /// The pose after the step before the last one, and after the last one.
    prev: Pose,
    curr: Pose,
    seen: u64,
}

#[derive(Clone, Copy, PartialEq)]
enum LinkKind {
    Body(RigidBody3D, Collider3D),
    Character(character::Capsule),
}

impl LinkKind {
    fn moves(self) -> bool {
        match self {
            LinkKind::Body(rigid, _) => rigid.kind != BodyKind::Fixed,
            LinkKind::Character(_) => true,
        }
    }
}

struct MeshSlot {
    shape: Option<rp::SharedShape>,
    /// From the entity's position to the shape's center (a heightfield is
    /// placed by its corner, rapier's by its center).
    offset: Vec3,
    generation: u32,
}

/// Collects collision events during a step, into a buffer that's reused.
#[derive(Default)]
struct Collector(Mutex<Vec<rp::CollisionEvent>>);

impl rp::EventHandler for Collector {
    fn handle_collision_event(
        &self,
        _: &rp::RigidBodySet,
        _: &rp::ColliderSet,
        event: rp::CollisionEvent,
        _: Option<&rp::ContactPair>,
    ) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    }
    fn handle_contact_force_event(
        &self,
        _: f32,
        _: &rp::RigidBodySet,
        _: &rp::ColliderSet,
        _: &rp::ContactPair,
        _: f32,
    ) {
    }
    fn handle_soft_body_tear_event(&self, _: &rp::SoftBodySet, _: &rp::SoftBodyTearEvent) {}
}

/// The 3D physics world: `ctx.physics3d`. Add it to a game with
/// [`Game::physics3d`](crate::Game::physics3d); then entities with
/// [`RigidBody3D`], [`Collider3D`] and [`Transform3D`] are simulated, and
/// entities with [`Character3D`] and [`Transform3D`] walk.
pub struct Physics3D {
    world: rp::PhysicsWorld,
    /// By entity index.
    links: Vec<Option<Link>>,
    gravity: Vec3,
    step: u64,
    collector: Collector,
    /// Events taken from the collector, kept to reuse its memory.
    events: Vec<rp::CollisionEvent>,
    meshes: Vec<MeshSlot>,
    free_meshes: Vec<u32>,
    /// Reused by the character controller.
    character: character::Scratch,
}

impl Default for Physics3D {
    fn default() -> Self {
        Self::new(Vec3::ZERO)
    }
}

pub(crate) fn to_rp(v: Vec3) -> rp::Vector {
    rp::Vector::new(v.x, v.y, v.z)
}
pub(crate) fn from_rp(v: rp::Vector) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}
fn rot_to_rp(q: Quat) -> rp::Rotation {
    let q = q.normalize();
    if q.is_finite() {
        rp::Rotation::from_xyzw(q.x, q.y, q.z, q.w)
    } else {
        rp::Rotation::IDENTITY
    }
}
fn rot_from_rp(q: rp::Rotation) -> Quat {
    Quat::from_xyzw(q.x, q.y, q.z, q.w)
}
pub(crate) fn pose_to_rp(position: Vec3, rotation: Quat) -> rp::Pose {
    rp::Pose::from_parts(to_rp(position), rot_to_rp(rotation))
}

impl Physics3D {
    /// Physics pulling everything by `gravity` (meters/s²; `vec3(0.0, -9.81, 0.0)`
    /// is Earth's).
    #[must_use]
    pub fn new(gravity: Vec3) -> Self {
        Self {
            world: rp::PhysicsWorld::new(),
            links: Vec::new(),
            gravity,
            step: 0,
            collector: Collector::default(),
            events: Vec::new(),
            meshes: Vec::new(),
            free_meshes: Vec::new(),
            character: character::Scratch::default(),
        }
    }

    /// The pull on every dynamic body and character.
    pub fn gravity(&self) -> Vec3 {
        self.gravity
    }
    /// Changes the pull on every dynamic body and character.
    pub fn set_gravity(&mut self, gravity: Vec3) {
        self.gravity = gravity;
    }

    /// How many bodies are simulated (characters included).
    pub fn body_count(&self) -> usize {
        self.world.bodies.len()
    }

    fn entity_of(&self, collider: rp::ColliderHandle) -> Option<Entity> {
        self.world
            .colliders
            .get(collider)
            .map(|c| Entity::from_bits(c.user_data as u64))
    }

    fn link(&self, e: Entity) -> Option<&Link> {
        self.links.get(e.index() as usize)?.as_ref().filter(|l| l.entity == e)
    }

    /// Pushes a dynamic body once (an explosion, a kick), in kg × m/s at its
    /// center of mass. Returns false if `e` isn't a physics body (yet:
    /// bodies join at the next physics step).
    pub fn apply_impulse(&mut self, e: Entity, impulse: Vec3) -> bool {
        let Some(link) = self.link(e) else { return false };
        let handle = link.body;
        match self.world.bodies.get_mut(handle) {
            Some(body) => {
                body.apply_impulse(to_rp(impulse), true);
                true
            }
            None => false,
        }
    }

    /// Pushes a dynamic body once at a point in the world (a bullet's hit),
    /// so it spins too. Returns false if `e` isn't a physics body.
    pub fn apply_impulse_at(&mut self, e: Entity, impulse: Vec3, point: Vec3) -> bool {
        let Some(link) = self.link(e) else { return false };
        let handle = link.body;
        match self.world.bodies.get_mut(handle) {
            Some(body) => {
                body.apply_impulse_at_point(to_rp(impulse), to_rp(point), true);
                true
            }
            None => false,
        }
    }

    /// The mass of a body in kg (from its collider's density and volume,
    /// or [`RigidBody3D::mass`]).
    pub fn mass(&self, e: Entity) -> Option<f32> {
        let link = self.link(e)?;
        self.world.bodies.get(link.body).map(|b| b.mass())
    }

    /// A body's center of mass in the world, as of the last physics step
    /// (see [`RigidBody3D::center_of_mass`]).
    pub fn center_of_mass(&self, e: Entity) -> Option<Vec3> {
        let link = self.link(e)?;
        self.world.bodies.get(link.body).map(|b| from_rp(b.center_of_mass()))
    }

    /// Where a body is as of the last physics step: what [`Transform3D`]
    /// holds right after the step (between steps it holds a blend of the
    /// last two, for drawing).
    pub fn position(&self, e: Entity) -> Option<Vec3> {
        self.link(e).map(|l| l.curr.position)
    }

    /// A convex hull around `points` (rocks, crates, vehicle bodies), for
    /// [`Collider3D::mesh`]. `None` if the points are all on one plane or
    /// not finite.
    ///
    /// ```
    /// use crabbyengine::*;
    /// let mut physics = Physics3D::default();
    /// let corners = [vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), vec3(0.0, 0.0, 1.0)];
    /// assert!(physics.convex_hull(&corners).is_some());
    /// assert!(physics.convex_hull(&corners[..3]).is_none(), "flat");
    /// ```
    pub fn convex_hull(&mut self, points: &[Vec3]) -> Option<CollisionMesh> {
        if points.len() < 4 || !points.iter().all(|p| p.is_finite()) {
            return None;
        }
        let points: Vec<rp::Vector> = points.iter().map(|&p| to_rp(p)).collect();
        let shape = rp::SharedShape::convex_hull(&points)?;
        // A hull of points on one plane has no volume.
        let mass = shape.mass_properties(1.0).mass();
        if !(mass > 0.0 && mass.is_finite()) {
            return None;
        }
        Some(self.add_mesh(shape, Vec3::ZERO))
    }

    /// A triangle mesh (buildings, rocks, any static level geometry), for
    /// [`Collider3D::mesh`]: `indices` are three per triangle into
    /// `vertices`. Best on fixed bodies: a dynamic one gets the mass of its
    /// bounding box.
    ///
    /// ```
    /// use crabbyengine::*;
    /// let mut physics = Physics3D::default();
    /// let floor = physics.trimesh(
    ///     &[vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0), vec3(1.0, 0.0, 1.0)],
    ///     &[[0, 2, 1], [1, 2, 3]],
    /// );
    /// assert!(floor.is_some());
    /// ```
    ///
    /// `None` if there are no triangles, an index is out of range or a
    /// vertex isn't finite.
    pub fn trimesh(&mut self, vertices: &[Vec3], indices: &[[u32; 3]]) -> Option<CollisionMesh> {
        if indices.is_empty()
            || !vertices.iter().all(|p| p.is_finite())
            || indices.iter().flatten().any(|&i| i as usize >= vertices.len())
        {
            return None;
        }
        let vertices: Vec<rp::Vector> = vertices.iter().map(|&p| to_rp(p)).collect();
        let shape =
            rp::SharedShape::trimesh_with_flags(vertices, indices.to_vec(), rp::TriMeshFlags::FIX_INTERNAL_EDGES)
                .ok()?;
        Some(self.add_mesh(shape, Vec3::ZERO))
    }

    /// Ground for a terrain tile, from the same heights and size as
    /// [`Ctx::create_terrain_tile`](crate::Ctx::create_terrain_tile) (n × n
    /// heights, row by row along +Z, each row along +X; NaN for a hole), so
    /// the collider matches the drawn ground triangle for triangle. Give it
    /// to a fixed body at the tile's [`Transform3D`]: the tile's corner.
    ///
    /// ```
    /// use crabbyengine::*;
    /// let mut physics = Physics3D::default();
    /// let heights = [0.0, 1.0, 2.0, 0.0, 1.0, 2.0, 0.0, 1.0, 2.0];
    /// let ground = physics.heightfield(&heights, 8.0);
    /// # let _ = ground;
    /// ```
    ///
    /// # Panics
    ///
    /// If `heights` isn't a square grid of at least 2 × 2 or `size` isn't above 0.
    #[track_caller]
    pub fn heightfield(&mut self, heights: &[f32], size: f32) -> CollisionMesh {
        let n = (heights.len() as f64).sqrt() as usize;
        assert!(
            n >= 2 && n * n == heights.len(),
            "Physics3D::heightfield: {} heights isn't a square grid of at least 2 x 2",
            heights.len()
        );
        assert!(
            size > 0.0 && size.is_finite(),
            "Physics3D::heightfield: size must be above 0 (got {size})"
        );
        // rapier's rows run along Z and its storage is by column: element
        // (z, x) sits at z + x * n, where ours is at z * n + x.
        let mut data = vec![0.0; n * n];
        for z in 0..n {
            for x in 0..n {
                let h = heights[z * n + x];
                data[z + x * n] = if h.is_finite() { h } else { 0.0 };
            }
        }
        let grid = rapier3d::parry::utils::Array2::new(n, n, data);
        let mut field = rapier3d::parry::shape::HeightField::with_flags(
            grid,
            rp::Vector::new(size, 1.0, size),
            rapier3d::parry::shape::HeightFieldFlags::FIX_INTERNAL_EDGES,
        );
        // Cells with a hole in any corner aren't drawn, so they aren't solid.
        for z in 0..n - 1 {
            for x in 0..n - 1 {
                let corners = [(x, z), (x + 1, z), (x, z + 1), (x + 1, z + 1)];
                if corners.iter().any(|&(x, z)| !heights[z * n + x].is_finite()) {
                    field.set_cell_status(z, x, rapier3d::parry::shape::HeightFieldCellStatus::CELL_REMOVED);
                }
            }
        }
        let half = size * 0.5;
        self.add_mesh(rp::SharedShape::new(field), Vec3::new(half, 0.0, half))
    }

    /// Lets go of a mesh made here. Colliders already using it keep it
    /// until they're removed; new colliders with this handle get no shape.
    pub fn free_mesh(&mut self, mesh: CollisionMesh) {
        if let Some(slot) = self.meshes.get_mut(mesh.index as usize)
            && slot.generation == mesh.generation
            && slot.shape.take().is_some()
        {
            slot.generation = slot.generation.wrapping_add(1);
            self.free_meshes.push(mesh.index);
        }
    }

    /// Whether `mesh` is a live mesh of this world.
    pub fn is_mesh_live(&self, mesh: CollisionMesh) -> bool {
        self.mesh(mesh).is_some()
    }

    fn mesh(&self, mesh: CollisionMesh) -> Option<&MeshSlot> {
        self.meshes
            .get(mesh.index as usize)
            .filter(|s| s.generation == mesh.generation && s.shape.is_some())
    }

    fn add_mesh(&mut self, shape: rp::SharedShape, offset: Vec3) -> CollisionMesh {
        if let Some(index) = self.free_meshes.pop() {
            let slot = &mut self.meshes[index as usize];
            slot.shape = Some(shape);
            slot.offset = offset;
            return CollisionMesh {
                index,
                generation: slot.generation,
            };
        }
        self.meshes.push(MeshSlot {
            shape: Some(shape),
            offset,
            generation: 0,
        });
        CollisionMesh {
            index: (self.meshes.len() - 1) as u32,
            generation: 0,
        }
    }

    /// A rapier collider for `c` (a speck touching nothing for a freed mesh).
    fn collider_of(&self, c: &Collider3D, e: Entity) -> rp::Collider {
        let tiny = f32::EPSILON;
        let (shape, offset) = match c.shape {
            Shape3D::Sphere { radius } => (rp::SharedShape::ball(radius.max(tiny)), Vec3::ZERO),
            Shape3D::Cuboid { size } => {
                let h = (size * 0.5).max(Vec3::splat(tiny));
                (rp::SharedShape::cuboid(h.x, h.y, h.z), Vec3::ZERO)
            }
            Shape3D::Capsule { height, radius } => (
                rp::SharedShape::capsule_y((height * 0.5).max(0.0), radius.max(tiny)),
                Vec3::ZERO,
            ),
            Shape3D::Mesh(m) => match self.mesh(m) {
                Some(slot) => (slot.shape.clone().expect("live slot"), slot.offset),
                // A freed mesh: a speck that touches nothing.
                None => (rp::SharedShape::ball(tiny), Vec3::ZERO),
            },
        };
        let mut types = rp::ActiveCollisionTypes::default();
        if c.sensor {
            // Characters are kinematic: let them set off fixed triggers.
            types |= rp::ActiveCollisionTypes::KINEMATIC_FIXED | rp::ActiveCollisionTypes::KINEMATIC_KINEMATIC;
        }
        let groups = match c.shape {
            Shape3D::Mesh(m) if self.mesh(m).is_none() => CollisionGroups::new(0, 0),
            _ => c.groups,
        };
        rp::ColliderBuilder::new(shape)
            .density(c.density)
            .friction(c.friction)
            .restitution(c.restitution)
            .sensor(c.sensor)
            .translation(to_rp(c.offset + offset))
            .collision_groups(groups.to_rp())
            .active_collision_types(types)
            .active_events(rp::ActiveEvents::COLLISION_EVENTS)
            .user_data(u128::from(e.to_bits()))
            .build()
    }

    fn insert_link(&mut self, e: Entity, t: &Transform3D, kind: LinkKind, velocity: Velocity3D) {
        let body = match kind {
            LinkKind::Body(rigid, shape) => {
                let body = self.world.insert_body(body_of(t, rigid, velocity));
                let mut collider = self.collider_of(&shape, e);
                set_mass(&mut collider, rigid);
                self.world.insert_collider(collider, Some(body));
                body
            }
            LinkKind::Character(capsule) => {
                let body = self.world.insert_body(
                    rp::RigidBodyBuilder::kinematic_position_based()
                        .pose(pose_to_rp(t.position, Quat::IDENTITY))
                        .build(),
                );
                self.world.insert_collider(capsule.collider(e), Some(body));
                body
            }
        };
        let i = e.index() as usize;
        if self.links.len() <= i {
            self.links.resize(i + 1, None);
        }
        let pose = Pose::of(t);
        self.links[i] = Some(Link {
            entity: e,
            body,
            kind,
            velocity,
            written: *t,
            prev: pose,
            curr: pose,
            seen: self.step,
        });
    }

    fn remove(&mut self, i: usize) {
        if let Some(link) = self.links[i].take() {
            self.world.remove_body(link.body);
        }
    }
}

/// The body's own mass and center of mass, if it has them, on its collider
/// (a collider's mass properties are in its own frame, shifted by its offset).
fn set_mass(collider: &mut rp::Collider, rigid: RigidBody3D) {
    let mass = rigid.mass.filter(|m| *m > 0.0 && m.is_finite());
    let center = rigid.center_of_mass.filter(|c| c.is_finite());
    if mass.is_none() && center.is_none() {
        return;
    }
    let mut props = collider.mass_properties();
    if let Some(mass) = mass {
        if props.mass() > 0.0 {
            props.set_mass(mass, true);
        } else {
            props = rp::MassProperties::new(props.local_com, mass, rp::Vector::splat(mass * 0.1));
        }
    }
    if let Some(center) = center {
        props.local_com = to_rp(center) - collider.position().translation;
    }
    collider.set_mass_properties(props);
}

fn body_of(t: &Transform3D, rigid: RigidBody3D, velocity: Velocity3D) -> rp::RigidBody {
    let builder = match rigid.kind {
        BodyKind::Fixed => rp::RigidBodyBuilder::fixed(),
        BodyKind::Kinematic => rp::RigidBodyBuilder::kinematic_velocity_based(),
        _ => rp::RigidBodyBuilder::dynamic(),
    };
    let mut builder = builder
        .pose(pose_to_rp(t.position, t.rotation))
        .linvel(to_rp(velocity.linear))
        .angvel(to_rp(velocity.angular))
        .gravity_scale(rigid.gravity_scale)
        .linear_damping(rigid.linear_damping)
        .angular_damping(rigid.angular_damping)
        .ccd_enabled(rigid.ccd);
    if rigid.lock_rotation {
        builder = builder.lock_rotations();
    }
    builder.build()
}

/// Sets up `ctx.physics3d` for a game that uses 3D physics, and makes its
/// components visible to the inspector, saves and agents.
pub(crate) fn start(ctx: &mut Ctx, gravity: Vec3) {
    use crate::WorldReflect;
    ctx.physics3d = Physics3D::new(gravity);
    ctx.world.reflect::<RigidBody3D>();
    crate::reflect::reflect_collider(&mut ctx.world);
    ctx.world.reflect::<Velocity3D>();
    ctx.world.reflect::<Character3D>();
    ctx.world.reflect::<Vehicle3D>();
}

/// Steps the 3D physics world one simulation tick: picks up new, changed
/// and removed bodies, moves characters, simulates, writes poses and
/// velocities back, and sends [`CollisionStarted`] / [`CollisionStopped`]
/// events. Added by [`Game::physics3d`](crate::Game::physics3d); remove or
/// reorder it like any other system.
pub fn step_physics3d(ctx: &mut Ctx) {
    let dt = ctx.dt();
    let physics = &mut ctx.physics3d;
    physics.step += 1;
    sync_into(physics, &mut ctx.world);
    physics.world.gravity = to_rp(physics.gravity);
    physics.world.integration_parameters.dt = dt;
    character::move_characters(physics, &mut ctx.world, dt);
    vehicle::step_vehicles(physics, &mut ctx.world, dt);
    physics.world.step_with_events(&(), &physics.collector);
    write_back(physics, &mut ctx.world);
    std::mem::swap(
        &mut physics.events,
        &mut *physics
            .collector
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    for i in 0..physics.events.len() {
        let (a, b, flags, started) = match physics.events[i] {
            rp::CollisionEvent::Started(a, b, f) => (a, b, f, true),
            rp::CollisionEvent::Stopped(a, b, f) => (a, b, f, false),
        };
        let (Some(a), Some(b)) = (physics.entity_of(a), physics.entity_of(b)) else {
            continue;
        };
        let sensor = flags.contains(rp::CollisionEventFlags::SENSOR);
        if started {
            ctx.events.send(CollisionStarted { a, b, sensor });
        } else {
            ctx.events.send(CollisionStopped { a, b, sensor });
        }
    }
    physics.events.clear();
}

/// Draws moving bodies smoothly: between two physics steps, sets each
/// moving body's [`Transform3D`] to a blend of its last two poses, by how
/// far the frame is between them ([`Ctx::fixed_alpha`]). Added as the first
/// frame system by [`Game::physics3d`](crate::Game::physics3d), so camera
/// code sees the blended pose too. Does nothing in headless runs, where
/// frames and ticks line up. Transforms you changed since the last step are
/// left alone (they teleport the body at the next step).
pub fn interpolate_physics3d(ctx: &mut Ctx) {
    if ctx.headless {
        return;
    }
    let alpha = ctx.fixed_alpha().clamp(0.0, 1.0);
    let physics = &mut ctx.physics3d;
    if physics.links.is_empty() {
        return;
    }
    ctx.world.query::<(Entity, &mut Transform3D)>().for_each(|(e, t)| {
        let Some(link) = physics
            .links
            .get_mut(e.index() as usize)
            .and_then(Option::as_mut)
            .filter(|l| l.entity == e && l.kind.moves())
        else {
            return;
        };
        if !written_matches(link, t) {
            return;
        }
        t.position = link.prev.position.lerp(link.curr.position, alpha);
        if let LinkKind::Body(..) = link.kind {
            t.rotation = link.prev.rotation.slerp(link.curr.rotation, alpha);
        }
        link.written = *t;
    });
}

/// Whether `t` is what physics last wrote (or saw): characters only own
/// their position, so turning one isn't a teleport.
fn written_matches(link: &Link, t: &Transform3D) -> bool {
    match link.kind {
        LinkKind::Character(_) => link.written.position == t.position,
        LinkKind::Body(..) => link.written == *t,
    }
}

/// New bodies in, your changes applied, gone ones out.
fn sync_into(physics: &mut Physics3D, world: &mut World) {
    let step = physics.step;
    world
        .query::<(Entity, &Transform3D, &RigidBody3D, &Collider3D, Option<&Velocity3D>)>()
        .without::<Character3D>()
        .for_each(|(e, t, rigid, shape, velocity)| {
            let kind = LinkKind::Body(*rigid, *shape);
            sync_one(physics, e, t, kind, velocity.copied(), step);
        });
    world
        .query::<(Entity, &Transform3D, &Character3D)>()
        .for_each(|(e, t, c)| {
            let kind = LinkKind::Character(character::Capsule::of(c));
            sync_one(physics, e, t, kind, None, step);
        });
    // Despawned entities and ones that lost a physics component, in index
    // order so runs stay deterministic.
    for i in 0..physics.links.len() {
        if physics.links[i].is_some_and(|l| l.seen != step) {
            physics.remove(i);
        }
    }
}

fn sync_one(
    physics: &mut Physics3D,
    e: Entity,
    t: &Transform3D,
    kind: LinkKind,
    velocity: Option<Velocity3D>,
    step: u64,
) {
    let i = e.index() as usize;
    let Some(link) = physics
        .links
        .get_mut(i)
        .and_then(Option::as_mut)
        .filter(|l| l.entity == e)
    else {
        if let Some(Some(old)) = physics.links.get(i).copied() {
            // A recycled index: the old entity's body goes first.
            physics.world.remove_body(old.body);
            physics.links[i] = None;
        }
        physics.insert_link(e, t, kind, velocity.unwrap_or_default());
        return;
    };
    link.seen = step;
    if link.kind != kind {
        // Rare (a door that becomes dynamic, a resized collider): rebuild.
        let current = link.velocity;
        physics.world.remove_body(link.body);
        physics.insert_link(e, t, kind, velocity.unwrap_or(current));
        return;
    }
    // `bodies.get_mut` marks a body as changed (rapier then redoes its
    // broad phase and wakes it), so only touch bodies you changed.
    if !written_matches(link, t)
        && let Some(body) = physics.world.bodies.get_mut(link.body)
    {
        let rotation = match kind {
            LinkKind::Body(..) => t.rotation,
            LinkKind::Character(_) => Quat::IDENTITY,
        };
        body.set_position(pose_to_rp(t.position, rotation), true);
        link.written = *t;
        // A teleport isn't blended.
        link.curr = Pose::of(t);
        link.prev = link.curr;
    }
    if let Some(v) = velocity
        && link.velocity != v
        && let Some(body) = physics.world.bodies.get_mut(link.body)
    {
        body.set_linvel(to_rp(v.linear), true);
        body.set_angvel(to_rp(v.angular), true);
        link.velocity = v;
    }
}

fn write_back(physics: &mut Physics3D, world: &mut World) {
    world
        .query::<(Entity, &mut Transform3D, Option<&mut Velocity3D>)>()
        .for_each(|(e, t, velocity)| {
            let Some(link) = physics
                .links
                .get_mut(e.index() as usize)
                .and_then(Option::as_mut)
                .filter(|l| l.entity == e && l.kind.moves())
            else {
                return;
            };
            link.prev = link.curr;
            let Some(body) = physics.world.bodies.get(link.body) else {
                return;
            };
            // A sleeping body hasn't moved since it fell asleep: it only
            // needs its speed cleared, once.
            if body.is_sleeping() {
                if let LinkKind::Body(..) = link.kind
                    && link.velocity != Velocity3D::default()
                {
                    link.velocity = Velocity3D::default();
                    if let Some(velocity) = velocity {
                        *velocity = link.velocity;
                    }
                }
                return;
            }
            link.curr.position = from_rp(body.translation());
            t.position = link.curr.position;
            if let LinkKind::Body(..) = link.kind {
                link.curr.rotation = rot_from_rp(*body.rotation());
                t.rotation = link.curr.rotation;
                let v = Velocity3D {
                    linear: from_rp(body.linvel()),
                    angular: from_rp(body.angvel()),
                };
                link.velocity = v;
                if let Some(velocity) = velocity {
                    *velocity = v;
                }
            }
            link.written = *t;
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::FrameTiming;
    use crate::{Vec2, vec3};

    fn tick(ctx: &mut Ctx) {
        ctx.set_in_sim(true);
        step_physics3d(ctx);
        ctx.set_in_sim(false);
    }

    fn frame(ctx: &mut Ctx, alpha: f32) {
        ctx.begin_frame(FrameTiming {
            dt: 0.01,
            fixed_steps: 0,
            alpha,
        });
        interpolate_physics3d(ctx);
    }

    #[test]
    fn frames_between_steps_draw_a_blend_and_steps_continue_from_the_physics_pose() {
        let mut ctx = Ctx::new(0.02, Vec2::ZERO);
        start(&mut ctx, Vec3::ZERO);
        let ball = ctx.world.spawn((
            Transform3D::default(),
            RigidBody3D::dynamic(),
            Collider3D::sphere(0.5),
            Velocity3D::linear(vec3(10.0, 0.0, 0.0)),
        ));
        tick(&mut ctx);
        tick(&mut ctx);
        let x = |ctx: &mut Ctx| ctx.world.get::<Transform3D>(ball).unwrap().position.x;
        assert!((x(&mut ctx) - 0.4).abs() < 1e-4, "two steps of 0.2 m");
        frame(&mut ctx, 0.5);
        assert!(
            (x(&mut ctx) - 0.3).abs() < 1e-4,
            "halfway between the last two: {}",
            x(&mut ctx)
        );
        frame(&mut ctx, 0.75);
        assert!((x(&mut ctx) - 0.35).abs() < 1e-4);
        // The blend isn't a teleport: the next step goes on from 0.4.
        tick(&mut ctx);
        assert!((x(&mut ctx) - 0.6).abs() < 1e-4, "{}", x(&mut ctx));
        // But your own change between steps is.
        ctx.world.get_mut::<Transform3D>(ball).unwrap().position = vec3(-5.0, 0.0, 0.0);
        frame(&mut ctx, 0.5);
        assert_eq!(x(&mut ctx), -5.0, "left alone by the blend");
        tick(&mut ctx);
        assert!((x(&mut ctx) + 4.8).abs() < 1e-4, "{}", x(&mut ctx));
    }

    #[test]
    fn headless_runs_draw_the_stepped_pose() {
        let mut ctx = Ctx::new(0.02, Vec2::ZERO);
        ctx.headless = true;
        start(&mut ctx, Vec3::ZERO);
        let ball = ctx.world.spawn((
            Transform3D::default(),
            RigidBody3D::dynamic(),
            Collider3D::sphere(0.5),
            Velocity3D::linear(vec3(10.0, 0.0, 0.0)),
        ));
        tick(&mut ctx);
        frame(&mut ctx, 0.5);
        assert!((ctx.world.get::<Transform3D>(ball).unwrap().position.x - 0.2).abs() < 1e-4);
    }
}
