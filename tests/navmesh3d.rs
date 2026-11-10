#![cfg(feature = "navmesh")]
use ponko::*;

fn source(mesh: MeshData, position: Vec3) -> NavSource {
    NavSource::from_mesh(&mesh, Transform3D::at(position))
}
fn floor() -> NavSource {
    source(MeshData::plane(20.0), Vec3::ZERO)
}
fn bake(sources: &[NavSource]) -> NavMesh3D {
    NavMesh3D::bake(sources, NavBakeSettings::default()).unwrap()
}
fn carving_bake(sources: &[NavSource]) -> NavMesh3D {
    let settings = NavBakeSettings {
        carving: true,
        ..Default::default()
    };
    NavMesh3D::bake(sources, settings).unwrap()
}
#[test]
fn snaps_and_rejects_distant_points() {
    let nav = bake(&[floor()]);
    let p = nav.sample_position(vec3(1.0, 1.0, 2.0), 2.0).unwrap();
    assert!(p.distance(vec3(1.0, 0.0, 2.0)) < 0.2);
    assert!(nav.sample_position(Vec3::splat(100.0), 1.0).is_none());
}
#[test]
fn paths_around_a_solid_box() {
    let nav = bake(&[floor(), source(MeshData::cube(4.0), vec3(0.0, 2.0, 0.0))]);
    let path = nav.find_path(vec3(-7.0, 0.0, 0.0), vec3(7.0, 0.0, 0.0)).unwrap();
    assert!(path.iter().any(|p| p.z.abs() > 2.0), "{path:?}");
    assert!(path.len() >= 3);
}
#[test]
fn steep_slopes_are_not_walkable() {
    let mesh = MeshData::plane(12.0);
    let slope = NavSource::from_mesh(
        &mesh,
        Transform3D::default().rotated(Quat::from_rotation_x(60_f32.to_radians())),
    );
    let nav = bake(&[slope]);
    assert!(nav.sample_position(Vec3::ZERO, 1.0).is_none());
}
#[test]
fn low_steps_connect_but_tall_steps_do_not() {
    for (height, connected) in [(0.2, true), (1.0, false)] {
        let a = source(MeshData::plane(8.0), vec3(-4.0, 0.0, 0.0));
        let b = source(MeshData::plane(8.0), vec3(4.0, height, 0.0));
        let nav = bake(&[a, b]);
        assert_eq!(
            nav.find_path(vec3(-3.0, 0.0, 0.0), vec3(3.0, height, 0.0)).is_some(),
            connected
        );
    }
}
#[test]
fn obstacles_block_and_removal_restores_paths() {
    let mut nav = bake(&[floor()]);
    let a = vec3(-7.0, 0.0, 0.0);
    let b = vec3(7.0, 0.0, 0.0);
    assert!(nav.find_path(a, b).is_some());
    nav.set_obstacles(&[NavObstacle3D::new(vec3(0.0, 1.0, 0.0), vec3(0.5, 2.0, 20.0))]);
    assert!(nav.find_path(a, b).is_none());
    nav.set_obstacles(&[]);
    assert!(nav.find_path(a, b).is_some());
}
#[test]
fn agent_reaches_destination_headless() {
    let mut world = Game::new("navigation", 1, 1)
        .fixed_hz(60)
        .sim_system(systems::step_navmesh3d)
        .run_headless(360, |ctx| {
            ctx.world.insert_resource(bake(&[floor()]));
            let mut agent = NavAgent3D::default();
            agent.destination = Some(vec3(5.0, 0.0, 0.0));
            ctx.world.spawn((Transform3D::at(vec3(-5.0, 0.0, 0.0)), agent));
        });
    let p = world.query::<&Transform3D>().iter().next().unwrap().position;
    assert!(p.distance(vec3(5.0, 0.1, 0.0)) < 0.25, "{p:?}");
}
#[test]
fn baking_and_paths_are_repeatable() {
    let sources = [floor(), source(MeshData::cube(4.0), vec3(0.0, 2.0, 0.0))];
    let a = bake(&sources);
    let b = bake(&sources);
    let start = vec3(-7.0, 0.0, 0.0);
    let end = vec3(7.0, 0.0, 0.0);
    assert_eq!(a.find_path(start, end), b.find_path(start, end));
}

