//! Each frame: the visible [`Model`]s, as GPU instances grouped by mesh, and
//! the models that cast shadows into the sun's shadow map.

use glam::{Mat4, Vec3, Vec4};

use super::{Assets3D, Camera3D, Lighting, Model, TERRAIN_LODS, TerrainTile, Transform3D, linear};
use crate::World;

/// Width and height of each of the sun's shadow maps, in texels.
pub(crate) const SHADOW_MAP_SIZE: u32 = 2048;

/// The most shadow cascades: shadow maps, each covering a stretch of the
/// view further out than the one before.
pub(crate) const MAX_CASCADES: usize = 4;

/// One drawn model, as the GPU reads it: the model matrix's columns, and the
/// tint in linear color.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Instance3D {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
}

/// An instance before sorting: its mesh, whether the camera sees it, and
/// which cascades' shadow maps it casts into (one bit each).
type Entry = (u32, Instance3D, bool, u8);

/// This frame's 3D drawing. `instances` holds the camera's models sorted by
/// mesh, then the shadow casters sorted by mesh; `runs` and `shadow_runs`
/// say where each mesh's instances are. Kept between frames so it doesn't
/// allocate once warm.
#[derive(Default)]
pub(crate) struct Frame3D {
    pub instances: Vec<Instance3D>,
    pub lights: super::lights::LightGrid,
    /// (mesh id, first instance, count) for the camera, in mesh order.
    pub runs: Vec<(u32, u32, u32)>,
    /// The same for each cascade's shadow map; empty without shadows.
    pub shadow_runs: [Vec<(u32, u32, u32)>; MAX_CASCADES],
    pub view_proj: Mat4,
    /// How many cascades are drawn: 0 without shadows.
    pub cascades: usize,
    /// Each cascade's view-projection, from the sun.
    pub sun_view_proj: [Mat4; MAX_CASCADES],
    /// Each cascade's size, in world units per shadow-map texel.
    pub cascade_texel: [f32; MAX_CASCADES],
    /// The screen effects to run, in order: (effect, material).
    pub effects: Vec<(u32, u32)>,
    /// The compute passes to run, in order: (compute, target texture, material).
    pub computes: Vec<(u32, u32, u32)>,
    /// How many of `instances` the camera sees (the rest cast shadows).
    pub visible: usize,
    /// What each target camera draws, in `Ctx::target_cameras` order (only
    /// the main frame has these).
    pub views: Vec<ViewFrame>,
    /// The light the sky gave last, and what it was worked out from.
    pub sky_light: Option<(Lighting, super::Sky, super::SkyLight)>,
    scratch: Vec<Entry>,
    counts: Vec<u32>,
}

/// What one target camera draws this frame.
#[derive(Default)]
pub(crate) struct ViewFrame {
    /// The render target's texture id and size.
    pub target: u32,
    pub size: (u32, u32),
    pub camera: Vec3,
    /// What it draws with: the camera's, tilted onto the clip plane.
    pub view_proj: Mat4,
    /// Its models, without shadow casters (it uses the main view's).
    pub frame: Frame3D,
}

