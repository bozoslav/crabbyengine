//! Attachments from C: PonkoAttached and ponko_game_follow_attachments.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use PonkoStatus::*;

#[derive(Default)]
struct Ids {
    transform: u32,
    attached: u32,
    base: u64,
    bar: u64,
    orphan: u64,
}

fn value<T>(component: u32, v: &T) -> PonkoValue {
    PonkoValue {
        component,
        data: (v as *const T).cast(),
        size: size_of::<T>(),
    }
}

unsafe extern "C" fn setup(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let w = ponko_ctx_world(ctx);
        assert_eq!(ponko_transform_component(w, &mut ids.transform), PONKO_OK);
        assert_eq!(ponko_attached_component(w, &mut ids.attached), PONKO_OK);
        let at = PonkoTransform2D {
            position: PonkoVec2 { x: 10.0, y: 0.0 },
            ..Default::default()
        };
        assert_eq!(ponko_spawn(w, &value(ids.transform, &at), 1, &mut ids.base), PONKO_OK);
        let doomed_at = PonkoTransform2D::default();
        let mut doomed = 0;
        assert_eq!(
            ponko_spawn(w, &value(ids.transform, &doomed_at), 1, &mut doomed),
            PONKO_OK
        );
        let entity = |bits: u64| PonkoEntity {
            index: bits as u32,
            generation: (bits >> 32) as u32,
        };
        let bar = PonkoAttached {
            to: entity(ids.base),
            offset: PonkoVec2 { x: 0.0, y: -5.0 },
            rotation: 0.0,
        };
        let values = [value(ids.transform, &doomed_at), value(ids.attached, &bar)];
        assert_eq!(ponko_spawn(w, values.as_ptr(), 2, &mut ids.bar), PONKO_OK);
        let orphan = PonkoAttached {
            to: entity(doomed),
            ..bar
        };
        let values = [value(ids.transform, &doomed_at), value(ids.attached, &orphan)];
        assert_eq!(ponko_spawn(w, values.as_ptr(), 2, &mut ids.orphan), PONKO_OK);
        assert_eq!(ponko_despawn(w, doomed), PONKO_OK);
    }
}

unsafe extern "C" fn push_base(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let ids = &*user.cast::<Ids>();
        let w = ponko_ctx_world(ctx);
        let mut t = PonkoTransform2D::default();
        let size = size_of::<PonkoTransform2D>();
        assert_eq!(
            ponko_get(w, ids.base, ids.transform, (&raw mut t).cast(), size),
            PONKO_OK
        );
        t.position.x += 1.0;
        assert_eq!(
            ponko_set(w, ids.base, ids.transform, (&raw const t).cast(), size),
            PONKO_OK
        );
    }
}

#[test]
fn c_attachments_follow_and_orphans_go() {
    let mut ids = Ids::default();
    let user = (&raw mut ids).cast::<c_void>();
    let mut world = null_mut();
    unsafe {
        let g = ponko_game_new(c"attach".as_ptr(), 1, 1);
        assert_eq!(
            ponko_game_sim_system(g, c"push".as_ptr(), 1, Some(push_base), user),
            PONKO_OK
        );
        assert_eq!(ponko_game_follow_attachments(g), PONKO_OK);
        assert_eq!(ponko_game_run_headless(g, 3, Some(setup), user, &mut world), PONKO_OK);
        let mut t = PonkoTransform2D::default();
        let size = size_of::<PonkoTransform2D>();
        assert_eq!(
            ponko_get(world, ids.bar, ids.transform, (&raw mut t).cast(), size),
            PONKO_OK
        );
        assert_eq!((t.position.x, t.position.y), (13.0, -5.0));
        let mut alive = true;
        assert_eq!(ponko_is_alive(world, ids.orphan, &mut alive), PONKO_OK);
        assert!(!alive, "attached to a despawned entity: despawned too");
        ponko_world_free(world);
    }
}
