//! What the frame's model collection keeps between frames, so a frame only
//! pays for what changed since the last one.
//!
//! Three parts, each owned by a [`Frame3D`](super::collect::Frame3D) (the
//! main view, every target camera and the global illumination capture keep
//! their own):
//!
//! - [`Lists`]: the instances each view draws (the camera, then every shadow
//!   view), grouped by mesh. Every drawn thing has a persistent id and, in
//!   each view that draws it, a slot in the frame's instance array that it
//!   keeps for as long as that view draws it with that mesh. A thing that
//!   enters a view takes the next free slot of its mesh's block; one that
//!   leaves hands its slot to the block's last instance. So a frame writes
//!   only the instances that changed, and the renderer, which uploads the
//!   pages of the array that differ from the frame before, uploads little
//!   more than those.
//! - [`Tracked`]: every plain [`Model`] in the world, by entity: a copy of
//!   what it was collected from (transform, mesh, tint, shadow mesh,
//!   material override, decal layers), its bounding spheres in the world,
//!   and which views saw it. A frame compares each model with its copy on
//!   all cores; only models that differ are worked out again, and models are
//!   culled again only when they or the views changed. A still scene under a
//!   still camera costs one compare per model.
//! - [`Transients`]: things whose drawing depends on the camera or is posed
//!   each frame (level-of-detail renderers, terrain tiles, skinned models,
//!   shadow-only casters). They are worked out every frame as before, and
//!   matched by a key to the id (and so the slots) they had.
//!
//! Change detection compares values, so it needs nothing from the ECS and
//! holds for any way a component is written. [`Tracked::sync`]'s compare
//! (`changed`) is the one place a change tick would replace: with ticks,
//! whole archetypes whose tick is old can skip the compare too.
//!
//! Costs to know about:
//! - The first frame, and any frame after the views empty, build everything
//!   (as every frame did before).
//! - A change to the meshes' bounds, shadow, skin or morph facts, or to the
//!   numbering of material overrides, works every model out again.
//! - Blocks grow by doubling and move to the end of the array when they
//!   outgrow their place. When the array is more than twice what its blocks
//!   need (space left behind, or kept by blocks that shrank) it is packed
//!   again, which uploads it whole once.
//! - Instances of one mesh are drawn in no particular order: it changes as
//!   models enter and leave the view.

use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::sync::atomic::{AtomicU32, Ordering};

use glam::{Quat, Vec3};

use super::collect::{Culler, FADED, Instance3D, MAX_CASCADES, MAX_SHADOW_VIEWS};
use super::motion::{IdHasher, Motion, NO_KEY};
use super::{Assets3D, Model, ModelMaterial, ReceivesDecals, ShadowMesh, Skeleton, SkinOf, Transform3D, linear};
use crate::World;

/// The views a frame draws models into: the camera (view 0), then the
/// shadow views (view `1 + v`). A thing's pass flags have bit `view` set
/// for each view that draws it.
pub(crate) const VIEWS: usize = 1 + MAX_SHADOW_VIEWS;
const VIEW_BITS: u32 = (1 << VIEWS) - 1;
/// Set in [`Applied::flags`] of a crossfading instance (see
/// [`Instance3D::with_fade`]): it is drawn in runs of its own.
const FADING: u32 = 1 << 31;
const NONE: u32 = u32::MAX;

/// The mesh a thing draws in the camera's view, then in each sun cascade
/// (light views draw the first cascade's).
pub(crate) type Meshes = [u32; 1 + MAX_CASCADES];

/// (mesh id, first instance, count).
type Run = (u32, u32, u32);

/// What the lists hold of one thing: where it is drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Applied {
    meshes: Meshes,
    /// Bit `view` for each view that draws it, and [`FADING`].
    flags: u32,
}

const NOWHERE: Applied = Applied {
    meshes: [0; 1 + MAX_CASCADES],
    flags: 0,
};

impl Applied {
    fn new(meshes: &Meshes, flags: u32, instance: &Instance3D) -> Self {
        let flags = flags & VIEW_BITS;
        if flags == 0 {
            return NOWHERE;
        }
        let fading = if instance.fade() != 0.0 { FADING } else { 0 };
        Self {
            meshes: *meshes,
            flags: flags | fading,
        }
    }

    /// The block `view` draws it in: one per mesh, and another for the
    /// mesh's crossfading instances.
    fn bucket(&self, view: usize) -> usize {
        let mesh = match view {
            0 => self.meshes[0],
            v if v <= MAX_CASCADES => self.meshes[v],
            _ => self.meshes[1],
        };
        mesh as usize * 2 + usize::from(self.flags & FADING != 0)
    }
}

/// One mesh's instances in one view: `len` instances from `start` in the
/// frame's instance array, with room for `cap`.
#[derive(Clone, Copy, Default)]
struct Block {
    start: u32,
    len: u32,
    cap: u32,
}

#[derive(Default)]
struct ViewList {
    /// By bucket (see [`Applied::bucket`]).
    blocks: Vec<Block>,
    /// By id: the thing's slot in the instance array, while this view
    /// draws it (stale otherwise).
    slots: Vec<u32>,
    /// How many instances the view draws.
    count: u32,
}

/// Each view's instances, kept between frames: see the module docs.
/// The instances themselves are the frame's (`Frame3D::instances`), passed
/// in as `arena`.
pub(crate) struct Lists {
    views: [ViewList; VIEWS],
    /// By id.
    applied: Vec<Applied>,
    /// Ids given back.
    free: Vec<u32>,
    /// By slot of the instance array: the id drawn there.
    owners: Vec<u32>,
    /// Whether no view drew anything when the frame began: it is built
    /// tightly packed, in view and mesh order.
    fresh: bool,
    /// The buffers the last packing read from, for the next.
    spare: (Vec<Instance3D>, Vec<u32>),
    /// How many times the array was packed again.
    pub repacks: u32,
}

