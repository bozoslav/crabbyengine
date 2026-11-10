//! Sprite sheets, the `Animation` component and tilemaps.
use std::mem::offset_of;

use ponko::{Animation, Color, SpriteSheet, TextureFilter, Vec2, d2::TileMap};

use crate::ctx::{ctx_access, out};
use crate::types::*;
use crate::{CrabbyCtx, CrabbyStatus, CrabbyWorld, Failure, access, boundary, builtin, error, invalid};

/// Many same-size frames in one texture, numbered left to right, top to
/// bottom. Make one with `crabby_sprite_sheet` (same layout as Rust `SpriteSheet`).
/// ```
/// assert_eq!(std::mem::size_of::<ponko_ffi::CrabbySpriteSheet>(), 28);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbySpriteSheet {
    /// The whole sheet.
    pub texture: CrabbyTexture,
    /// Size of one frame in texture pixels.
    pub frame_size: CrabbyVec2,
    /// Frames per row.
    pub columns: u32,
    /// Number of whole frames.
    pub len: u32,
}

/// Frame animation for an entity with a `Sprite`; the built-in
/// `animate_sprites` system advances it every frame. Same layout as Rust
/// `Animation`, so it works as a query column. `frames` must be at least 1
/// and `looping` true or false.
/// ```
/// assert_eq!(std::mem::size_of::<ponko_ffi::CrabbyAnimation>(), 48);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyAnimation {
    /// Where the frames come from.
    pub sheet: CrabbySpriteSheet,
    /// First frame of the animation in the sheet.
    pub first: u32,
    /// Number of frames, at least 1.
    pub frames: u32,
    /// Frames per second.
    pub fps: f32,
    /// Start over after the last frame (else hold it).
    pub looping: bool,
    /// Seconds played so far.
    pub time: f32,
}

macro_rules! same_layout {
    ($c:ty, $rust:ty, $($f:ident),*) => {
        const _: () = {
            assert!(size_of::<$c>() == size_of::<$rust>());
            assert!(align_of::<$c>() == align_of::<$rust>());
            $(assert!(offset_of!($c, $f) == offset_of!($rust, $f));)*
        };
    };
}
same_layout!(CrabbySpriteSheet, SpriteSheet, texture, frame_size, columns, len);
same_layout!(CrabbyAnimation, Animation, sheet, first, frames, fps, looping, time);

fn sheet_to_c(s: SpriteSheet) -> CrabbySpriteSheet {
    // SAFETY: same layout (checked above); every field is plain data.
    unsafe { std::mem::transmute(s) }
}
fn sheet_from_c(s: CrabbySpriteSheet) -> SpriteSheet {
    // SAFETY: as above. Made-up texture ids draw nothing; they are not UB.
    unsafe { std::mem::transmute(s) }
}

/// Splits `texture` into `frame_width` x `frame_height` frames.
/// ```
/// use ponko_ffi::*;
/// let tex = CrabbyTexture { id: 0, width: 64, height: 32 };
/// let mut sheet = std::mem::MaybeUninit::uninit();
/// assert_eq!(unsafe { crabby_sprite_sheet(tex, 16, 16, sheet.as_mut_ptr()) }, CrabbyStatus::CRABBY_OK);
/// assert_eq!(unsafe { sheet.assume_init() }.len, 8);
/// ```
/// Fails if a frame is empty or bigger than the texture.
/// # Safety
/// `out_sheet` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_sprite_sheet(
    texture: CrabbyTexture,
    frame_width: u32,
    frame_height: u32,
    out_sheet: *mut CrabbySpriteSheet,
) -> CrabbyStatus {
    boundary(|| {
        if frame_width == 0 || frame_height == 0 || frame_width > texture.width || frame_height > texture.height {
            return Err(invalid("frames must be non-empty and fit in the texture"));
        }
        let sheet = SpriteSheet::new(texture_from_c(texture), frame_width, frame_height);
        // SAFETY: forwarded caller contract.
        unsafe { out(out_sheet, sheet_to_c(sheet)) }
    })
}

