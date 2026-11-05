//! 3D navigation using the pure Rust Recast/Detour port `navmesh-rs`.
//!
//! The crate supplies voxelization, clearance/ledge filtering, erosion, regions,
//! tiled polygons, A* and funnel queries without Bevy, C++ or an async runtime.
//! Landmass needs a separate baker; Polyanya needs adaptation for 3D surfaces;
//! oxidized_navigation is a Bevy plugin. Recast bindings need a native toolchain.
//! This wrapper keeps the established algorithm in one dependency.
//!
//! Dynamic AABBs conservatively disable overlapping polygon bounds (expanded by
//! the baked agent clearance); they do not carve new edges or rebuild tiles.
//! Register `step_navmesh3d` as a sim system, including on headless servers.
//! Agents directly own their Transform3D: do not also attach a physics body or
//! character controller. There is no crowd avoidance. Bake for the largest
//! agent radius/height; agents larger than that radius are stopped.

use crate::{Ctx, MeshData, Transform3D, Vec3};
use navmesh_rs::{Limits, Workspace, bake, detour};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_REVISION: AtomicU64 = AtomicU64::new(1);

/// World-space, counter-clockwise triangle soup; include walls and ceilings too.
/// ```
/// use crabbyengine::*;
/// let source = NavSource::from_mesh(&MeshData::plane(20.0), Transform3D::default());
/// assert_eq!(source.triangles.len(), 2);
/// ```
#[derive(Clone, Debug, Default)]
pub struct NavSource {
    /// World-space triangles, facing up for walkable floors.
    /// ```
    /// use crabbyengine::*;
    /// let source = NavSource { triangles: vec![[Vec3::ZERO, Vec3::Z, Vec3::X]] };
    /// assert_eq!(source.triangles.len(), 1);
    /// ```
    pub triangles: Vec<[Vec3; 3]>,
}
impl NavSource {
    /// Transforms mesh triangles to world space, preserving winding under mirrors.
    /// Invalid indices are rejected later by returning an invalid source to `bake`.
    /// ```
    /// use crabbyengine::*;
    /// let source = NavSource::from_mesh(&MeshData::plane(8.0), Transform3D::at(Vec3::Y));
    /// assert_eq!(source.triangles[0][0].y, 1.0);
    /// ```
    #[must_use]
    pub fn from_mesh(mesh: &MeshData, transform: Transform3D) -> Self {
        let mut triangles = Vec::with_capacity(mesh.indices.len() / 3);
        if !mesh.indices.len().is_multiple_of(3) {
            return Self {
                triangles: vec![[Vec3::NAN; 3]],
            };
        }
        for indices in mesh.indices.as_chunks::<3>().0 {
            let mut tri = [Vec3::ZERO; 3];
            for (v, &index) in tri.iter_mut().zip(indices) {
                let Some(&p) = mesh.positions.get(index as usize) else {
                    return Self {
                        triangles: vec![[Vec3::NAN; 3]],
                    };
                };
                *v = transform.position + transform.rotation * (p * transform.scale);
            }
            if transform.scale.element_product() < 0.0 {
                tri.swap(1, 2);
            }
            triangles.push(tri);
        }
        Self { triangles }
    }
}

