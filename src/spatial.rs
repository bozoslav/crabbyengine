//! [`SpatialGrid`]: fast "who is near this point?" queries for huge numbers of entities.

use crabby_ecs::{Entity, World};
use glam::Vec2;

use crate::ctx::Ctx;
use crate::ecs::Transform2D;

/// One entity in the grid: its handle, where it was at the last rebuild,
/// and optional data stored with it (see [`SpatialGrid::rebuild_with_data`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridEntry<T = ()> {
    pub entity: Entity,
    pub position: Vec2,
    pub data: T,
}

/// A uniform grid over the whole (unbounded) world, rebuilt from scratch
/// each tick: answers radius, rectangle and nearest-neighbor queries without
/// checking every entity.
///
/// Enable it with [`Game::spatial_grid`](crate::Game::spatial_grid): every
/// entity with a [`Transform2D`] is then indexed at the start of each
/// simulation tick, and you query it through `ctx.spatial`, even from inside a
/// (parallel) query:
///
/// ```
/// use crabbyengine::*;
///
/// struct Unit;
/// struct Crowded(bool);
///
/// fn crowding(ctx: &mut Ctx) {
///     let grid = &ctx.spatial;
///     ctx.world.query::<(Entity, &Transform2D, &mut Crowded)>().par_for_each(|(me, t, crowded)| {
///         let mut near = 0;
///         grid.for_each_in_radius(t.position, 50.0, |other| {
///             if other.entity != me {
///                 near += 1;
///             }
///         });
///         crowded.0 = near >= 3;
///     });
/// }
///
/// let mut world = Game::new("sim", 800, 600)
///     .spatial_grid(50.0)
///     .sim_system(crowding)
///     .run_headless(1, |ctx| {
///         for i in 0..5 {
///             ctx.world.spawn((Transform2D::at(vec2(i as f32, 0.0)), Unit, Crowded(false)));
///         }
///         ctx.world.spawn((Transform2D::at(vec2(500.0, 500.0)), Unit, Crowded(false)));
///     });
/// assert_eq!(world.query::<&Crowded>().iter().filter(|c| c.0).count(), 5);
/// ```
///
/// Pick a cell size close to your typical query radius. Positions are those
/// at the last rebuild; entities that move during a tick are found where they
/// were when it started.
///
/// To index only some entities, remove the built-in
/// [`rebuild_spatial_grid`] system and call
/// [`rebuild_from`](Self::rebuild_from) yourself.
///
/// A grid can also carry data per entry (a velocity, a team), for queries
/// that need neighbors' state while the world is busy with a query. Keep
/// such a grid in your system's closure:
///
/// ```
/// use crabbyengine::*;
///
/// struct Vel(Vec2);
///
/// let align = {
///     let mut grid = SpatialGrid::with_data(20.0);
///     move |ctx: &mut Ctx| {
///         grid.rebuild_with_data(
///             ctx.world.query::<(Entity, &Transform2D, &Vel)>().iter().map(|(e, t, v)| (e, t.position, v.0)),
///         );
///         let grid = &grid;
///         ctx.world.query::<(&Transform2D, &mut Vel)>().par_for_each(|(t, v)| {
///             let mut sum = Vec2::ZERO;
///             grid.for_each_in_radius(t.position, 20.0, |n| sum += n.data);
///             v.0 = v.0.lerp(sum.normalize_or_zero() * v.0.length(), 0.1);
///         });
///     }
/// };
/// Game::new("flock", 800, 600).sim_system(align).run_headless(1, |_| {});
/// ```
#[derive(Debug, Clone)]
pub struct SpatialGrid<T = ()> {
    cell_size: f32,
    inv_cell: f32,
    /// Bucket count minus one (bucket count is a power of two).
    mask: usize,
    /// `starts[b]..starts[b + 1]` is bucket `b`'s range in `entries`.
    starts: Vec<u32>,
    /// All entries, sorted by bucket.
    entries: Vec<GridEntry<T>>,
    /// Packed cell of each entry (same order), so skipping hash-colliding
    /// cells is one integer compare instead of recomputing the cell.
    cells: Vec<u64>,
    // Reused between rebuilds so steady-state rebuilds don't allocate.
    unsorted: Vec<GridEntry<T>>,
    buckets: Vec<u32>,
    cursor: Vec<u32>,
    order: Vec<u32>,
}

impl<T: Copy> Default for SpatialGrid<T> {
    fn default() -> Self {
        Self::with_data(64.0)
    }
}

