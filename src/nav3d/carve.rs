use super::*;

pub(super) fn bake_layers(
    verts: &[f32],
    tris: &[i32],
    config: &bake::BakeConfig,
    original: detour::NavMesh,
) -> Result<(detour::NavMesh, tilecache::TileCache), NavBakeError> {
    let c = config.recast;
    if c.tile_size > 255 {
        return Err(error("tile cache requires tile_size <= 255"));
    }
    let layout = *original.params();
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
        if self.pending {
            return Ok(());
        }
        if !self.carving_changed() {
            return Ok(());
        }
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
            let r = self.cache.add_obstacle(shape).map_err(error)?;
            if index < self.carved.len() {
                self.cache.remove_obstacle(self.carve_handles[index]).map_err(error)?;
                self.carve_handles[index] = r;
                self.carved[index] = o;
            } else {
                self.carve_handles.push(r);
                self.carved.push(o);
            }
        } else {
            self.cache
                .remove_obstacle(self.carve_handles.pop().unwrap())
                .map_err(error)?;
            self.carved.pop();
        }
        self.pending = true;
        Ok(())
    }
    pub(super) fn update_carving(&mut self, budget: usize) -> Result<usize, NavBakeError> {
        let mut count = 0;
        if budget > 0 {
            self.sync_carving()?;
        }
        while self.pending && count < budget {
            self.pending = !self.cache.update(0.0, &mut self.mesh).map_err(error)?;
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
    fn refs(s: &NavState) -> Vec<Option<detour::PolyRef>> {
        (0..s.mesh.max_tiles() as usize)
            .map(|i| s.mesh.poly_ref_base(i))
            .collect()
    }
    #[test]
    fn rebuilding_preserves_untouched_tiles_and_respects_budget() {
        let floor = NavSource::from_mesh(&MeshData::plane(20.0), Transform3D::default());
        let mut s = NavState::bake(&[floor], NavBakeSettings::default()).unwrap();
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
