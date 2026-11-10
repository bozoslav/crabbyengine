//! `CrabbyGrid`: a spatial grid owned by C, with a `uint32_t` of your own per
//! item (an index into your arrays), like Rust's `SpatialGrid::with_data`.
use ponko::spatial::SpatialGrid;

use crate::types::*;
use crate::{CrabbyStatus, Failure, boundary, ctx::collect, invalid};

/// An owned spatial grid. Rebuild it from items, then query it, also from
/// many threads at once (queries only read).
pub struct CrabbyGrid(SpatialGrid<u32>);

/// Makes a grid with square cells of `cell_size` (about the query radius).
/// Returns null if `cell_size` isn't positive.
/// ```
/// let g = ponko_ffi::crabby_grid_new(32.0);
/// unsafe { ponko_ffi::crabby_grid_free(g); }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_grid_new(cell_size: f32) -> *mut CrabbyGrid {
    let mut grid = std::ptr::null_mut();
    let status = boundary(|| {
        if !(cell_size > 0.0 && cell_size.is_finite()) {
            return Err(invalid("cell size must be positive"));
        }
        grid = Box::into_raw(Box::new(CrabbyGrid(SpatialGrid::with_data(cell_size))));
        Ok(())
    });
    if status == CrabbyStatus::CRABBY_OK {
        grid
    } else {
        std::ptr::null_mut()
    }
}

/// Frees a grid. Null is ignored.
/// # Safety
/// `grid` must be null or a live grid, not used afterwards.
/// ```
/// unsafe { ponko_ffi::crabby_grid_free(std::ptr::null_mut()); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_grid_free(grid: *mut CrabbyGrid) {
    if !grid.is_null() {
        // SAFETY: caller passes a grid from crabby_grid_new, once.
        drop(unsafe { Box::from_raw(grid) });
    }
}

unsafe fn grid_mut<'a>(grid: *mut CrabbyGrid) -> Result<&'a mut SpatialGrid<u32>, Failure> {
    // SAFETY: caller supplies a live grid with no concurrent use.
    unsafe { grid.as_mut() }
        .map(|g| &mut g.0)
        .ok_or_else(|| invalid("grid is null"))
}
unsafe fn grid_ref<'a>(grid: *const CrabbyGrid) -> Result<&'a SpatialGrid<u32>, Failure> {
    // SAFETY: caller supplies a live grid that nothing rebuilds meanwhile.
    unsafe { grid.as_ref() }
        .map(|g| &g.0)
        .ok_or_else(|| invalid("grid is null"))
}

/// Replaces the grid's contents with `count` items. Doesn't allocate once the
/// grid has seen this many items before.
/// # Safety
/// Live grid not queried meanwhile; readable items.
/// ```
/// use ponko_ffi::*;
/// let g = crabby_grid_new(32.0);
/// let item = CrabbyGridItem { entity: CrabbyEntity { index: 0, generation: 0 }, position: CrabbyVec2 { x: 5.0, y: 5.0 }, data: 7 };
/// unsafe {
///     assert_eq!(crabby_grid_rebuild(g, &item, 1), CrabbyStatus::CRABBY_OK);
///     let mut hit = item;
///     let mut n = 0;
///     crabby_grid_radius(g, CrabbyVec2 { x: 0.0, y: 0.0 }, 10.0, &mut hit, 1, &mut n);
///     assert_eq!((n, hit.data), (1, 7));
///     crabby_grid_free(g);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_grid_rebuild(
    grid: *mut CrabbyGrid,
    items: *const CrabbyGridItem,
    count: usize,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: forwarded caller contract.
        let grid = unsafe { grid_mut(grid)? };
        if items.is_null() && count != 0 {
            return Err(invalid("items is null"));
        }
        let items = if count == 0 {
            &[]
        } else {
            // SAFETY: caller supplies `count` readable items.
            unsafe { std::slice::from_raw_parts(items, count) }
        };
        grid.rebuild_with_data(items.iter().map(|i| {
            let e: ponko::spatial::GridEntry<u32> = to_c(*i);
            (e.entity, e.position, e.data)
        }));
        Ok(())
    })
}

