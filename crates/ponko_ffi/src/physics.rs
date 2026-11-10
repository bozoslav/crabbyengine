//! 2D physics from C (the `physics` feature, on by default): bodies and
//! colliders as components, a game setting, ray and overlap queries, impulses
//! and collision events. Without the feature every call here fails with an
//! error naming it.
use crate::types::{CrabbyEntity, CrabbyVec2};
#[cfg(feature = "physics")]
use crate::{CMirror, Kind, Mirror, invalid, mirror};
use crate::{CrabbyGame, CrabbyStatus, CrabbyWorld, Failure};

/// How a body moves (`CrabbyRigidBody::kind`).
/// ```
/// assert_eq!(ponko_ffi::CrabbyBodyKind::CRABBY_BODY_FIXED as u32, 1);
/// ```
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyBodyKind {
    /// Moved by gravity, forces and collisions.
    CRABBY_BODY_DYNAMIC,
    /// Never moves (walls, ground).
    CRABBY_BODY_FIXED,
    /// Moved only by you, through its transform or velocity.
    CRABBY_BODY_KINEMATIC,
}

/// A collider's shape (`CrabbyCollider::shape`).
/// ```
/// assert_eq!(ponko_ffi::CrabbyShapeKind::CRABBY_SHAPE_RECT as u32, 1);
/// ```
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyShapeKind {
    /// Uses `radius`.
    CRABBY_SHAPE_CIRCLE,
    /// Uses `size` (width, height).
    CRABBY_SHAPE_RECT,
    /// A vertical capsule: `height` between the centers of its round ends, and `radius`.
    CRABBY_SHAPE_CAPSULE,
    /// A convex polygon, set with `crabby_set_polygon_collider`; its corners
    /// come from `crabby_collider_points`. Reading one gives its bounding
    /// box size in `size`.
    CRABBY_SHAPE_POLYGON,
}

/// The built-in `RigidBody` component: makes an entity a physics body (it
/// also needs a `CrabbyCollider` and a `CrabbyTransform2D`). Start from
/// `crabby_rigid_body(kind)`. Checked on spawn/set: an unknown kind or
/// negative damping is `CRABBY_INVALID_ARGUMENT`. Not a query column.
/// ```
/// let b = ponko_ffi::crabby_rigid_body(ponko_ffi::CrabbyBodyKind::CRABBY_BODY_DYNAMIC as u32);
/// assert_eq!(b.gravity_scale, 1.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyRigidBody {
    /// A `CrabbyBodyKind`.
    pub kind: u32,
    /// 1 normal, 0 floats, negative falls up.
    pub gravity_scale: f32,
    /// Slows movement down over time (air drag); 0 none.
    pub linear_damping: f32,
    /// Slows spinning down over time; 0 none.
    pub angular_damping: f32,
    /// Never rotates (characters).
    pub lock_rotation: bool,
    /// Continuous collision detection, so fast bodies don't pass through thin walls.
    pub ccd: bool,
}

/// The built-in `Collider` component: the solid shape of a body, in pixels,
/// centered on the entity plus `offset`. Start from `crabby_collider_circle`,
/// `_rect` or `_capsule`. Checked on spawn/set: sizes must be positive,
/// density, friction and restitution not negative. Not a query column.
/// ```
/// let c = ponko_ffi::crabby_collider_circle(8.0);
/// assert_eq!((c.radius, c.density, c.friction), (8.0, 1.0, 0.5));
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyCollider {
    /// A `CrabbyShapeKind`.
    pub shape: u32,
    /// Circle and capsule radius.
    pub radius: f32,
    /// Capsule height between the centers of its ends.
    pub height: f32,
    /// Rectangle width and height.
    pub size: CrabbyVec2,
    /// Where the shape sits relative to the entity's position.
    pub offset: CrabbyVec2,
    /// Mass per square pixel (mass = density × area).
    pub density: f32,
    /// 0 ice, 1 rubber.
    pub friction: f32,
    /// Bounciness: 0 none, 1 keeps all its speed.
    pub restitution: f32,
    /// Detects overlaps (collision events) but doesn't push.
    pub sensor: bool,
}