/// Bake clearances in world units, with slopes in degrees. +Y is up.
/// Height/radius round up to cells; climb rounds down, so clearance is conservative.
/// ```
/// use crabbyengine::*;
/// let settings = NavBakeSettings { agent_radius: 0.4, ..Default::default() };
/// assert!(settings.agent_height > 1.0);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct NavBakeSettings {
    /// Horizontal clearance; defaults to 0.3.
    /// ```
    /// # use crabbyengine::*;
    /// assert_eq!(NavBakeSettings::default().agent_radius, 0.3);
    /// ```
    pub agent_radius: f32,
    /// Minimum free height; defaults to 1.8.
    /// ```
    /// # use crabbyengine::*;
    /// assert_eq!(NavBakeSettings::default().agent_height, 1.8);
    /// ```
    pub agent_height: f32,
    /// Largest traversable step; defaults to 0.4.
    /// ```
    /// # use crabbyengine::*;
    /// assert_eq!(NavBakeSettings::default().max_climb, 0.4);
    /// ```
    pub max_climb: f32,
    /// Maximum walkable slope in degrees, strictly below 90; defaults to 45.
    /// ```
    /// # use crabbyengine::*;
    /// assert_eq!(NavBakeSettings::default().max_slope, 45.0);
    /// ```
    pub max_slope: f32,
    /// Horizontal voxel width; defaults to 0.2.
    /// ```
    /// # use crabbyengine::*;
    /// assert_eq!(NavBakeSettings::default().cell_size, 0.2);
    /// ```
    pub cell_size: f32,
    /// Vertical voxel height; defaults to 0.1.
    /// ```
    /// # use crabbyengine::*;
    /// assert_eq!(NavBakeSettings::default().cell_height, 0.1);
    /// ```
    pub cell_height: f32,
    /// Tile width in cells (8..=1024); smaller tiles localize polygon blocking.
    /// ```
    /// # use crabbyengine::*;
    /// assert_eq!(NavBakeSettings::default().tile_size, 32);
    /// ```
    pub tile_size: u32,
}
impl Default for NavBakeSettings {
    fn default() -> Self {
        Self {
            agent_radius: 0.3,
            agent_height: 1.8,
            max_climb: 0.4,
            max_slope: 45.0,
            cell_size: 0.2,
            cell_height: 0.1,
            tile_size: 32,
        }
    }
}

/// Invalid input, resource limits or a Recast/Detour failure. No partial mesh is returned.
/// ```
/// use crabbyengine::*;
/// let error = NavMesh3D::bake(&[], NavBakeSettings::default()).err().unwrap();
/// assert!(!error.to_string().is_empty());
/// ```
#[derive(Debug)]
pub struct NavBakeError(String);
impl std::fmt::Display for NavBakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for NavBakeError {}
fn error(value: impl std::fmt::Display) -> NavBakeError {
    NavBakeError(value.to_string())
}

struct PolygonBounds {
    reference: detour::PolyRef,
    min: Vec3,
    max: Vec3,
}

/// Tiled navmesh resource. Replace it with a new bake to invalidate agent paths.
/// Queries reject partial or truncated paths. Same ordered inputs yield the same
/// results; the backend's `libm` feature removes platform libm differences.
/// ```
/// use crabbyengine::*;
/// let floor = NavSource::from_mesh(&MeshData::plane(20.0), Transform3D::default());
/// let nav = NavMesh3D::bake(&[floor], NavBakeSettings::default()).unwrap();
/// assert!(nav.find_path(Vec3::ZERO, Vec3::X).is_some());
/// ```
pub struct NavMesh3D {
    // Keep the ECS resource's box in place while querying components. Moving this
    // owned state out and back avoids a boxed resource allocation on every tick.
    state: Option<NavState>,
}

