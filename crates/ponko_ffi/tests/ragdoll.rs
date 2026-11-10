//! Ragdolls from C: a skeleton handed over to physics bodies and jointed
//! together, pushed, and handed back. Headless, with 3D physics stepping.
#![cfg(feature = "physics3d")]
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use PonkoStatus::*;

const OK: PonkoStatus = PONKO_OK;
const BAD: PonkoStatus = PONKO_INVALID_ARGUMENT;

fn v3(x: f32, y: f32, z: f32) -> PonkoVec3 {
    PonkoVec3 { x, y, z }
}

#[derive(Default)]
struct Run {
    transform: u32,
    /// Has all three bones.
    full: u64,
    /// Has only the hips and the head.
    partial: u64,
    hips_body: u64,
    start_y: f32,
    fell_to: f32,
    joints_while_standing: usize,
    joints_after_stop: usize,
    checked_stop: bool,
}

unsafe fn spawn_at(ctx: *mut PonkoCtx, transform: u32, at: PonkoVec3) -> u64 {
    unsafe {
        let t = ponko_transform3d_at(at);
        let part = PonkoValue {
            component: transform,
            data: (&raw const t).cast(),
            size: size_of::<PonkoTransform3D>(),
        };
        let mut e = 0u64;
        assert_eq!(ponko_spawn(ponko_ctx_world(ctx), &part, 1, &mut e), OK);
        e
    }
}

/// `hips`, `spine` half a meter above, `head` half a meter above that.
unsafe fn skeleton(ctx: *mut PonkoCtx, e: u64) {
    unsafe {
        let joint = |name, parent, y| PonkoJoint {
            name,
            parent,
            rest: ponko_transform3d_at(v3(0.0, y, 0.0)),
        };
        let joints = [
            joint(c"hips".as_ptr(), -1, 0.0),
            joint(c"spine".as_ptr(), 0, 0.5),
            joint(c"head".as_ptr(), 1, 0.5),
        ];
        assert_eq!(ponko_ctx_set_skeleton(ctx, e, joints.as_ptr(), 3), OK);
    }
}

unsafe fn ragdoll_state(ctx: *mut PonkoCtx, e: u64) -> (bool, bool) {
    unsafe {
        let (mut ragdolled, mut blending) = (true, true);
        assert_eq!(ponko_ctx_ragdoll_state(ctx, e, &mut ragdolled, &mut blending), OK);
        (ragdolled, blending)
    }
}

unsafe fn body(ctx: *mut PonkoCtx, e: u64, joint: usize) -> Result<u64, PonkoStatus> {
    unsafe {
        let mut out = 0u64;
        match ponko_ctx_ragdoll_body(ctx, e, joint, &mut out) {
            OK => Ok(out),
            other => Err(other),
        }
    }
}

unsafe fn alive(ctx: *mut PonkoCtx, e: u64) -> bool {
    unsafe {
        let mut yes = true;
        assert_eq!(ponko_is_alive(ponko_ctx_world(ctx), e, &mut yes), OK);
        yes
    }
}

unsafe fn y_of(ctx: *mut PonkoCtx, transform: u32, e: u64) -> f32 {
    unsafe {
        let mut t = ponko_transform3d_at(v3(0.0, 0.0, 0.0));
        let size = size_of::<PonkoTransform3D>();
        assert_eq!(
            ponko_get(ponko_ctx_world(ctx), e, transform, (&raw mut t).cast(), size),
            OK
        );
        t.position.y
    }
}

