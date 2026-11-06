//! Each frame: the visible [`Model`]s, as GPU instances grouped by mesh, and
//! the models that cast shadows into the sun's shadow map.

use glam::{Mat4, Vec3, Vec4};

use super::{
    Assets3D, Camera3D, Lighting, LodGroup, LodPick, LodRenderer, LodSettings, LodShadow, Model, ReceivesDecals,
    ShadowMesh, Skeleton, SkinFrame, SkinOf, TERRAIN_LODS, TerrainTile, Transform3D, linear,
};
use crate::World;

/// Default width and height of each of the sun's shadow maps, in texels.
pub(crate) const SHADOW_MAP_SIZE: u32 = 2048;
/// Smallest and largest sun shadow map a game may ask for.
pub(crate) const MIN_SHADOW_RESOLUTION: u32 = 256;
pub(crate) const MAX_SHADOW_RESOLUTION: u32 = 4096;

/// The most shadow cascades: shadow maps, each covering a stretch of the
/// view further out than the one before.
pub(crate) const MAX_CASCADES: usize = 4;
/// Shadow views: the sun's cascades in `0..MAX_CASCADES`, then point and
/// spot lights' views.
pub(crate) const MAX_SHADOW_VIEWS: usize = MAX_CASCADES + super::lights::MAX_LIGHT_SHADOW_VIEWS;

/// One drawn model, as the GPU reads it: the model matrix's columns, and the
/// tint in linear color.
///
/// The model matrix is affine, so its fourth row is always (0, 0, 0, 1):
/// its first slot holds the decal layers the model takes, and its second
/// the instance's crossfade (see [`Instance3D::with_fade`]), which costs no
/// more instance data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Instance3D {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
}

impl Instance3D {
    /// The same, dithered for a crossfade: `fade` is 0 for none (drawn
    /// whole), else the share of pixels kept (0 to 1): positive for the
    /// level fading in, and negative for the one fading out, which keeps the
    /// pixels the other doesn't (`-(1 - fade)` for a level that is `fade`
    /// of the way to the other). The shader reads it from the second column's `w`.
    pub(crate) fn with_fade(mut self, fade: f32) -> Self {
        self.model[1][3] = fade;
        self
    }

    /// The crossfade set by [`with_fade`](Self::with_fade).
    pub(crate) fn fade(&self) -> f32 {
        self.model[1][3]
    }
}

/// Set in the mesh id of a run whose instances are crossfading (dithered):
/// their depth and shadow passes run a fragment shader for the dither.
pub(crate) const FADED: u32 = 1 << 31;

/// An instance before sorting: its mesh, whether the camera sees it, and
/// which shadow views it casts into (one bit each).
type Entry = (u32, Instance3D, bool, u32);

