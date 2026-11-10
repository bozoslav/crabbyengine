//! Sprite sheets, the Animation component and tilemaps through the C API.
use ponko_ffi::*;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};

use PonkoStatus::*;

const TEX: PonkoTexture = PonkoTexture {
    id: 0,
    width: 64,
    height: 40,
};

fn sheet() -> PonkoSpriteSheet {
    let mut s = MaybeUninit::uninit();
    assert_eq!(unsafe { ponko_sprite_sheet(TEX, 16, 16, s.as_mut_ptr()) }, PONKO_OK);
    unsafe { s.assume_init() }
}
fn last_error() -> String {
    unsafe { CStr::from_ptr(ponko_last_error()) }
        .to_string_lossy()
        .into_owned()
}
fn spawn_empty(w: *mut PonkoWorld) -> u64 {
    let mut e = 0;
    assert_eq!(unsafe { ponko_spawn(w, null(), 0, &mut e) }, PONKO_OK);
    e
}

#[test]
fn sheets_number_frames_row_by_row_like_rust() {
    let s = sheet();
    assert_eq!((s.columns, s.len), (4, 8), "the partial last row doesn't count");
    let frame = |i| ponko_sprite_sheet_frame(s, i);
    assert_eq!(frame(0).max, PonkoVec2 { x: 16.0, y: 16.0 });
    assert_eq!(frame(5).min, PonkoVec2 { x: 16.0, y: 16.0 });
    assert_eq!(frame(8), frame(0), "wraps");
    for i in 0..8 {
        let at = PonkoVec2 {
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
            unsafe { ponko_sprite_sheet(TEX, w, h, s.as_mut_ptr()) },
            PONKO_INVALID_ARGUMENT
        );
        assert!(last_error().contains("fit"), "{}", last_error());
    }
    assert_eq!(
        unsafe { ponko_sprite_sheet(TEX, 16, 16, null_mut()) },
        PONKO_INVALID_ARGUMENT
    );
}

#[test]
fn animations_are_plain_components_with_the_rust_layout() {
    let w = ponko_world_new();
    let mut anim = 0;
    assert_eq!(unsafe { ponko_animation_component(w, &mut anim) }, PONKO_OK);
    let a = PonkoAnimation {
        sheet: sheet(),
        first: 2,
        frames: 3,
        fps: 10.0,
        looping: false,
        time: 0.25,
    };
    let value = PonkoValue {
        component: anim,
        data: (&a as *const PonkoAnimation).cast(),
        size: size_of::<PonkoAnimation>(),
    };
    let mut e = 0;
    unsafe {
        assert_eq!(ponko_spawn(w, &value, 1, &mut e), PONKO_OK);
        let mut back = MaybeUninit::<PonkoAnimation>::uninit();
        assert_eq!(
            ponko_get(w, e, anim, back.as_mut_ptr().cast(), size_of::<PonkoAnimation>()),
            PONKO_OK
        );
        assert_eq!(back.assume_init(), a);
        // And it works as a query column.
        let mut seen = 0u32;
        unsafe extern "C" fn count(chunk: *const PonkoChunk, user: *mut std::ffi::c_void) {
            unsafe {
                let c = &*chunk;
                let col = *c.columns as *const PonkoAnimation;
                *user.cast::<u32>() += (*col).frames;
            }
        }
        assert_eq!(
            ponko_query(w, &anim, 1, Some(count), (&mut seen as *mut u32).cast()),
            PONKO_OK
        );
        assert_eq!(seen, 3);
        ponko_world_free(w);
    }
}

#[test]
fn tilemaps_are_written_in_place_and_styled_by_update() {
    let w = ponko_world_new();
    let e = spawn_empty(w);
    unsafe {
        assert_eq!(ponko_tilemap_add(w, e, sheet(), 3, 2), PONKO_OK);
        let mut map = MaybeUninit::uninit();
        assert_eq!(ponko_tilemap_get(w, e, map.as_mut_ptr()), PONKO_OK);
        let mut map = map.assume_init();
        assert_eq!((map.width, map.height, map.z), (3, 2, 0.0));
        assert_eq!(map.tile_size, PonkoVec2 { x: 16.0, y: 16.0 });
        let tiles = std::slice::from_raw_parts_mut(map.tiles, 6);
        assert!(tiles.iter().all(|&t| t == PONKO_TILE_EMPTY));
        tiles[4] = 7; // (1, 1)

        // Moving the entity to another archetype keeps the tiles pointer valid.
        let mut t = 0;
        ponko_transform_component(w, &mut t);
        let transform = PonkoTransform2D::default();
        let value = PonkoValue {
            component: t,
            data: (&transform as *const PonkoTransform2D).cast(),
            size: size_of::<PonkoTransform2D>(),
        };
        assert_eq!(ponko_set(w, e, t, value.data, value.size), PONKO_OK);
        tiles[0] = 1;

        map.z = 2.0;
        map.tile_size = PonkoVec2 { x: 8.0, y: 8.0 };
        map.width = 99; // ignored
        assert_eq!(ponko_tilemap_update(w, e, &map), PONKO_OK);

        let mut again = MaybeUninit::uninit();
        assert_eq!(ponko_tilemap_get(w, e, again.as_mut_ptr()), PONKO_OK);
        let again = again.assume_init();
        assert_eq!((again.z, again.tile_size.x, again.width), (2.0, 8.0, 3));
        let tiles = std::slice::from_raw_parts(again.tiles, 6);
        assert_eq!((tiles[4], tiles[0], tiles[2]), (7, 1, PONKO_TILE_EMPTY));
        ponko_world_free(w);
    }
}

#[test]
fn tilemap_calls_say_what_is_missing() {
    let w = ponko_world_new();
    let e = spawn_empty(w);
    let mut map = MaybeUninit::uninit();
    unsafe {
        assert_eq!(ponko_tilemap_get(w, e, map.as_mut_ptr()), PONKO_NOT_FOUND);
        assert!(last_error().contains("no tilemap"));
        assert_eq!(ponko_tilemap_get(w, 12345, map.as_mut_ptr()), PONKO_NOT_FOUND);
        assert!(last_error().contains("not alive"));
        assert_eq!(ponko_tilemap_add(w, 12345, sheet(), 1, 1), PONKO_NOT_FOUND);
        let mut empty = sheet();
        empty.len = 0;
        assert_eq!(ponko_tilemap_add(w, e, empty, 1, 1), PONKO_INVALID_ARGUMENT);
        assert_eq!(
            ponko_tilemap_add(w, e, sheet(), 1 << 16, 1 << 16),
            PONKO_INVALID_ARGUMENT
        );
        assert!(last_error().contains("too big"));
        assert_eq!(ponko_tilemap_get(w, e, null_mut()), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_tilemap_update(w, e, null()), PONKO_INVALID_ARGUMENT);
        ponko_world_free(w);
    }
}