/// The built-in `Velocity` component, in pixels and radians per second:
/// read it after a step, write it to set the speed. A plain value (same
/// layout as Rust's), so it works as a query column.
/// ```
/// assert_eq!(std::mem::size_of::<ponko_ffi::CrabbyVelocity>(), 12);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CrabbyVelocity {
    /// Pixels per second.
    pub linear: CrabbyVec2,
    /// Radians per second.
    pub angular: f32,
}

/// What a ray hit (`crabby_ctx_cast_ray`).
/// ```
/// let h = ponko_ffi::CrabbyRayHit::default();
/// assert_eq!(h.distance, 0.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CrabbyRayHit {
    /// Whose collider.
    pub entity: CrabbyEntity,
    /// Where.
    pub point: CrabbyVec2,
    /// The surface's outward direction at `point`.
    pub normal: CrabbyVec2,
    /// From the ray's start to `point`.
    pub distance: f32,
}

/// Two colliders that started or stopped touching (`crabby_ctx_collisions`).
/// ```
/// assert_eq!(std::mem::size_of::<ponko_ffi::CrabbyCollision>(), 20);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrabbyCollision {
    /// One entity.
    pub a: CrabbyEntity,
    /// The other.
    pub b: CrabbyEntity,
    /// One of them is a sensor (an overlap, not a contact).
    pub sensor: bool,
}

/// A body of this kind with default settings (gravity scale 1, no damping).
/// Unknown kinds give a dynamic body.
/// ```
/// let b = ponko_ffi::crabby_rigid_body(ponko_ffi::CrabbyBodyKind::CRABBY_BODY_FIXED as u32);
/// assert_eq!(b.kind, 1);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_rigid_body(kind: u32) -> CrabbyRigidBody {
    CrabbyRigidBody {
        kind: if kind <= CrabbyBodyKind::CRABBY_BODY_KINEMATIC as u32 {
            kind
        } else {
            0
        },
        gravity_scale: 1.0,
        linear_damping: 0.0,
        angular_damping: 0.0,
        lock_rotation: false,
        ccd: false,
    }
}

fn collider(shape: CrabbyShapeKind) -> CrabbyCollider {
    CrabbyCollider {
        shape: shape as u32,
        radius: 0.0,
        height: 0.0,
        size: CrabbyVec2::default(),
        offset: CrabbyVec2::default(),
        density: 1.0,
        friction: 0.5,
        restitution: 0.0,
        sensor: false,
    }
}

/// A circle collider (density 1, friction 0.5, no bounce).
/// ```
/// assert_eq!(ponko_ffi::crabby_collider_circle(4.0).shape, 0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_collider_circle(radius: f32) -> CrabbyCollider {
    CrabbyCollider {
        radius,
        ..collider(CrabbyShapeKind::CRABBY_SHAPE_CIRCLE)
    }
}

/// A rectangle collider `size` wide and high.
/// ```
/// let c = ponko_ffi::crabby_collider_rect(ponko_ffi::CrabbyVec2 { x: 4.0, y: 2.0 });
/// assert_eq!(c.size.y, 2.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_collider_rect(size: CrabbyVec2) -> CrabbyCollider {
    CrabbyCollider {
        size,
        ..collider(CrabbyShapeKind::CRABBY_SHAPE_RECT)
    }
}

/// A vertical capsule collider (good for characters).
/// ```
/// assert_eq!(ponko_ffi::crabby_collider_capsule(20.0, 5.0).height, 20.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_collider_capsule(height: f32, radius: f32) -> CrabbyCollider {
    CrabbyCollider {
        height,
        radius,
        ..collider(CrabbyShapeKind::CRABBY_SHAPE_CAPSULE)
    }
}

