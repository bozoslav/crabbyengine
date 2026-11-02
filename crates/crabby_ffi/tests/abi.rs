use crabby_ffi::*;
use std::ptr::{null, null_mut};

#[test]
fn components_round_trip_and_stale_entities_stay_dead() {
    unsafe {
        let w = crabby_world_new();
        let mut id = 0;
        assert_eq!(
            crabby_component_register(w, c"position".as_ptr(), 4, 4, &mut id),
            CrabbyStatus::CRABBY_OK
        );
        let value = 42u32;
        let v = CrabbyValue {
            component: id,
            data: (&value as *const u32).cast(),
            size: 4,
        };
        let mut e = 0;
        assert_eq!(crabby_spawn(w, &v, 1, &mut e), CrabbyStatus::CRABBY_OK);
        let mut out = 0u32;
        assert_eq!(
            crabby_get(w, e, id, (&mut out as *mut u32).cast(), 4),
            CrabbyStatus::CRABBY_OK
        );
        assert_eq!(out, value);
        assert_eq!(crabby_despawn(w, e), CrabbyStatus::CRABBY_OK);
        let mut alive = true;
        assert_eq!(crabby_is_alive(w, e, &mut alive), CrabbyStatus::CRABBY_OK);
        assert!(!alive);
        assert_eq!(crabby_world_free(w), CrabbyStatus::CRABBY_OK);
    }
}

