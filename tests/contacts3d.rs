//! Contact modification (feature `physics3d`): one-way surfaces, conveyor
//! belts and the custom contact hook.
#![cfg(feature = "physics3d")]
use ponko::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);

fn platform(ctx: &mut Ctx, at: Vec3, modifier: Option<ContactModifier3D>) -> Entity {
    let e = ctx.world.spawn((
        Transform3D::at(at),
        RigidBody3D::fixed(),
        Collider3D::cuboid(vec3(10.0, 0.2, 10.0)),
    ));
    if let Some(m) = modifier {
        ctx.world.insert(e, m);
    }
    e
}

fn ball(ctx: &mut Ctx, at: Vec3, velocity: Vec3) -> Entity {
    ctx.world.spawn((
        Transform3D::at(at),
        RigidBody3D::dynamic(),
        Collider3D::sphere(0.25),
        Velocity3D {
            linear: velocity,
            angular: Vec3::ZERO,
        },
    ))
}

fn y(world: &mut World, e: Entity) -> f32 {
    world.get::<Transform3D>(e).unwrap().position.y
}

#[test]
fn a_one_way_platform_lets_things_up_through_and_holds_them_on_top() {
    let (mut from_below, mut from_above, mut solid) = (None, None, None);
    let mut world = Game::new("t", 1, 1).physics3d(GRAVITY).run_headless(150, |ctx| {
        platform(ctx, Vec3::ZERO, Some(ContactModifier3D::one_way(Vec3::Y)));
        // Thrown up from 1 m below: it rises through and lands on top.
        from_below = Some(ball(ctx, vec3(0.0, -1.0, 0.0), vec3(0.0, 8.0, 0.0)));
        from_above = Some(ball(ctx, vec3(3.0, 2.0, 0.0), Vec3::ZERO));
        // The same throw at an ordinary platform bounces off its underside.
        platform(ctx, vec3(30.0, 0.0, 0.0), None);
        solid = Some(ball(ctx, vec3(30.0, -1.0, 0.0), vec3(0.0, 8.0, 0.0)));
    });
    let rest = 0.1 + 0.25;
    let below = y(&mut world, from_below.unwrap());
    assert!((below - rest).abs() < 0.05, "came up through and rests on top: {below}");
    let above = y(&mut world, from_above.unwrap());
    assert!((above - rest).abs() < 0.05, "fell onto it and stayed: {above}");
    assert!(y(&mut world, solid.unwrap()) < -0.3, "the plain platform blocked it");
}

#[test]
fn a_conveyor_carries_things_along_whichever_body_comes_first() {
    for belt_first in [true, false] {
        let mut box_entity = None;
        let world = Game::new("t", 1, 1).physics3d(GRAVITY).run_headless(150, |ctx| {
            let crate_box = |ctx: &mut Ctx| {
                ctx.world.spawn((
                    Transform3D::at(vec3(0.0, 0.4, 0.0)),
                    RigidBody3D::dynamic(),
                    Collider3D::cuboid(Vec3::splat(0.5)),
                ))
            };
            // The belt is turned a quarter turn around Y: local +X is world -Z.
            let belt = |ctx: &mut Ctx| {
                let e = platform(ctx, Vec3::ZERO, Some(ContactModifier3D::conveyor(vec3(2.0, 0.0, 0.0))));
                ctx.world.get_mut::<Transform3D>(e).unwrap().rotation =
                    Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
            };
            if belt_first {
                belt(ctx);
                box_entity = Some(crate_box(ctx));
            } else {
                box_entity = Some(crate_box(ctx));
                belt(ctx);
            }
        });
        let p = world.get::<Transform3D>(box_entity.unwrap()).unwrap().position;
        // Three seconds at up to 2 m/s, minus getting up to speed.
        assert!(
            p.z < -3.5 && p.z > -6.5,
            "carried along world -Z (belt first: {belt_first}): {p}"
        );
        assert!(p.x.abs() < 0.3, "{p}");
    }
}

