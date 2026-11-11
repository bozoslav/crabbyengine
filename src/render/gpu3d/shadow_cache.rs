//! Keeps shadow views whose content hasn't changed since they were drawn.
//!
//! Each sun cascade has a layer of the shadow map, and each point and spot
//! light view a tile of the light atlas. A view is drawn again only when
//! what it would draw differs from what its layer or tile already holds:
//! its view-projection, its tile, its casters (meshes, transforms, tints
//! and crossfades, compared exactly) and every resource their pass reads
//! (meshes, materials, shaders, textures and the shadow maps themselves,
//! tracked by an epoch that any change to them bumps). Views drawn from
//! data that can change without the renderer seeing it (skinned or
//! morphing models, buffer-driven meshes, shaders with their own `vertex`
//! function, cut-outs with their own `surface` or reading a render target)
//! are drawn every frame. So a kept view is the same depth the pass would
//! draw again, and the frame is unchanged.
//!
//! What a view holds is recorded once the frame that drew it is recorded
//! (see [`ShadowCache::commit`]), so a frame that is prepared but never
//! drawn leaves the record matching the maps.

use super::*;
use std::sync::atomic::AtomicBool;

/// What one shadow view drew (or would draw): enough to tell whether
/// drawing it again would give the same depth.
#[derive(Default)]
pub(super) struct ShadowContent {
    /// `ShadowCache::epoch` when drawn; 0 never matches (it starts at 1).
    epoch: u64,
    /// The view's view-projection, its eye (static batch distance culling),
    /// and its tile in the light atlas (zero for the sun's cascades).
    caster: [[f32; 4]; 4],
    eye: [f32; 3],
    tile: [u32; 3],
    /// (mesh, material, count, per-part materials) per run (see `Ident`),
    /// then every run's per-part materials and instances in order.
    runs: Vec<(u32, u32, u32, u32)>,
    parts: Vec<(u32, u32)>,
    instances: Vec<Instance3D>,
}

impl ShadowContent {
    fn matches(&self, epoch: u64, key: &ViewKey, runs: &[Run], ident: Ident<'_>, instances: &[Instance3D]) -> bool {
        self.epoch == epoch
            && self.caster == key.caster
            && self.eye == key.eye
            && self.tile == key.tile
            && self.runs.len() == runs.len()
            && {
                let mut at = 0;
                self.runs
                    .iter()
                    .zip(runs)
                    .all(|(&(m, material, n, listed), &(mesh, _, count))| {
                        let (mesh, its_material, its_parts) = ident(mesh);
                        let mine = &self.parts[at..at + listed as usize];
                        at += listed as usize;
                        (m, material, n) == (mesh, its_material, count) && mine == its_parts
                    })
            }
            && {
                let mut at = 0;
                runs.iter().all(|&(_, start, count)| {
                    let theirs = &instances[start as usize..(start + count) as usize];
                    let mine = &self.instances[at..at + count as usize];
                    at += count as usize;
                    bytemuck::cast_slice::<_, u8>(mine) == bytemuck::cast_slice::<_, u8>(theirs)
                })
            }
    }

    fn set(&mut self, epoch: u64, key: &ViewKey, runs: &[Run], ident: Ident<'_>, instances: &[Instance3D]) {
        self.epoch = epoch;
        self.caster = key.caster;
        self.eye = key.eye;
        self.tile = key.tile;
        self.runs.clear();
        self.parts.clear();
        for &(mesh, _, count) in runs {
            let (mesh, material, parts) = ident(mesh);
            self.runs.push((mesh, material, count, parts.len() as u32));
            self.parts.extend_from_slice(parts);
        }
        self.instances.clear();
        for &(_, start, count) in runs {
            self.instances
                .extend_from_slice(&instances[start as usize..(start + count) as usize]);
        }
    }
}