struct NavState {
    mesh: detour::NavMesh,
    settings: NavBakeSettings,
    polygons: Vec<PolygonBounds>,
    obstacles: Vec<NavObstacle3D>,
    scratch: Vec<NavObstacle3D>,
    revision: u64,
}
impl NavMesh3D {
    /// Bakes ordered world triangles with bounded backend memory/work budgets.
    /// Empty triangle soups and nonfinite inputs are errors; an entirely steep
    /// surface successfully produces an empty, unqueryable mesh.
    /// ```
    /// use crabbyengine::*;
    /// let source = NavSource::from_mesh(&MeshData::plane(10.0), Transform3D::default());
    /// assert!(NavMesh3D::bake(&[source], NavBakeSettings::default()).is_ok());
    /// ```
    pub fn bake(sources: &[NavSource], settings: NavBakeSettings) -> Result<Self, NavBakeError> {
        NavState::bake(sources, settings).map(|state| Self { state: Some(state) })
    }
    /// Nearest unblocked surface within a Euclidean distance, including Y.
    /// ```
    /// use crabbyengine::*;
    /// # let source = NavSource::from_mesh(&MeshData::plane(10.0), Transform3D::default());
    /// # let nav = NavMesh3D::bake(&[source], NavBakeSettings::default()).unwrap();
    /// assert!(nav.sample_position(Vec3::Y, 2.0).is_some());
    /// ```
    #[must_use]
    pub fn sample_position(&self, point: Vec3, max_distance: f32) -> Option<Vec3> {
        self.state.as_ref()?.sample_position(point, max_distance)
    }
    /// Complete string-pulled corners; endpoints must be within two cells of a
    /// surface horizontally and `max_climb + two cell heights` vertically.
    /// Unreachable endpoints return None instead of Detour's partial route.
    /// ```
    /// use crabbyengine::*;
    /// # let source = NavSource::from_mesh(&MeshData::plane(10.0), Transform3D::default());
    /// # let nav = NavMesh3D::bake(&[source], NavBakeSettings::default()).unwrap();
    /// assert!(nav.find_path(Vec3::ZERO, Vec3::X * 2.0).is_some());
    /// ```
    #[must_use]
    pub fn find_path(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        self.state.as_ref()?.find_path(from, to)
    }
    /// Replaces world-space blocking boxes, restoring polygons when boxes leave.
    /// Invalid boxes fail closed by blocking every polygon. Overlap tests use
    /// polygon AABBs, so thin boxes can close a much larger area. No carving.
    /// The sim system owns this list when registered: it gathers obstacle components.
    /// ```
    /// use crabbyengine::*;
    /// # let source = NavSource::from_mesh(&MeshData::plane(10.0), Transform3D::default());
    /// # let mut nav = NavMesh3D::bake(&[source], NavBakeSettings::default()).unwrap();
    /// nav.set_obstacles(&[NavObstacle3D::new(Vec3::ZERO, Vec3::splat(20.0))]);
    /// assert!(nav.find_path(Vec3::ZERO, Vec3::X).is_none());
    /// nav.set_obstacles(&[]);
    /// ```
    pub fn set_obstacles(&mut self, obstacles: &[NavObstacle3D]) {
        if let Some(state) = &mut self.state {
            state.set_obstacles(obstacles);
        }
    }
}

impl NavState {
    fn bake(sources: &[NavSource], settings: NavBakeSettings) -> Result<Self, NavBakeError> {
        let s = settings;
        if ![
            s.agent_radius,
            s.agent_height,
            s.max_climb,
            s.max_slope,
            s.cell_size,
            s.cell_height,
        ]
        .iter()
        .all(|x| x.is_finite())
            || s.agent_radius < 0.0
            || s.agent_height <= 0.0
            || s.max_climb < 0.0
            || !(0.0..90.0).contains(&s.max_slope)
            || s.cell_size <= 0.0
            || s.cell_height <= 0.0
            || !(8..=1024).contains(&s.tile_size)
        {
            return Err(error("invalid navigation bake settings"));
        }
        let mut verts = Vec::new();
        let mut tris = Vec::new();
        for source in sources {
            for triangle in &source.triangles {
                for p in triangle {
                    if !p.is_finite() {
                        return Err(error("nonfinite source triangle"));
                    }
                    tris.push(i32::try_from(verts.len() / 3).map_err(error)?);
                    verts.extend(p.to_array());
                }
            }
        }
        if tris.is_empty() {
            return Err(error("no source triangles"));
        }
        let mut config = bake::BakeConfig::default();
        config.recast.cs = s.cell_size;
        config.recast.ch = s.cell_height;
        config.recast.tile_size = s.tile_size as i32;
        config.recast.walkable_height = (s.agent_height / s.cell_height).ceil() as i32;
        config.recast.walkable_radius = (s.agent_radius / s.cell_size).ceil() as i32;
        config.recast.walkable_climb = (s.max_climb / s.cell_height).floor() as i32;
        config.recast.walkable_slope_angle = s.max_slope;
        config.recast.min_region_area = 0;
        config.recast.merge_region_area = 0;
        config.recast.detail_sample_dist = s.cell_size * 6.0;
        config.recast.detail_sample_max_error = s.cell_height;
        config.walkable_height = s.agent_height;
        config.walkable_radius = s.agent_radius;
        config.walkable_climb = s.max_climb;
        config.strict_triangulation = true;
        config.tile_limits.max_bytes = 64 << 20;
        config.tile_limits.max_work = 100_000_000;
        config.max_total_bytes = 2 << 30;
        let baked = bake::tiles(&bake::Input::new(&verts, &tris), &config, &mut Workspace::new()).map_err(error)?;
        let mesh = baked.into_navmesh(Limits::default()).map_err(error)?;
        let mut polygons = Vec::new();
        for i in 0..mesh.max_tiles() as usize {
            let Some(tile) = mesh.tile(i) else {
                continue;
            };
            let base = mesh.poly_ref_base(i).expect("resident tile has reference");
            for (j, poly) in tile.polys.iter().enumerate() {
                let mut min = Vec3::splat(f32::INFINITY);
                let mut max = Vec3::splat(f32::NEG_INFINITY);
                for &v in &poly.verts[..poly.vert_count as usize] {
                    let at = v as usize * 3;
                    let p = Vec3::from_slice(&tile.verts[at..at + 3]);
                    min = min.min(p);
                    max = max.max(p);
                }
                polygons.push(PolygonBounds {
                    reference: base | j as detour::PolyRef,
                    min,
                    max,
                });
            }
        }
        Ok(Self {
            mesh,
            settings,
            polygons,
            obstacles: Vec::new(),
            scratch: Vec::new(),
            revision: NEXT_REVISION.fetch_add(1, Ordering::Relaxed),
        })
    }