/// This frame's 3D drawing. `instances` holds the camera's models sorted by
/// mesh, then the shadow casters sorted by mesh; `runs` and `shadow_runs`
/// say where each mesh's instances are. Kept between frames so it doesn't
/// allocate once warm.
#[derive(Default)]
pub(crate) struct Frame3D {
    pub instances: Vec<Instance3D>,
    pub probes: [super::probe::ProbeUniform; 4],
    pub probe_uploads: Vec<(u32, std::sync::Arc<Vec<u16>>)>,
    pub probe_bakes: Vec<(u32, [u32; 6])>,
    pub particles: super::particles::ParticleFrame,
    pub lights: super::lights::LightGrid,
    /// Projected decals, in the lights' grid.
    pub decals: super::projected::DecalGrid,
    /// The decal texture array's id, when the game set one.
    pub decal_array: Option<u32>,
    /// (mesh id, first instance, count) for the camera, in mesh order.
    pub runs: Vec<(u32, u32, u32)>,
    /// The same for each shadow view (see [`MAX_SHADOW_VIEWS`]); empty
    /// without shadows.
    pub shadow_runs: [Vec<(u32, u32, u32)>; MAX_SHADOW_VIEWS],
    pub view_proj: Mat4,
    /// How many cascades are drawn: 0 without shadows.
    pub cascades: usize,
    /// The sun's shadow map size and the light shadow views' size this
    /// frame, in texels: what `Lighting` asks for, within the GPU's limit.
    pub shadow_size: u32,
    pub light_shadow_size: u32,
    /// How many point and spot light shadow views `Lighting` allows (0 with
    /// shadows off).
    pub light_budget: usize,
    /// The largest texture the GPU makes (0: not known yet).
    pub size_limit: u32,
    /// How many point and spot light shadow views are drawn (their matrices
    /// are `lights.shadow_views`).
    pub light_views: usize,
    /// Each cascade's view-projection, from the sun.
    pub sun_view_proj: [Mat4; MAX_CASCADES],
    /// Each cascade's size, in world units per shadow-map texel.
    pub cascade_texel: [f32; MAX_CASCADES],
    /// Which cascades are drawn this frame, one bit each: the rest keep
    /// what an earlier frame drew into them (see `Lighting::shadow_refresh`).
    pub redrawn: u32,
    /// What each far cascade's shadow map holds, from the frame that drew it.
    cascade_cache: [Option<CachedCascade>; MAX_CASCADES],
    /// Counts frames, for taking turns redrawing the far cascades.
    frame_number: u32,
    /// The screen effects to run, in order: (effect, material).
    pub effects: Vec<(u32, u32)>,
    /// The compute passes to run, in order: (compute, target texture, material).
    pub computes: Vec<(u32, u32, u32)>,
    /// How many of `instances` the camera sees (the rest cast shadows).
    pub visible: usize,
    /// An earlier frame's depth to cull hidden models with, when the camera
    /// has [`occlusion_culling`](Camera3D::occlusion_culling) on.
    pub occlusion: Option<super::occlusion::OcclusionGrid>,
    /// How many models (and shadow-only casters) in view were culled as
    /// hidden this frame.
    pub occluded: usize,
    /// What each target camera draws, in `Ctx::target_cameras` order (only
    /// the main frame has these).
    pub views: Vec<ViewFrame>,
    /// The light the sky gave last, and what it was worked out from.
    pub sky_light: Option<(Lighting, super::Sky, super::SkyLight)>,
    /// This frame's skinned models (only the main frame has these; target
    /// cameras draw the same). In `runs`, skinned model `k` is mesh id
    /// `assets.bounds.len() + k`.
    pub skins: SkinFrame,
    /// The global LOD settings to select with (set by the engine each frame).
    pub lod: LodSettings,
    /// Which models moved since last frame (when temporal antialiasing is on).
    pub motion: super::motion::Motion,
    scratch: Vec<Entry>,
    counts: Vec<u32>,
    /// Per model, which passes see it (see `collect`), kept between frames.
    flags: Vec<u32>,
    /// Per LOD group: what the camera picked, and which passes see any of it.
    lod_picks: Vec<(LodPick, u32)>,
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
    pub fn collect(
        &mut self,
        world: &mut World,
        assets: &Assets3D,
        skins: &SkinFrame,
        camera: &Camera3D,
        lighting: &Lighting,
    ) {
        self.view_proj = camera.view_proj();
        self.particles.collect(world, camera);
        self.shadow_size = lighting.shadow_size(self.size_limit);
        self.light_shadow_size = lighting.light_shadow_size(self.size_limit);
        self.light_budget = if lighting.shadows {
            lighting.light_shadow_budget()
        } else {
            0
        };
        self.lights
            .collect(world, camera, self.light_budget, self.light_shadow_size);
        self.decals.records.clear();
        self.decals.lists.clear();
        self.decal_array = assets.decal_array.map(|a| a.id);
        if assets.decals_supported {
            self.decals.collect(
                world,
                camera,
                self.lights.depth,
                assets.decal_array.map_or(0, |a| a.layers),
                self.lights.lists.len(),
                self.lights.lights.len(),
            );
        }
        self.light_views = self.lights.shadow_views.len();
        self.cascades = if lighting.shadows && lighting.shadow_distance > 0.0 {
            (lighting.shadow_cascades as usize).clamp(1, MAX_CASCADES)
        } else {
            0
        };
        let splits = cascade_splits(camera.near, lighting.shadow_distance, self.cascades);
        // Each cascade's box is open towards the sun: casters above it
        // still shade what's inside, so its near plane isn't tested.
        // Far cascades can keep an earlier frame's shadow map: each is drawn
        // into a box a little bigger than it needs, and kept while what the
        // camera sees of that stretch stays inside it, the sun stays put and
        // its turn hasn't come (see `Lighting::shadow_refresh`). Kept
        // cascades cull every caster, so they draw nothing.
        let refresh = lighting.shadow_refresh.clamp(1, MAX_SHADOW_REFRESH);
        let first_far = if refresh > 1 {
            far_cascades(self.cascades)
        } else {
            self.cascades
        };
        self.frame_number = self.frame_number.wrapping_add(1);
        self.redrawn = 0;
        let mut sun_planes = [[Vec4::ZERO; 5]; MAX_CASCADES];
        for i in 0..self.cascades {
            let near = if i == 0 { camera.near } else { splits[i - 1] };
            if i >= first_far {
                let (center, radius) = slice_sphere(camera, near, splits[i]);
                let turn = (self.frame_number + i as u32).is_multiple_of(refresh);
                let kept = self.cascade_cache[i].filter(|c| {
                    !turn
                        && c.size == self.shadow_size
                        && c.cascades == self.cascades
                        && c.sun == lighting.sun_direction
                        && center.distance(c.center) + radius <= c.reach
                });
                if let Some(c) = kept {
                    self.sun_view_proj[i] = c.view_proj;
                    self.cascade_texel[i] = c.texel;
                    // Planes no sphere is inside of: nothing is drawn.
                    sun_planes[i] = [Vec4::new(0.0, 0.0, 0.0, -f32::MAX); 5];
                    continue;
                }
                let (m, texel, reach) = sun_box(
                    camera,
                    lighting.sun_direction,
                    near,
                    splits[i],
                    self.shadow_size,
                    1.0 + CACHE_MARGIN,
                );
                self.cascade_cache[i] = Some(CachedCascade {
                    view_proj: m,
                    texel,
                    center,
                    reach,
                    sun: lighting.sun_direction,
                    size: self.shadow_size,
                    cascades: self.cascades,
                });
                self.sun_view_proj[i] = m;
                self.cascade_texel[i] = texel;
            } else {
                self.cascade_cache[i] = None;
                let (m, texel) = sun_view_proj(camera, lighting.sun_direction, near, splits[i], self.shadow_size);
                self.sun_view_proj[i] = m;
                self.cascade_texel[i] = texel;
            }
            self.redrawn |= 1 << i;
            let [l, r, b, t, _, f] = frustum_planes(self.sun_view_proj[i]);
            sun_planes[i] = [l, r, b, t, f];
        }
        for cache in &mut self.cascade_cache[self.cascades..] {
            *cache = None;
        }
        let sun_planes = &sun_planes[..self.cascades];
        let mut light_planes = [[Vec4::ZERO; 6]; super::lights::MAX_LIGHT_SHADOW_VIEWS];
        for (planes, m) in light_planes.iter_mut().zip(&self.lights.shadow_views) {
            *planes = frustum_planes(*m);
        }
        let light_planes = &light_planes[..self.light_views];
        self.instances.clear();
        self.runs.clear();
        self.motion.begin();
        for runs in &mut self.shadow_runs {
            runs.clear();
        }
        self.scratch.clear();
        let planes = frustum_planes(self.view_proj);
        // What the sun's shadows can fall on: the view out to the shadow
        // distance. A caster is drawn into the cascades only if its shadow,
        // swept along the sunlight, can reach that (Unity's caster sweep).
        let receivers = frustum_planes(
            Camera3D {
                far: lighting
                    .shadow_distance
                    .clamp(camera.near * 2.0, camera.far.max(camera.near * 2.0)),
                ..*camera
            }
            .view_proj(),
        );
        let sunlight = lighting.sun_direction.try_normalize().unwrap_or(Vec3::NEG_Y);
        let sweep = lighting.shadow_distance * 2.0;
        let texels = self.cascade_texel;
        let scratch = &mut self.scratch;
        let motion = &mut self.motion;
        // `meshes`: what the camera draws, then what each cascade's shadow
        // map draws (the same for models; coarser for distant terrain).
        // Which passes see a model: bit 0 the camera, bit 1 + i shadow view
        // i (cascades, then light views from MAX_CASCADES on).
        // `bounds`: the camera mesh's, and the shadow mesh's (`None`: no
        // shadow), as (center, radius) in model space.
        // Hidden behind what an earlier frame drew: still casts shadows.
        let occlusion = self.occlusion.as_ref().filter(|_| camera.occlusion_culling);
        let occluded = std::sync::atomic::AtomicUsize::new(0);
        let classify_at = |matrix: Mat4, scale: f32, bounds: (Vec3, f32), shadow_bounds: Option<(Vec3, f32)>| -> u32 {
            let (center, radius) = (matrix.transform_point3(bounds.0), bounds.1 * scale);
            let mut seen = sphere_visible(&planes, center, radius);
            if seen
                && let Some(grid) = occlusion
                && grid.hides(center - radius, center + radius, camera.position)
            {
                occluded.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                seen = false;
            }
            let mut flags = u32::from(seen);
            if let Some((center, radius)) = shadow_bounds {
                let (world_center, world_radius) = (matrix.transform_point3(center), radius * scale);
                // How far the shadow can reach: down through the whole view.
                let reach = world_center.distance(camera.position) + sweep;
                if capsule_visible(&receivers, world_center, world_center + sunlight * reach, world_radius) {
                    for (i, p) in sun_planes.iter().enumerate() {
                        // Smaller than a texel: its shadow wouldn't show.
                        if world_radius * 2.0 >= texels[i] && sphere_visible(p, world_center, world_radius) {
                            flags |= 2 << i;
                        }
                    }
                }
                for (i, p) in light_planes.iter().enumerate() {
                    if sphere_visible(p, world_center, world_radius) {
                        flags |= 2 << (MAX_CASCADES + i);
                    }
                }
            }
            flags
        };
        let classify = |t: &Transform3D, bounds: (Vec3, f32), shadow_bounds: Option<(Vec3, f32)>| -> u32 {
            classify_at(t.matrix(), t.scale.abs().max_element(), bounds, shadow_bounds)
        };
        // A model's meshes (camera, then each cascade's) and bounds.
        let model_meshes = |m: &Model, shadow: Option<&ShadowMesh>| {
            let bounds = assets.live_bounds(m.mesh.id)?;
            let caster = match shadow {
                Some(ShadowMesh(caster)) => *caster,
                None => Some(m.mesh),
            };
            let caster = caster.filter(|c| assets.casts_shadows(c.id));
            let shadow_bounds = caster.and_then(|c| assets.live_bounds(c.id));
            let mut meshes = [caster.map_or(m.mesh.id, |c| c.id); 1 + MAX_CASCADES];
            meshes[0] = m.mesh.id;
            Some((meshes, bounds, shadow_bounds))
        };
        // A shadow-only caster's mesh and bounds.
        let shadow_only = |shadow: &ShadowMesh| {
            let mesh = shadow.0.filter(|c| assets.casts_shadows(c.id))?;
            Some((mesh.id, assets.live_bounds(mesh.id)?))
        };
        // The entries for a model the passes in `flags` see.
        // Light views draw the first cascade's (finest) shadow mesh.
        let view_mesh = |meshes: &[u32; 1 + MAX_CASCADES], view: usize| {
            if view < MAX_CASCADES {
                meshes[view + 1]
            } else {
                meshes[1]
            }
        };
        let mut add_instance = |meshes: [u32; 1 + MAX_CASCADES], instance: Instance3D, flags: u32, key: u64| {
            let (seen, casts) = (flags & 1 != 0, flags >> 1);
            // One entry per distinct mesh, with the passes that draw it.
            let mut done = 0u32; // views already in an entry
            let camera_mesh = meshes[0];
            let in_cascades = |mesh: u32| {
                (0..MAX_SHADOW_VIEWS).fold(0u32, |m, i| {
                    if casts & (1 << i) != 0 && view_mesh(&meshes, i) == mesh {
                        m | 1 << i
                    } else {
                        m
                    }
                })
            };
            if seen {
                motion.note(key, camera_mesh, &instance);
                let mask = in_cascades(camera_mesh);
                done |= mask;
                scratch.push((camera_mesh, instance, true, mask));
            }
            while casts & !done != 0 {
                let first = (casts & !done).trailing_zeros() as usize;
                let mesh = view_mesh(&meshes, first);
                let mask = in_cascades(mesh);
                done |= mask;
                scratch.push((mesh, instance, false, mask));
            }
        };
        // The matrix's fourth row is always (0, 0, 0, 1): its first three
        // slots carry what the shader needs per model; the first holds the
        // decal layers the model takes.
        let instance = |t: &Transform3D, color: [f32; 4], receives: f32| {
            let mut model = t.matrix().to_cols_array_2d();
            model[0][3] = receives;
            Instance3D { model, color }
        };
        let all_decals = super::projected::receiver_mask(None);
        // Skinned models posed by a skeleton are drawn from `skins` instead.
        // So are meshes with morph targets that aren't skinned.
        let posed = |m: &Model, s: Option<&Skeleton>| match assets.skin(m.mesh.id) {
            Some(_) => s.is_some(),
            None => assets.morph(m.mesh.id).is_some(),
        };
        // Models: culled on all cores (most of the work, and most models are
        // culled), then the few that are seen are added in order.
        let flags = &mut self.flags;
        world
            .query::<(
                &Transform3D,
                &Model,
                Option<&ShadowMesh>,
                Option<&Skeleton>,
                Option<&ReceivesDecals>,
            )>()
            .without::<SkinOf>()
            .par_collect_into(flags, |(t, m, shadow, skeleton, _)| {
                if posed(m, skeleton) {
                    return 0;
                }
                model_meshes(m, shadow).map_or(0, |(_, bounds, shadow_bounds)| classify(t, bounds, shadow_bounds))
            });
        let mut at = 0;
        world
            .query::<(
                crate::Entity,
                &Transform3D,
                &Model,
                Option<&ShadowMesh>,
                Option<&Skeleton>,
                Option<&ReceivesDecals>,
            )>()
            .without::<SkinOf>()
            .for_each(|(e, t, m, shadow, _, receives)| {
                let f = flags[at];
                at += 1;
                if f != 0
                    && let Some((meshes, _, _)) = model_meshes(m, shadow)
                {
                    add_instance(
                        meshes,
                        instance(t, linear(m.color), super::projected::receiver_mask(receives)),
                        f,
                        e.to_bits(),
                    );
                }
            });
        // Shadows with no model: only the cascades that see them.
        world
            .query::<(&Transform3D, &ShadowMesh)>()
            .without::<Model>()
            .par_collect_into(flags, |(t, shadow)| {
                shadow_only(shadow).map_or(0, |(_, bounds)| classify(t, bounds, Some(bounds)) & !1)
            });
        let mut at = 0;
        world
            .query::<(&Transform3D, &ShadowMesh)>()
            .without::<Model>()
            .for_each(|(t, shadow)| {
                let f = flags[at];
                at += 1;
                if f != 0
                    && let Some((mesh, _)) = shadow_only(shadow)
                {
                    add_instance(
                        [mesh; 1 + MAX_CASCADES],
                        instance(t, [1.0; 4], all_decals),
                        f,
                        super::motion::NO_KEY,
                    );
                }
            });
        // Skinned models: each its own mesh, skinned on the GPU this frame.
        let real_meshes = assets.bounds.len() as u32;
        for (k, d) in skins.draws.iter().enumerate() {
            if d.bounds.1 < 0.0 {
                continue;
            }
            let caster = assets.casts_shadows(d.mesh).then_some(d.bounds);
            let f = classify_at(d.matrix, d.scale, d.bounds, caster);
            if f != 0 {
                add_instance([real_meshes + k as u32; 1 + MAX_CASCADES], d.instance, f, d.entity);
            }
        }
        // LOD groups: the level (or two, in a crossfade) the camera picks
        // for each, which the shadow maps draw too. Picking and culling the
        // renderers is parallel; the few that are seen are added in order.
        let lod = self.lod;
        let lod_flags = |t: &Transform3D, r: &LodRenderer, drawn: bool, casts: bool| -> Option<(Mat4, u32)> {
            let bounds = assets.live_bounds(r.mesh.id)?;
            let matrix = t.matrix() * r.offset.matrix();
            let scale = t.scale.abs().max_element() * r.offset.scale.abs().max_element();
            let shadow = (casts && assets.casts_shadows(r.mesh.id)).then_some(bounds);
            let seen = classify_at(matrix, scale, bounds, shadow);
            Some((matrix, if drawn { seen } else { seen & !1 }))
        };
        let picks = &mut self.lod_picks;
        world
            .query::<(&Transform3D, &LodGroup)>()
            .par_collect_into(picks, |(t, g)| {
                let pick = g.pick_for(t, camera, &lod);
                let mut seen = 0;
                lod_draws(g, pick, |r, drawn, casts, _| {
                    seen |= lod_flags(t, r, drawn, casts).map_or(0, |(_, f)| f);
                });
                (pick, seen)
            });
        let mut at = 0;
        world
            .query::<(&Transform3D, &LodGroup, Option<&ReceivesDecals>)>()
            .for_each(|(t, g, decals)| {
                let (pick, seen) = picks[at];
                at += 1;
                if seen != 0 {
                    let receives = super::projected::receiver_mask(decals);
                    lod_draws(g, pick, |r, drawn, casts, fade| {
                        if let Some((matrix, f)) = lod_flags(t, r, drawn, casts)
                            && f != 0
                        {
                            let mut model = matrix.to_cols_array_2d();
                            model[0][3] = receives;
                            let instance = Instance3D {
                                model,
                                color: linear(r.color),
                            };
                            // No motion of their own for temporal antialiasing: they
                            // reproject with the camera (scenery stays put).
                            add_instance(
                                [r.mesh.id; 1 + MAX_CASCADES],
                                instance.with_fade(fade),
                                f,
                                super::motion::NO_KEY,
                            );
                        }
                    });
                }
            });
        // Terrain: the detail level for how far the camera is from the
        // tile's bounds (the finest level's, which hold every coarser one).
        // Shadow cascade i draws at least level i: its texels are too big
        // for finer detail to show.
        // A `ShadowMesh(None)` on the tile turns its shadow off.
        world
            .query::<(crate::Entity, &Transform3D, &TerrainTile, Option<&ShadowMesh>)>()
            .for_each(|(e, t, tile, shadow)| {
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
                    let casts = !matches!(shadow, Some(ShadowMesh(None)));
                    let f = classify(t, (center, radius), casts.then_some((center, radius)));
                    if f != 0 {
                        add_instance(meshes, instance(t, [1.0; 4], all_decals), f, e.to_bits());
                    }
                }
            });
        self.motion.finish();
        let meshes = assets.bounds.len() + skins.draws.len();
        let (counts, instances) = (&mut self.counts, &mut self.instances);
        self.occluded = occluded.into_inner();
        self.visible = sort_runs(&self.scratch, |e| e.2, meshes, counts, instances, &mut self.runs);
        for i in shadow_views(self.cascades, self.light_views) {
            let runs = &mut self.shadow_runs[i];
            sort_runs(&self.scratch, |e| e.3 & (1 << i) != 0, meshes, counts, instances, runs);
        }
    }
}