#[test]
fn invalid_inputs_return_errors_or_no_position() {
    let settings = NavBakeSettings {
        cell_size: 0.0,
        ..Default::default()
    };
    assert!(NavMesh3D::bake(&[floor()], settings).is_err());
    assert!(
        NavMesh3D::bake(
            &[NavSource {
                triangles: vec![[Vec3::NAN; 3]]
            }],
            NavBakeSettings::default()
        )
        .is_err()
    );
    let nav = bake(&[floor()]);
    assert!(nav.sample_position(Vec3::ZERO, -1.0).is_none());
    assert!(nav.find_path(Vec3::NAN, Vec3::ZERO).is_none());
}

#[test]
fn clearance_excludes_low_ceilings() {
    let mut ceiling = floor();
    for t in &mut ceiling.triangles {
        for p in t.iter_mut() {
            p.y = 1.0;
        }
        t.swap(1, 2);
    }
    let nav = bake(&[floor(), ceiling]);
    assert!(nav.sample_position(Vec3::ZERO, 0.3).is_none());
}

#[test]
fn agent_follows_detour_without_entering_solid_box() {
    fn check(ctx: &mut Ctx) {
        ctx.world.query::<&Transform3D>().for_each(|t| {
            assert!(t.position.x.abs() > 2.0 || t.position.z.abs() > 2.0, "{:?}", t.position);
        });
    }
    let mut world = Game::new("navigation", 1, 1)
        .fixed_hz(60)
        .sim_system(systems::step_navmesh3d)
        .sim_system(check)
        .run_headless(600, |ctx| {
            ctx.world
                .insert_resource(bake(&[floor(), source(MeshData::cube(4.0), vec3(0.0, 2.0, 0.0))]));
            let mut agent = NavAgent3D::default();
            agent.destination = Some(vec3(7.0, 0.0, 0.0));
            ctx.world.spawn((Transform3D::at(vec3(-7.0, 0.0, 0.0)), agent));
        });
    let position = world.query::<&Transform3D>().iter().next().unwrap().position;
    assert!(position.distance(vec3(7.0, 0.0, 0.0)) < 0.25, "{position:?}");
}

#[test]
fn mesh_replacement_invalidates_an_existing_agent_path() {
    struct Tick(u32);
    fn replace(ctx: &mut Ctx) {
        ctx.world.resource_mut::<Tick>().0 += 1;
        if ctx.world.resource::<Tick>().0 == 40 {
            let empty = NavSource::from_mesh(
                &MeshData::plane(12.0),
                Transform3D::default().rotated(Quat::from_rotation_x(60_f32.to_radians())),
            );
            ctx.world.insert_resource(bake(&[empty]));
        }
    }
    let mut world = Game::new("navigation", 1, 1)
        .fixed_hz(60)
        .sim_system(replace)
        .sim_system(systems::step_navmesh3d)
        .run_headless(300, |ctx| {
            ctx.world.insert_resource(Tick(0));
            ctx.world.insert_resource(bake(&[floor()]));
            let mut agent = NavAgent3D::default();
            agent.destination = Some(vec3(7.0, 0.0, 0.0));
            ctx.world.spawn((Transform3D::at(vec3(-7.0, 0.0, 0.0)), agent));
        });
    let x = world.query::<&Transform3D>().iter().next().unwrap().position.x;
    assert!(x < -4.0, "{x}");
}

#[test]
fn obstacle_components_stop_agents_then_removal_replans_deterministically() {
    #[derive(Clone, Copy)]
    struct State {
        tick: u32,
        obstacle: Entity,
    }
    fn update(ctx: &mut Ctx) {
        let state = ctx.world.resource_mut::<State>();
        state.tick += 1;
        let state = *state;
        if state.tick == 30 {
            ctx.world.get_mut::<NavObstacle3D>(state.obstacle).unwrap().center.x = 0.0;
        }
        if state.tick == 100 {
            ctx.world
                .query::<&Transform3D>()
                .for_each(|t| assert!(t.position.x < 0.0));
            ctx.world.despawn(state.obstacle);
        }
    }
    let run = || {
        let mut world = Game::new("navigation", 1, 1)
            .fixed_hz(60)
            .sim_system(update)
            .sim_system(systems::step_navmesh3d)
            .run_headless(500, |ctx| {
                ctx.world.insert_resource(bake(&[floor()]));
                let obstacle = ctx
                    .world
                    .spawn((NavObstacle3D::new(vec3(30.0, 1.0, 0.0), vec3(0.5, 2.0, 20.0)),));
                ctx.world.insert_resource(State { tick: 0, obstacle });
                let mut agent = NavAgent3D::default();
                agent.destination = Some(vec3(7.0, 0.0, 0.0));
                ctx.world.spawn((Transform3D::at(vec3(-7.0, 0.0, 0.0)), agent));
            });
        world.query::<&Transform3D>().iter().next().unwrap().position
    };
    let a = run();
    let b = run();
    assert_eq!(a, b);
    assert!(a.distance(vec3(7.0, 0.0, 0.0)) < 0.25, "{a:?}");
}