/// Items within `radius` of `center`: writes up to `capacity`, sets
/// `*out_count` to the total. Thread-safe.
/// # Safety
/// Live grid; writable `capacity` items (null if 0); writable count.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_grid_radius(std::ptr::null(), Default::default(), 1.0, std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_grid_radius(
    grid: *const CrabbyGrid,
    center: CrabbyVec2,
    radius: f32,
    out_items: *mut CrabbyGridItem,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: forwarded caller contract.
        let grid = unsafe { grid_ref(grid)? };
        let mut n = 0;
        // SAFETY: forwarded caller contract.
        unsafe {
            collect(out_items, capacity, out_count, &mut n, |push| {
                grid.for_each_in_radius(to_c(center), radius, |e| push.push(to_c(*e)))
            })
        }
    })
}

/// Number of items in the grid.
/// # Safety
/// `grid` must be null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_grid_len(std::ptr::null()) }, 0);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_grid_len(grid: *const CrabbyGrid) -> usize {
    // SAFETY: forwarded caller contract.
    unsafe { grid_ref(grid) }.map_or(0, |g| g.len())
}

/// A read-only look inside a spatial grid, so C++ can walk it inline
/// (`crabby::grid_view` in crabby.hpp does; C code should prefer the query
/// functions). Valid until the grid is rebuilt or freed.
///
/// Entry `i` (`entry_size` bytes: `CrabbyGridItem` or `CrabbyGridHit`) lies
/// in cell `cells[i]` = `(uint32_t)cx << 32 | (uint32_t)cy`, where
/// `cx = floor(x * inv_cell)` saturated to int32. Bucket `b` holds entries
/// `starts[b] .. starts[b + 1]`; see crabby.hpp for the cell-to-bucket hash.
/// ```
/// let v = ponko_ffi::CrabbyGridView::default();
/// assert_eq!(v.len, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyGridView {
    /// `len` entries.
    pub entries: *const std::ffi::c_void,
    /// Size of one entry in bytes.
    pub entry_size: usize,
    /// Cell key of each entry.
    pub cells: *const u64,
    /// `bucket_count + 1` offsets into `entries`.
    pub starts: *const u32,
    /// Number of entries.
    pub len: usize,
    /// Number of buckets (a power of two).
    pub bucket_count: usize,
    /// 1 / cell size.
    pub inv_cell: f32,
    /// `bucket_count - 1`.
    pub bucket_mask: u64,
    /// Bucket hash parameter.
    pub x_bits: u32,
    /// Bucket hash parameter.
    pub y_bits: u32,
}

impl Default for CrabbyGridView {
    fn default() -> Self {
        Self {
            entries: std::ptr::null(),
            entry_size: 0,
            cells: std::ptr::null(),
            starts: std::ptr::null(),
            len: 0,
            bucket_count: 0,
            inv_cell: 0.0,
            bucket_mask: 0,
            x_bits: 0,
            y_bits: 0,
        }
    }
}

pub(crate) fn view_of<T: Copy + Send + Sync>(grid: &SpatialGrid<T>) -> CrabbyGridView {
    let raw = grid.raw_view();
    CrabbyGridView {
        entries: raw.entries.as_ptr().cast(),
        entry_size: size_of::<ponko::spatial::GridEntry<T>>(),
        cells: raw.cells.as_ptr(),
        starts: raw.starts.as_ptr(),
        len: raw.entries.len(),
        bucket_count: raw.starts.len().saturating_sub(1),
        inv_cell: raw.inv_cell,
        bucket_mask: raw.bucket_mask,
        x_bits: raw.x_bits,
        y_bits: raw.y_bits,
    }
}

/// A view of an owned grid, valid until its next rebuild.
/// # Safety
/// Live grid; writable output.
/// ```
/// use ponko_ffi::*;
/// let g = crabby_grid_new(8.0);
/// let mut v = CrabbyGridView::default();
/// unsafe { assert_eq!(crabby_grid_view(g, &mut v), CrabbyStatus::CRABBY_OK); crabby_grid_free(g); }
/// assert_eq!(v.len, 0);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_grid_view(grid: *const CrabbyGrid, out_view: *mut CrabbyGridView) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: forwarded caller contract.
        let grid = unsafe { grid_ref(grid)? };
        if out_view.is_null() {
            return Err(invalid("output is null"));
        }
        // SAFETY: checked non-null.
        unsafe { out_view.write(view_of(grid)) };
        Ok(())
    })
}
