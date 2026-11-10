//! 3D particle emitters from C: settings are checked, emitters are put on and
//! taken off entities, and bursts queue. Headless runs don't simulate the
//! visual particle pools (there is nothing to draw), so a burst stays queued
//! and the live count stays 0; the pools themselves are tested in the engine.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use PonkoStatus::*;

const OK: PonkoStatus = PONKO_OK;
const BAD: PonkoStatus = PONKO_INVALID_ARGUMENT;

fn v3(x: f32, y: f32, z: f32) -> PonkoVec3 {
    PonkoVec3 { x, y, z }
}

struct Run {
    done: bool,
}

unsafe fn spawn_at(ctx: *mut PonkoCtx) -> u64 {
    unsafe {
        let w = ponko_ctx_world(ctx);
        let mut transform = 0;
        assert_eq!(ponko_transform3d_component(w, &mut transform), OK);
        let t = ponko_transform3d_at(v3(0.0, 0.0, 0.0));
        let part = PonkoValue {
            component: transform,
            data: (&raw const t).cast(),
            size: size_of::<PonkoTransform3D>(),
        };
        let mut e = 0u64;
        assert_eq!(ponko_spawn(w, &part, 1, &mut e), OK);
        e
    }
}

unsafe fn live(ctx: *mut PonkoCtx, e: u64) -> Result<u32, PonkoStatus> {
    unsafe {
        let mut n = 99;
        match ponko_ctx_particle_live_count(ctx, e, &mut n) {
            OK => Ok(n),
            other => Err(other),
        }
    }
}

#[test]
fn the_default_settings_are_a_valid_quiet_emitter() {
    let d = ponko_particle_settings3d_default();
    assert_eq!(d.shape, PonkoParticleShape3D::PONKO_PARTICLES_POINT as u32);
    assert_eq!(d.blend, PonkoParticleBlend3D::PONKO_PARTICLES_ALPHA as u32);
    assert_eq!(d.space, PonkoParticleSpace3D::PONKO_PARTICLES_WORLD as u32);
    assert!(d.size_keys.is_null() && d.size_key_count == 0);
    assert!(d.color_keys.is_null() && d.color_key_count == 0);
    assert!(!d.has_texture);
    assert!(d.max_particles > 0 && d.sheet_columns == 1 && d.sheet_rows == 1);
    assert!(d.lifetime_min > 0.0 && d.lifetime_min <= d.lifetime_max);
    assert!(d.speed_min <= d.speed_max && d.size_min <= d.size_max);
}