impl Default for Lists {
    fn default() -> Self {
        Self {
            views: std::array::from_fn(|_| ViewList::default()),
            applied: Vec::new(),
            free: Vec::new(),
            owners: Vec::new(),
            fresh: true,
            spare: Default::default(),
            repacks: 0,
        }
    }
}

impl Lists {
    /// Starts a frame.
    pub fn begin(&mut self, arena: &mut Vec<Instance3D>) {
        self.fresh = self.views.iter().all(|v| v.count == 0);
        if self.fresh {
            arena.clear();
            self.owners.clear();
            for view in &mut self.views {
                view.blocks.clear();
            }
        }
    }

    /// A new id, drawn nowhere.
    pub fn alloc(&mut self) -> u32 {
        self.free.pop().unwrap_or_else(|| {
            self.applied.push(NOWHERE);
            self.applied.len() as u32 - 1
        })
    }

    /// Takes `id` out of every view and gives the id back.
    pub fn release(&mut self, arena: &mut Vec<Instance3D>, id: u32) {
        self.clear(arena, id);
        self.free.push(id);
    }

    /// Takes `id` out of every view.
    pub fn clear(&mut self, arena: &mut Vec<Instance3D>, id: u32) {
        self.set(arena, id, &NOWHERE.meshes, 0, &Instance3D::default());
    }

    /// The views that draw `id` (bit `view` each).
    pub fn flags(&self, id: u32) -> u32 {
        self.applied[id as usize].flags & VIEW_BITS
    }

    /// Whether `id` is drawn in exactly these views with these meshes.
    pub fn holds(&self, id: u32, meshes: &Meshes, flags: u32, instance: &Instance3D) -> bool {
        self.applied[id as usize] == Applied::new(meshes, flags, instance)
    }

    /// Where `id` is in the instance array for `view`, which draws it.
    pub fn slot(&self, view: usize, id: u32) -> usize {
        debug_assert!(self.applied[id as usize].flags & (1 << view) != 0);
        self.views[view].slots[id as usize] as usize
    }

    /// Draws `id` as `instance` in the views of `flags` (bit `view` each;
    /// none: nowhere), with `meshes`: it keeps its slot in every view that
    /// drew it with the same mesh, leaves the others and enters the new.
    pub fn set(&mut self, arena: &mut Vec<Instance3D>, id: u32, meshes: &Meshes, flags: u32, instance: &Instance3D) {
        let old = self.applied[id as usize];
        let new = Applied::new(meshes, flags, instance);
        let mut views = (old.flags | new.flags) & VIEW_BITS;
        while views != 0 {
            let view = views.trailing_zeros() as usize;
            views &= views - 1;
            let bit = 1 << view;
            match (old.flags & bit != 0, new.flags & bit != 0) {
                (true, true) => {
                    let (from, to) = (old.bucket(view), new.bucket(view));
                    if from == to {
                        arena[self.views[view].slots[id as usize] as usize] = *instance;
                    } else {
                        self.leave(arena, view, from, id);
                        self.enter(arena, view, to, id, instance);
                    }
                }
                (true, false) => self.leave(arena, view, old.bucket(view), id),
                _ => self.enter(arena, view, new.bucket(view), id, instance),
            }
        }
        self.applied[id as usize] = new;
    }

    fn enter(&mut self, arena: &mut Vec<Instance3D>, view: usize, bucket: usize, id: u32, instance: &Instance3D) {
        let ids = self.applied.len();
        let (list, owners) = (&mut self.views[view], &mut self.owners);
        if list.blocks.len() <= bucket {
            list.blocks.resize(bucket + 1, Block::default());
        }
        if list.slots.len() < ids {
            list.slots.resize(ids, NONE);
        }
        let mut block = list.blocks[bucket];
        if block.len == block.cap {
            let cap = (block.cap * 2).max(4);
            let (from, n) = (block.start as usize, block.len as usize);
            if block.cap > 0 && from + block.cap as usize == arena.len() {
                // The array's last block grows where it is.
                arena.resize(from + cap as usize, Instance3D::default());
                owners.resize(from + cap as usize, NONE);
            } else {
                // The others move to its end, leaving their place behind.
                let to = arena.len();
                assert!(to + (cap as usize) < NONE as usize, "too many 3D instances");
                arena.resize(to + cap as usize, Instance3D::default());
                owners.resize(to + cap as usize, NONE);
                arena.copy_within(from..from + n, to);
                owners.copy_within(from..from + n, to);
                for (k, &owner) in owners[to..to + n].iter().enumerate() {
                    list.slots[owner as usize] = (to + k) as u32;
                }
                block.start = to as u32;
            }
            block.cap = cap;
        }
        let at = block.start + block.len;
        arena[at as usize] = *instance;
        owners[at as usize] = id;
        list.slots[id as usize] = at;
        block.len += 1;
        list.blocks[bucket] = block;
        list.count += 1;
    }

    fn leave(&mut self, arena: &mut [Instance3D], view: usize, bucket: usize, id: u32) {
        let (list, owners) = (&mut self.views[view], &mut self.owners);
        let block = &mut list.blocks[bucket];
        let at = list.slots[id as usize] as usize;
        let last = (block.start + block.len - 1) as usize;
        debug_assert_eq!(owners[at], id);
        if at != last {
            // The block's last instance takes the freed slot.
            arena[at] = arena[last];
            let moved = owners[last];
            owners[at] = moved;
            list.slots[moved as usize] = at as u32;
        }
        block.len -= 1;
        list.count -= 1;
    }