/// Gives `entity` a convex polygon collider: the convex hull of `count`
/// corners (3 to 8) at `points`, around the entity's position. `settings`
/// gives density, friction, restitution, sensor and offset (its shape is
/// ignored), e.g. `crabby_collider_circle(1)` for the defaults.
/// # Safety
/// Live world; `points` readable for `count` values.
/// ```
/// use ponko_ffi::*;
/// let tri = [CrabbyVec2 { x: 0.0, y: 0.0 }, CrabbyVec2 { x: 10.0, y: 0.0 }, CrabbyVec2 { x: 0.0, y: 10.0 }];
/// let status = unsafe { crabby_set_polygon_collider(std::ptr::null_mut(), 0, tri.as_ptr(), 3, crabby_collider_circle(1.0)) };
/// assert_eq!(status, CrabbyStatus::CRABBY_INVALID_ARGUMENT); // no world
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_set_polygon_collider(
    world: *mut CrabbyWorld,
    entity: u64,
    points: *const CrabbyVec2,
    count: u32,
    settings: CrabbyCollider,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::set_polygon(world, entity, points, count, settings) }
}

/// `entity`'s polygon corners: writes up to 8 into `out_points` and how
/// many into `*out_count` (0 if its collider isn't a polygon).
/// # Safety
/// Live world; `out_points` writable for 8 values; writable `out_count`.
/// ```
/// use ponko_ffi::*;
/// let mut points = [CrabbyVec2 { x: 0.0, y: 0.0 }; 8];
/// let mut n = 0;
/// let status = unsafe { crabby_collider_points(std::ptr::null_mut(), 0, points.as_mut_ptr(), &mut n) };
/// assert_eq!(status, CrabbyStatus::CRABBY_INVALID_ARGUMENT); // no world
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_collider_points(
    world: *mut CrabbyWorld,
    entity: u64,
    out_points: *mut CrabbyVec2,
    out_count: *mut u32,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::polygon_points(world, entity, out_points, out_count) }
}

/// The id of the built-in `RigidBody` component (`CrabbyRigidBody` in
/// spawn/get/set), registering it if needed.
/// # Safety
/// Live handle and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 99; assert_eq!(crabby_rigid_body_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_rigid_body_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::component(world, out_id, 0) }
}

/// The id of the built-in `Collider` component (`CrabbyCollider` in
/// spawn/get/set), registering it if needed.
/// # Safety
/// Live handle and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 99; assert_eq!(crabby_collider_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_collider_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::component(world, out_id, 1) }
}

/// The id of the built-in `Velocity` component (`CrabbyVelocity`, also as a
/// query column), registering it if needed.
/// # Safety
/// Live handle and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 99; assert_eq!(crabby_velocity_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_velocity_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::component(world, out_id, 2) }
}

/// Turns on 2D physics, pulling dynamic bodies by `gravity` (pixels/s²; y
/// is down). Adds the physics step as a simulation system at this point in
/// the order: add it after the systems that move things. Then entities with
/// a rigid body, a collider and a transform are simulated.
/// # Safety
/// `game` must be null or a live game.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_game_physics(std::ptr::null_mut(), Default::default()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_physics(game: *mut CrabbyGame, gravity: CrabbyVec2) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::game_physics(game, gravity) }
}

/// Changes gravity for every dynamic body.
/// # Safety
/// Live ctx, outside query callbacks.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_gravity(std::ptr::null_mut(), Default::default()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_gravity(ctx: *mut crate::CrabbyCtx, gravity: CrabbyVec2) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::set_gravity(ctx, gravity) }
}

/// The first collider a ray from `origin` towards `direction` hits within
/// `max_distance`, as of the last physics step. `*out_found` says whether
/// anything was hit; `*out_hit` is written only then.
/// # Safety
/// Live ctx, outside query callbacks; writable outputs.
/// ```
/// use ponko_ffi::*;
/// let (mut found, mut hit) = (false, CrabbyRayHit::default());
/// let s = unsafe { crabby_ctx_cast_ray(std::ptr::null_mut(), Default::default(), Default::default(), 1.0, &mut found, &mut hit) };
/// assert_eq!(s, CrabbyStatus::CRABBY_INVALID_ARGUMENT);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_cast_ray(
    ctx: *mut crate::CrabbyCtx,
    origin: CrabbyVec2,
    direction: CrabbyVec2,
    max_distance: f32,
    out_found: *mut bool,
    out_hit: *mut CrabbyRayHit,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::cast_ray(ctx, origin, direction, max_distance, out_found, out_hit) }
}