    fn sample_position(&self, point: Vec3, max_distance: f32) -> Option<Vec3> {
        if !point.is_finite() || !max_distance.is_finite() || max_distance < 0.0 {
            return None;
        }
        let query = detour::NavMeshQuery::new(&self.mesh, 1).ok()?;
        let (r, p, _) = query
            .find_nearest_poly(&point.to_array(), &[max_distance; 3], &detour::DefaultFilter::default())
            .ok()?;
        let p = Vec3::from_array(p);
        (r != 0 && p.distance(point) <= max_distance).then_some(p)
    }

    fn find_path(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        if !from.is_finite() || !to.is_finite() {
            return None;
        }
        let mut query = detour::NavMeshQuery::new(&self.mesh, 8192).ok()?;
        let s = self.settings;
        let ext = [s.cell_size * 2.0, s.max_climb + s.cell_height * 2.0, s.cell_size * 2.0];
        let filter = detour::DefaultFilter::default();
        let (a, pa, _) = query.find_nearest_poly(&from.to_array(), &ext, &filter).ok()?;
        let (b, pb, _) = query.find_nearest_poly(&to.to_array(), &ext, &filter).ok()?;
        if a == 0 || b == 0 {
            return None;
        }
        let mut path = Vec::new();
        let status = query.find_path(a, b, &pa, &pb, &filter, &mut path, 8192).ok()?;
        if status.has_detail(
            detour::Status::PARTIAL_RESULT | detour::Status::OUT_OF_NODES | detour::Status::BUFFER_TOO_SMALL,
        ) || path.last() != Some(&b)
        {
            return None;
        }
        let mut corners = Vec::new();
        query.find_straight_path(&pa, &pb, &path, &mut corners, 8192, 0).ok()?;
        if corners.last()?.flags & detour::STRAIGHTPATH_END == 0 {
            return None;
        }
        Some(corners.into_iter().map(|p| Vec3::from_array(p.pos)).collect())
    }

