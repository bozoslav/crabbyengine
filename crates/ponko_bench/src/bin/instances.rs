//! Fixed-camera render workload. Run with --features render --bin instances.
use ponko::*;

fn main() {
    Game::new("Instance streaming benchmark", 1280, 720)
        .setup(|ctx| {
            ctx.camera3d.position = vec3(0.0, 20.0, 30.0);
            ctx.camera3d.look_at(vec3(0.0, 0.0, -150.0));
            ctx.camera3d.far = 1000.0;
            ctx.lighting.shadow_distance = 300.0;
            let material = ctx.create_material(MaterialDesc::default());
            let mesh = ctx.create_mesh(&MeshData::cube(0.3), material);
            for region in 0..300 {
                let origin = vec3((region % 20) as f32 * 32.0 - 320.0, 0.0, -(region / 20) as f32 * 32.0);
                let transforms: Vec<_> = (0..1000)
                    .map(|i| Mat4::from_translation(origin + vec3((i % 32) as f32, 0.0, (i / 32) as f32)))
                    .collect();
                let options = InstanceBatchOptions {
                    max_distance: 500.0,
                    cast_shadows: region % 3 == 0,
                    ..Default::default()
                };
                if std::env::var_os("PONKO_INSTANCE_MODELS").is_some() {
                    for matrix in &transforms {
                        ctx.world
                            .spawn((Transform3D::at(matrix.w_axis.truncate()), Model::new(mesh)));
                    }
                } else if std::env::var_os("PONKO_INSTANCE_LEGACY").is_some() {
                    ctx.create_instance_batch(mesh, &transforms, options);
                } else {
                    let levels = [(0.0, 50.0), (50.0, 150.0), (150.0, 300.0), (300.0, 500.0)].map(
                        |(min_distance, max_distance)| InstanceLevel {
                            mesh,
                            min_distance,
                            max_distance,
                            camera: true,
                            shadows: max_distance <= 300.0,
                        },
                    );
                    ctx.create_instance_batch_levels(&levels, &transforms, &[], options);
                }
            }
        })
        .run();
}