/// The rectangle of frame `index` in texture pixels (wrapping past the end),
/// for `CrabbySprite::region` or `CrabbyDrawParams::region`.
/// ```
/// use ponko_ffi::*;
/// let sheet = CrabbySpriteSheet { texture: CrabbyTexture { id: 0, width: 64, height: 32 }, frame_size: CrabbyVec2 { x: 16.0, y: 16.0 }, columns: 4, len: 8 };
/// assert_eq!(crabby_sprite_sheet_frame(sheet, 5).min, CrabbyVec2 { x: 16.0, y: 16.0 });
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_sprite_sheet_frame(sheet: CrabbySpriteSheet, index: u32) -> CrabbyRect {
    if sheet.columns == 0 {
        return CrabbyRect::default();
    }
    to_c(sheet_from_c(sheet).frame(index))
}

/// The id of the built-in `Animation` component (`CrabbyAnimation`).
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 0; assert_eq!(crabby_animation_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
/// # Safety
/// `world` must be null or live; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_animation_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { builtin::<Animation>(world, out_id) }
}

/// Draws `texture` with square pixels (`nearest`, for pixel art) or smoothed (the default).
/// ```
/// let t = ponko_ffi::CrabbyTexture { id: 0, width: 1, height: 1 };
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_texture_filter(std::ptr::null_mut(), t, true) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_texture_filter(
    ctx: *mut CrabbyCtx,
    texture: CrabbyTexture,
    nearest: bool,
) -> CrabbyStatus {
    let filter = if nearest {
        TextureFilter::Nearest
    } else {
        TextureFilter::Linear
    };
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx.set_texture_filter(&texture_from_c(texture), filter);
            Ok(())
        })
    })
}

// ---- tilemaps -------------------------------------------------------------------

/// A tilemap on an entity, as `crabby_tilemap_get` sees it. The entity's
/// `Transform2D` position is the map's top-left corner.
/// ```
/// assert_eq!(ponko_ffi::CRABBY_TILE_EMPTY, u16::MAX);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyTileMap {
    /// Where tile frames come from.
    pub sheet: CrabbySpriteSheet,
    /// Tiles across.
    pub width: u32,
    /// Tiles down.
    pub height: u32,
    /// Size of one tile in world units. Default: the sheet's frame size.
    pub tile_size: CrabbyVec2,
    /// Draw order (maps are drawn before sprites with the same z). Default 0.
    pub z: f32,
    /// Tint. Default white.
    pub color: CrabbyColor,
    /// `width * height` frame numbers, row by row; `CRABBY_TILE_EMPTY` is a
    /// hole. Writable in place, until the entity is despawned, loses its
    /// map, or gets a new one.
    pub tiles: *mut u16,
}

/// A tile with nothing in it.
pub const CRABBY_TILE_EMPTY: u16 = 0xFFFF;
const _: () = assert!(CRABBY_TILE_EMPTY == TileMap::EMPTY);

unsafe fn tilemap_access<R>(
    world: *mut CrabbyWorld,
    entity: u64,
    f: impl FnOnce(&mut TileMap) -> Result<R, Failure>,
) -> Result<R, Failure> {
    // SAFETY: forwarded caller contract.
    unsafe {
        access(world, |w, _| {
            let e = ponko_ecs::Entity::from_bits(entity);
            if !w.is_alive(e) {
                return Err(error(CrabbyStatus::CRABBY_NOT_FOUND, "entity is not alive"));
            }
            let map = w
                .get_mut::<TileMap>(e)
                .ok_or_else(|| error(CrabbyStatus::CRABBY_NOT_FOUND, "entity has no tilemap"))?;
            f(map)
        })
    }
}