/// Entities whose collider contains `point`. Writes up to `capacity` of
/// them and sets `*out_count` to how many there are in total, so a
/// too-small buffer can be grown and the call repeated.
/// # Safety
/// Live ctx, outside query callbacks; `out_entities` writable for `capacity`
/// entities (may be null if 0); writable count.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_physics_at(std::ptr::null_mut(), Default::default(), std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_physics_at(
    ctx: *mut crate::CrabbyCtx,
    point: CrabbyVec2,
    out_entities: *mut CrabbyEntity,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::overlaps(ctx, point, None, out_entities, capacity, out_count) }
}

/// Like `crabby_ctx_physics_at`, for colliders overlapping a circle.
/// # Safety
/// As `crabby_ctx_physics_at`.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_physics_in_circle(std::ptr::null_mut(), Default::default(), 1.0, std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_physics_in_circle(
    ctx: *mut crate::CrabbyCtx,
    center: CrabbyVec2,
    radius: f32,
    out_entities: *mut CrabbyEntity,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::overlaps(ctx, center, Some(radius), out_entities, capacity, out_count) }
}

/// Pushes a dynamic body once (an explosion, a kick), in mass × pixels/s.
/// `CRABBY_NOT_FOUND` if `entity` isn't a physics body (yet: bodies join at
/// the next physics step).
/// # Safety
/// Live ctx, outside query callbacks.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_apply_impulse(std::ptr::null_mut(), 0, Default::default()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_apply_impulse(
    ctx: *mut crate::CrabbyCtx,
    entity: u64,
    impulse: CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::apply_impulse(ctx, entity, impulse) }
}

/// The mass of a body (density × area). `CRABBY_NOT_FOUND` if `entity`
/// isn't a physics body.
/// # Safety
/// Live ctx, outside query callbacks; writable output.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_body_mass(std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_body_mass(
    ctx: *mut crate::CrabbyCtx,
    entity: u64,
    out_mass: *mut f32,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::mass(ctx, entity, out_mass) }
}

/// Collisions this system hasn't seen yet, oldest first: `started` true for
/// pairs that started touching, false for pairs that stopped. `*out_count`
/// values at `*out_data`, valid until the system returns. Each system that
/// runs every tick sees each collision once.
/// # Safety
/// Live ctx; writable outputs.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_collisions(std::ptr::null_mut(), true, std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_collisions(
    ctx: *mut crate::CrabbyCtx,
    started: bool,
    out_data: *mut *const CrabbyCollision,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { imp::collisions(ctx, started, out_data, out_count) }
}

#[cfg(feature = "physics")]
mod imp {
    use std::cell::RefCell;

    use ponko::d2::Collider;
    use ponko::d2::RigidBody;
    use ponko::d2::Shape;
    use ponko::d2::Velocity;
    use ponko::{BodyKind, CollisionStarted, CollisionStopped, Vec2};
    use ponko_ecs::Entity;

    use super::*;
    use crate::ctx::{collect, ctx_access, events, out};
    use crate::game::{build, ok};
    use crate::types::to_c;
    use crate::{CrabbyCtx, boundary, builtin, error};

    const _: () = {
        assert!(size_of::<CrabbyVelocity>() == size_of::<Velocity>());
        assert!(align_of::<CrabbyVelocity>() == align_of::<Velocity>());
        assert!(std::mem::offset_of!(CrabbyVelocity, linear) == std::mem::offset_of!(Velocity, linear));
        assert!(std::mem::offset_of!(CrabbyVelocity, angular) == std::mem::offset_of!(Velocity, angular));
        // Collision events are handed to C in place.
        assert!(size_of::<CrabbyCollision>() == size_of::<CollisionStarted>());
        assert!(align_of::<CrabbyCollision>() == align_of::<CollisionStarted>());
        assert!(std::mem::offset_of!(CrabbyCollision, a) == std::mem::offset_of!(CollisionStarted, a));
        assert!(std::mem::offset_of!(CrabbyCollision, b) == std::mem::offset_of!(CollisionStarted, b));
        assert!(std::mem::offset_of!(CrabbyCollision, sensor) == std::mem::offset_of!(CollisionStarted, sensor));
        assert!(size_of::<CrabbyCollision>() == size_of::<CollisionStopped>());
        assert!(align_of::<CrabbyCollision>() == align_of::<CollisionStopped>());
        assert!(std::mem::offset_of!(CrabbyCollision, a) == std::mem::offset_of!(CollisionStopped, a));
        assert!(std::mem::offset_of!(CrabbyCollision, b) == std::mem::offset_of!(CollisionStopped, b));
        assert!(std::mem::offset_of!(CrabbyCollision, sensor) == std::mem::offset_of!(CollisionStopped, sensor));
    };

