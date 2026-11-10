#![cfg(feature = "3d")]
use ponko::*;

#[test]
fn probe_weights_fade_inside_box_and_projection_hits_wall() {
    let probe = ReflectionProbe3D::default();
    assert_eq!(probe.weight(Vec3::ZERO, Vec3::ZERO), 1.0);
    assert_eq!(probe.weight(Vec3::ZERO, vec3(6.0, 0.0, 0.0)), 0.0);
    assert_eq!(
        probe.project(Vec3::ZERO, vec3(1.0, 0.0, 0.0), Vec3::Z),
        vec3(1.0, 0.0, 5.0)
    );
    let invalid = ReflectionProbe3D {
        size: Vec3::splat(f32::NAN),
        ..probe
    };
    assert_eq!(invalid.weight(Vec3::ZERO, Vec3::ZERO), 0.0);
}

#[test]
fn bake_requires_a_live_probe_and_allocates_content_once() {
    Game::new("probe", 1, 1).run_headless(0, |ctx| {
        let entity = ctx.world.spawn((Transform3D::default(), ReflectionProbe3D::default()));
        ctx.gfx.bake_reflection_probe(&mut ctx.world, entity).unwrap();
        let content = ctx.world.get::<ReflectionProbe3D>(entity).unwrap().content;
        assert_ne!(content, 0);
        ctx.gfx.bake_reflection_probe(&mut ctx.world, entity).unwrap();
        assert_eq!(ctx.world.get::<ReflectionProbe3D>(entity).unwrap().content, content);
        let empty = ctx.world.spawn((0u32,));
        assert!(ctx.gfx.bake_reflection_probe(&mut ctx.world, empty).is_err());
    });
}

fn write_image(name: &str, width: u32, height: u32, value: f32) -> String {
    let path = std::env::temp_dir().join(format!("ponko-probe-{}-{name}.exr", std::process::id()));
    image::Rgb32FImage::from_pixel(width, height, image::Rgb([value; 3]))
        .save(&path)
        .unwrap();
    path.to_str().unwrap().to_string()
}

#[test]
fn loading_needs_a_two_to_one_finite_image() {
    let good = write_image("good", 64, 32, 1.0);
    let square = write_image("square", 32, 32, 1.0);
    let broken = write_image("nan", 64, 32, f32::NAN);
    Game::new("probe", 1, 1).run_headless(0, |ctx| {
        let entity = ctx.world.spawn((Transform3D::default(), ReflectionProbe3D::default()));
        let bad = ctx
            .gfx
            .load_reflection_probe(&mut ctx.world, entity, &square)
            .unwrap_err();
        assert!(bad.contains("2:1"), "{bad}");
        assert!(
            ctx.gfx
                .load_reflection_probe(&mut ctx.world, entity, &broken)
                .unwrap_err()
                .contains("finite")
        );
        assert!(
            ctx.gfx
                .load_reflection_probe(&mut ctx.world, entity, "missing-probe.exr")
                .is_err()
        );
        assert_eq!(ctx.world.get::<ReflectionProbe3D>(entity).unwrap().content, 0);
        ctx.gfx.load_reflection_probe(&mut ctx.world, entity, &good).unwrap();
        assert_ne!(ctx.world.get::<ReflectionProbe3D>(entity).unwrap().content, 0);
    });
    for path in [good, square, broken] {
        std::fs::remove_file(path).ok();
    }
}

#[test]
fn a_fifth_environment_is_refused() {
    Game::new("probe", 1, 1).run_headless(0, |ctx| {
        for _ in 0..4 {
            let e = ctx.world.spawn((Transform3D::default(), ReflectionProbe3D::default()));
            ctx.gfx.bake_reflection_probe(&mut ctx.world, e).unwrap();
        }
        let fifth = ctx.world.spawn((Transform3D::default(), ReflectionProbe3D::default()));
        assert!(
            ctx.gfx
                .bake_reflection_probe(&mut ctx.world, fifth)
                .unwrap_err()
                .contains("four")
        );
    });
}