#[test]
fn the_contact_hook_sees_hooked_pairs_and_can_drop_them() {
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    let (mut hooked_ball, mut plain_ball, mut hooked_floor) = (None, None, None);
    let mut world = Game::new("t", 1, 1).physics3d(GRAVITY).run_headless(100, |ctx| {
        let floor = platform(ctx, Vec3::ZERO, Some(ContactModifier3D::hook()));
        hooked_floor = Some(floor);
        ctx.physics3d.set_contact_hook(move |pair| {
            seen.fetch_add(1, Ordering::Relaxed);
            assert!(pair.a == floor || pair.b == floor);
            // Only the ball at x = 3 is let through.
            if pair.point_count() > 0 && pair.point(0).unwrap().0.x > 2.0 {
                pair.ignore();
            } else {
                pair.set_friction(0.5);
            }
        });
        hooked_ball = Some(ball(ctx, vec3(3.0, 1.0, 0.0), Vec3::ZERO));
        plain_ball = Some(ball(ctx, vec3(-3.0, 1.0, 0.0), Vec3::ZERO));
    });
    assert!(calls.load(Ordering::Relaxed) > 0);
    assert!(
        y(&mut world, hooked_ball.unwrap()) < -1.0,
        "dropped contacts: it fell through"
    );
    assert!(
        y(&mut world, plain_ball.unwrap()) > 0.3,
        "the other one stands on the floor"
    );
}

#[test]
fn removing_the_modifier_makes_the_body_plain_again() {
    let mut falling = None;
    let mut floor = None;
    let mut world = Game::new("t", 1, 1)
        .physics3d(GRAVITY)
        .sim_system(|ctx: &mut Ctx| {
            // After a few steps the hook-everything floor turns plain.
            if ctx.time.sim_tick() == 5 {
                let f = ctx
                    .world
                    .query::<(Entity, &ContactModifier3D)>()
                    .iter()
                    .next()
                    .map(|(e, _)| e);
                if let Some(f) = f {
                    ctx.world.remove::<ContactModifier3D>(f);
                }
            }
        })
        .run_headless(100, |ctx| {
            floor = Some(platform(ctx, Vec3::ZERO, Some(ContactModifier3D::hook())));
            ctx.physics3d.set_contact_hook(|pair| pair.ignore());
            falling = Some(ball(ctx, vec3(0.0, 1.0, 0.0), Vec3::ZERO));
        });
    assert!(world.get::<ContactModifier3D>(floor.unwrap()).is_none());
    assert!(y(&mut world, falling.unwrap()) > 0.3, "the plain floor holds it");
}

#[test]
fn guide_contacts_snippet_compiles_and_steps() {
    Game::new("contacts", 1, 1).physics3d(GRAVITY).run_headless(5, |ctx| {
        // ANCHOR: contacts
        // A platform to jump up through: solid only from above.
        ctx.world.spawn((
            Transform3D::at(vec3(0.0, 3.0, 0.0)),
            RigidBody3D::fixed(),
            Collider3D::cuboid(vec3(4.0, 0.2, 4.0)),
            ContactModifier3D::one_way(Vec3::Y),
        ));
        // A belt carrying whatever stands on it along +X at 2 m/s.
        ctx.world.spawn((
            Transform3D::at(vec3(10.0, 0.0, 0.0)),
            RigidBody3D::fixed(),
            Collider3D::cuboid(vec3(8.0, 0.2, 1.0)),
            ContactModifier3D::conveyor(vec3(2.0, 0.0, 0.0)),
        ));
        // A hook for the rest: the ghost walks through the hooked wall.
        let ghost = ctx.world.spawn((
            Transform3D::at(vec3(-10.0, 1.0, 0.0)),
            RigidBody3D::dynamic(),
            Collider3D::sphere(0.5),
        ));
        ctx.world.spawn((
            Transform3D::at(vec3(-10.0, 1.0, 3.0)),
            RigidBody3D::fixed(),
            Collider3D::cuboid(vec3(4.0, 2.0, 0.2)),
            ContactModifier3D::hook(),
        ));
        ctx.physics3d.set_contact_hook(move |pair| {
            if pair.a == ghost || pair.b == ghost {
                pair.ignore();
            }
        });
        // ANCHOR_END: contacts
    });
}