    fn non_negative(x: f32, what: &'static str) -> Result<f32, Failure> {
        if x >= 0.0 && x.is_finite() {
            Ok(x)
        } else {
            Err(invalid(what))
        }
    }
    fn positive(x: f32, what: &'static str) -> Result<f32, Failure> {
        if x > 0.0 && x.is_finite() {
            Ok(x)
        } else {
            Err(invalid(what))
        }
    }
    fn finite(v: CrabbyVec2, what: &'static str) -> Result<Vec2, Failure> {
        if v.x.is_finite() && v.y.is_finite() {
            Ok(to_c(v))
        } else {
            Err(invalid(what))
        }
    }

    impl CMirror for RigidBody {
        type C = CrabbyRigidBody;
        fn from_c(c: &CrabbyRigidBody) -> Result<Self, Failure> {
            let kind = match c.kind {
                0 => BodyKind::Dynamic,
                1 => BodyKind::Fixed,
                2 => BodyKind::Kinematic,
                _ => return Err(invalid("unknown rigid body kind")),
            };
            if !c.gravity_scale.is_finite() {
                return Err(invalid("gravity scale must be finite"));
            }
            Ok(RigidBody {
                kind,
                gravity_scale: c.gravity_scale,
                linear_damping: non_negative(c.linear_damping, "damping must not be negative")?,
                angular_damping: non_negative(c.angular_damping, "damping must not be negative")?,
                lock_rotation: c.lock_rotation,
                ccd: c.ccd,
            })
        }
        fn to_c(&self) -> CrabbyRigidBody {
            CrabbyRigidBody {
                kind: match self.kind {
                    BodyKind::Dynamic => 0,
                    BodyKind::Fixed => 1,
                    BodyKind::Kinematic => 2,
                    _ => unreachable!("a new BodyKind needs a CrabbyBodyKind"),
                },
                gravity_scale: self.gravity_scale,
                linear_damping: self.linear_damping,
                angular_damping: self.angular_damping,
                lock_rotation: self.lock_rotation,
                ccd: self.ccd,
            }
        }
    }

    impl CMirror for Collider {
        type C = CrabbyCollider;
        fn from_c(c: &CrabbyCollider) -> Result<Self, Failure> {
            const SIZE: &str = "collider sizes must be positive";
            let shape = match c.shape {
                0 => Shape::Circle {
                    radius: positive(c.radius, SIZE)?,
                },
                1 => Shape::Rect {
                    size: Vec2::new(positive(c.size.x, SIZE)?, positive(c.size.y, SIZE)?),
                },
                2 => Shape::Capsule {
                    height: non_negative(c.height, SIZE)?,
                    radius: positive(c.radius, SIZE)?,
                },
                3 => return Err(invalid("polygon colliders are set with crabby_set_polygon_collider")),
                _ => return Err(invalid("unknown collider shape")),
            };
            const AMOUNT: &str = "density, friction and restitution must not be negative";
            Ok(Collider {
                shape,
                density: non_negative(c.density, AMOUNT)?,
                friction: non_negative(c.friction, AMOUNT)?,
                restitution: non_negative(c.restitution, AMOUNT)?,
                sensor: c.sensor,
                offset: finite(c.offset, "collider offset must be finite")?,
            })
        }
        fn to_c(&self) -> CrabbyCollider {
            let base = match self.shape {
                Shape::Circle { radius } => crabby_collider_circle(radius),
                Shape::Rect { size } => crabby_collider_rect(to_c(size)),
                Shape::Capsule { height, radius } => crabby_collider_capsule(height, radius),
                Shape::Polygon { .. } => {
                    let points = self.shape.points();
                    let lo = points.iter().fold(Vec2::splat(f32::INFINITY), |a, &p| a.min(p));
                    let hi = points.iter().fold(Vec2::splat(f32::NEG_INFINITY), |a, &p| a.max(p));
                    CrabbyCollider {
                        size: to_c(hi - lo),
                        ..collider(CrabbyShapeKind::CRABBY_SHAPE_POLYGON)
                    }
                }
                _ => unreachable!("a new Shape needs a CrabbyShapeKind"),
            };
            CrabbyCollider {
                density: self.density,
                friction: self.friction,
                restitution: self.restitution,
                sensor: self.sensor,
                offset: to_c(self.offset),
                ..base
            }
        }
    }

