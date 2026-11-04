//! Streaming big worlds: the cell streamer, terrain tiles built on any
//! thread, and freeing meshes and tiles. Background work itself is the
//! game's (C++ has `std::async` and threads); Rust's `Job` has no C API.

use crabbyengine::{CellStreamer, TerrainTileData, vec3};

use crate::ctx::{ctx_access, out};
use crate::three::{CrabbyMaterial, CrabbyMesh, CrabbyTerrainTile, CrabbyVec3, square_grid};
use crate::types::to_c;
use crate::{CrabbyCtx, CrabbyStatus, boundary, invalid};

/// One cell of a `CrabbyCellStreamer`'s grid: it covers `x * cell_size`
/// to `(x + 1) * cell_size` along X, and the same along Z.
/// ```
/// let c = crabby_ffi::CrabbyCell { x: 2, z: -1 };
/// assert_eq!(c.z, -1);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyCell {
    /// Along X.
    pub x: i32,
    /// Along Z.
    pub z: i32,
}

/// Which cells of a square grid on the ground to have loaded around a
/// point (Rust `CellStreamer`). Make it with `crabby_cells_new`.
pub struct CrabbyCellStreamer {
    cells: CellStreamer,
    to_load: Vec<CrabbyCell>,
    to_drop: Vec<CrabbyCell>,
}

/// Makes a streamer of cells `cell_size` wide, loading those within
/// `load_radius` of the point and dropping those beyond `keep_radius`.
/// Returns null (see `crabby_last_error`) if `cell_size` isn't above 0 or
/// a radius is negative.
/// ```
/// let s = crabby_ffi::crabby_cells_new(256.0, 600.0, 800.0);
/// assert!(!s.is_null());
/// unsafe { crabby_ffi::crabby_cells_free(s) };
/// assert!(crabby_ffi::crabby_cells_new(0.0, 1.0, 1.0).is_null());
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_cells_new(cell_size: f32, load_radius: f32, keep_radius: f32) -> *mut CrabbyCellStreamer {
    let mut made = std::ptr::null_mut();
    let status = boundary(|| {
        if !(cell_size > 0.0 && cell_size.is_finite()) {
            return Err(invalid("cell size must be above 0"));
        }
        if !(load_radius >= 0.0 && keep_radius >= 0.0) {
            return Err(invalid("the radii can't be negative"));
        }
        made = Box::into_raw(Box::new(CrabbyCellStreamer {
            cells: CellStreamer::new(cell_size, load_radius, keep_radius),
            to_load: Vec::new(),
            to_drop: Vec::new(),
        }));
        Ok(())
    });
    if status == CrabbyStatus::CRABBY_OK {
        made
    } else {
        std::ptr::null_mut()
    }
}

/// Frees a cell streamer. Null is ignored.
/// ```
/// unsafe { crabby_ffi::crabby_cells_free(std::ptr::null_mut()) };
/// ```
/// # Safety
/// `cells` must be null or live, and not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_cells_free(cells: *mut CrabbyCellStreamer) {
    if !cells.is_null() {
        // SAFETY: caller passes a streamer from crabby_cells_new, once.
        drop(unsafe { Box::from_raw(cells) });
    }
}

unsafe fn cells_mut<'a>(cells: *mut CrabbyCellStreamer) -> Result<&'a mut CrabbyCellStreamer, crate::Failure> {
    // SAFETY: caller supplies a live streamer with no concurrent use.
    unsafe { cells.as_mut() }.ok_or_else(|| invalid("cell streamer is null"))
}

/// Works out the cells to load and drop around `at` (its height is
/// ignored): at most `max_new` new ones, nearest first (Rust
/// `CellStreamer::update`). Read them with `crabby_cells_to_load` and
/// `crabby_cells_to_drop`.
/// ```
/// use crabby_ffi::*;
/// let s = crabby_cells_new(1.0, 0.4, 0.4);
/// let (mut cells, mut len) = (std::ptr::null(), 0);
/// unsafe {
///     assert_eq!(crabby_cells_update(s, CrabbyVec3 { x: 0.5, y: 0.0, z: 0.5 }, 10), CrabbyStatus::CRABBY_OK);
///     assert_eq!(crabby_cells_to_load(s, &mut cells, &mut len), CrabbyStatus::CRABBY_OK);
///     assert_eq!((len, *cells), (1, CrabbyCell { x: 0, z: 0 }));
///     crabby_cells_free(s);
/// }
/// ```
/// # Safety
/// `cells` must be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_cells_update(cells: *mut CrabbyCellStreamer, at: CrabbyVec3, max_new: usize) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: forwarded caller contract.
        let s = unsafe { cells_mut(cells)? };
        s.cells.update(vec3(at.x, at.y, at.z), max_new);
        let cell = |&(x, z): &(i32, i32)| CrabbyCell { x, z };
        s.to_load.clear();
        s.to_load.extend(s.cells.to_load().iter().map(cell));
        s.to_drop.clear();
        s.to_drop.extend(s.cells.to_drop().iter().map(cell));
        Ok(())
    })
}

