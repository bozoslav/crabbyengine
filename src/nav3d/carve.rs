use super::*;

/// The tile grid a bake produces, computed the way the backend's tile baker
/// does so both bake paths agree on tile positions and polygon references.
pub(super) struct TileGrid {
    origin: [f32; 3],
    tile_world: f32,
    pub(super) count: usize,
    /// The tile width in cells that was chosen.
    pub(super) tile_size: u32,
    border: u32,
}
impl TileGrid {
    /// Lays out the tiles, growing `tile_size` (at most to 255) when the level
    /// would otherwise need more than `MAX_TILES`.
    pub(super) fn new(verts: &[f32], s: &NavBakeSettings) -> Result<Self, NavBakeError> {
        let (min, max) = recast::calc_bounds(verts);
        let cells =
            |axis: usize| ((f64::from(max[axis]) - f64::from(min[axis])) / f64::from(s.cell_size) + 0.5).floor();
        let (gw, gh) = (cells(0), cells(2));
        // Voxel coordinates are 32-bit in the backend.
        if gw.max(gh) >= f64::from(i32::MAX) / 2.0 {
            return Err(error(format!(
                "navigation bake grid is too large; raise NavBakeSettings::cell_size (now {})",
                s.cell_size
            )));
        }
        let tiles = |ts: u32| {
            let ts = f64::from(ts);
            (
                ((gw + ts - 1.0) / ts).floor().max(1.0),
                ((gh + ts - 1.0) / ts).floor().max(1.0),
            )
        };
        let tile_size = (s.tile_size..=255).find(|&ts| {
            let (tw, th) = tiles(ts);
            tw * th <= MAX_TILES as f64
        });
        let Some(tile_size) = tile_size else {
            let (tw, th) = tiles(255);
            return Err(error(format!(
                "navigation bake needs {tw} x {th} tiles even at the largest tile_size (255), but a navmesh \
                 holds at most {MAX_TILES} tiles; raise NavBakeSettings::cell_size (now {}) or bake the level \
                 in separate meshes",
                s.cell_size
            )));
        };
        let (tw, th) = tiles(tile_size);
        Ok(Self {
            origin: min,
            tile_world: tile_size as f32 * s.cell_size,
            count: (tw * th) as usize,
            tile_size,
            border: (s.agent_radius / s.cell_size).ceil() as u32 + 3,
        })
    }
    /// Cap on one tile's navmesh bytes. A tile has at most `(size + 2 * border)^2`
    /// cells and a polygon mesh needs well under 64 bytes per cell.
    pub(super) fn max_tile_bytes(&self) -> usize {
        let side = (self.tile_size + 2 * self.border) as usize;
        (side * side * 64).max(1 << 20)
    }
    /// The navmesh layout the backend's tile baker would choose for this grid.
    pub(super) fn layout(&self) -> detour::NavMeshParams {
        let tile_bits = self.count.next_power_of_two().trailing_zeros();
        detour::NavMeshParams {
            orig: self.origin,
            tile_width: self.tile_world,
            tile_height: self.tile_world,
            max_tiles: 1 << tile_bits,
            max_polys: 1 << (22 - tile_bits).min(20),
        }
    }
}