/// The shadow views drawn this frame: the sun's cascades, then the light views.
pub(crate) fn shadow_views(cascades: usize, light_views: usize) -> impl Iterator<Item = usize> + Clone {
    (0..cascades).chain(MAX_CASCADES..MAX_CASCADES + light_views)
}

/// Appends the entries of `scratch` that `keep` picks to `instances`, sorted
/// by mesh (a counting sort), and their runs to `runs`. Returns how many.
/// Crossfading entries (see [`Instance3D::with_fade`]) come after the whole
/// ones, in runs of their own with [`FADED`] set in the mesh id.
fn sort_runs(
    scratch: &[Entry],
    keep: impl Fn(&Entry) -> bool,
    meshes: usize,
    counts: &mut Vec<u32>,
    instances: &mut Vec<Instance3D>,
    runs: &mut Vec<(u32, u32, u32)>,
) -> usize {
    let bucket = |e: &Entry| e.0 as usize + if e.1.fade() != 0.0 { meshes } else { 0 };
    counts.clear();
    counts.resize(meshes * 2, 0);
    for e in scratch.iter().filter(|e| keep(e)) {
        counts[bucket(e)] += 1;
    }
    let base = instances.len() as u32;
    let mut start = base;
    for (b, count) in counts.iter_mut().enumerate() {
        if *count > 0 {
            let faded = if b >= meshes { FADED } else { 0 };
            runs.push(((b % meshes) as u32 | faded, start, *count));
        }
        let n = *count;
        *count = start; // now: where this bucket's next instance goes
        start += n;
    }
    let added = (start - base) as usize;
    instances.resize(instances.len() + added, Instance3D::default());
    for e in scratch.iter().filter(|e| keep(e)) {
        let at = &mut counts[bucket(e)];
        instances[*at as usize] = e.1;
        *at += 1;
    }
    added
}