impl Frame3D {
    /// Fills the frame with what the camera sees and, with shadows on,
    /// what's inside the sun's shadow box.
    pub fn collect(&mut self, world: &mut World, assets: &Assets3D, camera: &Camera3D, lighting: &Lighting) {
        self.view_proj = camera.view_proj();
        self.lights.collect(world, camera);
        self.cascades = if lighting.shadows && lighting.shadow_distance > 0.0 {
            (lighting.shadow_cascades as usize).clamp(1, MAX_CASCADES)
        } else {
            0
        };
        let splits = cascade_splits(camera.near, lighting.shadow_distance, self.cascades);
        // Each cascade's box is open towards the sun: casters above it
        // still shade what's inside, so its near plane isn't tested.
        let mut sun_planes = [[Vec4::ZERO; 5]; MAX_CASCADES];
        for i in 0..self.cascades {
            let near = if i == 0 { camera.near } else { splits[i - 1] };
            let (m, texel) = sun_view_proj(camera, lighting.sun_direction, near, splits[i]);
            self.sun_view_proj[i] = m;
            self.cascade_texel[i] = texel;
            let [l, r, b, t, _, f] = frustum_planes(m);
            sun_planes[i] = [l, r, b, t, f];
        }
        let sun_planes = &sun_planes[..self.cascades];
        self.instances.clear();
        self.runs.clear();
        for runs in &mut self.shadow_runs {
            runs.clear();
        }
        self.scratch.clear();
        let planes = frustum_planes(self.view_proj);
        let scratch = &mut self.scratch;
        // `meshes`: what the camera draws, then what each cascade's shadow
        // map draws (the same for models; coarser for distant terrain).
        let mut add = |meshes: [u32; 1 + MAX_CASCADES],
                       t: &Transform3D,
                       color: [f32; 4],
                       center: Vec3,
                       radius: f32,
                       shadows: bool| {
            let matrix = t.matrix();
            let world_center = matrix.transform_point3(center);
            let world_radius = radius * t.scale.abs().max_element();
            let seen = sphere_visible(&planes, world_center, world_radius);
            let mut casts = 0;
            for (i, p) in sun_planes.iter().enumerate().filter(|_| shadows) {
                if sphere_visible(p, world_center, world_radius) {
                    casts |= 1 << i;
                }
            }
            if !seen && casts == 0 {
                return;
            }
            let instance = Instance3D {
                model: matrix.to_cols_array_2d(),
                color,
            };
            // One entry per distinct mesh, with the passes that draw it.
            let mut done = 0u8; // cascades already in an entry
            let camera_mesh = meshes[0];
            let in_cascades = |mesh: u32| {
                (0..MAX_CASCADES).fold(0u8, |m, i| {
                    if casts & (1 << i) != 0 && meshes[i + 1] == mesh {
                        m | 1 << i
                    } else {
                        m
                    }
                })
            };
            if seen {
                let mask = in_cascades(camera_mesh);
                done |= mask;
                scratch.push((camera_mesh, instance, true, mask));
            }
            while casts & !done != 0 {
                let first = (casts & !done).trailing_zeros() as usize;
                let mesh = meshes[first + 1];
                let mask = in_cascades(mesh);
                done |= mask;
                scratch.push((mesh, instance, false, mask));
            }
        };
        world.query::<(&Transform3D, &Model)>().for_each(|(t, m)| {
            if let Some((center, radius)) = assets.live_bounds(m.mesh.id) {
                let shadows = assets.casts_shadows(m.mesh.id);
                add(
                    [m.mesh.id; 1 + MAX_CASCADES],
                    t,
                    linear(m.color),
                    center,
                    radius,
                    shadows,
                );
            }
        });
        // Terrain: the detail level for how far the camera is from the
        // tile's bounds (the finest level's, which hold every coarser one).
        // Shadow cascade i draws at least level i: its texels are too big
        // for finer detail to show.
        world.query::<(&Transform3D, &TerrainTile)>().for_each(|(t, tile)| {
            let Some((center, radius)) = assets.live_bounds(tile.lods[0].id) else {
                return;
            };
            let scale = t.scale.abs().max_element();
            let away = t.matrix().transform_point3(center).distance(camera.position) - radius * scale;
            let level = tile.lod(away.max(0.0));
            let meshes: [u32; 1 + MAX_CASCADES] = std::array::from_fn(|i| {
                tile.lods[if i == 0 {
                    level
                } else {
                    level.max(i - 1).min(TERRAIN_LODS - 1)
                }]
                .id
            });
            if meshes.iter().all(|&m| assets.live_bounds(m).is_some()) {
                add(meshes, t, [1.0; 4], center, radius, true);
            }
        });
        let meshes = assets.bounds.len();
        let (counts, instances) = (&mut self.counts, &mut self.instances);
        self.visible = sort_runs(&self.scratch, |e| e.2, meshes, counts, instances, &mut self.runs);
        for (i, runs) in self.shadow_runs[..self.cascades].iter_mut().enumerate() {
            sort_runs(&self.scratch, |e| e.3 & (1 << i) != 0, meshes, counts, instances, runs);
        }
    }
}

