//! 3D navigation: agents walking across a baked navmesh (`step_navmesh3d`
//! registered as a sim system), path following without crowd avoidance.
use ponko::d3::NavAgent as NavAgent3D;
use ponko::d3::NavMesh as NavMesh3D;
use ponko::d3::Transform as Transform3D;
use ponko::nav::NavBakeSettings;
use ponko::nav::NavSource;
use ponko::render::MeshData;
use ponko::systems::step_navmesh3d;
use ponko::{Game, vec3};

use crate::Bench;

pub const TICKS: u64 = 250;

pub fn run(b: &mut Bench) {
    for n in [1_000, 10_000] {
        b.run(
            &format!("nav3d/agents-{n}"),
            3,
            || (),
            |_| {
                let world = Game::new("bench", 1, 1)
                    .sim_system(step_navmesh3d)
                    .run_headless(TICKS, |ctx| {
                        let source = NavSource::from_mesh(&MeshData::plane(100.0), Transform3D::default());
                        let nav = NavMesh3D::bake(&[source], NavBakeSettings::default()).expect("bake");
                        ctx.world.insert_resource(nav);
                        let side = (n as f32).sqrt().ceil() as usize;
                        for i in 0..n {
                            let (x, z) = ((i % side) as f32, (i / side) as f32);
                            let at = vec3(x * 0.8 - 40.0, 0.0, z * 0.8 - 40.0);
                            let mut agent = NavAgent3D::default();
                            agent.destination = Some(vec3(-at.x, 0.0, -at.z));
                            ctx.world.spawn((Transform3D::at(at), agent));
                        }
                    });
                std::hint::black_box(world);
            },
        );
    }
}