    // Resolve heights through connected polygons, never a global nearest lookup
    // that could jump between stacked floors. This work happens only on replans.
    fn agent_route(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        let corners = self.find_path(from, to)?;
        let mut query = detour::NavMeshQuery::new(&self.mesh, 64).ok()?;
        let filter = detour::DefaultFilter::default();
        let (mut poly, mut position, _) = query
            .find_nearest_poly(&corners.first()?.to_array(), &[self.settings.cell_size; 3], &filter)
            .ok()?;
        if poly == 0 {
            return None;
        }
        let mut route = vec![Vec3::from_array(position)];
        let mut visited = Vec::with_capacity(64);
        for segment in corners.windows(2) {
            let count = (segment[0].distance(segment[1]) / self.settings.cell_size)
                .ceil()
                .max(1.0) as usize;
            // Bound route storage for accidental enormous worlds/tiny cells.
            if count > 1_000_000usize.saturating_sub(route.len()) {
                return None;
            }
            for i in 1..=count {
                let target = segment[0].lerp(segment[1], i as f32 / count as f32);
                let (mut next, status) = query
                    .move_along_surface(poly, &position, &target.to_array(), &filter, &mut visited, 64)
                    .ok()?;
                if status.has_detail(detour::Status::BUFFER_TOO_SMALL | detour::Status::OUT_OF_NODES) {
                    return None;
                }
                poly = *visited.last()?;
                // Surface moves can land a few ulps outside an edge; closest-point
                // projection handles that boundary and reads detail height safely.
                next = query.closest_point_on_poly(poly, &next).ok()?.0;
                let point = Vec3::from_array(next);
                if (point - target).with_y(0.0).length() > self.settings.cell_size {
                    return None;
                }
                route.push(point);
                position = next;
            }
        }
        Some(route)
    }

    fn set_obstacles(&mut self, obstacles: &[NavObstacle3D]) {
        if self.obstacles == obstacles {
            return;
        }
        self.obstacles.clear();
        self.obstacles.extend_from_slice(obstacles);
        self.apply_obstacles();
    }
    fn apply_obstacles(&mut self) {
        let radius = Vec3::new(self.settings.agent_radius, 0.0, self.settings.agent_radius);
        for poly in &self.polygons {
            let blocked = self.obstacles.iter().any(|o| {
                !o.center.is_finite() || !o.half_extents.is_finite() || o.half_extents.min_element() < 0.0 || {
                    let min = o.center - o.half_extents - radius;
                    let max = o.center + o.half_extents + radius;
                    poly.max.cmpge(min - Vec3::Y * self.settings.agent_height).all() && poly.min.cmple(max).all()
                }
            });
            self.mesh
                .set_poly_flags(poly.reference, u16::from(!blocked))
                .expect("cached polygon reference");
        }
        self.revision = NEXT_REVISION.fetch_add(1, Ordering::Relaxed);
    }
}

/// World-space axis-aligned blocking box component; no Transform3D is needed.
/// ```
/// use crabbyengine::*;
/// let obstacle = NavObstacle3D::new(Vec3::ZERO, Vec3::ONE);
/// assert_eq!(obstacle.center, Vec3::ZERO);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavObstacle3D {
    /// Center in world space.
    /// ```
    /// # use crabbyengine::*;
    /// assert_eq!(NavObstacle3D::new(Vec3::Y, Vec3::ONE).center, Vec3::Y);
    /// ```
    pub center: Vec3,
    /// Nonnegative half sizes along world X,Y,Z.
    /// ```
    /// # use crabbyengine::*;
    /// assert_eq!(NavObstacle3D::new(Vec3::ZERO, Vec3::ONE).half_extents, Vec3::ONE);
    /// ```
    pub half_extents: Vec3,
}
impl NavObstacle3D {
    /// Creates an axis-aligned world box. Invalid values block the entire mesh.
    /// ```
    /// # use crabbyengine::*;
    /// let obstacle = NavObstacle3D::new(Vec3::Y, Vec3::ONE);
    /// assert_eq!(obstacle.half_extents.y, 1.0);
    /// ```
    #[must_use]
    pub fn new(center: Vec3, half_extents: Vec3) -> Self {
        Self { center, half_extents }
    }
}