/// Appends the entries of `scratch` that `keep` picks to `instances`, sorted
/// by mesh (a counting sort), and their runs to `runs`. Returns how many.
fn sort_runs(
    scratch: &[Entry],
    keep: impl Fn(&Entry) -> bool,
    meshes: usize,
    counts: &mut Vec<u32>,
    instances: &mut Vec<Instance3D>,
    runs: &mut Vec<(u32, u32, u32)>,
) -> usize {
    counts.clear();
    counts.resize(meshes, 0);
    for e in scratch.iter().filter(|e| keep(e)) {
        counts[e.0 as usize] += 1;
    }
    let base = instances.len() as u32;
    let mut start = base;
    for (mesh, count) in counts.iter_mut().enumerate() {
        if *count > 0 {
            runs.push((mesh as u32, start, *count));
        }
        let n = *count;
        *count = start; // now: where this mesh's next instance goes
        start += n;
    }
    let added = (start - base) as usize;
    instances.resize(instances.len() + added, Instance3D::default());
    for e in scratch.iter().filter(|e| keep(e)) {
        let at = &mut counts[e.0 as usize];
        instances[*at as usize] = e.1;
        *at += 1;
    }
    added
}

/// Where each of `count` cascades ends, as distances from the camera, up to
/// `distance`: a blend of even steps and steps growing by the same factor
/// (the "practical" split), so the near cascades are small and sharp.
pub(crate) fn cascade_splits(near: f32, distance: f32, count: usize) -> [f32; MAX_CASCADES] {
    const LOG_SHARE: f32 = 0.75;
    let near = near.max(1e-3);
    let distance = distance.max(near * 2.0);
    let mut splits = [distance; MAX_CASCADES];
    for (i, split) in splits.iter_mut().enumerate().take(count.saturating_sub(1)) {
        let k = (i + 1) as f32 / count as f32;
        let even = near + (distance - near) * k;
        let log = near * (distance / near).powf(k);
        *split = even + (log - even) * LOG_SHARE;
    }
    splits
}

/// The sun's view-projection for a shadow map covering what the camera sees
/// from `near` to `far` away, and its size in world units per texel: an
/// orthographic box around that part of the view, facing along the
/// sunlight, moved in whole shadow-map texels so shadow edges don't shimmer
/// as the camera moves.
pub(crate) fn sun_view_proj(camera: &Camera3D, sun_direction: Vec3, near: f32, far: f32) -> (Mat4, f32) {
    let light = sun_direction.try_normalize().unwrap_or(Vec3::NEG_Y);
    let far = far.max(camera.near * 2.0);
    // The bounding sphere of the view's slice: it doesn't change size as
    // the camera turns, so neither do texels.
    let slice = Camera3D {
        near: near.clamp(camera.near, far * 0.5),
        far,
        ..*camera
    }
    .view_proj()
    .inverse();
    let mut corners = [Vec3::ZERO; 8];
    for (i, corner) in corners.iter_mut().enumerate() {
        let ndc = Vec4::new(
            if i & 1 == 0 { -1.0 } else { 1.0 },
            if i & 2 == 0 { -1.0 } else { 1.0 },
            if i & 4 == 0 { 0.0 } else { 1.0 },
            1.0,
        );
        let p = slice * ndc;
        *corner = p.truncate() / p.w;
    }
    let center = corners.iter().copied().sum::<Vec3>() / 8.0;
    let radius = corners.iter().map(|c| c.distance(center)).fold(0.0, f32::max);
    let radius = (radius * 16.0).ceil() / 16.0;

    let up = if light.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let rotation = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, light, up);
    // Snap the center to whole texels in the light's view.
    let texel = 2.0 * radius / SHADOW_MAP_SIZE as f32;
    let snapped = (rotation.transform_point3(center) / texel).round() * texel;
    let center = rotation.inverse().transform_point3(snapped);

    // Reach far back towards the sun, so tall things outside the view still
    // cast into it.
    let back = radius * 4.0;
    let view = glam::camera::rh::view::look_to_mat4(center - light * back, light, up);
    let projection =
        glam::camera::rh::proj::directx::orthographic(-radius, radius, -radius, radius, 0.0, back + radius);
    (projection * view, texel)
}