unsafe extern "C" fn setup(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        assert_eq!(
            ponko_transform3d_component(ponko_ctx_world(ctx), &mut run.transform),
            OK
        );
        run.full = spawn_at(ctx, run.transform, v3(0.0, 10.0, 0.0));
        skeleton(ctx, run.full);
        run.partial = spawn_at(ctx, run.transform, v3(5.0, 10.0, 0.0));
        skeleton(ctx, run.partial);
        let bare = spawn_at(ctx, run.transform, v3(9.0, 10.0, 0.0));
        let (full, partial) = (run.full, run.partial);

        // The bone helper's defaults.
        let hips = ponko_ragdoll_bone(c"hips".as_ptr(), 0.15);
        let spine = ponko_ragdoll_bone(c"spine".as_ptr(), 0.12);
        let head = ponko_ragdoll_bone(c"head".as_ptr(), 0.1);
        assert_eq!((hips.swing, hips.twist, hips.density), (0.7, 0.3, 1000.0));

        // Nobody is a ragdoll yet.
        assert_eq!(ragdoll_state(ctx, full), (false, false));
        assert_eq!(body(ctx, full, 0), Err(PONKO_NOT_FOUND));

        // Bad input is refused, and leaves the entity alone.
        let all = [hips, spine, head];
        assert_eq!(
            ponko_ctx_start_ragdoll(ctx, 12345, all.as_ptr(), 3),
            BAD,
            "no such entity"
        );
        assert_eq!(ponko_ctx_start_ragdoll(ctx, bare, all.as_ptr(), 3), BAD, "no skeleton");
        assert_eq!(ponko_ctx_start_ragdoll(ctx, full, null(), 3), BAD, "null bones");
        assert_eq!(ponko_ctx_start_ragdoll(null_mut(), full, all.as_ptr(), 3), BAD);
        let tail = [ponko_ragdoll_bone(c"tail".as_ptr(), 0.1)];
        assert_eq!(
            ponko_ctx_start_ragdoll(ctx, full, tail.as_ptr(), 1),
            BAD,
            "unknown joint"
        );
        let twice = [hips, hips];
        assert_eq!(
            ponko_ctx_start_ragdoll(ctx, full, twice.as_ptr(), 2),
            BAD,
            "repeated joint"
        );
        for radius in [0.0, -1.0, f32::NAN] {
            let thin = [ponko_ragdoll_bone(c"hips".as_ptr(), radius)];
            assert_eq!(
                ponko_ctx_start_ragdoll(ctx, full, thin.as_ptr(), 1),
                BAD,
                "radius {radius}"
            );
        }
        let mut light = hips;
        light.density = 0.0;
        assert_eq!(ponko_ctx_start_ragdoll(ctx, full, &light, 1), BAD, "density");
        let nameless = [ponko_ragdoll_bone(null(), 0.1)];
        assert_eq!(ponko_ctx_start_ragdoll(ctx, full, nameless.as_ptr(), 1), BAD);
        assert_eq!(ragdoll_state(ctx, full), (false, false));

        // The whole body.
        assert_eq!(ponko_ctx_start_ragdoll(ctx, full, all.as_ptr(), 3), OK);
        assert_eq!(ragdoll_state(ctx, full), (true, false));
        let bodies: Vec<u64> = (0..3).map(|j| body(ctx, full, j).unwrap()).collect();
        assert!(bodies[0] != bodies[1] && bodies[1] != bodies[2] && bodies[0] != bodies[2]);
        run.hips_body = bodies[0];
        assert_eq!(body(ctx, full, 3), Err(PONKO_NOT_FOUND), "no fourth joint");
        // Bodies can be pushed at once, before any step.
        assert_eq!(ponko_ctx_apply_impulse3d(ctx, bodies[2], v3(1.0, 0.0, 0.0)), OK);
        // Starting again replaces the old one: new bodies, same state.
        assert_eq!(ponko_ctx_start_ragdoll(ctx, full, all.as_ptr(), 3), OK);
        let again = body(ctx, full, 0).unwrap();
        assert_ne!(again, bodies[0]);
        run.hips_body = again;

        // Only some bones: the joints in between get no body.
        let some = [hips, head];
        assert_eq!(ponko_ctx_start_ragdoll(ctx, partial, some.as_ptr(), 2), OK);
        assert!(body(ctx, partial, 0).is_ok());
        assert_eq!(body(ctx, partial, 1), Err(PONKO_NOT_FOUND), "the spine has none");
        assert!(body(ctx, partial, 2).is_ok());

        // Errors reading back.
        let (mut yes, mut blending) = (false, false);
        let mut out = 0u64;
        assert_eq!(ponko_ctx_ragdoll_state(ctx, full, null_mut(), &mut blending), BAD);
        assert_eq!(ponko_ctx_ragdoll_state(ctx, full, &mut yes, null_mut()), BAD);
        assert_eq!(ponko_ctx_ragdoll_state(null_mut(), full, &mut yes, &mut blending), BAD);
        assert_eq!(ponko_ctx_ragdoll_body(ctx, full, 0, null_mut()), BAD);
        assert_eq!(ponko_ctx_ragdoll_body(null_mut(), full, 0, &mut out), BAD);
        assert_eq!(ponko_ctx_stop_ragdoll(ctx, full, -1.0), BAD);
        assert_eq!(ponko_ctx_stop_ragdoll(ctx, full, f32::NAN), BAD);
        assert_eq!(ponko_ctx_stop_ragdoll(null_mut(), full, 0.0), BAD);
        assert_eq!(
            ponko_ctx_stop_ragdoll(ctx, bare, 0.0),
            OK,
            "not a ragdoll: nothing to do"
        );
        assert_eq!(ragdoll_state(ctx, full), (true, false), "bad calls changed nothing");
    }
}