    /// Ends a frame: packs the array again when it was built from nothing,
    /// or is more than twice what its blocks need with room to grow (space
    /// left behind by blocks that moved, or kept by blocks that shrank).
    pub fn finish(&mut self, arena: &mut Vec<Instance3D>) {
        if self.fresh {
            if !arena.is_empty() {
                self.repack(arena, true);
            }
            return;
        }
        let roomy: usize = (self.views.iter().flat_map(|v| &v.blocks))
            .filter(|b| b.len > 0)
            .map(|b| roomy(b.len as usize))
            .sum();
        if arena.len() > 2 * roomy + 1024 {
            self.repack(arena, false);
        }
    }

    /// Lays the blocks out again without holes: the camera's by mesh, then
    /// each shadow view's, every block's instances in their order. `tight`
    /// leaves no room to grow (a frame built from nothing is laid out as if
    /// it was sorted); otherwise blocks keep room for as many again, if
    /// they had it.
    fn repack(&mut self, arena: &mut Vec<Instance3D>, tight: bool) {
        let (mut packed, mut owners) = std::mem::take(&mut self.spare);
        packed.clear();
        owners.clear();
        for list in &mut self.views {
            for faded in 0..2 {
                for bucket in (faded..list.blocks.len()).step_by(2) {
                    let block = &mut list.blocks[bucket];
                    let (from, n) = (block.start as usize, block.len as usize);
                    if n == 0 {
                        *block = Block::default();
                        continue;
                    }
                    let start = packed.len();
                    packed.extend_from_slice(&arena[from..from + n]);
                    owners.extend_from_slice(&self.owners[from..from + n]);
                    for (k, &owner) in owners[start..].iter().enumerate() {
                        list.slots[owner as usize] = (start + k) as u32;
                    }
                    let cap = if tight { n } else { roomy(n).min(block.cap as usize) };
                    packed.resize(start + cap, Instance3D::default());
                    owners.resize(start + cap, NONE);
                    *block = Block {
                        start: start as u32,
                        len: n as u32,
                        cap: cap as u32,
                    };
                }
            }
        }
        std::mem::swap(arena, &mut packed);
        std::mem::swap(&mut self.owners, &mut owners);
        self.spare = (packed, owners);
        self.repacks += 1;
    }

    /// How many instances `view` draws.
    pub fn count(&self, view: usize) -> usize {
        self.views[view].count as usize
    }

    /// Replaces `runs` with `view`'s: its meshes' whole instances in mesh
    /// order, then its crossfading ones (with [`FADED`] set in the mesh id).
    pub fn runs(&self, view: usize, runs: &mut Vec<Run>) {
        runs.clear();
        let blocks = &self.views[view].blocks;
        for (first, faded) in [(0, 0), (1, FADED)] {
            for bucket in (first..blocks.len()).step_by(2) {
                let block = blocks[bucket];
                if block.len > 0 {
                    runs.push(((bucket / 2) as u32 | faded, block.start, block.len));
                }
            }
        }
    }

    /// Bytes kept between frames, without the instance array itself.
    pub fn bytes(&self) -> usize {
        let views: usize = self
            .views
            .iter()
            .map(|v| v.blocks.capacity() * size_of::<Block>() + v.slots.capacity() * size_of::<u32>())
            .sum();
        views
            + self.applied.capacity() * size_of::<Applied>()
            + (self.free.capacity() + self.owners.capacity() + self.spare.1.capacity()) * size_of::<u32>()
            + self.spare.0.capacity() * size_of::<Instance3D>()
    }
}

/// The room a block of `n` instances is packed with: as many again.
fn roomy(n: usize) -> usize {
    (n * 2).max(4)
}

/// What a [`Model`] is collected from: compared byte for byte with the
/// copy kept from the frame before.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Source {
    rotation: [f32; 4],
    position: [f32; 3],
    scale: [f32; 3],
    /// The model's tint, as set (not yet linear).
    color: [f32; 4],
    /// The decal layers it takes (see `projected::receiver_mask`).
    receives: f32,
    mesh: u32,
    /// Its [`ShadowMesh`]: [`OWN_SHADOW`] without one, [`NO_SHADOW`] for
    /// `ShadowMesh(None)`.
    shadow: u32,
    /// Its [`ModelMaterial`], or [`NONE`].
    material: u32,
    /// Whether it has a [`Skeleton`].
    skeleton: u32,
}

const OWN_SHADOW: u32 = u32::MAX;
const NO_SHADOW: u32 = u32::MAX - 1;

type ModelItem<'a> = (
    crate::Entity,
    &'a Transform3D,
    &'a Model,
    Option<&'a ShadowMesh>,
    Option<&'a Skeleton>,
    Option<&'a ReceivesDecals>,
    Option<&'a ModelMaterial>,
);

impl Source {
    fn of(
        t: &Transform3D,
        m: &Model,
        shadow: Option<&ShadowMesh>,
        skeleton: Option<&Skeleton>,
        receives: Option<&ReceivesDecals>,
        material: Option<&ModelMaterial>,
    ) -> Self {
        Self {
            rotation: t.rotation.to_array(),
            position: t.position.to_array(),
            scale: t.scale.to_array(),
            color: [m.color.r, m.color.g, m.color.b, m.color.a],
            receives: super::projected::receiver_mask(receives),
            mesh: m.mesh.id,
            shadow: match shadow {
                None => OWN_SHADOW,
                Some(ShadowMesh(None)) => NO_SHADOW,
                Some(ShadowMesh(Some(mesh))) => mesh.id,
            },
            material: material.map_or(NONE, |o| o.0.id),
            skeleton: u32::from(skeleton.is_some()),
        }
    }