/// The six planes of a view volume (`xyz` normal pointing in, `w`
/// distance), from a projection-times-view matrix with 0..1 depth: left,
/// right, bottom, top, near, far.
fn frustum_planes(m: Mat4) -> [Vec4; 6] {
    let r = [m.row(0), m.row(1), m.row(2), m.row(3)];
    let planes = [r[3] + r[0], r[3] - r[0], r[3] + r[1], r[3] - r[1], r[2], r[3] - r[2]];
    planes.map(|p| p / p.truncate().length().max(f32::MIN_POSITIVE))
}

fn sphere_visible(planes: &[Vec4], center: Vec3, radius: f32) -> bool {
    planes.iter().all(|p| p.truncate().dot(center) + p.w >= -radius)
}

#[cfg(test)]
mod tests {
    use super::super::TERRAIN_LODS;
    use super::*;
    use crate::{Color, Mesh};
    use glam::Vec2;

    fn camera() -> Camera3D {
        let mut camera = Camera3D::new(Vec2::new(800.0, 800.0));
        camera.position = Vec3::new(0.0, 0.0, 10.0);
        camera.look_at(Vec3::ZERO);
        camera.far = 100.0;
        camera
    }

    #[test]
    fn spheres_outside_the_view_are_culled() {
        let planes = frustum_planes(camera().view_proj());
        let seen = |at: Vec3| sphere_visible(&planes, at, 1.0);
        assert!(seen(Vec3::ZERO));
        assert!(!seen(Vec3::new(0.0, 0.0, 20.0)), "behind");
        assert!(!seen(Vec3::new(0.0, 0.0, -200.0)), "past far");
        assert!(!seen(Vec3::new(50.0, 0.0, 0.0)), "far right");
        assert!(seen(Vec3::new(5.0, 0.0, 0.0)), "just in");
        assert!(seen(Vec3::new(0.0, 0.0, 10.5)), "big enough to reach in");
    }

    #[test]
    fn visible_models_are_grouped_by_mesh() {
        let mut world = World::new();
        let assets = Assets3D {
            bounds: vec![(Vec3::ZERO, 1.0); 3],
            ..Default::default()
        };
        let model = |id| Model::new(Mesh { id }).with_color(Color::WHITE);
        for (i, mesh) in [2, 0, 2, 2, 0].into_iter().enumerate() {
            world.spawn((Transform3D::at(Vec3::new(i as f32, 0.0, 0.0)), model(mesh)));
        }
        world.spawn((Transform3D::at(Vec3::new(0.0, 0.0, 50.0)), model(1))); // behind the camera
        let mut frame = Frame3D::default();
        let lighting = Lighting {
            shadows: false,
            ..Default::default()
        };
        frame.collect(&mut world, &assets, &camera(), &lighting);
        assert_eq!(frame.runs, vec![(0, 0, 2), (2, 2, 3)]);
        assert_eq!(frame.cascades, 0);
        assert!(frame.shadow_runs.iter().all(Vec::is_empty));
        let xs: Vec<f32> = frame.instances.iter().map(|i| i.model[3][0]).collect();
        assert_eq!(xs, vec![1.0, 4.0, 0.0, 2.0, 3.0]);
    }

