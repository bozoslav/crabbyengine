//! Same seed, same simulation: everything that could make runs diverge
//! (threads, the grid, sorting, reduced rates, spawning) at once.

use ponko::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Walker {
    heading: f32,
    energy: f32,
    crowded: u32,
}

#[derive(Serialize, Deserialize)]
struct Food(f32);

fn wander(ctx: &mut Ctx) {
    let (dt, random, grid) = (ctx.dt(), ctx.random(), &ctx.spatial);
    ctx.world
        .query::<(Entity, &mut Transform2D, &mut Walker)>()
        .par_for_each(|(e, t, w)| {
            let mut rng = random.of(e);
            let mut near = 0;
            grid.for_each_in_radius(t.position, 8.0, |o| near += (o.entity != e) as u32);
            w.crowded = near;
            w.heading += rng.range(-0.5, 0.5) + near as f32 * 0.1;
            t.position += Vec2::from_angle(w.heading) * 20.0 * dt;
            w.energy -= dt;
        });
}

fn eat(ctx: &mut Ctx) {
    let turn = ctx.time.turns(3);
    let grid = &ctx.spatial;
    let food: Vec<(Entity, Vec2)> = ctx
        .world
        .query::<(Entity, &Transform2D, &Food)>()
        .iter()
        .map(|(e, t, _)| (e, t.position))
        .collect();
    let mut eaten = Vec::new();
    for (f, p) in food {
        if !turn.includes(f) {
            continue;
        }
        if let Some(w) = grid.nearest(p, 6.0, |o| o.entity != f) {
            eaten.push((f, w.entity));
        }
    }
    for (f, w) in eaten {
        if let Some(walker) = ctx.world.get_mut::<Walker>(w) {
            walker.energy += 1.0;
            ctx.commands.despawn(f);
        }
    }
}

fn breed_and_starve(ctx: &mut Ctx) {
    let mut rng = ctx.random().stream(0);
    let cmds = &mut ctx.commands;
    ctx.world
        .query::<(Entity, &Transform2D, &Walker)>()
        .for_each(|(e, t, w)| {
            if w.energy <= 0.0 {
                cmds.despawn(e);
            } else if w.energy > 2.0 && rng.chance(0.3) {
                cmds.spawn((
                    Transform2D::at(t.position),
                    Walker {
                        heading: rng.angle(),
                        energy: 1.0,
                        crowded: 0,
                    },
                ));
            }
        });
    for _ in 0..3 {
        let p = vec2(rng.range(0.0, 200.0), rng.range(0.0, 200.0));
        cmds.spawn((Transform2D::at(p), Food(1.0)));
    }
}

fn run(seed: u64, ticks: u64) -> Value {
    let world = Game::new("det", 1, 1)
        .seed(seed)
        .spatial_grid(8.0)
        .sim_system(wander)
        .sim_system(eat)
        .sim_system_every(5, breed_and_starve)
        .run_headless(ticks, |ctx| {
            ctx.world.reflect::<Walker>();
            ctx.world.reflect::<Food>();
            let mut rng = Rng::new(1);
            ctx.world.spawn_batch((0..3000).map(|_| {
                let p = vec2(rng.range(0.0, 200.0), rng.range(0.0, 200.0));
                (
                    Transform2D::at(p),
                    Walker {
                        heading: rng.angle(),
                        energy: 2.0,
                        crowded: 0,
                    },
                )
            }));
            ctx.world.spawn_batch((0..500).map(|_| {
                (
                    Transform2D::at(vec2(rng.range(0.0, 200.0), rng.range(0.0, 200.0))),
                    Food(1.0),
                )
            }));
        });
    world.save()
}

#[test]
fn the_same_seed_gives_the_same_world() {
    // 100 ticks: three spatial sorts, despawns and births through commands.
    let a = run(7, 100);
    let b = run(7, 100);
    assert!(
        a["entities"].as_array().unwrap().len() > 100,
        "the sim should still be alive"
    );
    assert_eq!(a.to_string(), b.to_string());
}

#[test]
fn a_different_seed_gives_a_different_world() {
    assert_ne!(run(7, 30).to_string(), run(8, 30).to_string());
}