unsafe fn cell_list(
    cells: *mut CrabbyCellStreamer,
    out_cells: *mut *const CrabbyCell,
    out_len: *mut usize,
    pick: impl FnOnce(&CrabbyCellStreamer) -> &[CrabbyCell],
) -> CrabbyStatus {
    boundary(|| {
        if out_cells.is_null() || out_len.is_null() {
            return Err(invalid("output is null"));
        }
        // SAFETY: forwarded caller contract; outputs checked non-null.
        unsafe {
            let list = pick(cells_mut(cells)?);
            out_cells.write(list.as_ptr());
            out_len.write(list.len());
        }
        Ok(())
    })
}

/// The cells the last `crabby_cells_update` started loading, nearest
/// first. Valid until the next update or free.
/// ```
/// let (mut c, mut n) = (std::ptr::null(), 0);
/// assert_eq!(unsafe { crabby_ffi::crabby_cells_to_load(std::ptr::null_mut(), &mut c, &mut n) } as u32, 1);
/// ```
/// # Safety
/// `cells` must be null or live; outputs writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_cells_to_load(
    cells: *mut CrabbyCellStreamer,
    out_cells: *mut *const CrabbyCell,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { cell_list(cells, out_cells, out_len, |s| &s.to_load) }
}

/// The cells the last `crabby_cells_update` dropped, in grid order. Valid
/// until the next update or free.
/// ```
/// use crabby_ffi::*;
/// let s = crabby_cells_new(1.0, 0.4, 0.4);
/// let (mut cells, mut len) = (std::ptr::null(), 0);
/// unsafe {
///     crabby_cells_update(s, CrabbyVec3 { x: 0.5, y: 0.0, z: 0.5 }, 10);
///     crabby_cells_update(s, CrabbyVec3 { x: 10.5, y: 0.0, z: 0.5 }, 10);
///     crabby_cells_to_drop(s, &mut cells, &mut len);
///     assert_eq!((len, *cells), (1, CrabbyCell { x: 0, z: 0 }));
///     crabby_cells_free(s);
/// }
/// ```
/// # Safety
/// `cells` must be null or live; outputs writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_cells_to_drop(
    cells: *mut CrabbyCellStreamer,
    out_cells: *mut *const CrabbyCell,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { cell_list(cells, out_cells, out_len, |s| &s.to_drop) }
}

/// Whether `cell` is loaded (or being loaded), and how many are.
/// `out_count` may be null.
/// ```
/// use crabby_ffi::*;
/// let s = crabby_cells_new(1.0, 0.4, 0.4);
/// let (mut loaded, mut count) = (false, 0);
/// unsafe {
///     crabby_cells_update(s, CrabbyVec3 { x: 0.5, y: 0.0, z: 0.5 }, 10);
///     crabby_cells_is_loaded(s, CrabbyCell { x: 0, z: 0 }, &mut loaded, &mut count);
///     assert!(loaded);
///     assert_eq!(count, 1);
///     crabby_cells_free(s);
/// }
/// ```
/// # Safety
/// `cells` must be null or live; `out_loaded` writable, `out_count` null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_cells_is_loaded(
    cells: *mut CrabbyCellStreamer,
    cell: CrabbyCell,
    out_loaded: *mut bool,
    out_count: *mut usize,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: forwarded caller contract.
        unsafe {
            let s = cells_mut(cells)?;
            out(out_loaded, s.cells.is_loaded((cell.x, cell.z)))?;
            if !out_count.is_null() {
                out_count.write(s.cells.loaded_count());
            }
        }
        Ok(())
    })
}

/// Forgets every cell, as if none were loaded.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_cells_clear(std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// `cells` must be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_cells_clear(cells: *mut CrabbyCellStreamer) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: forwarded caller contract.
        let s = unsafe { cells_mut(cells)? };
        s.cells.clear();
        s.to_load.clear();
        s.to_drop.clear();
        Ok(())
    })
}

