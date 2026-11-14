//! 2D particle emitters from C: settings are checked, emitters go on and come
//! off entities, and advancing one by hand (`ponko_ctx_particle_update2d`)
//! emits its burst and caps it at `max_particles`.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use PonkoStatus::*;

const OK: PonkoStatus = PONKO_OK;
const BAD: PonkoStatus = PONKO_INVALID_ARGUMENT;

struct Run {
    done: bool,
}

unsafe fn spawn_at(ctx: *mut PonkoCtx) -> u64 {
    unsafe {
        let w = ponko_ctx_world(ctx);
        let mut transform = 0;
        assert_eq!(ponko_transform_component(w, &mut transform), OK);
        let t = PonkoTransform2D {
            position: PonkoVec2 { x: 40.0, y: 30.0 },
            ..Default::default()
        };
        let part = PonkoValue {
            component: transform,
            data: (&raw const t).cast(),
            size: size_of::<PonkoTransform2D>(),
        };
        let mut e = 0u64;
        assert_eq!(ponko_spawn(w, &part, 1, &mut e), OK);
        e
    }
}

unsafe fn live(ctx: *mut PonkoCtx, e: u64) -> Result<u32, PonkoStatus> {
    unsafe {
        let mut n = 99;
        match ponko_ctx_particle_live_count2d(ctx, e, &mut n) {
            OK => Ok(n),
            other => Err(other),
        }
    }
}

#[test]
fn the_default_2d_settings_are_a_valid_emitter() {
    let d = ponko_particle_settings2d_default();
    assert_eq!(d.shape, PonkoParticleShape2D::PONKO_PARTICLES2D_POINT as u32);
    assert_eq!(d.blend, PONKO_BLEND_ALPHA);
    assert_eq!(d.space, PonkoParticleSpace3D::PONKO_PARTICLES_WORLD as u32);
    assert_eq!(d.material.id, PONKO_HANDLE_NONE);
    assert!(d.size_keys.is_null() && d.size_key_count == 0);
    assert!(d.color_keys.is_null() && d.color_key_count == 0);
    assert!(!d.has_texture);
    assert!(d.max_particles > 0 && d.sheet_columns == 1 && d.sheet_rows == 1);
    assert!(d.lifetime_min > 0.0 && d.lifetime_min <= d.lifetime_max);
}

unsafe extern "C" fn setup(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        let e = spawn_at(ctx);
        let defaults = ponko_particle_settings2d_default();

        // Nothing there yet: reads, bursts and updates name the missing emitter.
        assert_eq!(live(ctx, e), Err(BAD));
        assert_eq!(ponko_ctx_particle_burst2d(ctx, e, 5), BAD);
        assert_eq!(ponko_ctx_particle_update2d(ctx, e, 0.1), BAD);

        // A burst of 12 that never emits on its own, capped at 64.
        let mut sparks = defaults;
        sparks.rate = 0.0;
        sparks.burst = 12;
        sparks.looping = false;
        sparks.lifetime_min = 5.0;
        sparks.lifetime_max = 5.0;
        sparks.shape = PonkoParticleShape2D::PONKO_PARTICLES2D_CONE as u32;
        sparks.shape_size = PonkoVec2 { x: 0.5, y: 0.0 };
        sparks.blend = PONKO_BLEND_ADDITIVE;
        sparks.max_particles = 64;
        assert_eq!(ponko_ctx_set_particle_emitter2d(ctx, e, &sparks), OK);
        assert_eq!(live(ctx, e), Ok(0), "nothing emits before an update");
        assert_eq!(ponko_ctx_particle_update2d(ctx, e, 0.0), OK);
        assert_eq!(live(ctx, e), Ok(12), "the first update emits the burst");
        assert_eq!(ponko_ctx_particle_burst2d(ctx, e, 100), OK);
        assert_eq!(ponko_ctx_particle_update2d(ctx, e, 0.1), OK);
        assert_eq!(live(ctx, e), Ok(64), "capped by max_particles");

        // Every shape is accepted, and setting replaces the emitter and its particles.
        for shape in [
            PonkoParticleShape2D::PONKO_PARTICLES2D_POINT,
            PonkoParticleShape2D::PONKO_PARTICLES2D_CIRCLE,
            PonkoParticleShape2D::PONKO_PARTICLES2D_CONE,
            PonkoParticleShape2D::PONKO_PARTICLES2D_RECT,
        ] {
            let mut s = sparks;
            s.shape = shape as u32;
            assert_eq!(ponko_ctx_set_particle_emitter2d(ctx, e, &s), OK);
            assert_eq!(live(ctx, e), Ok(0));
        }

        // Settings the engine would panic on are errors instead.
        let bad = |what: &str, change: &dyn Fn(&mut PonkoParticleSettings2D)| {
            let mut s = defaults;
            change(&mut s);
            assert_eq!(ponko_ctx_set_particle_emitter2d(ctx, e, &s), BAD, "{what}");
        };
        bad("lifetime of zero", &|s| s.lifetime_min = 0.0);
        bad("size backwards", &|s| {
            s.size_min = 2.0;
            s.size_max = 1.0
        });
        bad("no pool", &|s| s.max_particles = 0);
        bad("unknown shape", &|s| s.shape = 9);
        bad("unknown blend", &|s| s.blend = 9);
        bad("unknown space", &|s| s.space = 9);
        bad("NaN z", &|s| s.z = f32::NAN);

        // Entity and pointer errors.
        assert_eq!(ponko_ctx_set_particle_emitter2d(ctx, 12345, &defaults), BAD);
        assert_eq!(ponko_ctx_set_particle_emitter2d(ctx, e, std::ptr::null()), BAD);
        assert_eq!(ponko_ctx_set_particle_emitter2d(null_mut(), e, &defaults), BAD);
        assert_eq!(ponko_ctx_particle_live_count2d(ctx, e, null_mut()), BAD);
        assert_eq!(ponko_ctx_particle_update2d(null_mut(), e, 0.1), BAD);

        // Taking it off: the entity has no emitter, and removing twice is fine.
        assert_eq!(ponko_ctx_remove_particle_emitter2d(ctx, e), OK);
        assert_eq!(live(ctx, e), Err(BAD));
        assert_eq!(ponko_ctx_particle_burst2d(ctx, e, 1), BAD);
        assert_eq!(ponko_ctx_remove_particle_emitter2d(ctx, e), OK);
        assert_eq!(ponko_ctx_remove_particle_emitter2d(ctx, 12345), BAD);
        assert_eq!(ponko_ctx_remove_particle_emitter2d(null_mut(), e), BAD);
        run.done = true;
    }
}

#[test]
fn emitters_2d_take_settings_updates_and_removal_from_c() {
    let mut run = Run { done: false };
    unsafe {
        let g = ponko_game_new(c"particles 2d".as_ptr(), 1, 1);
        let mut world = null_mut();
        let user = (&raw mut run).cast();
        assert_eq!(ponko_game_run_headless(g, 1, Some(setup), user, &mut world), OK);
        ponko_world_free(world);
    }
    assert!(run.done);
}
