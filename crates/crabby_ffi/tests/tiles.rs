//! Sprite sheets, the Animation component and tilemaps through the C API.
use crabby_ffi::*;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

const TEX: CrabbyTexture = CrabbyTexture {
    id: 0,
    width: 64,
    height: 40,
};

fn sheet() -> CrabbySpriteSheet {
    let mut s = MaybeUninit::uninit();
    assert_eq!(unsafe { crabby_sprite_sheet(TEX, 16, 16, s.as_mut_ptr()) }, CRABBY_OK);
    unsafe { s.assume_init() }
}
fn last_error() -> String {
    unsafe { CStr::from_ptr(crabby_last_error()) }
        .to_string_lossy()
        .into_owned()
}
fn spawn_empty(w: *mut CrabbyWorld) -> u64 {
    let mut e = 0;
    assert_eq!(unsafe { crabby_spawn(w, null(), 0, &mut e) }, CRABBY_OK);
    e
}

#[test]
fn sheets_number_frames_row_by_row_like_rust() {
    let s = sheet();
    assert_eq!((s.columns, s.len), (4, 8), "the partial last row doesn't count");
    let frame = |i| crabby_sprite_sheet_frame(s, i);
    assert_eq!(frame(0).max, CrabbyVec2 { x: 16.0, y: 16.0 });
    assert_eq!(frame(5).min, CrabbyVec2 { x: 16.0, y: 16.0 });
    assert_eq!(frame(8), frame(0), "wraps");
    for i in 0..8 {
        let at = CrabbyVec2 {
            x: (i % 4 * 16) as f32,
            y: (i / 4 * 16) as f32,
        };
        assert_eq!(frame(i).min, at);
    }
}

#[test]
fn sheets_with_frames_that_dont_fit_are_errors() {
    let mut s = MaybeUninit::uninit();
    for (w, h) in [(0, 16), (16, 0), (65, 16), (16, 41)] {
        assert_eq!(
            unsafe { crabby_sprite_sheet(TEX, w, h, s.as_mut_ptr()) },
            CRABBY_INVALID_ARGUMENT
        );
        assert!(last_error().contains("fit"), "{}", last_error());
    }
    assert_eq!(
        unsafe { crabby_sprite_sheet(TEX, 16, 16, null_mut()) },
        CRABBY_INVALID_ARGUMENT
    );
}

#[test]
fn animations_are_plain_components_with_the_rust_layout() {
    let w = crabby_world_new();
    let mut anim = 0;
    assert_eq!(unsafe { crabby_animation_component(w, &mut anim) }, CRABBY_OK);
    let a = CrabbyAnimation {
        sheet: sheet(),
        first: 2,
        frames: 3,
        fps: 10.0,
        looping: false,
        time: 0.25,
    };
    let value = CrabbyValue {
        component: anim,
        data: (&a as *const CrabbyAnimation).cast(),
        size: size_of::<CrabbyAnimation>(),
    };
    let mut e = 0;
    unsafe {
        assert_eq!(crabby_spawn(w, &value, 1, &mut e), CRABBY_OK);
        let mut back = MaybeUninit::<CrabbyAnimation>::uninit();
        assert_eq!(
            crabby_get(w, e, anim, back.as_mut_ptr().cast(), size_of::<CrabbyAnimation>()),
            CRABBY_OK
        );
        assert_eq!(back.assume_init(), a);
        // And it works as a query column.
        let mut seen = 0u32;
        unsafe extern "C" fn count(chunk: *const CrabbyChunk, user: *mut std::ffi::c_void) {
            unsafe {
                let c = &*chunk;
                let col = *c.columns as *const CrabbyAnimation;
                *user.cast::<u32>() += (*col).frames;
            }
        }
        assert_eq!(
            crabby_query(w, &anim, 1, Some(count), (&mut seen as *mut u32).cast()),
            CRABBY_OK
        );
        assert_eq!(seen, 3);
        crabby_world_free(w);
    }
}

#[test]
fn tilemaps_are_written_in_place_and_styled_by_update() {
    let w = crabby_world_new();
    let e = spawn_empty(w);
    unsafe {
        assert_eq!(crabby_tilemap_add(w, e, sheet(), 3, 2), CRABBY_OK);
        let mut map = MaybeUninit::uninit();
        assert_eq!(crabby_tilemap_get(w, e, map.as_mut_ptr()), CRABBY_OK);
        let mut map = map.assume_init();
        assert_eq!((map.width, map.height, map.z), (3, 2, 0.0));
        assert_eq!(map.tile_size, CrabbyVec2 { x: 16.0, y: 16.0 });
        let tiles = std::slice::from_raw_parts_mut(map.tiles, 6);
        assert!(tiles.iter().all(|&t| t == CRABBY_TILE_EMPTY));
        tiles[4] = 7; // (1, 1)

        // Moving the entity to another archetype keeps the tiles pointer valid.
        let mut t = 0;
        crabby_transform_component(w, &mut t);
        let transform = CrabbyTransform2D::default();
        let value = CrabbyValue {
            component: t,
            data: (&transform as *const CrabbyTransform2D).cast(),
            size: size_of::<CrabbyTransform2D>(),
        };
        assert_eq!(crabby_set(w, e, t, value.data, value.size), CRABBY_OK);
        tiles[0] = 1;

        map.z = 2.0;
        map.tile_size = CrabbyVec2 { x: 8.0, y: 8.0 };
        map.width = 99; // ignored
        assert_eq!(crabby_tilemap_update(w, e, &map), CRABBY_OK);

        let mut again = MaybeUninit::uninit();
        assert_eq!(crabby_tilemap_get(w, e, again.as_mut_ptr()), CRABBY_OK);
        let again = again.assume_init();
        assert_eq!((again.z, again.tile_size.x, again.width), (2.0, 8.0, 3));
        let tiles = std::slice::from_raw_parts(again.tiles, 6);
        assert_eq!((tiles[4], tiles[0], tiles[2]), (7, 1, CRABBY_TILE_EMPTY));
        crabby_world_free(w);
    }
}

#[test]
fn tilemap_calls_say_what_is_missing() {
    let w = crabby_world_new();
    let e = spawn_empty(w);
    let mut map = MaybeUninit::uninit();
    unsafe {
        assert_eq!(crabby_tilemap_get(w, e, map.as_mut_ptr()), CRABBY_NOT_FOUND);
        assert!(last_error().contains("no tilemap"));
        assert_eq!(crabby_tilemap_get(w, 12345, map.as_mut_ptr()), CRABBY_NOT_FOUND);
        assert!(last_error().contains("not alive"));
        assert_eq!(crabby_tilemap_add(w, 12345, sheet(), 1, 1), CRABBY_NOT_FOUND);
        let mut empty = sheet();
        empty.len = 0;
        assert_eq!(crabby_tilemap_add(w, e, empty, 1, 1), CRABBY_INVALID_ARGUMENT);
        assert_eq!(
            crabby_tilemap_add(w, e, sheet(), 1 << 16, 1 << 16),
            CRABBY_INVALID_ARGUMENT
        );
        assert!(last_error().contains("too big"));
        assert_eq!(crabby_tilemap_get(w, e, null_mut()), CRABBY_INVALID_ARGUMENT);
        assert_eq!(crabby_tilemap_update(w, e, null()), CRABBY_INVALID_ARGUMENT);
        crabby_world_free(w);
    }
}
