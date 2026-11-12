//! `ponko_resource_scope`: a resource lent with the world next to it.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use PonkoStatus::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Total(u64);
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Wide([u32; 4]);

unsafe fn set<T: Copy>(w: *mut PonkoWorld, name: &std::ffi::CStr, value: T) {
    let status = unsafe {
        ponko_resource_set(
            w,
            name.as_ptr(),
            (&raw const value).cast(),
            size_of::<T>(),
            align_of::<T>(),
        )
    };
    assert_eq!(status, PONKO_OK);
}
unsafe fn get<T: Copy + Default>(w: *mut PonkoWorld, name: &std::ffi::CStr) -> Result<T, PonkoStatus> {
    let mut out = T::default();
    match unsafe { ponko_resource_get(w, name.as_ptr(), (&raw mut out).cast(), size_of::<T>()) } {
        PONKO_OK => Ok(out),
        s => Err(s),
    }
}
/// Calls the closure at `user` with the scope's world and value.
unsafe extern "C" fn call(world: *mut PonkoWorld, value: *mut c_void, user: *mut c_void) {
    let f = unsafe { &mut *user.cast::<&mut dyn FnMut(*mut PonkoWorld, *mut c_void)>() };
    f(world, value);
}
unsafe fn scope(
    w: *mut PonkoWorld,
    name: &std::ffi::CStr,
    size: usize,
    mut f: impl FnMut(*mut PonkoWorld, *mut c_void),
) -> PonkoStatus {
    let mut f: &mut dyn FnMut(*mut PonkoWorld, *mut c_void) = &mut f;
    unsafe { ponko_resource_scope(w, name.as_ptr(), size, Some(call), (&raw mut f).cast()) }
}

#[test]
fn a_scope_changes_the_resource_in_place_with_the_world_free() {
    unsafe {
        let w = ponko_world_new();
        let mut value_id = 0;
        assert_eq!(
            ponko_component_register(w, c"Value".as_ptr(), 8, 8, &mut value_id),
            PONKO_OK
        );
        for n in [3u64, 4] {
            let v = PonkoValue {
                component: value_id,
                data: (&raw const n).cast(),
                size: 8,
            };
            let mut e = 0;
            assert_eq!(ponko_spawn(w, &v, 1, &mut e), PONKO_OK);
        }
        set(w, c"Total", Total(1));
        let status = scope(w, c"Total", 8, |world, value| {
            assert_eq!(get::<Total>(world, c"Total"), Err(PONKO_NOT_FOUND), "lent out");
            set(world, c"Other", 9u32);
            // A query loop that uses the resource: Rust's `with_resources`.
            unsafe extern "C" fn sum(chunk: *const PonkoChunk, user: *mut c_void) {
                let chunk = unsafe { &*chunk };
                let total = unsafe { &mut *user.cast::<Total>() };
                let column = unsafe { *chunk.columns }.cast::<u64>();
                for i in 0..chunk.count {
                    total.0 += unsafe { *column.add(i) };
                }
            }
            assert_eq!(ponko_query(world, &value_id, 1, Some(sum), value), PONKO_OK);
        });
        assert_eq!(status, PONKO_OK);
        assert_eq!(get::<Total>(w, c"Total"), Ok(Total(8)));
        assert_eq!(get::<u32>(w, c"Other"), Ok(9));
        ponko_world_free(w);
    }
}

#[test]
fn scopes_nest_for_different_names_and_not_for_the_same() {
    unsafe {
        let w = ponko_world_new();
        set(w, c"Total", Total(1));
        set(w, c"Wide", Wide([1, 2, 3, 4]));
        let status = scope(w, c"Total", 8, |world, total| {
            let inner = scope(world, c"Wide", 16, |world, wide| {
                assert!(wide.cast::<Wide>().is_aligned(), "aligned for the resource");
                (*wide.cast::<Wide>()).0[0] += (*total.cast::<Total>()).0 as u32;
                (*total.cast::<Total>()).0 += 10;
                assert_eq!(
                    scope(world, c"Total", 8, |_, _| panic!("not lent twice")),
                    PONKO_NOT_FOUND
                );
            });
            assert_eq!(inner, PONKO_OK);
        });
        assert_eq!(status, PONKO_OK);
        assert_eq!(get::<Total>(w, c"Total"), Ok(Total(11)));
        assert_eq!(get::<Wide>(w, c"Wide"), Ok(Wide([2, 2, 3, 4])));
        ponko_world_free(w);
    }
}

#[test]
fn the_scoped_value_replaces_what_was_set_meanwhile() {
    unsafe {
        let w = ponko_world_new();
        set(w, c"Total", Total(5));
        let status = scope(w, c"Total", 8, |world, _| {
            set(world, c"Total", 7u32); // another size, even
        });
        assert_eq!(status, PONKO_OK);
        assert_eq!(get::<Total>(w, c"Total"), Ok(Total(5)));
        ponko_world_free(w);
    }
}

#[test]
fn missing_resources_wrong_sizes_and_busy_worlds_are_refused() {
    unsafe {
        let w = ponko_world_new();
        assert_eq!(scope(w, c"Total", 8, |_, _| panic!("no resource")), PONKO_NOT_FOUND);
        set(w, c"Total", Total(5));
        assert_eq!(
            scope(w, c"Total", 4, |_, _| panic!("wrong size")),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            get::<Total>(w, c"Total"),
            Ok(Total(5)),
            "a refused scope keeps the resource"
        );
        assert_eq!(
            ponko_resource_scope(w, c"Total".as_ptr(), 8, None, null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            scope(null_mut(), c"Total", 8, |_, _| panic!("no world")),
            PONKO_INVALID_ARGUMENT
        );
        // Inside a query the world is borrowed.
        unsafe extern "C" fn inside(_: *const PonkoChunk, user: *mut c_void) {
            let w = user.cast::<PonkoWorld>();
            assert_eq!(unsafe { scope(w, c"Total", 8, |_, _| panic!("busy")) }, PONKO_BUSY);
        }
        let mut id = 0;
        assert_eq!(ponko_component_register(w, c"Tag".as_ptr(), 1, 1, &mut id), PONKO_OK);
        let tag = 1u8;
        let v = PonkoValue {
            component: id,
            data: (&raw const tag).cast(),
            size: 1,
        };
        let mut e = 0;
        assert_eq!(ponko_spawn(w, &v, 1, &mut e), PONKO_OK);
        assert_eq!(ponko_query(w, &id, 1, Some(inside), w.cast()), PONKO_OK);
        ponko_world_free(w);
    }
}