/// What a run's mesh id draws, the same from frame to frame: (mesh with
/// its [`FADED`] bit, material override or `u32::MAX`). Material override
/// ids are numbered afresh each frame, so they're compared by what they draw.
type Ident<'a> = &'a dyn Fn(u32) -> (u32, u32, &'a [(u32, u32)]);

/// What places a view: see [`ShadowContent`].
pub(super) struct ViewKey {
    pub caster: [[f32; 4]; 4],
    pub eye: [f32; 3],
    pub tile: [u32; 3],
}

/// Which views this frame draws and keeps, and what each layer and tile holds.
pub(super) struct ShadowCache {
    /// Bumped by every change to what shadow passes read.
    epoch: u64,
    /// What each view's layer or tile holds now.
    drawn: [ShadowContent; MAX_SHADOW_VIEWS],
    /// What this frame draws into each view in `pending`.
    pending: [ShadowContent; MAX_SHADOW_VIEWS],
    pending_mask: u32,
    /// Views whose record goes once this frame is drawn: drawn from data
    /// that can't be compared, or light views whose tile may be overdrawn.
    forget_mask: u32,
    /// The light views kept this frame (bit `i`: light view `i`).
    pub light_kept: u32,
    /// Shadow views this frame keeps, cascades and light views together
    /// (bit `v`: shadow view `v`).
    pub kept: u32,
    /// Set when a frame is recorded, so its views' records are committed.
    recorded: AtomicBool,
    /// Per material shader: whether it has its own `vertex` and `surface`.
    pub shaders: Vec<(bool, bool)>,
}

impl Default for ShadowCache {
    fn default() -> Self {
        Self {
            epoch: 1,
            drawn: Default::default(),
            pending: Default::default(),
            pending_mask: 0,
            forget_mask: 0,
            light_kept: 0,
            kept: 0,
            recorded: AtomicBool::new(false),
            shaders: vec![(false, false)],
        }
    }
}

impl ShadowCache {
    /// Something shadow passes read changed: no view is kept on its record.
    pub fn invalidate(&mut self) {
        self.epoch += 1;
    }

    /// Whether material shader source `user` makes shadows that only the
    /// renderer's own data decides: (its own `vertex`, its own `surface`).
    pub fn shader_traits(user: &str) -> (bool, bool) {
        (defines(user, "vertex"), defines(user, "surface"))
    }

    /// Notes that this frame's commands were recorded (called by `draw`).
    pub fn recorded(&self) {
        self.recorded.store(true, Ordering::Relaxed);
    }

    /// Takes the last recorded frame's views as what the maps now hold.
    pub fn commit(&mut self) {
        if !self.recorded.swap(false, Ordering::Relaxed) {
            return;
        }
        for v in 0..MAX_SHADOW_VIEWS {
            if self.pending_mask & (1 << v) != 0 {
                std::mem::swap(&mut self.drawn[v], &mut self.pending[v]);
            }
            if self.forget_mask & (1 << v) != 0 {
                self.drawn[v].epoch = 0;
            }
        }
        self.pending_mask = 0;
        self.forget_mask = 0;
    }

    /// Decides for view `v` (which the frame would draw): true to keep it.
    /// `volatile`: drawn from data that can't be compared.
    pub fn plan(
        &mut self,
        v: usize,
        key: &ViewKey,
        runs: &[Run],
        ident: Ident<'_>,
        instances: &[Instance3D],
        volatile: bool,
    ) -> bool {
        let bit = 1 << v;
        if !volatile && self.drawn[v].matches(self.epoch, key, runs, ident, instances) {
            self.kept |= bit;
            return true;
        }
        if volatile {
            self.forget_mask |= bit;
        } else {
            self.pending[v].set(self.epoch, key, runs, ident, instances);
            self.pending_mask |= bit;
        }
        false
    }

    /// Starts planning a frame (records of the last one are committed).
    pub fn begin(&mut self) {
        self.commit();
        self.pending_mask = 0;
        self.forget_mask = 0;
        self.kept = 0;
        self.light_kept = 0;
    }

    /// A view the frame doesn't draw or keep: its record goes, as its layer
    /// or tile may be drawn over.
    pub fn forget(&mut self, v: usize) {
        self.forget_mask |= 1 << v;
    }
}

impl Renderer3D {
    /// Whether shadow view `v`'s runs, or the static batches it draws, use
    /// data the shadow cache can't compare (see [`ShadowCache`]).
    fn shadow_volatile(&self, runs: &[Run]) -> bool {
        let real = self.resources.meshes.len() as u32;
        let mesh_volatile = |run_mesh: u32| {
            let mesh = self.base(run_mesh);
            if self.material_override(run_mesh).is_none() && run_mesh & !FADED >= real {
                return true; // skinned or morphing: posed this frame
            }
            let m = &self.resources.meshes[mesh as usize];
            m.gpu_buffer.is_some()
                || m.parts
                    .iter()
                    .enumerate()
                    .any(|(part, &(_, own))| self.material_volatile(self.part_material(run_mesh, part, own)))
        };
        runs.iter().any(|&(mesh, _, _)| mesh_volatile(mesh))
            || self
                .static_batches
                .iter()
                .flatten()
                .filter(|b| b.casts())
                .any(|b| b.levels.iter().any(|l| l.shadows && mesh_volatile(l.mesh.slot())))
    }

    fn material_volatile(&self, material: u32) -> bool {
        let Some(m) = self.resources.materials.get(material as usize) else {
            return false;
        };
        if m.transparent {
            return false; // casts no shadow
        }
        let (vertex, surface) = self
            .shadow_cache
            .shaders
            .get(m.shader as usize)
            .copied()
            .unwrap_or((true, true));
        vertex
            || (m.cutout
                && (surface
                    || m.bindings
                        .maps
                        .iter()
                        .flatten()
                        .any(|t| self.render_targets.textures.iter().any(|&(id, _)| id == *t))))
    }

    /// A texture's pixels changed: cut-out shadows using it are drawn again.
    pub fn texture_changed(&mut self, id: u32) {
        if self
            .resources
            .materials
            .iter()
            .any(|m| m.cutout && m.bindings.maps.contains(&Some(id)))
        {
            self.shadow_cache.invalidate();
        }
    }

    /// Picks the shadow views this frame keeps from what their maps hold,
    /// and drops their runs (nothing of theirs is drawn or counted).
    pub(super) fn plan_shadows(&mut self, frame: &Frame3D, camera: Vec3) {
        self.shadow_cache.begin();
        // Static batches cull the cascades' casters by distance from the camera.
        let by_distance = self.static_batches.iter().flatten().any(|b| b.shadow_distance());
        for v in shadow_views(self.lists.cascades, self.lists.light_views) {
            let key = if v < MAX_CASCADES {
                if self.lists.redrawn & (1 << v) == 0 {
                    continue; // kept by the collector: holds what it drew
                }
                ViewKey {
                    caster: frame.sun_view_proj[v].to_cols_array_2d(),
                    eye: if by_distance { camera.to_array() } else { [0.0; 3] },
                    tile: [0; 3],
                }
            } else {
                let i = v - MAX_CASCADES;
                ViewKey {
                    caster: frame.lights.shadow_views[i].to_cols_array_2d(),
                    eye: frame.lights.shadow_positions.get(i).map_or([0.0; 3], |p| p.to_array()),
                    tile: self.lists.light_tiles.get(i).copied().unwrap_or_default(),
                }
            };
            let volatile = self.shadow_volatile(&self.lists.shadow_runs[v]);
            let (overrides, base) = (&self.lists.material_overrides, self.lists.override_base);
            let listed = &self.lists.override_parts;
            let ident = |mesh: u32| {
                let k = (mesh & !FADED).checked_sub(base);
                match k.and_then(|k| overrides.get(k as usize)) {
                    Some(o) => {
                        let (start, len) = (o.parts.0 as usize, o.parts.1 as usize);
                        (o.mesh | (mesh & FADED), o.material, &listed[start..start + len])
                    }
                    None => (mesh, u32::MAX, &[][..]),
                }
            };
            if self
                .shadow_cache
                .plan(v, &key, &self.lists.shadow_runs[v], &ident, &frame.instances, volatile)
            {
                self.lists.shadow_runs[v].clear();
                if v < MAX_CASCADES {
                    self.lists.redrawn &= !(1 << v);
                } else {
                    self.shadow_cache.light_kept |= 1 << (v - MAX_CASCADES);
                }
            }
        }
        // Light views not drawn this frame: their tiles may be drawn over.
        for v in MAX_CASCADES + self.lists.light_views..MAX_SHADOW_VIEWS {
            self.shadow_cache.forget(v);
        }
    }
}

/// Clears one tile of the light atlas to the far depth (1), where the
/// pass's viewport and scissor are: a triangle over the whole viewport at
/// depth 1, written whatever is there. The same as a load-op clear, for
/// one tile while the others are kept.
pub(super) fn tile_clear_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("3d shadow tile clear"),
        source: wgpu::ShaderSource::Wgsl(
            "@vertex fn main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
                let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u)) * 2.0 - 1.0;
                return vec4<f32>(p, 1.0, 1.0);
            }"
            .into(),
        ),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("3d shadow tile clear"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("3d shadow tile clear"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: None,
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
impl Renderer3D {
    /// Number of sun cascade passes actually redrawn after both cache layers.
    pub fn shadow_cascades_redrawn(&self) -> u32 {
        self.lists.redrawn.count_ones()
    }

    /// The shadow views the last prepared frame kept (bit `v`: view `v`).
    pub fn shadow_views_kept(&self) -> u32 {
        self.shadow_cache.kept
    }

    /// Makes the next frame draw every shadow view (a reference to compare with).
    pub fn forget_shadows(&mut self) {
        self.shadow_cache.invalidate();
    }
}
