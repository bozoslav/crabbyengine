//! Attachments from C: CrabbyAttached and crabby_game_follow_attachments.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use CrabbyStatus::*;

#[derive(Default)]
struct Ids {
    transform: u32,
    attached: u32,
    base: u64,
    bar: u64,
    orphan: u64,
}

fn value<T>(component: u32, v: &T) -> CrabbyValue {
    CrabbyValue {
        component,
        data: (v as *const T).cast(),
        size: size_of::<T>(),
    }
}

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let w = crabby_ctx_world(ctx);
        assert_eq!(crabby_transform_component(w, &mut ids.transform), CRABBY_OK);
        assert_eq!(crabby_attached_component(w, &mut ids.attached), CRABBY_OK);
        let at = CrabbyTransform2D {
            position: CrabbyVec2 { x: 10.0, y: 0.0 },
            ..Default::default()
        };
        assert_eq!(crabby_spawn(w, &value(ids.transform, &at), 1, &mut ids.base), CRABBY_OK);
        let doomed_at = CrabbyTransform2D::default();
        let mut doomed = 0;
        assert_eq!(
            crabby_spawn(w, &value(ids.transform, &doomed_at), 1, &mut doomed),
            CRABBY_OK
        );
        let entity = |bits: u64| CrabbyEntity {
            index: bits as u32,
            generation: (bits >> 32) as u32,
        };
        let bar = CrabbyAttached {
            to: entity(ids.base),
            offset: CrabbyVec2 { x: 0.0, y: -5.0 },
            rotation: 0.0,
        };
        let values = [value(ids.transform, &doomed_at), value(ids.attached, &bar)];
        assert_eq!(crabby_spawn(w, values.as_ptr(), 2, &mut ids.bar), CRABBY_OK);
        let orphan = CrabbyAttached {
            to: entity(doomed),
            ..bar
        };
        let values = [value(ids.transform, &doomed_at), value(ids.attached, &orphan)];
        assert_eq!(crabby_spawn(w, values.as_ptr(), 2, &mut ids.orphan), CRABBY_OK);
        assert_eq!(crabby_despawn(w, doomed), CRABBY_OK);
    }
}

unsafe extern "C" fn push_base(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &*user.cast::<Ids>();
        let w = crabby_ctx_world(ctx);
        let mut t = CrabbyTransform2D::default();
        let size = size_of::<CrabbyTransform2D>();
        assert_eq!(
            crabby_get(w, ids.base, ids.transform, (&raw mut t).cast(), size),
            CRABBY_OK
        );
        t.position.x += 1.0;
        assert_eq!(
            crabby_set(w, ids.base, ids.transform, (&raw const t).cast(), size),
            CRABBY_OK
        );
    }
}

#[test]
fn c_attachments_follow_and_orphans_go() {
    let mut ids = Ids::default();
    let user = (&raw mut ids).cast::<c_void>();
    let mut world = null_mut();
    unsafe {
        let g = crabby_game_new(c"attach".as_ptr(), 1, 1);
        assert_eq!(
            crabby_game_sim_system(g, c"push".as_ptr(), 1, Some(push_base), user),
            CRABBY_OK
        );
        assert_eq!(crabby_game_follow_attachments(g), CRABBY_OK);
        assert_eq!(crabby_game_run_headless(g, 3, Some(setup), user, &mut world), CRABBY_OK);
        let mut t = CrabbyTransform2D::default();
        let size = size_of::<CrabbyTransform2D>();
        assert_eq!(
            crabby_get(world, ids.bar, ids.transform, (&raw mut t).cast(), size),
            CRABBY_OK
        );
        assert_eq!((t.position.x, t.position.y), (13.0, -5.0));
        let mut alive = true;
        assert_eq!(crabby_is_alive(world, ids.orphan, &mut alive), CRABBY_OK);
        assert!(!alive, "attached to a despawned entity: despawned too");
        crabby_world_free(world);
    }
}