#[test]
fn agents_follow_steps_and_keep_their_floor_in_stacked_geometry() {
    for (upper, avoidance) in [(false, false), (true, false), (false, true), (true, true)] {
        let level = if upper { 4.0 } else { 0.0 };
        let mut world = Game::new("navigation", 1, 1)
            .fixed_hz(60)
            .sim_system(systems::step_navmesh3d)
            .run_headless(360, |ctx| {
                let sources = [
                    source(MeshData::plane(8.0), vec3(-4.0, level, 0.0)),
                    source(MeshData::plane(8.0), vec3(4.0, level + 0.2, 0.0)),
                    source(MeshData::plane(20.0), vec3(0.0, if upper { 0.0 } else { 4.0 }, 0.0)),
                ];
                ctx.world.insert_resource(bake(&sources));
                let mut agent = NavAgent3D::default();
                agent.destination = Some(vec3(3.0, level + 0.2, 0.0));
                agent.avoidance = avoidance;
                ctx.world.spawn((Transform3D::at(vec3(-3.0, level, 0.0)), agent));
            });
        let position = world.query::<&Transform3D>().iter().next().unwrap().position;
        assert!(position.distance(vec3(3.0, level + 0.2, 0.0)) < 0.25, "{position:?}");
    }
}

fn crowd_agent(goal: Option<Vec3>) -> NavAgent3D {
    let mut a = NavAgent3D::default();
    a.avoidance = true;
    a.destination = goal;
    a
}
fn check_crowd(ctx: &mut Ctx) {
    let agents: Vec<_> = ctx
        .world
        .query::<(&Transform3D, &NavAgent3D)>()
        .iter()
        .map(|(t, a)| (t.position, a.radius))
        .collect();
    for (i, (p, r)) in agents.iter().enumerate() {
        assert!(
            ctx.world.resource::<NavMesh3D>().sample_position(*p, 0.02).is_some(),
            "off mesh {p:?}"
        );
        for (q, s) in &agents[i + 1..] {
            assert!((p - q).with_y(0.0).length() >= r + s - 0.025, "overlap {p:?} {q:?}");
        }
    }
}
#[test]
fn crowd_head_on_corridor_passes_and_arrives_repeatably() {
    let run = || {
        let mut world = Game::new("crowd", 1, 1)
            .fixed_hz(60)
            .sim_system(systems::step_navmesh3d)
            .sim_system(check_crowd)
            .run_headless(600, |ctx| {
                let floor = NavSource::from_mesh(
                    &MeshData::plane(20.0),
                    Transform3D {
                        scale: vec3(1.0, 1.0, 0.15),
                        ..Default::default()
                    },
                );
                ctx.world.insert_resource(bake(&[floor]));
                for sign in [-1.0, 1.0] {
                    ctx.world.spawn((
                        Transform3D::at(vec3(sign * 7.0, 0.0, 0.0)),
                        crowd_agent(Some(vec3(-sign * 7.0, 0.0, 0.0))),
                    ));
                }
            });
        world
            .query::<(&Transform3D, &NavAgent3D)>()
            .iter()
            .map(|(t, a)| {
                assert!(t.position.distance(a.destination.unwrap()) < 0.25, "{:?}", t.position);
                t.position
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}
#[test]
fn crowd_walks_around_a_stopped_agent() {
    let mut world = Game::new("crowd", 1, 1)
        .fixed_hz(60)
        .sim_system(systems::step_navmesh3d)
        .sim_system(check_crowd)
        .run_headless(600, |ctx| {
            ctx.world.insert_resource(bake(&[floor()]));
            ctx.world.spawn((Transform3D::default(), crowd_agent(None)));
            ctx.world.spawn((
                Transform3D::at(vec3(-7.0, 0.0, 0.0)),
                crowd_agent(Some(vec3(7.0, 0.0, 0.0))),
            ));
        });
    world.query::<(&Transform3D, &NavAgent3D)>().for_each(|(t, a)| {
        let target = a.destination.unwrap_or(Vec3::ZERO);
        assert!(t.position.distance(target) < 0.25, "{:?}", t.position);
    });
}
#[test]
fn fifty_agents_reach_a_shared_goal_region_without_overlaps() {
    let mut world = Game::new("crowd", 1, 1)
        .fixed_hz(60)
        .sim_system(systems::step_navmesh3d)
        .sim_system(check_crowd)
        .run_headless(1800, |ctx| {
            ctx.world.insert_resource(bake(&[floor()]));
            for i in 0..50 {
                let angle = i as f32 * std::f32::consts::TAU / 50.0;
                ctx.world.spawn((
                    Transform3D::at(vec3(angle.cos() * 8.0, 0.0, angle.sin() * 8.0)),
                    crowd_agent(Some(Vec3::ZERO)),
                ));
            }
        });
    // Fifty radius-.3 discs need a region, rather than a single occupied point.
    world
        .query::<&Transform3D>()
        .for_each(|t| assert!(t.position.with_y(0.0).length() < 4.5, "{:?}", t.position));
}

#[test]
fn five_hundred_agents_step_within_a_coarse_budget() {
    let start = std::time::Instant::now();
    let mut world = Game::new("crowd", 1, 1)
        .fixed_hz(60)
        .sim_system(systems::step_navmesh3d)
        .run_headless(60, |ctx| {
            ctx.world
                .insert_resource(bake(&[source(MeshData::plane(20.0), Vec3::ZERO)]));
            for i in 0..500 {
                let position = vec3((i % 25) as f32 * 0.72 - 8.64, 0.0, (i / 25) as f32 * 0.72 - 6.84);
                let mut a = crowd_agent(Some(position + Vec3::Z));
                a.radius = 0.2;
                a.avoidance_weight = 0.0;
                ctx.world.spawn((Transform3D::at(position), a));
            }
        });
    assert_eq!(world.query::<&NavAgent3D>().iter().count(), 500);
    world.query::<(&Transform3D, &NavAgent3D)>().for_each(|(t, a)| {
        assert!(
            t.position.with_y(0.0).distance(a.destination.unwrap()) < 0.999,
            "agent did not move: {:?}",
            t.position
        );
    });
    assert!(start.elapsed() < std::time::Duration::from_secs(30));
}

#[test]
fn crowd_slots_survive_changes_without_reusing_another_agents_state() {
    struct State {
        tick: u32,
        moving: Entity,
        stopped: Entity,
    }
    fn edit(ctx: &mut Ctx) {
        let s = ctx.world.resource_mut::<State>();
        s.tick += 1;
        let (tick, moving, stopped) = (s.tick, s.moving, s.stopped);
        if tick == 60 {
            ctx.world.get_mut::<Transform3D>(moving).unwrap().position = vec3(-6.0, 0.0, 2.0);
            ctx.world.get_mut::<NavAgent3D>(moving).unwrap().destination = Some(vec3(6.0, 0.0, 2.0));
            ctx.world.despawn(stopped);
            // Cloning a live component must not clone ownership of its crowd slot.
            let mut clone = ctx.world.get::<NavAgent3D>(moving).unwrap().clone();
            clone.destination = Some(vec3(6.0, 0.0, -2.0));
            ctx.world.spawn((Transform3D::at(vec3(-6.0, 0.0, -2.0)), clone));
        }
        if tick == 120 {
            ctx.world.get_mut::<NavAgent3D>(moving).unwrap().avoidance = false;
        }
        if tick == 150 {
            ctx.world.get_mut::<NavAgent3D>(moving).unwrap().avoidance = true;
        }
        if tick == 180 {
            // More agents than the initial capacity forces a slot/corridor rebuild.
            for i in 0..20 {
                ctx.world.spawn((
                    Transform3D::at(vec3(-8.0 + (i % 10) as f32, 0.0, 5.0 + (i / 10) as f32)),
                    crowd_agent(None),
                ));
            }
        }
    }
    let mut world = Game::new("crowd", 1, 1)
        .fixed_hz(60)
        .sim_system(edit)
        .sim_system(systems::step_navmesh3d)
        .sim_system(check_crowd)
        .run_headless(600, |ctx| {
            ctx.world.insert_resource(bake(&[floor()]));
            let moving = ctx.world.spawn((
                Transform3D::at(vec3(-7.0, 0.0, 0.0)),
                crowd_agent(Some(vec3(7.0, 0.0, 0.0))),
            ));
            let stopped = ctx.world.spawn((Transform3D::default(), crowd_agent(None)));
            ctx.world.insert_resource(State {
                tick: 0,
                moving,
                stopped,
            });
        });
    world.query::<(&Transform3D, &NavAgent3D)>().for_each(|(t, a)| {
        if let Some(target) = a.destination {
            assert!(t.position.distance(target) < 0.25, "{:?}", t.position);
        }
    });
}

fn drain_carving(nav: &mut NavMesh3D, budget: usize) -> usize {
    let mut steps = 0;
    for _ in 0..200 {
        if !nav.carving_pending() {
            return steps;
        }
        let n = nav.update_carving(budget).unwrap();
        assert!(n <= budget);
        steps += n;
    }
    panic!("carving did not drain");
}
#[test]
fn carving_cuts_edges_and_removal_restores_straight_path() {
    let mut nav = carving_bake(&[floor()]);
    let from = vec3(-7.0, 0.0, 0.0);
    let to = vec3(7.0, 0.0, 0.0);
    let mut o = NavObstacle3D::new(vec3(0.0, 1.0, 0.0), vec3(0.5, 1.0, 2.0));
    o.carve = true;
    nav.set_obstacles(&[o]);
    assert!(nav.carving_pending());
    assert_eq!(nav.update_carving(0).unwrap(), 0);
    let rebuilt = drain_carving(&mut nav, 1);
    assert!(rebuilt > 0 && rebuilt <= 8, "{rebuilt}");
    let path = nav.find_path(from, to).unwrap();
    assert!(path.iter().any(|p| p.z.abs() > 2.0), "{path:?}");
    assert!(nav.sample_position(Vec3::ZERO, 0.2).is_none());
    o.center.x += 0.01;
    nav.set_obstacles(&[o]);
    assert!(!nav.carving_pending());
    assert_eq!(nav.update_carving(1).unwrap(), 0);
    o.center.x += 1.0;
    nav.set_obstacles(&[o]);
    assert!(drain_carving(&mut nav, 1) <= 16);
    nav.set_obstacles(&[]);
    drain_carving(&mut nav, 1);
    let path = nav.find_path(from, to).unwrap();
    assert!(path.iter().all(|p| p.z.abs() < 0.3), "{path:?}");
}
#[test]
fn carving_updates_received_while_pending_are_not_lost() {
    let mut nav = carving_bake(&[floor()]);
    let mut o = NavObstacle3D::new(vec3(0.0, 1.0, 0.0), Vec3::ONE);
    o.carve = true;
    nav.set_obstacles(&[o]);
    nav.update_carving(1).unwrap();
    nav.set_obstacles(&[]);
    drain_carving(&mut nav, 1);
    assert!(nav.sample_position(Vec3::ZERO, 0.3).is_some());
}

#[test]
fn crowd_reroutes_when_a_hole_is_carved_across_its_path() {
    struct Tick(u32);
    fn drop_box(ctx: &mut Ctx) {
        let t = ctx.world.resource_mut::<Tick>();
        t.0 += 1;
        if t.0 == 30 {
            let mut o = NavObstacle3D::new(vec3(0.0, 1.0, 0.0), vec3(0.5, 1.0, 2.0));
            o.carve = true;
            ctx.world.spawn((o,));
        }
    }
    fn run() -> Vec3 {
        let mut w = Game::new("carving", 1, 1)
            .fixed_hz(60)
            .sim_system(drop_box)
            .sim_system(systems::step_navmesh3d)
            .sim_system(check_crowd)
            .run_headless(600, |ctx| {
                ctx.world.insert_resource(Tick(0));
                ctx.world.insert_resource(carving_bake(&[floor()]));
                ctx.world.spawn((
                    Transform3D::at(vec3(-7.0, 0.0, 0.0)),
                    crowd_agent(Some(vec3(7.0, 0.0, 0.0))),
                ));
            });
        w.query::<&Transform3D>().iter().next().unwrap().position
    }
    let a = run();
    assert!(a.distance(vec3(7.0, 0.1, 0.0)) < 0.3, "{a:?}");
    assert_eq!(a, run());
}