/// Calls `f(renderer, drawn, casts, fade)` for each renderer that `pick`
/// shows: whether the camera draws it, whether it casts a shadow, and its
/// dither (see [`Instance3D::with_fade`]).
fn lod_draws(group: &LodGroup, pick: LodPick, mut f: impl FnMut(&LodRenderer, bool, bool, f32)) {
    for (level, fade) in group.drawn(pick).into_iter().flatten() {
        let own_shadow = matches!(level.shadow, LodShadow::Same);
        for r in &level.renderers {
            f(r, true, own_shadow, fade);
        }
        if let LodShadow::Casters(casters) = &level.shadow {
            for r in casters {
                f(r, false, true, fade);
            }
        }
    }
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
pub(crate) fn sun_view_proj(camera: &Camera3D, sun_direction: Vec3, near: f32, far: f32, size: u32) -> (Mat4, f32) {
    let (m, texel, _) = sun_box(camera, sun_direction, near, far, size, 1.0);
    (m, texel)
}

/// The bounding sphere of what the camera sees from `near` to `far` away:
/// its center and radius. It doesn't change size as the camera turns.
fn slice_sphere(camera: &Camera3D, near: f32, far: f32) -> (Vec3, f32) {
    let far = far.max(camera.near * 2.0);
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
    (center, radius)
}

/// [`sun_view_proj`] with the box `grow` times as wide as the view's
/// slice needs, and how far from the slice's center the box reaches
/// (anything within that distance of it is inside the box).
fn sun_box(camera: &Camera3D, sun_direction: Vec3, near: f32, far: f32, size: u32, grow: f32) -> (Mat4, f32, f32) {
    let light = sun_direction.try_normalize().unwrap_or(Vec3::NEG_Y);
    let (center, radius) = slice_sphere(camera, near, far);
    let radius = (radius * grow * 16.0).ceil() / 16.0;

    let up = if light.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let rotation = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, light, up);
    // Snap the center to whole texels in the light's view.
    let texel = 2.0 * radius / size.max(1) as f32;
    let snapped = (rotation.transform_point3(center) / texel).round() * texel;
    let center = rotation.inverse().transform_point3(snapped);

    // Reach far back towards the sun, so tall things outside the view still
    // cast into it.
    let back = radius * 4.0;
    let view = glam::camera::rh::view::look_to_mat4(center - light * back, light, up);
    let projection =
        glam::camera::rh::proj::directx::orthographic(-radius, radius, -radius, radius, 0.0, back + radius);
    // Snapping moved the box by up to a texel in each direction.
    (projection * view, texel, radius - texel * 2.0)
}