    fn transform(&self) -> Transform3D {
        Transform3D {
            rotation: Quat::from_array(self.rotation),
            position: Vec3::from_array(self.position),
            scale: Vec3::from_array(self.scale),
        }
    }

    /// The model as the GPU reads it. The matrix's fourth row is always
    /// (0, 0, 0, 1): its first slot holds the decal layers the model takes.
    fn instance(&self) -> Instance3D {
        let mut model = self.transform().matrix().to_cols_array_2d();
        model[0][3] = self.receives;
        let [r, g, b, a] = self.color;
        Instance3D {
            model,
            color: linear(crate::Color { r, g, b, a }),
        }
    }
}

/// What culling needs of a model, worked out from its [`Source`] and the
/// meshes: its bounding spheres in the world (`xyz` center, `w` radius).
#[derive(Clone, Copy)]
struct Derived {
    /// Its own mesh's; negative radius: nothing to draw (a freed mesh, or a
    /// model that a skeleton or morph targets pose, drawn with the skins).
    sphere: [f32; 4],
    /// Its shadow caster's; negative radius: it casts none.
    caster_sphere: [f32; 4],
    /// The mesh id the camera draws (a material override's id, with one).
    mesh: u32,
    /// The mesh id the shadow views draw.
    caster: u32,
}

const NOTHING: Derived = Derived {
    sphere: [0.0, 0.0, 0.0, -1.0],
    caster_sphere: [0.0, 0.0, 0.0, -1.0],
    mesh: 0,
    caster: 0,
};

/// What of a mesh the models' [`Derived`] is worked out from: its bounds,
/// and whether it casts no shadow, is skinned, has morph targets.
type Fact = (Vec3, f32, bool, bool, bool);

/// The material overrides' numbering: each (mesh, material) pair's index,
/// and the mesh id of the first.
pub(crate) struct Overrides<'a> {
    pub index: &'a HashMap<(u32, u32), u32>,
    pub base: u32,
    /// The pairs in order, to tell when the numbering changes.
    pub pairs: &'a [(u32, u32)],
}

fn derive(source: &Source, assets: &Assets3D, overrides: &Overrides<'_>) -> Derived {
    // Skinned models posed by a skeleton are drawn from the skins instead.
    // So are meshes with morph targets that aren't skinned.
    let posed = match assets.skin(source.mesh) {
        Some(_) => source.skeleton != 0,
        None => assets.morph(source.mesh).is_some(),
    };
    let Some(bounds) = assets.live_bounds(source.mesh).filter(|_| !posed) else {
        return NOTHING;
    };
    let caster = match source.shadow {
        OWN_SHADOW => Some(source.mesh),
        NO_SHADOW => None,
        mesh => Some(mesh),
    }
    .filter(|&c| assets.casts_shadows(c));
    let caster_bounds = caster.and_then(|c| assets.live_bounds(c));
    let transform = source.transform();
    let (matrix, scale) = (transform.matrix(), transform.scale.abs().max_element());
    let world = |(center, radius): (Vec3, f32)| matrix.transform_point3(center).extend(radius * scale).to_array();
    // The model's own mesh, wherever it is drawn (not a separate
    // `ShadowMesh`), takes the override.
    let own = match overrides.index.get(&(source.mesh, source.material)) {
        Some(&k) if source.material != NONE => overrides.base + k,
        _ => source.mesh,
    };
    Derived {
        sphere: world(bounds),
        caster_sphere: caster_bounds.map_or(NOTHING.caster_sphere, world),
        mesh: own,
        caster: match caster.filter(|_| caster_bounds.is_some()) {
            Some(c) if c != source.mesh => c,
            _ => own,
        },
    }
}

/// A buffer's start, shared with the tasks of a parallel query: each task
/// writes only the elements it has claimed.
struct Shared<T>(*mut T);
impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Shared<T> {}
impl<T> Shared<T> {
    /// A method, so closures capture the whole `Shared` (which is `Sync`),
    /// not its raw pointer field.
    fn at(self, i: usize) -> *mut T {
        self.0.wrapping_add(i)
    }
}
// SAFETY: the pointer is only dereferenced by the task that claimed the
// element (see `Tracked::sync`), and `T` may be sent between threads.
unsafe impl<T: Send> Send for Shared<T> {}
unsafe impl<T: Send> Sync for Shared<T> {}

/// In `Tracked::results`: nothing to do for the model.
const UNCHANGED: u32 = u32::MAX;
/// In `Tracked::results`: the model isn't tracked yet.
const NEW: u32 = u32::MAX - 1;
const DEAD: u64 = u64::MAX;

/// Every plain [`Model`] in the world: see the module docs.
#[derive(Default)]
pub(crate) struct Tracked {
    /// By entity index: the entity's object, or [`NONE`]. An entry can be
    /// stale (its entity gone, the index reused): `entities` tells.
    slot_of: Vec<u32>,
    /// By object, like everything below: the entity's bits, or [`DEAD`].
    entities: Vec<u64>,
    sources: Vec<Source>,
    derived: Vec<Derived>,
    /// The views that see it, as last worked out.
    pending: Vec<u32>,
    /// The views that draw it in the lists.
    applied: Vec<u32>,
    /// Its id in the lists.
    ids: Vec<u32>,
    /// The frame that last found the object's entity in the world. Claimed
    /// atomically: the task that sets it to this frame owns the object.
    stamps: Vec<AtomicU32>,
    free: Vec<u32>,
    live: usize,
    frame: u32,
    /// Per model, in query order: [`UNCHANGED`], [`NEW`], or the object
    /// whose lists need bringing up to date.
    results: Vec<u32>,
    /// What the models were last worked out and culled with.
    facts: Vec<Fact>,
    overrides: (u32, Vec<(u32, u32)>),
    view: Vec<f32>,
}