    #[test]
    fn terrain_tiles_draw_coarser_further_away() {
        let mut world = World::new();
        let assets = Assets3D {
            bounds: vec![(Vec3::ZERO, 1.0); TERRAIN_LODS],
            ..Default::default()
        };
        let tile = TerrainTile {
            lods: std::array::from_fn(|id| Mesh { id: id as u32 }),
            lod_distance: 5.0,
        };
        // The camera is at z = 10, looking at the origin.
        world.spawn((Transform3D::at(Vec3::new(0.0, 0.0, 7.0)), tile)); // 2 away: finest
        world.spawn((Transform3D::at(Vec3::ZERO), tile)); // 9 away: every 2nd height
        world.spawn((Transform3D::at(Vec3::new(0.0, 0.0, -30.0)), tile)); // 39 away: coarsest
        let lighting = Lighting {
            shadows: false,
            ..Default::default()
        };
        let mut frame = Frame3D::default();
        frame.collect(&mut world, &assets, &camera(), &lighting);
        assert_eq!(frame.runs, vec![(0, 0, 1), (1, 1, 1), (3, 2, 1)]);
    }

    #[test]
    fn shadow_casters_include_models_outside_the_view() {
        let mut world = World::new();
        let assets = Assets3D {
            bounds: vec![(Vec3::ZERO, 1.0)],
            ..Default::default()
        };
        let mut camera = camera();
        camera.position = Vec3::new(0.0, 5.0, 10.0);
        camera.look_at(Vec3::ZERO);
        let lighting = Lighting {
            sun_direction: Vec3::new(0.0, -1.0, -0.2),
            shadow_distance: 30.0,
            shadow_cascades: 1,
            ..Default::default()
        };
        let model = Model::new(Mesh { id: 0 });
        world.spawn((Transform3D::at(Vec3::ZERO), model)); // seen
        world.spawn((Transform3D::at(Vec3::new(0.0, 40.0, 2.0)), model)); // high above: casts, unseen
        world.spawn((Transform3D::at(Vec3::new(500.0, 0.0, 0.0)), model)); // far away: neither
        let mut frame = Frame3D::default();
        frame.collect(&mut world, &assets, &camera, &lighting);
        assert_eq!(frame.visible, 1);
        assert_eq!(frame.runs, vec![(0, 0, 1)]);
        assert_eq!(frame.shadow_runs[0], vec![(0, 1, 2)]);
    }

    #[test]
    fn shadowless_meshes_cast_no_shadow() {
        let mut world = World::new();
        let assets = Assets3D {
            bounds: vec![(Vec3::ZERO, 1.0); 2],
            shadowless: vec![false, true],
            ..Default::default()
        };
        let mut camera = camera();
        camera.position = Vec3::new(0.0, 5.0, 10.0);
        camera.look_at(Vec3::ZERO);
        let lighting = Lighting {
            sun_direction: Vec3::new(0.0, -1.0, -0.2),
            shadow_distance: 30.0,
            shadow_cascades: 1,
            ..Default::default()
        };
        world.spawn((Transform3D::at(Vec3::ZERO), Model::new(Mesh { id: 0 })));
        world.spawn((Transform3D::at(Vec3::X), Model::new(Mesh { id: 1 }))); // seen, casts nothing
        world.spawn((Transform3D::at(Vec3::new(0.0, 40.0, 2.0)), Model::new(Mesh { id: 1 }))); // neither
        let mut frame = Frame3D::default();
        frame.collect(&mut world, &assets, &camera, &lighting);
        assert_eq!(frame.visible, 2);
        assert_eq!(frame.runs, vec![(0, 0, 1), (1, 1, 1)]);
        assert_eq!(frame.shadow_runs[0], vec![(0, 2, 1)]);
    }

