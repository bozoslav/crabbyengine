#![cfg(feature = "navmesh")]
use crabbyengine::*;

fn source(mesh: MeshData, position: Vec3) -> NavSource {
    NavSource::from_mesh(&mesh, Transform3D::at(position))
}
fn floor() -> NavSource {
    source(MeshData::plane(20.0), Vec3::ZERO)
}
fn bake(sources: &[NavSource]) -> NavMesh3D {
    NavMesh3D::bake(sources, NavBakeSettings::default()).unwrap()
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
        .sim_system(step_navmesh3d)
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
        .sim_system(step_navmesh3d)
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
        .sim_system(step_navmesh3d)
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
            .sim_system(step_navmesh3d)
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
    for upper in [false, true] {
        let level = if upper { 4.0 } else { 0.0 };
        let mut world = Game::new("navigation", 1, 1)
            .fixed_hz(60)
            .sim_system(step_navmesh3d)
            .run_headless(360, |ctx| {
                let sources = [
                    source(MeshData::plane(8.0), vec3(-4.0, level, 0.0)),
                    source(MeshData::plane(8.0), vec3(4.0, level + 0.2, 0.0)),
                    source(MeshData::plane(20.0), vec3(0.0, if upper { 0.0 } else { 4.0 }, 0.0)),
                ];
                ctx.world.insert_resource(bake(&sources));
                let mut agent = NavAgent3D::default();
                agent.destination = Some(vec3(3.0, level + 0.2, 0.0));
                ctx.world.spawn((Transform3D::at(vec3(-3.0, level, 0.0)), agent));
            });
        let position = world.query::<&Transform3D>().iter().next().unwrap().position;
        assert!(position.distance(vec3(3.0, level + 0.2, 0.0)) < 0.25, "{position:?}");
    }
}