impl Tracked {
    /// Brings every model up to date with the world, the meshes and the
    /// views, and the lists with them.
    #[allow(clippy::too_many_arguments)]
    pub fn sync(
        &mut self,
        world: &mut World,
        assets: &Assets3D,
        culler: &Culler<'_>,
        overrides: &Overrides<'_>,
        lists: &mut Lists,
        arena: &mut Vec<Instance3D>,
        motion: &mut Motion,
    ) {
        // Everything is worked out again when what it is worked out from
        // changed: the meshes' facts or the overrides' numbering.
        let facts = (0..assets.bounds.len() as u32).map(|id| {
            let (center, radius) = assets.bounds[id as usize];
            let shadowless = !assets.casts_shadows(id);
            (
                center,
                radius,
                shadowless,
                assets.skin(id).is_some(),
                assets.morph(id).is_some(),
            )
        });
        let mut force = false;
        if !self.facts.iter().copied().eq(facts.clone()) {
            self.facts.clear();
            self.facts.extend(facts);
            force = true;
        }
        if self.overrides.0 != overrides.base || self.overrides.1 != overrides.pairs {
            self.overrides.0 = overrides.base;
            self.overrides.1.clear();
            self.overrides.1.extend_from_slice(overrides.pairs);
            force = true;
        }
        // Everything is culled again when the views changed.
        let mut view = std::mem::take(&mut self.view);
        let cull = culler.changed_since(&mut view);
        self.view = view;

        self.frame = self.frame.wrapping_add(1).max(1);
        let frame = self.frame;
        let len = self.sources.len();
        let (slot_of, entities, applied, stamps) = (&self.slot_of, &self.entities, &self.applied, &self.stamps);
        let sources = Shared(self.sources.as_mut_ptr());
        let derived = Shared(self.derived.as_mut_ptr());
        let pending = Shared(self.pending.as_mut_ptr());
        // On all cores: each model is compared with its copy; one that
        // differs is worked out again, and culled like every model is when
        // the views changed.
        world.query::<ModelItem<'_>>().without::<SkinOf>().par_collect_into(
            &mut self.results,
            |(entity, t, m, shadow, skeleton, receives, material)| {
                let k = match slot_of.get(entity.index() as usize) {
                    Some(&k) if (k as usize) < len && entities[k as usize] == entity.to_bits() => k as usize,
                    _ => return NEW,
                };
                if stamps[k].swap(frame, Ordering::Relaxed) == frame {
                    return UNCHANGED; // already claimed (an entity is found once)
                }
                // SAFETY: `k < len`, the length of the three buffers, and
                // the swap above lets one task per frame get here with
                // `k`: nothing else reads or writes these elements
                // until the query is over.
                let (kept, derived, pending) =
                    unsafe { (&mut *sources.at(k), &mut *derived.at(k), &mut *pending.at(k)) };
                let source = Source::of(t, m, shadow, skeleton, receives, material);
                // With ECS change ticks, this compare is what a tick
                // test replaces.
                let changed = force || bytemuck::bytes_of(kept) != bytemuck::bytes_of(&source);
                if changed {
                    *kept = source;
                    *derived = derive(&source, assets, overrides);
                }
                if changed || cull {
                    let flags = culler.classify_spheres(derived.sphere, derived.caster_sphere);
                    *pending = flags;
                    if flags != applied[k] || (changed && flags != 0) {
                        return k as u32;
                    }
                }
                UNCHANGED
            },
        );
        let mut new = 0;
        for at in 0..self.results.len() {
            match self.results[at] {
                UNCHANGED => {}
                NEW => new += 1,
                k => self.apply(k as usize, lists, arena, motion),
            }
        }
        let found = self.results.len() - new;
        if new > 0 {
            // New models: found again in order, each given an object.
            let mut at = 0;
            let results = std::mem::take(&mut self.results);
            world.query::<ModelItem<'_>>().without::<SkinOf>().for_each(
                |(entity, t, m, shadow, skeleton, receives, material)| {
                    at += 1;
                    if results[at - 1] != NEW {
                        return;
                    }
                    let source = Source::of(t, m, shadow, skeleton, receives, material);
                    let derived = derive(&source, assets, overrides);
                    let flags = culler.classify_spheres(derived.sphere, derived.caster_sphere);
                    let k = self.add(entity, source, derived, flags, lists);
                    self.apply(k, lists, arena, motion);
                },
            );
            self.results = results;
        }
        if found != self.live {
            // Some tracked models are gone from the world (despawned, or
            // without the components now): out of the lists.
            for k in 0..self.entities.len() {
                if self.entities[k] != DEAD && *self.stamps[k].get_mut() != frame {
                    lists.clear(arena, self.ids[k]);
                    self.applied[k] = 0;
                    let index = self.entities[k] as u32 as usize;
                    if self.slot_of[index] == k as u32 {
                        self.slot_of[index] = NONE;
                    }
                    self.entities[k] = DEAD;
                    self.free.push(k as u32);
                }
            }
        }
        self.live = found + new;
    }

    /// A new object for `entity`.
    fn add(&mut self, entity: crate::Entity, source: Source, derived: Derived, flags: u32, lists: &mut Lists) -> usize {
        let k = match self.free.pop() {
            Some(k) => k as usize,
            None => {
                self.entities.push(DEAD);
                self.sources.push(source);
                self.derived.push(derived);
                self.pending.push(0);
                self.applied.push(0);
                self.ids.push(lists.alloc());
                self.stamps.push(AtomicU32::new(0));
                self.entities.len() - 1
            }
        };
        self.entities[k] = entity.to_bits();
        self.sources[k] = source;
        self.derived[k] = derived;
        self.pending[k] = flags;
        *self.stamps[k].get_mut() = self.frame;
        let index = entity.index() as usize;
        if self.slot_of.len() <= index {
            self.slot_of.resize(index + 1, NONE);
        }
        self.slot_of[index] = k as u32;
        k
    }

    /// Brings the lists up to date with object `k`.
    fn apply(&mut self, k: usize, lists: &mut Lists, arena: &mut Vec<Instance3D>, motion: &mut Motion) {
        let (flags, was, id) = (self.pending[k], self.applied[k], self.ids[k]);
        if flags == 0 {
            if was != 0 {
                lists.clear(arena, id);
            }
        } else {
            let derived = self.derived[k];
            let instance = self.sources[k].instance();
            if was & flags & 1 != 0 && motion.tracking() {
                let before = arena[lists.slot(0, id)].model;
                motion.moved(derived.mesh, &instance, before);
            }
            let mut meshes = [derived.caster; 1 + MAX_CASCADES];
            meshes[0] = derived.mesh;
            lists.set(arena, id, &meshes, flags, &instance);
        }
        self.applied[k] = flags;
    }

    /// Bytes kept between frames.
    pub fn bytes(&self) -> usize {
        let per_object = size_of::<u64>() + size_of::<Source>() + size_of::<Derived>() + 4 * size_of::<u32>();
        self.entities.capacity() * per_object
            + (self.slot_of.capacity() + self.free.capacity() + self.results.capacity()) * size_of::<u32>()
            + self.facts.capacity() * size_of::<Fact>()
            + self.overrides.1.capacity() * size_of::<(u32, u32)>()
            + self.view.capacity() * size_of::<f32>()
    }
}