impl SpatialGrid {
    /// An empty grid with square cells of `cell_size` world units.
    ///
    /// # Panics
    /// If `cell_size` isn't a positive, finite number.
    #[track_caller]
    pub fn new(cell_size: f32) -> Self {
        Self::with_data(cell_size)
    }

    /// Re-indexes every entity that has a [`Transform2D`].
    pub fn rebuild(&mut self, world: &mut World) {
        let mut unsorted = std::mem::take(&mut self.unsorted);
        unsorted.clear();
        world.query::<(Entity, &Transform2D)>().for_each(|(entity, t)| {
            unsorted.push(GridEntry {
                entity,
                position: t.position,
                data: (),
            })
        });
        self.unsorted = unsorted;
        self.sort_unsorted();
    }

    /// Re-indexes from your own list of `(entity, position)`, replacing
    /// everything indexed before.
    pub fn rebuild_from(&mut self, items: impl IntoIterator<Item = (Entity, Vec2)>) {
        self.rebuild_with_data(items.into_iter().map(|(e, p)| (e, p, ())));
    }
}

impl<T: Copy> SpatialGrid<T> {
    /// An empty grid that stores a `T` with every entry (see
    /// [`rebuild_with_data`](Self::rebuild_with_data)), with square cells of
    /// `cell_size` world units.
    ///
    /// # Panics
    /// If `cell_size` isn't a positive, finite number.
    #[track_caller]
    pub fn with_data(cell_size: f32) -> Self {
        assert!(
            cell_size.is_finite() && cell_size > 0.0,
            "SpatialGrid: cell size must be a positive number (got {cell_size})"
        );
        SpatialGrid {
            cell_size,
            inv_cell: 1.0 / cell_size,
            mask: 0,
            starts: vec![0, 0],
            entries: Vec::new(),
            cells: Vec::new(),
            unsorted: Vec::new(),
            buckets: Vec::new(),
            cursor: Vec::new(),
            order: Vec::new(),
        }
    }

    /// Side length of one cell.
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    /// Number of indexed entities.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` if nothing is indexed.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every indexed entity, grouped by cell.
    pub fn entries(&self) -> &[GridEntry<T>] {
        &self.entries
    }

    /// Re-indexes from `(entity, position, data)`, replacing everything
    /// indexed before. Queries hand each entry's `data` back.
    pub fn rebuild_with_data(&mut self, items: impl IntoIterator<Item = (Entity, Vec2, T)>) {
        self.unsorted.clear();
        self.unsorted.extend(
            items
                .into_iter()
                .map(|(entity, position, data)| GridEntry { entity, position, data }),
        );
        self.sort_unsorted();
    }

    /// Counting sort of `unsorted` into `entries` by bucket.
    fn sort_unsorted(&mut self) {
        let n = self.unsorted.len();
        assert!(n < u32::MAX as usize, "SpatialGrid: too many entities");
        let bucket_count = n.max(16).next_power_of_two();
        self.mask = bucket_count - 1;

        self.buckets.clear();
        self.buckets.extend(self.unsorted.iter().map(|e| {
            let (cx, cy) = cell_of(e.position, self.inv_cell);
            bucket(cx, cy, self.mask) as u32
        }));

        self.starts.clear();
        self.starts.resize(bucket_count + 1, 0);
        for &b in &self.buckets {
            self.starts[b as usize + 1] += 1;
        }
        for i in 0..bucket_count {
            self.starts[i + 1] += self.starts[i];
        }

        self.cursor.clear();
        self.cursor.extend_from_slice(&self.starts[..bucket_count]);
        self.order.clear();
        self.order.resize(n, 0);
        for (i, &b) in self.buckets.iter().enumerate() {
            let slot = &mut self.cursor[b as usize];
            self.order[*slot as usize] = i as u32;
            *slot += 1;
        }
        self.entries.clear();
        self.entries
            .extend(self.order.iter().map(|&i| self.unsorted[i as usize]));
        self.cells.clear();
        self.cells.extend(self.entries.iter().map(|e| {
            let (cx, cy) = cell_of(e.position, self.inv_cell);
            pack(cx, cy)
        }));
    }

    /// Calls `f` for every entry in cell `(cx, cy)`. Buckets can hold several
    /// cells (hash collisions), so entries from other cells are skipped.
    #[inline]
    fn for_each_in_cell(&self, cx: i32, cy: i32, mut f: impl FnMut(&GridEntry<T>)) {
        let b = bucket(cx, cy, self.mask);
        let range = self.starts[b] as usize..self.starts[b + 1] as usize;
        let key = pack(cx, cy);
        for (e, &c) in self.entries[range.clone()].iter().zip(&self.cells[range]) {
            if c == key {
                f(e);
            }
        }
    }

