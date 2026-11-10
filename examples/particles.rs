//! CPU billboards: `cargo run --features 3d --example particles`.
use ponko::*;

// ANCHOR: particles
fn setup(ctx: &mut Ctx) {
    ctx.camera3d.position = vec3(0.0, 2.0, 7.0);
    ctx.camera3d.look_at(vec3(0.0, 1.0, 0.0));
    ctx.sky.atmosphere = false;
    let emitter = ParticleEmitter3D::new(ParticleSettings3D {
        rate: 120.0,
        burst: 40,
        lifetime: (0.7, 1.4),
        speed: (2.0, 4.0),
        shape: ParticleShape3D::Cone { angle: 0.45 },
        gravity: vec3(0.0, -3.0, 0.0),
        drag: 0.2,
        size: (0.05, 0.13),
        rotation: (0.0, std::f32::consts::TAU),
        size_over_life: ParticleCurve::new(vec![(0.0, 0.5), (0.2, 1.0), (1.0, 0.0)]),
        color_over_life: ParticleGradient::new(vec![
            (0.0, Color::rgba(1.0, 0.7, 0.2, 1.0)),
            (1.0, Color::rgba(1.0, 0.1, 0.0, 0.0)),
        ]),
        blend: ParticleBlend3D::Additive,
        max_particles: 512,
        seed: 42,
        ..Default::default()
    });
    ctx.world.spawn((Transform3D::default(), emitter));
}
// ANCHOR_END: particles

fn main() {
    Game::new("Particles", 960, 640).setup(setup).run();
}