unsafe extern "C" fn step(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        match ponko_ctx_sim_tick(ctx) {
            2 => {
                // Two skeletons: 2 joints for three bones, 1 for two.
                let mut n = 99;
                assert_eq!(ponko_ctx_physics3d_joint_count(ctx, &mut n), OK);
                run.joints_while_standing = n;
                assert_eq!(ponko_ctx_physics3d_joint_count(ctx, null_mut()), BAD);
            }
            30 => {
                run.start_y = 10.0;
                run.fell_to = y_of(ctx, run.transform, run.hips_body);
            }
            40 => {
                // Handed back with a blend: still a ragdoll, now blending.
                assert_eq!(ponko_ctx_stop_ragdoll(ctx, run.full, 0.5), OK);
                assert_eq!(ragdoll_state(ctx, run.full), (true, true));
                // Handed back at once: no longer one, and its bodies are gone.
                let bodies: Vec<u64> = (0..3).filter_map(|j| body(ctx, run.partial, j).ok()).collect();
                assert_eq!(bodies.len(), 2);
                assert_eq!(ponko_ctx_stop_ragdoll(ctx, run.partial, 0.0), OK);
                assert_eq!(ragdoll_state(ctx, run.partial), (false, false));
                assert_eq!(body(ctx, run.partial, 0), Err(PONKO_NOT_FOUND));
                assert_eq!(
                    ponko_ctx_stop_ragdoll(ctx, run.partial, 0.0),
                    OK,
                    "stopping twice is fine"
                );
                for b in bodies {
                    assert!(!alive(ctx, b), "the bodies are removed with the ragdoll");
                }
            }
            42 => {
                let mut n = 99;
                assert_eq!(ponko_ctx_physics3d_joint_count(ctx, &mut n), OK);
                run.joints_after_stop = n;
                run.checked_stop = true;
            }
            _ => {}
        }
    }
}

#[test]
fn ragdolls_fall_under_physics_and_hand_back_to_animation() {
    let mut run = Run::default();
    let user = (&raw mut run).cast::<c_void>();
    unsafe {
        let g = ponko_game_new(c"ragdoll".as_ptr(), 1, 1);
        assert_eq!(ponko_game_fixed_hz(g, 60), OK);
        assert_eq!(ponko_game_physics3d(g, v3(0.0, -9.81, 0.0)), OK);
        assert_eq!(ponko_game_sim_system(g, c"step".as_ptr(), 1, Some(step), user), OK);
        let mut world = null_mut();
        assert_eq!(ponko_game_run_headless(g, 45, Some(setup), user, &mut world), OK);
        ponko_world_free(world);
    }
    assert_eq!(
        run.joints_while_standing, 3,
        "two for the whole body, one for the partial"
    );
    // Free fall for half a second drops a body about 1.2 m.
    assert!(
        run.fell_to < run.start_y - 0.5 && run.fell_to > run.start_y - 3.0,
        "the hips fell from {} to {}",
        run.start_y,
        run.fell_to
    );
    assert!(run.checked_stop);
    assert_eq!(
        run.joints_after_stop, 0,
        "the joints went with the bodies: a blending ragdoll has none"
    );
}