    /// `true` when a query box spans more cells than there are entries, so
    /// walking every entry beats visiting every cell (huge radii).
    #[inline]
    fn cheaper_to_scan(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        let cells = (i64::from(x1) - i64::from(x0) + 1) * (i64::from(y1) - i64::from(y0) + 1);
        cells > self.entries.len() as i64
    }

    /// Calls `f` for every entity within `radius` of `center` (inclusive).
    pub fn for_each_in_radius(&self, center: Vec2, radius: f32, mut f: impl FnMut(&GridEntry<T>)) {
        if self.entries.is_empty() || radius.is_nan() || radius < 0.0 {
            return;
        }
        let r2 = radius * radius;
        let (x0, y0) = cell_of(center - Vec2::splat(radius), self.inv_cell);
        let (x1, y1) = cell_of(center + Vec2::splat(radius), self.inv_cell);
        if self.cheaper_to_scan(x0, y0, x1, y1) {
            self.entries
                .iter()
                .filter(|e| e.position.distance_squared(center) <= r2)
                .for_each(f);
            return;
        }
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                self.for_each_in_cell(cx, cy, |e| {
                    if e.position.distance_squared(center) <= r2 {
                        f(e);
                    }
                });
            }
        }
    }

    /// Calls `f` for every entity inside the rectangle `min..=max`.
    pub fn for_each_in_rect(&self, min: Vec2, max: Vec2, mut f: impl FnMut(&GridEntry<T>)) {
        if self.entries.is_empty() || min.x > max.x || min.y > max.y {
            return;
        }
        let (x0, y0) = cell_of(min, self.inv_cell);
        let (x1, y1) = cell_of(max, self.inv_cell);
        let inside = |e: &&GridEntry<T>| e.position.cmpge(min).all() && e.position.cmple(max).all();
        if self.cheaper_to_scan(x0, y0, x1, y1) {
            self.entries.iter().filter(inside).for_each(f);
            return;
        }
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                self.for_each_in_cell(cx, cy, |e| {
                    if inside(&e) {
                        f(e);
                    }
                });
            }
        }
    }

    /// Number of entities within `radius` of `center`.
    pub fn count_in_radius(&self, center: Vec2, radius: f32) -> usize {
        let mut n = 0;
        self.for_each_in_radius(center, radius, |_| n += 1);
        n
    }

    /// The closest entity to `center` within `max_radius` for which `accept`
    /// returns `true` (use it to skip yourself or pick a team), if any.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(grid: &SpatialGrid, me: Entity, pos: Vec2) {
    /// let target = grid.nearest(pos, 200.0, |e| e.entity != me);
    /// # }
    /// ```
    pub fn nearest(
        &self,
        center: Vec2,
        max_radius: f32,
        mut accept: impl FnMut(&GridEntry<T>) -> bool,
    ) -> Option<GridEntry<T>> {
        let mut best: Option<(f32, GridEntry<T>)> = None;
        self.for_each_in_radius(center, max_radius, |e| {
            let d = e.position.distance_squared(center);
            if best.is_none_or(|(bd, _)| d < bd) && accept(e) {
                best = Some((d, *e));
            }
        });
        best.map(|(_, e)| e)
    }
}

#[inline(always)]
fn cell_of(p: Vec2, inv_cell: f32) -> (i32, i32) {
    // Saturating float→int casts keep far-away or non-finite points in edge cells.
    ((p.x * inv_cell).floor() as i32, (p.y * inv_cell).floor() as i32)
}

#[inline(always)]
fn pack(cx: i32, cy: i32) -> u64 {
    (cx as u32 as u64) << 32 | cy as u32 as u64
}

#[inline(always)]
fn bucket(cx: i32, cy: i32, mask: usize) -> usize {
    let h = (cx as u32).wrapping_mul(0x9E37_79B1) ^ (cy as u32).wrapping_mul(0x85EB_CA77);
    (h ^ (h >> 15)) as usize & mask
}

/// Built-in system added by [`Game::spatial_grid`](crate::Game::spatial_grid):
/// re-indexes every [`Transform2D`] entity into `ctx.spatial`. It runs first
/// in every simulation tick.
pub fn rebuild_spatial_grid(ctx: &mut Ctx) {
    ctx.spatial.rebuild(&mut ctx.world);
}
