#![cfg(feature = "3d")]
#[allow(unused_imports)]
#[cfg(feature = "3d")]
use ponko::d3::{PointLight as PointLight3D, SpotLight as SpotLight3D, Transform as Transform3D};
use ponko::prelude::*;

#[test]
fn entity_lights_roundtrip_and_spots_follow_forward() {
    Game::new("lights", 64, 64).run_headless(0, |ctx| {
        let t = Transform3D::at(vec3(2.0, 3.0, 4.0)).looking_at(vec3(2.0, 3.0, 0.0));
        let point = PointLight3D {
            color: Color::rgb(1.0, 0.2, 0.1),
            intensity: 12.0,
            range: 8.0,
            shadows: true,
            shadow_resolution: 512,
        };
        let spot = SpotLight3D {
            inner_angle: 0.2,
            outer_angle: 0.5,
            ..Default::default()
        };
        let e = ctx.world.spawn((t, point, spot));
        assert_eq!(*ctx.world.get::<PointLight3D>(e).unwrap(), point);
        assert_eq!(*ctx.world.get::<SpotLight3D>(e).unwrap(), spot);
        let json = ctx.world.get_json(e, "PointLight3D").unwrap();
        assert_eq!(json["intensity"], 12.0);
        assert_eq!(json["shadow_resolution"], 512);
        ctx.world
            .set_json(
                e,
                "SpotLight3D",
                serde_json::json!({"intensity": 25.0, "range": 9.0, "inner_angle": 0.1, "outer_angle": 0.4}),
            )
            .unwrap();
        assert_eq!(ctx.world.get::<SpotLight3D>(e).unwrap().intensity, 25.0);
        assert!((t.forward() - Vec3::NEG_Z).length() < 1e-5);
    });
}