/// The first of `cascades` cascades that can be kept between frames: the
/// far half (cascades 2 and 3 of 4, 1 of 2).
pub(crate) fn far_cascades(cascades: usize) -> usize {
    cascades.div_ceil(2).max(1)
}

/// What a far cascade's shadow map holds (see `Frame3D::cascade_cache`).
#[derive(Clone, Copy, Debug)]
struct CachedCascade {
    view_proj: Mat4,
    texel: f32,
    /// The center of the view's slice it was drawn for, and how far from
    /// there its box reaches.
    center: Vec3,
    reach: f32,
    /// What it was drawn with: a change to any of them draws it again.
    sun: Vec3,
    size: u32,
    cascades: usize,
}

/// How much bigger a far cascade's box is than it needs to be, so the
/// camera can move a while before it must be drawn again.
const CACHE_MARGIN: f32 = 0.2;

/// The most frames a far cascade may be kept for.
pub(crate) const MAX_SHADOW_REFRESH: u32 = 8;

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

/// Whether a capsule (a sphere of `radius` swept from `a` to `b`) can be
/// inside the planes: false only when it is wholly outside one of them.
fn capsule_visible(planes: &[Vec4], a: Vec3, b: Vec3, radius: f32) -> bool {
    planes
        .iter()
        .all(|p| (p.truncate().dot(a) + p.w).max(p.truncate().dot(b) + p.w) >= -radius)
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
        frame.collect(&mut world, &assets, &SkinFrame::default(), &camera(), &lighting);
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
        frame.collect(&mut world, &assets, &SkinFrame::default(), &camera(), &lighting);
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
        frame.collect(&mut world, &assets, &SkinFrame::default(), &camera, &lighting);
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
        frame.collect(&mut world, &assets, &SkinFrame::default(), &camera, &lighting);
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
        frame.collect(&mut world, &assets, &SkinFrame::default(), &camera(), &lighting);
        assert_eq!(frame.runs.iter().map(|r| r.0).collect::<Vec<_>>(), vec![0]);
        let meshes: Vec<Vec<u32>> = frame
            .shadow_runs
            .iter()
            .take(MAX_CASCADES)
            .map(|c| c.iter().map(|r| r.0).collect())
            .collect();
        assert_eq!(meshes, vec![vec![0], vec![1], vec![2], vec![3]]);
    }

    #[test]
    fn the_shadow_box_holds_the_near_view_and_snaps_to_texels() {
        let camera = camera();
        let sun_dir = Vec3::new(-0.3, -1.0, -0.2);
        let (sun, _) = sun_view_proj(&camera, sun_dir, camera.near, 20.0, SHADOW_MAP_SIZE);
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
        let b = sun_view_proj(&moved, sun_dir, moved.near, 20.0, SHADOW_MAP_SIZE).0 * Vec4::new(0.0, 0.0, 0.0, 1.0);
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
        frame.collect(&mut world, &assets, &SkinFrame::default(), &camera, &lighting);
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

    #[test]
    fn lod_instances_carry_their_decal_layers_and_fade_in_separate_slots() {
        use crate::three::projected::ReceivesDecals;
        use crate::{LodGroup, LodLevel, LodRenderer};
        let mut world = World::new();
        let assets = Assets3D {
            bounds: vec![(Vec3::ZERO, 0.5), (Vec3::ZERO, 0.5)],
            ..Default::default()
        };
        let camera = camera(); // 10 from the origin, 60 degree lens
        // A group 4 tall covers about 35% of the screen: inside level 0's
        // fade band (0.3 up to 0.65).
        let group = LodGroup::new(
            4.0,
            vec![
                LodLevel::new(0.3, vec![LodRenderer::new(Mesh { id: 0 })]).with_fade(0.5),
                LodLevel::new(0.05, vec![LodRenderer::new(Mesh { id: 1 })]),
            ],
        );
        world.spawn((Transform3D::default(), group, ReceivesDecals(0b101)));
        let mut frame = Frame3D::default();
        let lighting = Lighting {
            shadows: false,
            ..Default::default()
        };
        frame.collect(&mut world, &assets, &SkinFrame::default(), &camera, &lighting);
        let seen = &frame.instances[..frame.visible];
        assert_eq!(seen.len(), 2, "both levels draw during the fade");
        for i in seen {
            assert_eq!(i.model[0][3], 5.0, "the group's decal layers");
            assert!(i.fade() != 0.0 && i.fade().abs() < 1.0, "{}", i.fade());
        }
        assert!(seen[0].fade() * seen[1].fade() < 0.0, "complementary patterns");
    }

    /// A world with one model near the camera and one far, the camera and
    /// lighting for the shadow cache tests.
    fn cache_scene(refresh: u32) -> (World, Assets3D, Camera3D, Lighting) {
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
            sun_direction: Vec3::new(0.2, -1.0, 0.1),
            shadow_distance: 400.0,
            shadow_refresh: refresh,
            ..Default::default()
        };
        let model = Model::new(Mesh { id: 0 });
        world.spawn((Transform3D::at(Vec3::new(0.0, 0.0, -3.0)), model));
        world.spawn((Transform3D::at(Vec3::new(0.0, 0.0, -350.0)), model));
        (world, assets, camera, lighting)
    }

    #[test]
    fn far_cascades_take_turns_and_keep_their_maps_in_between() {
        let (mut world, assets, camera, lighting) = cache_scene(2);
        let mut frame = Frame3D::default();
        let mut collect = |frame: &mut Frame3D, camera: &Camera3D, lighting: &Lighting| {
            frame.collect(&mut world, &assets, &SkinFrame::default(), camera, lighting);
            (frame.redrawn, frame.sun_view_proj, frame.shadow_runs.clone())
        };
        let (first, matrices, _) = collect(&mut frame, &camera, &lighting);
        assert_eq!(first, 0b1111, "the first frame draws every cascade");
        let (second, kept, runs) = collect(&mut frame, &camera, &lighting);
        let (third, _, _) = collect(&mut frame, &camera, &lighting);
        // Near cascades every frame; the far two (2 and 3) alternate.
        assert_eq!(second & 0b11, 0b11);
        assert_eq!(third & 0b11, 0b11);
        assert_eq!((second | third) & 0b1100, 0b1100);
        assert_eq!((second & third) & 0b1100, 0, "{second:b} {third:b}");
        let skipped = if second & 0b100 == 0 { 2 } else { 3 };
        assert_eq!(
            kept[skipped], matrices[skipped],
            "a kept cascade samples the map it drew"
        );
        assert!(runs[skipped].is_empty(), "and draws nothing");
        let drawn = 5 - skipped;
        assert!(!runs[drawn].is_empty(), "the other far cascade draws its casters");
    }

    #[test]
    fn a_kept_cascade_is_drawn_again_when_the_camera_leaves_it_or_the_sun_moves() {
        let (mut world, assets, mut camera, mut lighting) = cache_scene(8);
        let mut frame = Frame3D::default();
        let mut redrawn = |camera: &Camera3D, lighting: &Lighting| {
            frame.collect(&mut world, &assets, &SkinFrame::default(), camera, lighting);
            frame.redrawn
        };
        redrawn(&camera, &lighting);
        // Still: at most one far cascade's turn comes up each frame.
        let still = redrawn(&camera, &lighting);
        assert!((still & 0b1100).count_ones() <= 1, "{still:b}");
        // A small step stays inside the bigger boxes.
        camera.position.x += 1.0;
        let step = redrawn(&camera, &lighting);
        assert!((step & 0b1100).count_ones() <= 1, "{step:b}");
        // A long way: every cascade is drawn again.
        camera.position.x += 500.0;
        assert_eq!(redrawn(&camera, &lighting), 0b1111);
        redrawn(&camera, &lighting);
        // The sun moves: the same.
        lighting.sun_direction.x += 0.01;
        assert_eq!(redrawn(&camera, &lighting), 0b1111);
        // So does a new shadow map size.
        lighting.shadow_resolution = 1024;
        assert_eq!(redrawn(&camera, &lighting), 0b1111);
    }

    #[test]
    fn refresh_1_draws_every_cascade_every_frame_in_tight_boxes() {
        let (mut world, assets, camera, lighting) = cache_scene(1);
        let mut frame = Frame3D::default();
        for _ in 0..3 {
            frame.collect(&mut world, &assets, &SkinFrame::default(), &camera, &lighting);
            assert_eq!(frame.redrawn, 0b1111);
        }
        let splits = cascade_splits(camera.near, lighting.shadow_distance, 4);
        let (tight, _) = sun_view_proj(
            &camera,
            lighting.sun_direction,
            splits[2],
            splits[3],
            lighting.shadow_size(0),
        );
        assert_eq!(frame.sun_view_proj[3], tight);
        // With caching on, the far boxes are a little bigger: coarser texels.
        let (mut world, assets, camera, lighting) = cache_scene(2);
        let mut cached = Frame3D::default();
        cached.collect(&mut world, &assets, &SkinFrame::default(), &camera, &lighting);
        assert!(cached.cascade_texel[3] > frame.cascade_texel[3] * 1.15);
        assert_eq!(cached.cascade_texel[0], frame.cascade_texel[0]);
    }

    #[test]
    fn the_far_half_of_the_cascades_can_be_kept() {
        assert_eq!((1..=4).map(far_cascades).collect::<Vec<_>>(), [1, 1, 2, 2]);
    }

    /// How many shadow instances each cascade draws, after one collect of
    /// `models` (position, radius of a unit-mesh model scaled to it).
    fn cascade_counts(models: &[(Vec3, f32)], sun: Vec3) -> [u32; MAX_CASCADES] {
        let mut world = World::new();
        let assets = Assets3D {
            bounds: vec![(Vec3::ZERO, 1.0)],
            ..Default::default()
        };
        let mut camera = camera();
        camera.position = Vec3::new(0.0, 2.0, 0.0);
        camera.look_at(Vec3::new(0.0, 2.0, -1.0)); // looking down -z
        camera.near = 0.1;
        camera.far = 1000.0;
        let lighting = Lighting {
            sun_direction: sun,
            shadow_distance: 200.0,
            shadow_refresh: 1,
            ..Default::default()
        };
        for &(at, radius) in models {
            let mut t = Transform3D::at(at);
            t.scale = Vec3::splat(radius);
            world.spawn((t, Model::new(Mesh { id: 0 })));
        }
        let mut frame = Frame3D::default();
        frame.collect(&mut world, &assets, &SkinFrame::default(), &camera, &lighting);
        let mut counts = [0; MAX_CASCADES];
        for (i, runs) in frame.shadow_runs.iter().take(MAX_CASCADES).enumerate() {
            counts[i] = runs.iter().map(|r| r.2).sum();
        }
        counts
    }

    #[test]
    fn casters_whose_shadow_cant_reach_the_view_are_skipped() {
        // A tree 15 m behind the camera. With the sun behind the camera the
        // shadow falls forwards into the view; with the sun in front it falls
        // away, behind the camera, where nothing it shades is seen.
        let tree = [(Vec3::new(0.0, 3.0, 15.0), 3.0)];
        let towards_view = cascade_counts(&tree, Vec3::new(0.0, -0.5, -1.0));
        let away = cascade_counts(&tree, Vec3::new(0.0, -0.5, 1.0));
        assert!(towards_view.iter().sum::<u32>() > 0, "{towards_view:?}");
        assert_eq!(away, [0; MAX_CASCADES]);
    }

    #[test]
    fn casters_smaller_than_a_texel_are_left_out_of_coarse_cascades() {
        let sun = Vec3::new(0.2, -1.0, 0.1);
        // Pebbles 150 m away, in the last cascade (texels of about 10 cm).
        let small = cascade_counts(&[(Vec3::new(0.0, 0.0, -150.0), 0.02)], sun);
        let big = cascade_counts(&[(Vec3::new(0.0, 0.0, -150.0), 0.5)], sun);
        assert_eq!(small[3], 0, "{small:?}");
        assert_eq!(big[3], 1, "{big:?}");
        // Near the camera (in view, at eye height) the same pebble still
        // casts: fine texels.
        let near = cascade_counts(&[(Vec3::new(0.0, 2.0, -3.0), 0.02)], sun);
        assert_eq!(near[0], 1, "{near:?}");
    }
}