/// A thing drawn this frame that isn't a tracked [`Model`]: the entity it
/// belongs to (its bits), and which of the entity's things it is.
pub(crate) type Key = (u64, u32);

/// In a [`Key`]: a shadow-only caster, a skinned model, a terrain tile, or
/// (added to the renderer's number within its group) a level-of-detail
/// renderer.
pub(crate) const SHADOW_ONLY: u32 = 1 << 28;
pub(crate) const SKINNED: u32 = 2 << 28;
pub(crate) const TERRAIN: u32 = 3 << 28;
pub(crate) const LOD: u32 = 4 << 28;
/// Added to a key that was already used this frame, to tell them apart.
const AGAIN: u32 = 1 << 20;

struct Transient {
    key: Key,
    /// Its id in the lists.
    id: u32,
    /// The frame that last drew it.
    stamp: u32,
    /// Where it was in the frame's order of things.
    seq: u32,
    /// As last drawn.
    instance: Instance3D,
}

/// The things worked out every frame, matched to the ids they had: see the
/// module docs. A frame calls [`touch`](Self::touch) for each thing some
/// view draws, then [`finish`](Self::finish), which drops the rest.
#[derive(Default)]
pub(crate) struct Transients {
    items: Vec<Transient>,
    free: Vec<u32>,
    by_key: HashMap<Key, u32, BuildHasherDefault<IdHasher>>,
    /// The items the last frame drew, in its order, and this frame's so far.
    order: Vec<u32>,
    next: Vec<u32>,
    /// Where in `order` the next thing is expected: frames mostly draw the
    /// same things in the same order, which costs no lookup.
    cursor: usize,
    frame: u32,
}

impl Transients {
    /// Starts a frame.
    pub fn begin(&mut self) {
        self.frame = self.frame.wrapping_add(1).max(1);
        self.cursor = 0;
    }

    /// Draws the thing `key` as `instance` in the views of `flags` (not
    /// none) with `meshes`. `motion_key` is [`NO_KEY`] for things without
    /// motion vectors of their own.
    #[allow(clippy::too_many_arguments)]
    pub fn touch(
        &mut self,
        lists: &mut Lists,
        arena: &mut Vec<Instance3D>,
        motion: &mut Motion,
        mut key: Key,
        motion_key: u64,
        meshes: &Meshes,
        flags: u32,
        instance: &Instance3D,
    ) {
        let frame = self.frame;
        let expected = self
            .order
            .get(self.cursor)
            .copied()
            .filter(|&t| self.items[t as usize].key == key && self.items[t as usize].stamp != frame);
        let t = if let Some(t) = expected {
            self.cursor += 1;
            t
        } else {
            let found = loop {
                match self.by_key.get(&key) {
                    // The same key twice in a frame: another thing.
                    Some(&t) if self.items[t as usize].stamp == frame => key.1 = key.1.wrapping_add(AGAIN),
                    found => break found.copied(),
                }
            };
            match found {
                Some(t) => {
                    // Things before it left or new ones came: carry on
                    // from where it was.
                    self.cursor = self.items[t as usize].seq as usize + 1;
                    t
                }
                None => {
                    let item = Transient {
                        key,
                        id: lists.alloc(),
                        stamp: 0,
                        seq: 0,
                        instance: Instance3D::default(),
                    };
                    let t = match self.free.pop() {
                        Some(t) => {
                            self.items[t as usize] = item;
                            t
                        }
                        None => {
                            self.items.push(item);
                            self.items.len() as u32 - 1
                        }
                    };
                    self.by_key.insert(key, t);
                    t
                }
            }
        };
        let item = &mut self.items[t as usize];
        item.stamp = frame;
        item.seq = self.next.len() as u32;
        self.next.push(t);
        if motion_key != NO_KEY && lists.flags(item.id) & flags & 1 != 0 {
            motion.moved(meshes[0], instance, item.instance.model);
        }
        if !lists.holds(item.id, meshes, flags, instance)
            || bytemuck::bytes_of(&item.instance) != bytemuck::bytes_of(instance)
        {
            lists.set(arena, item.id, meshes, flags, instance);
            item.instance = *instance;
        }
    }