    pub(super) unsafe fn set_polygon(
        world: *mut CrabbyWorld,
        entity: u64,
        points: *const CrabbyVec2,
        count: u32,
        settings: CrabbyCollider,
    ) -> CrabbyStatus {
        boundary(|| {
            let count = count as usize;
            if !(3..=ponko::physics::MAX_POLYGON_POINTS).contains(&count) {
                return Err(invalid("a polygon has 3 to 8 points"));
            }
            if points.is_null() {
                return Err(invalid("points is null"));
            }
            // SAFETY: caller supplies `count` readable points.
            let corners: Vec<Vec2> = unsafe { std::slice::from_raw_parts(points, count) }
                .iter()
                .map(|p| Vec2::new(p.x, p.y))
                .collect();
            if !corners.iter().all(|p| p.is_finite()) {
                return Err(invalid("polygon points must be finite"));
            }
            // The other settings are checked like any collider's.
            let mut c = Collider::from_c(&CrabbyCollider {
                shape: CrabbyShapeKind::CRABBY_SHAPE_CIRCLE as u32,
                radius: 1.0,
                ..settings
            })?;
            c.shape = Collider::polygon(&corners).shape;
            // SAFETY: forwarded caller contract.
            unsafe {
                crate::access(world, |w, _| {
                    if w.insert(Entity::from_bits(entity), c) {
                        Ok(())
                    } else {
                        Err(error(CrabbyStatus::CRABBY_NOT_FOUND, "no such entity"))
                    }
                })
            }
        })
    }

    pub(super) unsafe fn polygon_points(
        world: *mut CrabbyWorld,
        entity: u64,
        out_points: *mut CrabbyVec2,
        out_count: *mut u32,
    ) -> CrabbyStatus {
        boundary(|| {
            if out_points.is_null() {
                return Err(invalid("output is null"));
            }
            // SAFETY: forwarded caller contract.
            unsafe {
                let corners = crate::access(world, |w, _| {
                    let c = w
                        .get::<Collider>(Entity::from_bits(entity))
                        .ok_or_else(|| error(CrabbyStatus::CRABBY_NOT_FOUND, "entity or collider not found"))?;
                    Ok(c.shape.points().to_vec())
                })?;
                for (i, p) in corners.iter().enumerate() {
                    out_points.add(i).write(to_c(*p));
                }
                out(out_count, corners.len() as u32)
            }
        })
    }

    static RIGID_BODY: Mirror = mirror::<RigidBody>();
    static COLLIDER: Mirror = mirror::<Collider>();

    pub(crate) fn kind(t: std::any::TypeId) -> Option<Kind> {
        use std::any::TypeId;
        if t == TypeId::of::<RigidBody>() {
            Some(Kind::Mirror(&RIGID_BODY))
        } else if t == TypeId::of::<Collider>() {
            Some(Kind::Mirror(&COLLIDER))
        } else if t == TypeId::of::<Velocity>() {
            // repr(C), layout checked against CrabbyVelocity; any bits are valid.
            Some(Kind::Plain)
        } else {
            None
        }
    }