/// Gives `entity` an empty `width` x `height` tilemap of `sheet` frames
/// (replacing any it had). Draw it by giving the entity a `Transform2D`.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// let sheet = CrabbySpriteSheet { texture: CrabbyTexture { id: 0, width: 16, height: 16 }, frame_size: CrabbyVec2 { x: 16.0, y: 16.0 }, columns: 1, len: 1 };
/// unsafe {
///     let mut e = 0;
///     crabby_spawn(w, std::ptr::null(), 0, &mut e);
///     assert_eq!(crabby_tilemap_add(w, e, sheet, 4, 4), CrabbyStatus::CRABBY_OK);
///     crabby_world_free(w);
/// }
/// ```
/// # Safety
/// `world` must be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_tilemap_add(
    world: *mut CrabbyWorld,
    entity: u64,
    sheet: CrabbySpriteSheet,
    width: u32,
    height: u32,
) -> CrabbyStatus {
    boundary(|| {
        if sheet.len == 0 || sheet.columns == 0 {
            return Err(invalid("the sheet has no frames (make it with crabby_sprite_sheet)"));
        }
        if (width as u64) * (height as u64) > 1 << 30 {
            return Err(invalid("the map is too big (at most 2^30 tiles)"));
        }
        // SAFETY: forwarded caller contract.
        unsafe {
            access(world, |w, _| {
                let e = ponko_ecs::Entity::from_bits(entity);
                if !w.is_alive(e) {
                    return Err(error(CrabbyStatus::CRABBY_NOT_FOUND, "entity is not alive"));
                }
                w.insert(e, TileMap::new(sheet_from_c(sheet), width, height));
                Ok(())
            })
        }
    })
}

/// Reads `entity`'s tilemap, with a pointer to its tiles for changing them in place.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_tilemap_get(std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
/// Returns `CRABBY_NOT_FOUND` if the entity is dead or has no map.
/// # Safety
/// `world` must be null or live; `out_map` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_tilemap_get(
    world: *mut CrabbyWorld,
    entity: u64,
    out_map: *mut CrabbyTileMap,
) -> CrabbyStatus {
    boundary(|| {
        if out_map.is_null() {
            return Err(invalid("output is null"));
        }
        // SAFETY: forwarded caller contract; `out_map` checked non-null.
        unsafe {
            tilemap_access(world, entity, |m| {
                out(
                    out_map,
                    CrabbyTileMap {
                        sheet: sheet_to_c(m.sheet),
                        width: m.size.x,
                        height: m.size.y,
                        tile_size: to_c(m.tile_size),
                        z: m.z,
                        color: to_c(m.color),
                        // The Vec's buffer doesn't move when the map's entity
                        // changes archetype; it lives until the map is dropped.
                        tiles: m.tiles.as_mut_ptr(),
                    },
                )
            })
        }
    })
}

/// Changes how `entity`'s tilemap is drawn: `sheet`, `tile_size`, `z` and
/// `color` from `map` (its size and tiles are ignored; use the tiles pointer).
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_tilemap_update(std::ptr::null_mut(), 0, std::ptr::null()) } as u32, 1);
/// ```
/// # Safety
/// `world` must be null or live; `map` readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_tilemap_update(
    world: *mut CrabbyWorld,
    entity: u64,
    map: *const CrabbyTileMap,
) -> CrabbyStatus {
    boundary(|| {
        if map.is_null() {
            return Err(invalid("map is null"));
        }
        // SAFETY: checked non-null; caller supplies a readable map.
        let m = unsafe { *map };
        if m.sheet.len == 0 || m.sheet.columns == 0 {
            return Err(invalid("the sheet has no frames (make it with crabby_sprite_sheet)"));
        }
        // SAFETY: forwarded caller contract.
        unsafe {
            tilemap_access(world, entity, |t| {
                t.sheet = sheet_from_c(m.sheet);
                t.tile_size = to_c::<_, Vec2>(m.tile_size);
                t.z = m.z;
                t.color = to_c::<_, Color>(m.color);
                Ok(())
            })
        }
    })
}