#[test]
fn invalid_arguments_are_errors() {
    unsafe {
        let w = crabby_world_new();
        let mut id = 0;
        assert_eq!(
            crabby_component_register(w, c"bad".as_ptr(), 3, 2, &mut id),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_component_register(w, c"bad".as_ptr(), 4, 3, &mut id),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_world_len(null_mut(), &mut 0),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_spawn(w, null(), 1, &mut 0),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(crabby_world_free(w), CrabbyStatus::CRABBY_OK);
    }
}

#[test]
fn copy_sizes_ids_and_duplicate_values_are_validated() {
    unsafe {
        let w = crabby_world_new();
        let mut id = 99;
        assert_eq!(
            crabby_component_register(w, c"x".as_ptr(), 4, 4, &mut id),
            CrabbyStatus::CRABBY_OK
        );
        let mut same = 99;
        assert_eq!(
            crabby_component_register(w, c"x".as_ptr(), 4, 4, &mut same),
            CrabbyStatus::CRABBY_OK
        );
        assert_eq!(id, same);
        assert_eq!(
            crabby_component_register(w, c"x".as_ptr(), 8, 8, &mut same),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_component_register(w, c"".as_ptr(), 1, 1, &mut same),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        let mut e = 0;
        assert_eq!(crabby_spawn(w, null(), 0, &mut e), CrabbyStatus::CRABBY_OK);
        let x = 77u32;
        assert_eq!(
            crabby_set(w, e, id, (&x as *const u32).cast(), 3),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_set(w, e, 999, (&x as *const u32).cast(), 4),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(crabby_set(w, e, id, null(), 4), CrabbyStatus::CRABBY_INVALID_ARGUMENT);
        assert_eq!(
            crabby_set(w, e, id, (&x as *const u32).cast(), 4),
            CrabbyStatus::CRABBY_OK
        );
        let y = 88u32;
        assert_eq!(
            crabby_set(w, e, id, (&y as *const u32).cast(), 4),
            CrabbyStatus::CRABBY_OK
        );
        let mut out = 0u32;
        assert_eq!(
            crabby_get(w, e, id, (&mut out as *mut u32).cast(), 3),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_get(w, e, id, (&mut out as *mut u32).cast(), 4),
            CrabbyStatus::CRABBY_OK
        );
        assert_eq!(out, 88);
        let v = CrabbyValue {
            component: id,
            data: (&x as *const u32).cast(),
            size: 4,
        };
        assert_eq!(
            crabby_spawn(w, [v, v].as_ptr(), 2, &mut 0),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(crabby_remove(w, e, id), CrabbyStatus::CRABBY_OK);
        assert_eq!(crabby_remove(w, e, id), CrabbyStatus::CRABBY_NOT_FOUND);
        assert_eq!(
            crabby_get(w, e, id, (&mut out as *mut u32).cast(), 4),
            CrabbyStatus::CRABBY_NOT_FOUND
        );
        assert_eq!(crabby_despawn(w, e), CrabbyStatus::CRABBY_OK);
        assert_eq!(
            crabby_set(w, e, id, (&x as *const u32).cast(), 4),
            CrabbyStatus::CRABBY_NOT_FOUND
        );
        assert_eq!(crabby_world_free(w), CrabbyStatus::CRABBY_OK);
    }
}

struct Visit {
    world: *mut CrabbyWorld,
    rows: usize,
    entities: Vec<u64>,
    reentry: Vec<CrabbyStatus>,
}
unsafe extern "C" fn update(chunk: *const CrabbyChunk, user: *mut std::ffi::c_void) {
    unsafe {
        let c = &*chunk;
        let visit = &mut *user.cast::<Visit>();
        visit.rows += c.count;
        for i in 0..c.count {
            visit.entities.push(crabby_entity_bits(*c.entities.add(i)));
            if c.column_count == 2 {
                let velocity = *(*c.columns).cast::<u32>().add(i);
                *(*c.columns.add(1)).cast::<u32>().add(i) += velocity;
            }
        }
        visit.reentry.push(crabby_world_len(visit.world, &mut 0));
        visit.reentry.push(crabby_world_free(visit.world));
        visit.reentry.push(crabby_despawn(visit.world, visit.entities[0]));
        visit
            .reentry
            .push(crabby_query(visit.world, null(), 0, Some(update), user));
    }
}

#[test]
fn query_preserves_column_order_mutates_values_and_rejects_reentry() {
    unsafe {
        let w = crabby_world_new();
        let (mut p, mut v) = (0, 0);
        crabby_component_register(w, c"p".as_ptr(), 4, 4, &mut p);
        crabby_component_register(w, c"v".as_ptr(), 4, 4, &mut v);
        let (position, velocity) = (10u32, 3u32);
        let values = [
            CrabbyValue {
                component: p,
                data: (&position as *const u32).cast(),
                size: 4,
            },
            CrabbyValue {
                component: v,
                data: (&velocity as *const u32).cast(),
                size: 4,
            },
        ];
        let mut entity = 0;
        crabby_spawn(w, values.as_ptr(), 2, &mut entity);
        let mut visit = Visit {
            world: w,
            rows: 0,
            entities: vec![],
            reentry: vec![],
        };
        assert_eq!(
            crabby_query(w, [v, p].as_ptr(), 2, Some(update), (&mut visit as *mut Visit).cast()),
            CrabbyStatus::CRABBY_OK
        );
        assert_eq!(visit.rows, 1);
        assert_eq!(visit.entities, [entity]);
        assert!(visit.reentry.iter().all(|s| *s == CrabbyStatus::CRABBY_BUSY));
        let mut out = 0u32;
        assert_eq!(
            crabby_get(w, entity, p, (&mut out as *mut u32).cast(), 4),
            CrabbyStatus::CRABBY_OK
        );
        assert_eq!(out, 13);
        assert_eq!(
            crabby_query(w, [p, p].as_ptr(), 2, Some(update), null_mut()),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_query(w, [999].as_ptr(), 1, Some(update), null_mut()),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_query(w, null(), 33, Some(update), null_mut()),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_query(w, null(), 0, None, null_mut()),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        visit.rows = 0;
        assert_eq!(
            crabby_query(w, null(), 0, Some(update), (&mut visit as *mut Visit).cast()),
            CrabbyStatus::CRABBY_OK
        );
        assert_eq!(visit.rows, 1);
        crabby_world_free(w);
    }
}

#[test]
fn empty_world_queries_do_not_call_back() {
    unsafe extern "C" fn count(_: *const CrabbyChunk, user: *mut std::ffi::c_void) {
        unsafe {
            *user.cast::<usize>() += 1;
        }
    }
    unsafe {
        let w = crabby_world_new();
        let mut calls = 0usize;
        assert_eq!(
            crabby_query(w, null(), 0, Some(count), (&mut calls as *mut usize).cast()),
            CrabbyStatus::CRABBY_OK
        );
        assert_eq!(calls, 0);
        crabby_world_free(w);
    }
}

#[test]
fn null_outputs_overflow_counts_and_zero_sized_values_are_handled() {
    unsafe {
        let w = crabby_world_new();
        assert_eq!(crabby_world_free(null_mut()), CrabbyStatus::CRABBY_INVALID_ARGUMENT);
        assert_eq!(crabby_world_len(w, null_mut()), CrabbyStatus::CRABBY_INVALID_ARGUMENT);
        assert_eq!(crabby_is_alive(w, 0, null_mut()), CrabbyStatus::CRABBY_INVALID_ARGUMENT);
        assert_eq!(
            crabby_component_register(w, null(), 1, 1, &mut 0),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_component_register(w, c"x".as_ptr(), 1, 1, null_mut()),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        let invalid_utf8 = [0xffu8, 0];
        assert_eq!(
            crabby_component_register(w, invalid_utf8.as_ptr().cast(), 1, 1, &mut 0),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_component_register(w, c"x".as_ptr(), usize::MAX, 1, &mut 0),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_spawn(w, null(), usize::MAX, &mut 0),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_spawn(w, null(), 0, null_mut()),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        let mut id = 0;
        assert_eq!(
            crabby_component_register(w, c"tag".as_ptr(), 0, 8, &mut id),
            CrabbyStatus::CRABBY_OK
        );
        let mut dummy = 0u64;
        let value = CrabbyValue {
            component: id,
            data: (&dummy as *const u64).cast(),
            size: 0,
        };
        let mut entity = 0;
        assert_eq!(crabby_spawn(w, &value, 1, &mut entity), CrabbyStatus::CRABBY_OK);
        assert_eq!(
            crabby_get(w, entity, id, (&mut dummy as *mut u64).cast(), 0),
            CrabbyStatus::CRABBY_OK
        );
        crabby_world_free(w);
    }
}

#[test]
fn last_error_is_thread_local_and_success_preserves_it() {
    unsafe {
        crabby_world_free(null_mut());
    }
    let text = unsafe { std::ffi::CStr::from_ptr(crabby_last_error()) }
        .to_bytes()
        .to_vec();
    assert!(!text.is_empty());
    assert_eq!(crabby_abi_version(), 2);
    let child = std::thread::spawn(|| {
        unsafe { std::ffi::CStr::from_ptr(crabby_last_error()) }
            .to_bytes()
            .to_vec()
    })
    .join()
    .unwrap();
    assert!(child.is_empty());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(crabby_last_error()) }.to_bytes(),
        text
    );
}