    /// Ends a frame: the things the last frame drew and this one didn't
    /// leave the lists.
    pub fn finish(&mut self, lists: &mut Lists, arena: &mut Vec<Instance3D>) {
        for &t in &self.order {
            let item = &self.items[t as usize];
            if item.stamp != self.frame {
                lists.release(arena, item.id);
                self.by_key.remove(&item.key);
                self.free.push(t);
            }
        }
        std::mem::swap(&mut self.order, &mut self.next);
        self.next.clear();
    }

    /// Bytes kept between frames.
    pub fn bytes(&self) -> usize {
        self.items.capacity() * size_of::<Transient>()
            + (self.free.capacity() + self.order.capacity() + self.next.capacity()) * size_of::<u32>()
            + self.by_key.capacity() * (size_of::<Key>() + size_of::<u32>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f32) -> Instance3D {
        let mut instance = Instance3D::default();
        instance.model[3][0] = x;
        instance
    }

    /// The `x` of every instance `view` draws, by run.
    fn drawn(lists: &Lists, arena: &[Instance3D], view: usize) -> Vec<(u32, Vec<f32>)> {
        let mut runs = Vec::new();
        lists.runs(view, &mut runs);
        runs.iter()
            .map(|&(mesh, start, count)| {
                let xs = arena[start as usize..(start + count) as usize]
                    .iter()
                    .map(|i| i.model[3][0])
                    .collect();
                (mesh, xs)
            })
            .collect()
    }

    #[test]
    fn a_frame_built_from_nothing_is_packed_by_view_then_mesh() {
        let (mut lists, mut arena) = (Lists::default(), Vec::new());
        lists.begin(&mut arena);
        for (x, mesh, flags) in [(0.0, 2, 0b11), (1.0, 0, 0b01), (2.0, 2, 0b10), (3.0, 0, 0b11)] {
            let id = lists.alloc();
            lists.set(&mut arena, id, &[mesh; 5], flags, &at(x));
        }
        lists.finish(&mut arena);
        let xs: Vec<f32> = arena.iter().map(|i| i.model[3][0]).collect();
        assert_eq!(
            xs,
            [1.0, 3.0, 0.0, 3.0, 0.0, 2.0],
            "the camera's by mesh, then the shadow view's"
        );
        let mut runs = Vec::new();
        lists.runs(0, &mut runs);
        assert_eq!(runs, [(0, 0, 2), (2, 2, 1)]);
        lists.runs(1, &mut runs);
        assert_eq!(runs, [(0, 3, 1), (2, 4, 2)]);
        assert_eq!((lists.count(0), lists.count(1)), (3, 3));
    }

    #[test]
    fn a_thing_keeps_its_slot_while_others_enter_and_leave() {
        let (mut lists, mut arena) = (Lists::default(), Vec::new());
        lists.begin(&mut arena);
        let ids: Vec<u32> = (0..6).map(|_| lists.alloc()).collect();
        for (i, &id) in ids.iter().enumerate() {
            lists.set(&mut arena, id, &[7; 5], 1, &at(i as f32));
        }
        lists.finish(&mut arena);
        let slot = lists.slot(0, ids[1]);
        lists.begin(&mut arena);
        // One leaves: the last takes its place, the rest stay put.
        lists.clear(&mut arena, ids[3]);
        assert_eq!(drawn(&lists, &arena, 0), [(7, vec![0.0, 1.0, 2.0, 5.0, 4.0])]);
        // New ones enter (the block outgrows its room), and the thing
        // moves: its slot's instance is rewritten.
        for x in [10.0, 11.0, 12.0] {
            let id = lists.alloc();
            lists.set(&mut arena, id, &[7; 5], 1, &at(x));
        }
        lists.set(&mut arena, ids[1], &[7; 5], 1, &at(-1.0));
        lists.finish(&mut arena);
        assert_eq!(
            drawn(&lists, &arena, 0),
            [(7, vec![0.0, -1.0, 2.0, 5.0, 4.0, 10.0, 11.0, 12.0])]
        );
        assert_eq!(arena[lists.slot(0, ids[1])].model[3][0], -1.0);
        assert_eq!(lists.slot(0, ids[1]), slot);
        assert_eq!(lists.count(0), 8);
    }

    #[test]
    fn a_new_mesh_or_fade_moves_a_thing_between_runs() {
        let (mut lists, mut arena) = (Lists::default(), Vec::new());
        lists.begin(&mut arena);
        let (a, b) = (lists.alloc(), lists.alloc());
        lists.set(&mut arena, a, &[1; 5], 0b11, &at(1.0));
        lists.set(&mut arena, b, &[1; 5], 0b11, &at(2.0));
        lists.finish(&mut arena);
        lists.begin(&mut arena);
        // Another mesh for the camera only: the shadow view keeps its run.
        lists.set(&mut arena, a, &[3, 1, 1, 1, 1], 0b11, &at(1.0));
        assert_eq!(drawn(&lists, &arena, 0), [(1, vec![2.0]), (3, vec![1.0])]);
        assert_eq!(drawn(&lists, &arena, 1), [(1, vec![1.0, 2.0])]);
        // Crossfading: runs of its own, after the whole ones.
        lists.set(&mut arena, b, &[1; 5], 0b11, &at(2.0).with_fade(0.5));
        assert_eq!(drawn(&lists, &arena, 0), [(3, vec![1.0]), (1 | FADED, vec![2.0])]);
        assert_eq!(drawn(&lists, &arena, 1), [(1, vec![1.0]), (1 | FADED, vec![2.0])]);
        // Released ids are drawn nowhere and handed out again.
        lists.release(&mut arena, a);
        assert_eq!(drawn(&lists, &arena, 0), [(1 | FADED, vec![2.0])]);
        assert_eq!(lists.alloc(), a);
        assert_eq!(lists.flags(a), 0);
    }

    #[test]
    fn a_block_that_outgrows_its_place_moves_and_its_things_follow() {
        let (mut lists, mut arena) = (Lists::default(), Vec::new());
        lists.begin(&mut arena);
        let (a, b) = (lists.alloc(), lists.alloc());
        lists.set(&mut arena, a, &[0; 5], 1, &at(1.0));
        lists.set(&mut arena, b, &[1; 5], 1, &at(2.0));
        lists.finish(&mut arena);
        assert_eq!((lists.slot(0, a), lists.slot(0, b), arena.len()), (0, 1, 2));
        // Mesh 0's block has mesh 1's behind it: it moves to the end, with
        // room for four.
        lists.begin(&mut arena);
        let c = lists.alloc();
        lists.set(&mut arena, c, &[0; 5], 1, &at(3.0));
        lists.finish(&mut arena);
        assert_eq!((lists.slot(0, a), lists.slot(0, c), lists.slot(0, b)), (2, 3, 1));
        assert_eq!(arena.len(), 6);
        assert_eq!(drawn(&lists, &arena, 0), [(0, vec![1.0, 3.0]), (1, vec![2.0])]);
        assert_eq!(lists.repacks, 1, "only the first frame was packed");
    }

    #[test]
    fn an_array_twice_what_it_needs_is_packed_again_with_room_to_grow() {
        let (mut lists, mut arena) = (Lists::default(), Vec::new());
        lists.begin(&mut arena);
        let ids: Vec<u32> = (0..3000).map(|_| lists.alloc()).collect();
        for (i, &id) in ids.iter().enumerate() {
            lists.set(&mut arena, id, &[(i % 3) as u32; 5], 1, &at(i as f32));
        }
        lists.finish(&mut arena);
        assert_eq!((arena.len(), lists.repacks), (3000, 1));
        // Most leave: the array is packed down to what stays, each block
        // with room for as many again, in the order they had.
        lists.begin(&mut arena);
        for &id in &ids[60..] {
            lists.clear(&mut arena, id);
        }
        lists.finish(&mut arena);
        assert_eq!((arena.len(), lists.repacks), (120, 2));
        assert_eq!(lists.count(0), 60);
        let all = drawn(&lists, &arena, 0);
        assert_eq!(all.len(), 3);
        assert_eq!(all[1].1[..4], [1.0, 4.0, 7.0, 10.0]);
        // Still, nothing more is packed or moved; and they can double
        // without a block moving.
        let before = arena.clone();
        lists.begin(&mut arena);
        lists.finish(&mut arena);
        assert_eq!(lists.repacks, 2);
        assert_eq!(
            bytemuck::cast_slice::<_, u8>(&before),
            bytemuck::cast_slice::<_, u8>(&arena)
        );
        lists.begin(&mut arena);
        for (i, &id) in ids[60..120].iter().enumerate() {
            lists.set(&mut arena, id, &[(i % 3) as u32; 5], 1, &at(0.5));
        }
        lists.finish(&mut arena);
        assert_eq!((arena.len(), lists.count(0), lists.repacks), (120, 120, 2));
    }

    #[test]
    fn transients_keep_their_ids_when_the_frame_draws_them_in_another_order() {
        let (mut lists, mut arena, mut motion) = (Lists::default(), Vec::new(), Motion::default());
        let mut things = Transients::default();
        let mut frame = |things: &mut Transients, lists: &mut Lists, arena: &mut Vec<Instance3D>, keys: &[u64]| {
            lists.begin(arena);
            things.begin();
            for &key in keys {
                let (key, instance) = ((key, LOD), at(key as f32));
                things.touch(lists, arena, &mut motion, key, NO_KEY, &[0; 5], 1, &instance);
            }
            things.finish(lists, arena);
            lists.finish(arena);
            drawn(lists, arena, 0)
        };
        assert_eq!(
            frame(&mut things, &mut lists, &mut arena, &[1, 2, 3, 4]),
            [(0, vec![1.0, 2.0, 3.0, 4.0])]
        );
        // 2 is gone and 5 is new: the others keep their slots (5, the
        // last by then, takes 2's).
        assert_eq!(
            frame(&mut things, &mut lists, &mut arena, &[1, 3, 4, 5]),
            [(0, vec![1.0, 5.0, 3.0, 4.0])]
        );
        // Another order draws the same.
        assert_eq!(
            frame(&mut things, &mut lists, &mut arena, &[5, 4, 3, 1]),
            [(0, vec![1.0, 5.0, 3.0, 4.0])]
        );
        // The same key twice in a frame is two things.
        assert_eq!(
            frame(&mut things, &mut lists, &mut arena, &[7, 7]),
            [(0, vec![7.0, 7.0])]
        );
        assert_eq!(frame(&mut things, &mut lists, &mut arena, &[]), []);
        assert_eq!(things.by_key.len(), 0);
    }
}