    pub(super) unsafe fn component(world: *mut CrabbyWorld, out_id: *mut u32, which: u8) -> CrabbyStatus {
        // SAFETY: forwarded caller contract.
        unsafe {
            match which {
                0 => builtin::<RigidBody>(world, out_id),
                1 => builtin::<Collider>(world, out_id),
                _ => builtin::<Velocity>(world, out_id),
            }
        }
    }

    pub(super) unsafe fn game_physics(game: *mut CrabbyGame, gravity: CrabbyVec2) -> CrabbyStatus {
        // SAFETY: forwarded caller contract.
        unsafe {
            build(game, |g| match finite(gravity, "gravity must be finite") {
                Ok(gravity) => ok(g.physics(gravity)),
                Err(f) => (g, Err(f)),
            })
        }
    }

    pub(super) unsafe fn set_gravity(ctx: *mut CrabbyCtx, gravity: CrabbyVec2) -> CrabbyStatus {
        boundary(|| {
            let gravity = finite(gravity, "gravity must be finite")?;
            // SAFETY: forwarded caller contract.
            unsafe {
                ctx_access(ctx, |c| {
                    c.physics.set_gravity(gravity);
                    Ok(())
                })
            }
        })
    }

    pub(super) unsafe fn cast_ray(
        ctx: *mut CrabbyCtx,
        origin: CrabbyVec2,
        direction: CrabbyVec2,
        max_distance: f32,
        out_found: *mut bool,
        out_hit: *mut CrabbyRayHit,
    ) -> CrabbyStatus {
        boundary(|| {
            if out_found.is_null() || out_hit.is_null() {
                return Err(invalid("output is null"));
            }
            // SAFETY: forwarded caller contract.
            let hit = unsafe {
                ctx_access(ctx, |c| {
                    Ok(c.physics.cast_ray(to_c(origin), to_c(direction), max_distance))
                })?
            };
            // SAFETY: checked non-null; the caller supplies writable outputs.
            unsafe {
                if let Some(h) = hit {
                    out_hit.write(CrabbyRayHit {
                        entity: to_c(h.entity),
                        point: to_c(h.point),
                        normal: to_c(h.normal),
                        distance: h.distance,
                    });
                }
                out(out_found, hit.is_some())
            }
        })
    }

    thread_local! {
        /// Reused between overlap queries, so they don't allocate each time.
        static FOUND: RefCell<Vec<Entity>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) unsafe fn overlaps(
        ctx: *mut CrabbyCtx,
        at: CrabbyVec2,
        radius: Option<f32>,
        out_entities: *mut CrabbyEntity,
        capacity: usize,
        out_count: *mut usize,
    ) -> CrabbyStatus {
        boundary(|| {
            FOUND.with_borrow_mut(|found| {
                // SAFETY: forwarded caller contract.
                unsafe {
                    ctx_access(ctx, |c| {
                        match radius {
                            None => c.physics.entities_at(to_c(at), found),
                            Some(r) => c.physics.entities_in_circle(to_c(at), r, found),
                        }
                        Ok(())
                    })?;
                    let mut n = 0;
                    collect(out_entities, capacity, out_count, &mut n, |sink| {
                        for &e in found.iter() {
                            sink.push(to_c(e));
                        }
                    })
                }
            })
        })
    }

    pub(super) unsafe fn apply_impulse(ctx: *mut CrabbyCtx, entity: u64, impulse: CrabbyVec2) -> CrabbyStatus {
        boundary(|| {
            let impulse = finite(impulse, "impulse must be finite")?;
            // SAFETY: forwarded caller contract.
            let applied =
                unsafe { ctx_access(ctx, |c| Ok(c.physics.apply_impulse(Entity::from_bits(entity), impulse)))? };
            if applied {
                Ok(())
            } else {
                Err(error(CrabbyStatus::CRABBY_NOT_FOUND, "entity is not a physics body"))
            }
        })
    }