pub(super) fn bake_layers(
    verts: &[f32],
    tris: &[i32],
    config: &bake::BakeConfig,
    layout: detour::NavMeshParams,
) -> Result<(detour::NavMesh, tilecache::TileCache), NavBakeError> {
    let c = config.recast;
    let mut params = tilecache::TileCacheParams::new(
        c.tile_size as u8,
        c.tile_size as u8,
        c.cs,
        c.ch,
        layout.max_tiles as usize * 4,
        256,
    );
    params.orig = layout.orig;
    params.walkable_height = config.walkable_height;
    params.walkable_radius = config.walkable_radius;
    params.walkable_climb = config.walkable_climb;
    let mut cache = tilecache::TileCache::new(
        params,
        tilecache::RawCompressor,
        tilecache::DefaultMeshProcess,
        Limits::default(),
    )
    .map_err(error)?;
    let mut mesh = detour::NavMesh::new(&detour::NavMeshParams {
        max_tiles: layout.max_tiles * 4,
        ..layout
    })
    .map_err(error)?;
    let mut ws = Workspace::new();
    let mut areas = vec![0; tris.len() / 3];
    recast::mark_walkable_triangles(&mut ws, c.walkable_slope_angle, verts, tris, &mut areas).map_err(error)?;
    let max = verts
        .as_chunks::<3>()
        .0
        .iter()
        .fold(Vec3::splat(f32::NEG_INFINITY), |m, v| m.max(Vec3::from_slice(v)));
    let origin = Vec3::from_array(layout.orig);
    let nx = ((max.x - origin.x) / layout.tile_width).ceil() as i32;
    let nz = ((max.z - origin.z) / layout.tile_height).ceil() as i32;
    let border = c.walkable_radius + 3;
    for z in 0..nz {
        for x in 0..nx {
            let mut lo = origin + Vec3::new(x as f32 * layout.tile_width, 0.0, z as f32 * layout.tile_height);
            let mut hi = lo + Vec3::new(layout.tile_width, max.y - origin.y + c.ch, layout.tile_height);
            lo -= Vec3::new(border as f32 * c.cs, 0.0, border as f32 * c.cs);
            hi += Vec3::new(border as f32 * c.cs, 0.0, border as f32 * c.cs);
            let size = c.tile_size + 2 * border;
            let mut hf = recast::Heightfield::new(&mut ws, size, size, lo.to_array(), hi.to_array(), c.cs, c.ch)
                .map_err(error)?;
            hf.rasterize_triangles(&mut ws, verts, tris, &areas, c.walkable_climb)
                .map_err(error)?;
            hf.filter_low_hanging_walkable_obstacles(&mut ws, c.walkable_climb)
                .map_err(error)?;
            hf.filter_ledge_spans(&mut ws, c.walkable_height, c.walkable_climb)
                .map_err(error)?;
            hf.filter_walkable_low_height_spans(&mut ws, c.walkable_height)
                .map_err(error)?;
            let mut chf =
                recast::CompactHeightfield::build(&mut ws, c.walkable_height, c.walkable_climb, &hf).map_err(error)?;
            recast::erode_walkable_area(&mut ws, c.walkable_radius, &mut chf).map_err(error)?;
            let layers = recast::HeightfieldLayerSet::build(&mut ws, &chf, border, c.walkable_height).map_err(error)?;
            for (i, l) in layers.layers.iter().enumerate() {
                let mut h = tilecache::LayerHeader::new(l.width as u8, l.height as u8);
                h.tx = x;
                h.ty = z;
                h.tlayer = i as i32;
                h.bmin = l.bmin;
                h.bmax = l.bmax;
                h.hmin = l.hmin as u16;
                h.hmax = l.hmax as u16;
                h.minx = l.minx as u8;
                h.maxx = l.maxx as u8;
                h.miny = l.miny as u8;
                h.maxy = l.maxy as u8;
                let data =
                    tilecache::build_layer(&mut ws, &tilecache::RawCompressor, &h, &l.heights, &l.areas, &l.cons)
                        .map_err(error)?;
                let r = cache.add_tile(data).map_err(error)?;
                cache.build_nav_mesh_tile(r, &mut mesh).map_err(error)?;
            }
        }
    }
    Ok((mesh, cache))
}