    #[test]
    fn farther_shadow_cascades_draw_coarser_terrain() {
        let mut world = World::new();
        // Bounds big enough to reach into every cascade.
        let assets = Assets3D {
            bounds: vec![(Vec3::ZERO, 500.0); TERRAIN_LODS],
            ..Default::default()
        };
        let tile = TerrainTile {
            lods: std::array::from_fn(|id| Mesh { id: id as u32 }),
            lod_distance: 5.0,
        };
        world.spawn((Transform3D::default(), tile));
        let lighting = Lighting {
            shadow_distance: 100.0,
            shadow_cascades: 4,
            ..Default::default()
        };
        let mut frame = Frame3D::default();
        frame.collect(&mut world, &assets, &camera(), &lighting);
        assert_eq!(frame.runs.iter().map(|r| r.0).collect::<Vec<_>>(), vec![0]);
        let meshes: Vec<Vec<u32>> = frame
            .shadow_runs
            .iter()
            .map(|c| c.iter().map(|r| r.0).collect())
            .collect();
        assert_eq!(meshes, vec![vec![0], vec![1], vec![2], vec![3]]);
    }

    #[test]
    fn the_shadow_box_holds_the_near_view_and_snaps_to_texels() {
        let camera = camera();
        let sun_dir = Vec3::new(-0.3, -1.0, -0.2);
        let (sun, _) = sun_view_proj(&camera, sun_dir, camera.near, 20.0);
        let in_box = |p: Vec3| {
            let c = sun * p.extend(1.0);
            let n = c.truncate() / c.w;
            n.x.abs() <= 1.0 && n.y.abs() <= 1.0 && (0.0..=1.0).contains(&n.z)
        };
        assert!(in_box(Vec3::ZERO), "what the camera looks at");
        assert!(in_box(camera.position + camera.forward() * 15.0));
        assert!(!in_box(Vec3::new(0.0, 0.0, -200.0)), "past the shadow distance");
        // A small camera move gives the same box, or one moved by whole texels.
        let mut moved = camera;
        moved.position.x += 0.013;
        let a = sun * Vec4::new(0.0, 0.0, 0.0, 1.0);
        let b = sun_view_proj(&moved, sun_dir, moved.near, 20.0).0 * Vec4::new(0.0, 0.0, 0.0, 1.0);
        let texels = (a - b).truncate().truncate() * SHADOW_MAP_SIZE as f32 / 2.0;
        assert!((texels - texels.round()).length() < 2e-2, "{texels}");
    }

    #[test]
    fn cascades_grow_outwards_and_end_at_the_shadow_distance() {
        let s = cascade_splits(0.1, 900.0, 4);
        assert!(s[0] > 0.1 && s[0] < s[1] && s[1] < s[2] && s[2] < s[3], "{s:?}");
        assert_eq!(s[3], 900.0);
        assert!(
            s[0] < 900.0 / 8.0,
            "the first cascade is much smaller than even steps: {s:?}"
        );
        assert_eq!(cascade_splits(0.1, 60.0, 1)[0], 60.0);
    }

    #[test]
    fn near_models_cast_into_near_cascades_only() {
        let mut world = World::new();
        let assets = Assets3D {
            bounds: vec![(Vec3::ZERO, 0.5)],
            ..Default::default()
        };
        let mut camera = camera();
        camera.position = Vec3::new(0.0, 2.0, 0.0);
        camera.look_at(Vec3::new(0.0, 2.0, -1.0));
        camera.near = 0.1;
        camera.far = 1000.0;
        let lighting = Lighting {
            sun_direction: Vec3::new(0.0, -1.0, 0.0),
            shadow_distance: 400.0,
            ..Default::default()
        };
        let model = Model::new(Mesh { id: 0 });
        world.spawn((Transform3D::at(Vec3::new(0.0, 0.0, -3.0)), model)); // near
        world.spawn((Transform3D::at(Vec3::new(0.0, 0.0, -350.0)), model)); // far
        let mut frame = Frame3D::default();
        frame.collect(&mut world, &assets, &camera, &lighting);
        assert_eq!(frame.cascades, 4);
        let counts = frame.shadow_runs.map(|r| r.iter().map(|r| r.2).sum::<u32>());
        assert_eq!(counts[0], 1, "only the near model is in the first cascade");
        assert_eq!(
            counts[3], 2,
            "the last cascade's box reaches the far model, and back to the camera"
        );
        assert!(
            frame.cascade_texel[0] < frame.cascade_texel[3] / 10.0,
            "near texels are much finer"
        );
    }
}