unsafe extern "C" fn setup(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        let e = spawn_at(ctx);
        let defaults = ponko_particle_settings3d_default();

        // Nothing there yet: reads and bursts name the missing emitter.
        assert_eq!(live(ctx, e), Err(BAD));
        assert_eq!(ponko_ctx_particle_burst(ctx, e, 5), BAD);

        // An emitter with the defaults, then one with every option filled in.
        assert_eq!(ponko_ctx_set_particle_emitter3d(ctx, e, &defaults), OK);
        assert_eq!(live(ctx, e), Ok(0));
        assert_eq!(ponko_ctx_particle_burst(ctx, e, 5), OK);
        assert_eq!(ponko_ctx_particle_burst(ctx, e, 0), OK);
        assert_eq!(
            ponko_ctx_particle_burst(ctx, e, u32::MAX),
            OK,
            "capped by the pool, not an error"
        );

        let size_keys = [
            PonkoParticleKey { t: 0.0, value: 0.2 },
            PonkoParticleKey { t: 1.0, value: 1.0 },
        ];
        let color = |r, a| PonkoColor { r, g: 0.5, b: 0.0, a };
        let color_keys = [
            PonkoParticleColorKey {
                t: 0.0,
                color: color(1.0, 1.0),
            },
            PonkoParticleColorKey {
                t: 1.0,
                color: color(0.2, 0.0),
            },
        ];
        let mut full = defaults;
        full.rate = 20.0;
        full.burst = 8;
        full.duration = 2.0;
        full.looping = false;
        full.lifetime_min = 0.5;
        full.lifetime_max = 1.5;
        full.speed_min = 1.0;
        full.speed_max = 3.0;
        full.shape = PonkoParticleShape3D::PONKO_PARTICLES_CONE as u32;
        full.shape_size = v3(0.4, 0.0, 0.0);
        full.gravity = v3(0.0, -9.81, 0.0);
        full.drag = 0.1;
        full.size_min = 0.05;
        full.size_max = 0.1;
        full.rotation_min = -1.0;
        full.rotation_max = 1.0;
        full.size_keys = size_keys.as_ptr();
        full.size_key_count = 2;
        full.color_keys = color_keys.as_ptr();
        full.color_key_count = 2;
        full.blend = PonkoParticleBlend3D::PONKO_PARTICLES_ADDITIVE as u32;
        full.space = PonkoParticleSpace3D::PONKO_PARTICLES_LOCAL as u32;
        full.max_particles = 64;
        full.seed = 7;
        assert_eq!(
            ponko_ctx_set_particle_emitter3d(ctx, e, &full),
            OK,
            "replaces the first"
        );
        assert_eq!(live(ctx, e), Ok(0));
        // Every shape.
        for shape in [
            PonkoParticleShape3D::PONKO_PARTICLES_POINT,
            PonkoParticleShape3D::PONKO_PARTICLES_SPHERE,
            PonkoParticleShape3D::PONKO_PARTICLES_CONE,
            PonkoParticleShape3D::PONKO_PARTICLES_BOX,
        ] {
            let mut s = full;
            s.shape = shape as u32;
            assert_eq!(ponko_ctx_set_particle_emitter3d(ctx, e, &s), OK);
        }

        // Settings the engine would panic on are errors instead.
        let bad = |what: &str, change: &dyn Fn(&mut PonkoParticleSettings3D)| {
            let mut s = defaults;
            change(&mut s);
            assert_eq!(ponko_ctx_set_particle_emitter3d(ctx, e, &s), BAD, "{what}");
        };
        bad("lifetime of zero", &|s| s.lifetime_min = 0.0);
        bad("lifetime backwards", &|s| {
            s.lifetime_min = 2.0;
            s.lifetime_max = 1.0
        });
        bad("negative speed", &|s| s.speed_min = -1.0);
        bad("NaN speed", &|s| s.speed_max = f32::NAN);
        bad("negative size", &|s| s.size_min = -0.1);
        bad("rotation backwards", &|s| {
            s.rotation_min = 1.0;
            s.rotation_max = -1.0
        });
        bad("negative rate", &|s| s.rate = -1.0);
        bad("infinite duration", &|s| s.duration = f32::INFINITY);
        bad("negative drag", &|s| s.drag = -1.0);
        bad("NaN gravity", &|s| s.gravity = v3(f32::NAN, 0.0, 0.0));
        bad("no pool", &|s| s.max_particles = 0);
        bad("empty sheet columns", &|s| s.sheet_columns = 0);
        bad("empty sheet rows", &|s| s.sheet_rows = 0);
        bad("unknown shape", &|s| s.shape = 9);
        bad("unknown blend", &|s| s.blend = 9);
        bad("unknown space", &|s| s.space = 9);
        bad("size keys but no pointer", &|s| s.size_key_count = 2);
        bad("color keys but no pointer", &|s| s.color_key_count = 2);
        let unordered = [
            PonkoParticleKey { t: 1.0, value: 1.0 },
            PonkoParticleKey { t: 0.0, value: 0.0 },
        ];
        // Keys out of order or outside 0..=1 are refused with a message, not a panic.
        bad("unordered size keys", &|s| {
            s.size_keys = unordered.as_ptr();
            s.size_key_count = 2
        });
        let late = [PonkoParticleKey { t: 1.5, value: 1.0 }];
        bad("a key past the end", &|s| {
            s.size_keys = late.as_ptr();
            s.size_key_count = 1
        });
        let backwards = [
            PonkoParticleColorKey {
                t: 0.5,
                color: PonkoColor {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
            },
            PonkoParticleColorKey {
                t: 0.5,
                color: PonkoColor {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.0,
                },
            },
        ];
        bad("two color keys at one time", &|s| {
            s.color_keys = backwards.as_ptr();
            s.color_key_count = 2
        });
        let infinite = [PonkoParticleKey {
            t: 0.0,
            value: f32::INFINITY,
        }];
        bad("infinite curve key", &|s| {
            s.size_keys = infinite.as_ptr();
            s.size_key_count = 1
        });
        // A bad replacement leaves the working emitter alone.
        assert_eq!(live(ctx, e), Ok(0));
        assert_eq!(ponko_ctx_particle_burst(ctx, e, 3), OK);

        // Entity and pointer errors.
        assert_eq!(ponko_ctx_set_particle_emitter3d(ctx, 12345, &defaults), BAD);
        assert_eq!(ponko_ctx_set_particle_emitter3d(ctx, e, std::ptr::null()), BAD);
        assert_eq!(ponko_ctx_set_particle_emitter3d(null_mut(), e, &defaults), BAD);
        assert_eq!(ponko_ctx_particle_burst(ctx, 12345, 1), BAD);
        assert_eq!(ponko_ctx_particle_burst(null_mut(), e, 1), BAD);
        assert_eq!(ponko_ctx_particle_live_count(ctx, e, null_mut()), BAD);
        assert_eq!(live(ctx, 12345), Err(BAD));
        assert_eq!(live(null_mut(), e), Err(BAD));

        // Taking it off: the entity has no emitter, and removing twice is fine.
        assert_eq!(ponko_ctx_remove_particle_emitter3d(ctx, e), OK);
        assert_eq!(live(ctx, e), Err(BAD));
        assert_eq!(ponko_ctx_remove_particle_emitter3d(ctx, e), OK);
        assert_eq!(ponko_ctx_remove_particle_emitter3d(ctx, 12345), BAD);
        assert_eq!(ponko_ctx_remove_particle_emitter3d(null_mut(), e), BAD);
        // And a new one can go back on.
        assert_eq!(ponko_ctx_set_particle_emitter3d(ctx, e, &defaults), OK);
        assert_eq!(live(ctx, e), Ok(0));
        run.done = true;
    }
}

#[test]
fn emitters_take_settings_bursts_and_removal_from_c() {
    let mut run = Run { done: false };
    unsafe {
        let g = ponko_game_new(c"particles".as_ptr(), 1, 1);
        let mut world = null_mut();
        let user = (&raw mut run).cast();
        assert_eq!(ponko_game_run_headless(g, 5, Some(setup), user, &mut world), OK);
        ponko_world_free(world);
    }
    assert!(run.done);
}