/// A terrain tile's meshes at every detail level, built on the CPU (Rust
/// `TerrainTileData`): make it on any thread with
/// `crabby_terrain_tile_data_new`, then upload it on the game's with
/// `crabby_ctx_upload_terrain_tile`.
pub struct CrabbyTerrainTileData(TerrainTileData);

/// Builds a tile `size` wide from `count` heights (n x n, row after row
/// along +Z, NaN for a hole). Safe to call from any thread. Returns null
/// (see `crabby_last_error`) for bad input.
/// ```
/// let h = [0.0f32; 9];
/// let d = unsafe { crabby_ffi::crabby_terrain_tile_data_new(h.as_ptr(), 9, 4.0) };
/// assert!(!d.is_null());
/// unsafe { crabby_ffi::crabby_terrain_tile_data_free(d) };
/// assert!(unsafe { crabby_ffi::crabby_terrain_tile_data_new(h.as_ptr(), 8, 4.0) }.is_null());
/// ```
/// # Safety
/// `heights` holds `count` floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_terrain_tile_data_new(
    heights: *const f32,
    count: usize,
    size: f32,
) -> *mut CrabbyTerrainTileData {
    let mut made = std::ptr::null_mut();
    let status = boundary(|| {
        if heights.is_null() {
            return Err(invalid("heights is null"));
        }
        if !square_grid(count) {
            return Err(invalid("the heights aren't a square grid of at least 2 x 2"));
        }
        if !(size > 0.0 && size.is_finite()) {
            return Err(invalid("size must be above 0"));
        }
        // SAFETY: caller supplies `count` floats.
        let heights = unsafe { std::slice::from_raw_parts(heights, count) };
        made = Box::into_raw(Box::new(CrabbyTerrainTileData(TerrainTileData::new(heights, size))));
        Ok(())
    });
    if status == CrabbyStatus::CRABBY_OK {
        made
    } else {
        std::ptr::null_mut()
    }
}

/// Frees tile data. Null is ignored.
/// ```
/// unsafe { crabby_ffi::crabby_terrain_tile_data_free(std::ptr::null_mut()) };
/// ```
/// # Safety
/// `data` must be null or live, and not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_terrain_tile_data_free(data: *mut CrabbyTerrainTileData) {
    if !data.is_null() {
        // SAFETY: caller passes data from crabby_terrain_tile_data_new, once.
        drop(unsafe { Box::from_raw(data) });
    }
}

/// Puts a tile built by `crabby_terrain_tile_data_new` on the GPU (Rust
/// `Ctx::upload_terrain_tile`). The data stays the caller's to free.
/// # Safety
/// `ctx` null or live; `data` live; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_upload_terrain_tile(std::ptr::null_mut(), std::ptr::null(), Default::default(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_upload_terrain_tile(
    ctx: *mut CrabbyCtx,
    data: *const CrabbyTerrainTileData,
    material: CrabbyMaterial,
    out_tile: *mut CrabbyTerrainTile,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(data) = data.as_ref() else {
                return Err(invalid("tile data is null"));
            };
            if !c.has_material(to_c(material)) {
                return Err(invalid("that material wasn't made by this game"));
            }
            out(out_tile, to_c(c.upload_terrain_tile(&data.0, to_c(material))))
        })
    })
}

/// Frees `mesh`'s GPU memory (Rust `Ctx::free_mesh`). Models still showing
/// it draw nothing. Freeing it again does nothing.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_free_mesh(std::ptr::null_mut(), Default::default()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_free_mesh(ctx: *mut CrabbyCtx, mesh: CrabbyMesh) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.free_mesh(to_c(mesh));
            Ok(())
        })
    })
}

/// Whether `mesh` was made by this game and not freed.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// let mut live = true;
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_is_mesh_live(std::ptr::null_mut(), Default::default(), &mut live) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_is_mesh_live(ctx: *mut CrabbyCtx, mesh: CrabbyMesh, out_live: *mut bool) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_live, c.is_mesh_live(to_c(mesh)))) })
}

/// Frees every detail level of `tile` (Rust `Ctx::free_terrain_tile`);
/// despawn its entity too.
/// # Safety
/// `ctx` null or live.
/// ```
/// let t = crabby_ffi::CrabbyTerrainTile { lods: [Default::default(); 4], lod_distance: 1.0 };
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_free_terrain_tile(std::ptr::null_mut(), t) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_free_terrain_tile(ctx: *mut CrabbyCtx, tile: CrabbyTerrainTile) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.free_terrain_tile(to_c(tile));
            Ok(())
        })
    })
}