impl NavState {
    pub(super) fn carving_changed(&self) -> bool {
        if self.cache.is_none() {
            return false;
        }
        let mut desired = self.obstacles.iter().filter(|o| o.carve);
        for b in &self.carved {
            let Some(a) = desired.next() else {
                return true;
            };
            if a.half_extents != b.half_extents
                || !a.center.is_finite()
                || a.center.distance(b.center) >= self.settings.cell_size * 0.5
            {
                return true;
            }
        }
        desired.next().is_some()
    }
    fn sync_carving(&mut self) -> Result<(), NavBakeError> {
        if self.pending || !self.carving_changed() {
            return Ok(());
        }
        let Some(cache) = &mut self.cache else {
            return Ok(());
        };
        let desired: Vec<_> = self.obstacles.iter().copied().filter(|o| o.carve).collect();
        // One edit at a time keeps the backend request queue bounded and avoids
        // rebuilding unchanged obstacles. Ordered ECS components provide identity.
        let changed = desired.iter().zip(&self.carved).position(|(a, b)| {
            a.half_extents != b.half_extents
                || !a.center.is_finite()
                || a.center.distance(b.center) >= self.settings.cell_size * 0.5
        });
        let index = changed.unwrap_or(desired.len().min(self.carved.len()));
        if changed.is_none() && desired.len() == self.carved.len() {
            return Ok(());
        }
        if index < desired.len() {
            let o = desired[index];
            let radius = Vec3::new(
                self.settings.agent_radius,
                self.settings.agent_height,
                self.settings.agent_radius,
            );
            let shape = tilecache::ObstacleShape::Box {
                bmin: (o.center - o.half_extents - radius).to_array(),
                bmax: (o.center + o.half_extents + Vec3::new(radius.x, 0.0, radius.z)).to_array(),
            };
            // Validate/add before removing the old obstacle: errors leave it intact.
            let r = cache.add_obstacle(shape).map_err(error)?;
            if index < self.carved.len() {
                cache.remove_obstacle(self.carve_handles[index]).map_err(error)?;
                self.carve_handles[index] = r;
                self.carved[index] = o;
            } else {
                self.carve_handles.push(r);
                self.carved.push(o);
            }
        } else {
            cache
                .remove_obstacle(self.carve_handles.pop().unwrap())
                .map_err(error)?;
            self.carved.pop();
        }
        self.pending = true;
        Ok(())
    }
    pub(super) fn update_carving(&mut self, budget: usize) -> Result<usize, NavBakeError> {
        let mut count = 0;
        if self.cache.is_none() {
            return Ok(0);
        }
        if budget > 0 {
            self.sync_carving()?;
        }
        while self.pending && count < budget {
            let cache = self.cache.as_mut().expect("checked above");
            self.pending = !cache.update(0.0, &mut self.mesh).map_err(error)?;
            count += 1;
        }
        let recover = self.carve_failed;
        self.carve_failed = false;
        if count > 0 || recover {
            self.polygons.clear();
            for i in 0..self.mesh.max_tiles() as usize {
                if let Some(tile) = self.mesh.tile(i) {
                    let base = self.mesh.poly_ref_base(i).unwrap();
                    for (j, p) in tile.polys.iter().enumerate() {
                        let mut min = Vec3::splat(f32::INFINITY);
                        let mut max = Vec3::splat(f32::NEG_INFINITY);
                        for &v in &p.verts[..p.vert_count as usize] {
                            let at = v as usize * 3;
                            let v = Vec3::from_slice(&tile.verts[at..at + 3]);
                            min = min.min(v);
                            max = max.max(v);
                        }
                        self.polygons.push(PolygonBounds {
                            reference: base | j as detour::PolyRef,
                            min,
                            max,
                        });
                    }
                }
            }
            self.apply_obstacles();
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grid_for(size: f32, s: &NavBakeSettings) -> Result<TileGrid, NavBakeError> {
        let h = size / 2.0;
        TileGrid::new(&[-h, 0.0, -h, h, 0.0, h], s)
    }
    #[test]
    fn grid_layout_matches_the_backend_tile_baker() {
        for (size, tile_size, cell_size) in [
            (20.0, 32, 0.2),
            (40.0, 64, 0.2),
            (333.0, 48, 0.3),
            (7.0, 255, 0.1),
            (600.0, 8, 0.5),
        ] {
            let s = NavBakeSettings {
                tile_size,
                cell_size,
                ..Default::default()
            };
            let h = size / 2.0;
            let verts = [-h, 0.0, -h, -h, 0.0, h, h, 0.0, h, h, 0.0, -h];
            let tris = [0, 1, 2, 0, 2, 3];
            let grid = TileGrid::new(&verts, &s).unwrap();
            let mut config = bake::BakeConfig::default();
            config.recast.cs = cell_size;
            config.recast.tile_size = grid.tile_size as i32;
            let baked = bake::tiles(&bake::Input::new(&verts, &tris), &config, &mut Workspace::new()).unwrap();
            assert_eq!(grid.layout(), baked.params, "{size} {tile_size} {cell_size}");
            assert_eq!(grid.count, baked.tiles.len());
        }
    }
    #[test]
    fn tiles_grow_for_big_levels_until_the_largest_size_cannot_fit() {
        let s = NavBakeSettings::default();
        assert_eq!(grid_for(800.0, &s).unwrap().tile_size, 32);
        let grown = grid_for(1700.0, &s).unwrap();
        assert!(grown.tile_size > 32 && grown.count <= MAX_TILES);
        assert_eq!(
            grid_for(1700.0, &NavBakeSettings { tile_size: 100, ..s })
                .unwrap()
                .tile_size,
            100
        );
        // 255 cells of 0.2 m: 51 m tiles, 16384 of them, 6.5 km square.
        assert!(grid_for(6500.0, &s).unwrap().tile_size >= 250);
        let message = grid_for(7000.0, &s).err().unwrap().to_string();
        assert!(message.contains("16384") && message.contains("cell_size"), "{message}");
    }
    fn refs(s: &NavState) -> Vec<Option<detour::PolyRef>> {
        (0..s.mesh.max_tiles() as usize)
            .map(|i| s.mesh.poly_ref_base(i))
            .collect()
    }
    #[test]
    fn rebuilding_preserves_untouched_tiles_and_respects_budget() {
        let floor = NavSource::from_mesh(&MeshData::plane(20.0), Transform3D::default());
        let settings = NavBakeSettings {
            carving: true,
            ..Default::default()
        };
        let mut s = NavState::bake(&[floor], settings).unwrap();
        let before = refs(&s);
        let mut o = NavObstacle3D::new(Vec3::new(0.0, 1.0, 0.0), Vec3::ONE);
        o.carve = true;
        s.set_obstacles(&[o]);
        let mut total = 0;
        while s.pending || s.carving_changed() {
            let old = refs(&s);
            let n = s.update_carving(1).unwrap();
            let changed = old.iter().zip(refs(&s)).filter(|(a, b)| **a != *b).count();
            assert!(changed <= 1, "{changed}");
            total += n;
        }
        assert!(total > 0 && total <= 8);
        let changed = before.iter().zip(refs(&s)).filter(|(a, b)| **a != *b).count();
        assert_eq!(changed, total);
        assert!(changed < before.iter().filter(|r| r.is_some()).count());
    }
}