    pub(super) unsafe fn mass(ctx: *mut CrabbyCtx, entity: u64, out_mass: *mut f32) -> CrabbyStatus {
        boundary(|| {
            // SAFETY: forwarded caller contract.
            let mass = unsafe { ctx_access(ctx, |c| Ok(c.physics.mass(Entity::from_bits(entity))))? }
                .ok_or_else(|| error(CrabbyStatus::CRABBY_NOT_FOUND, "entity is not a physics body"))?;
            // SAFETY: forwarded caller contract.
            unsafe { out(out_mass, mass) }
        })
    }

    pub(super) unsafe fn collisions(
        ctx: *mut CrabbyCtx,
        started: bool,
        out_data: *mut *const CrabbyCollision,
        out_count: *mut usize,
    ) -> CrabbyStatus {
        boundary(|| {
            if out_data.is_null() || out_count.is_null() {
                return Err(invalid("output is null"));
            }
            // SAFETY: forwarded caller contract; both event types have
            // CrabbyCollision's layout (asserted above), and the slice lives
            // in the events until the next send of its type, which only the
            // physics step does, between systems.
            unsafe {
                let (ptr, len) = events(ctx, |ev| {
                    Ok(if started {
                        let s = ev.read::<CollisionStarted>();
                        (s.as_ptr().cast::<CrabbyCollision>(), s.len())
                    } else {
                        let s = ev.read::<CollisionStopped>();
                        (s.as_ptr().cast::<CrabbyCollision>(), s.len())
                    })
                })?;
                out(out_data, ptr)?;
                out(out_count, len)
            }
        })
    }
}

#[cfg(not(feature = "physics"))]
mod imp {
    //! Without the feature: every call fails, saying why.
    use super::*;
    use crate::{CrabbyCtx, Kind, boundary, invalid};

    fn off() -> Result<(), Failure> {
        Err(invalid("ponko_ffi was built without the `physics` feature"))
    }
    pub(crate) fn kind(_: std::any::TypeId) -> Option<Kind> {
        None
    }
    pub(super) unsafe fn component(_: *mut CrabbyWorld, _: *mut u32, _: u8) -> CrabbyStatus {
        boundary(off)
    }
    pub(super) unsafe fn game_physics(_: *mut CrabbyGame, _: CrabbyVec2) -> CrabbyStatus {
        boundary(off)
    }
    pub(super) unsafe fn set_polygon(
        _: *mut CrabbyWorld,
        _: u64,
        _: *const CrabbyVec2,
        _: u32,
        _: CrabbyCollider,
    ) -> CrabbyStatus {
        boundary(off)
    }
    pub(super) unsafe fn polygon_points(_: *mut CrabbyWorld, _: u64, _: *mut CrabbyVec2, _: *mut u32) -> CrabbyStatus {
        boundary(off)
    }
    pub(super) unsafe fn set_gravity(_: *mut CrabbyCtx, _: CrabbyVec2) -> CrabbyStatus {
        boundary(off)
    }
    pub(super) unsafe fn cast_ray(
        _: *mut CrabbyCtx,
        _: CrabbyVec2,
        _: CrabbyVec2,
        _: f32,
        _: *mut bool,
        _: *mut CrabbyRayHit,
    ) -> CrabbyStatus {
        boundary(off)
    }
    pub(super) unsafe fn overlaps(
        _: *mut CrabbyCtx,
        _: CrabbyVec2,
        _: Option<f32>,
        _: *mut CrabbyEntity,
        _: usize,
        _: *mut usize,
    ) -> CrabbyStatus {
        boundary(off)
    }
    pub(super) unsafe fn apply_impulse(_: *mut CrabbyCtx, _: u64, _: CrabbyVec2) -> CrabbyStatus {
        boundary(off)
    }
    pub(super) unsafe fn mass(_: *mut CrabbyCtx, _: u64, _: *mut f32) -> CrabbyStatus {
        boundary(off)
    }
    pub(super) unsafe fn collisions(
        _: *mut CrabbyCtx,
        _: bool,
        _: *mut *const CrabbyCollision,
        _: *mut usize,
    ) -> CrabbyStatus {
        boundary(off)
    }
}

pub(crate) use imp::kind;