/// Agent component; its feet follow the surface, without gravity or crowd avoidance.
/// Path storage is retained between ticks; path queries allocate only on replans.
/// ```
/// use crabbyengine::*;
/// let mut agent = NavAgent3D::default();
/// agent.destination = Some(Vec3::X * 4.0);
/// ```
#[derive(Clone, Debug)]
pub struct NavAgent3D {
    /// Maximum speed in world units per second. Nonpositive/nonfinite stops.
    /// ```
    /// # use crabbyengine::*;
    /// assert!(NavAgent3D::default().speed > 0.0);
    /// ```
    pub speed: f32,
    /// Agent radius, no larger than the bake radius. Defaults to 0.3.
    /// ```
    /// # use crabbyengine::*;
    /// assert_eq!(NavAgent3D::default().radius, 0.3);
    /// ```
    pub radius: f32,
    /// Speed increase per second; direction changes at corners without inertia.
    /// ```
    /// # use crabbyengine::*;
    /// assert!(NavAgent3D::default().acceleration > 0.0);
    /// ```
    pub acceleration: f32,
    /// Requested feet position. None stops; changing it triggers a replan.
    /// ```
    /// # use crabbyengine::*;
    /// assert!(NavAgent3D::default().destination.is_none());
    /// ```
    pub destination: Option<Vec3>,
    path: Vec<Vec3>,
    corner: usize,
    revision: u64,
    planned_destination: Option<Vec3>,
    expected_position: Vec3,
    velocity: f32,
}
impl Default for NavAgent3D {
    fn default() -> Self {
        Self {
            speed: 3.0,
            radius: 0.3,
            acceleration: 10.0,
            destination: None,
            path: Vec::new(),
            corner: 0,
            revision: 0,
            planned_destination: None,
            expected_position: Vec3::NAN,
            velocity: 0.0,
        }
    }
}

/// Register with `Game::sim_system`, just like the exposed physics step.
/// No resource means no-op. Each tick gathers obstacle components, invalidates
/// paths on mesh/obstacle/destination changes or teleports, then moves agents.
/// Stalled agents retry when those inputs change. No window/GPU is required.
/// ```
/// use crabbyengine::*;
/// Game::new("server", 1, 1).sim_system(step_navmesh3d).run_headless(1, |ctx| {
///     let source = NavSource::from_mesh(&MeshData::plane(20.0), Transform3D::default());
///     ctx.world.insert_resource(NavMesh3D::bake(&[source], NavBakeSettings::default()).unwrap());
///     ctx.world.spawn((Transform3D::default(), NavAgent3D::default()));
/// });
/// ```
pub fn step_navmesh3d(ctx: &mut Ctx) {
    let dt = ctx.dt();
    if !dt.is_finite() || dt <= 0.0 {
        return;
    }
    let Some(mut nav) = ctx
        .world
        .get_resource_mut::<NavMesh3D>()
        .and_then(|nav| nav.state.take())
    else {
        return;
    };
    nav.scratch.clear();
    ctx.world.query::<&NavObstacle3D>().for_each(|o| nav.scratch.push(*o));
    if nav.scratch != nav.obstacles {
        std::mem::swap(&mut nav.scratch, &mut nav.obstacles);
        nav.apply_obstacles();
    }
    ctx.world
        .query::<(&mut Transform3D, &mut NavAgent3D)>()
        .for_each(|(t, a)| {
            let valid = a.radius.is_finite()
                && a.radius >= 0.0
                && a.radius <= nav.settings.agent_radius
                && a.speed.is_finite()
                && a.speed > 0.0
                && a.acceleration.is_finite()
                && a.acceleration > 0.0
                && a.destination.is_some_and(|p| p.is_finite())
                && t.position.is_finite();
            if !valid || a.destination.is_none() {
                a.velocity = 0.0;
                a.revision = 0;
                return;
            }
            if a.revision != nav.revision || a.destination != a.planned_destination || t.position != a.expected_position
            {
                a.path.clear();
                a.corner = 0;
                a.velocity = 0.0;
                if let Some(path) = nav.agent_route(t.position, a.destination.unwrap()) {
                    a.path.extend(path);
                }
                a.revision = nav.revision;
                a.planned_destination = a.destination;
            }
            a.velocity = (a.velocity + a.acceleration * dt).min(a.speed);
            let mut remaining = a.velocity * dt;
            while let Some(&target) = a.path.get(a.corner) {
                let delta = target - t.position;
                let distance = delta.length();
                if distance <= remaining {
                    t.position = target;
                    remaining -= distance;
                    a.corner += 1;
                } else {
                    t.position += delta / distance * remaining;
                    break;
                }
            }
            if a.corner == a.path.len() {
                a.velocity = 0.0;
            }
            a.expected_position = t.position;
        });
    ctx.world.resource_mut::<NavMesh3D>().state = Some(nav);
}
