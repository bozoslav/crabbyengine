//! Meshes streamed in without waiting ([`Ctx::stream_mesh`]), and LOD
//! groups whose levels stream in as the camera needs them ([`StreamedLod`]).

use std::sync::Arc;

use ponko_ecs::Entity;

use super::{LodGroup, LodPick, Mesh, Transform3D, gltf};
use crate::assets::{self, Origin};
use crate::residency::ResidencyKind;
use crate::streaming::{AssetState, StreamedAsset, StreamedMesh};
use crate::{Ctx, stream::Job};

/// A decoded glTF file waiting for the game thread.
pub(crate) enum LoadedFile {
    /// Parsed on a worker.
    Parsed(Box<gltf::Loaded>),
    /// Its bytes only: it came from an added asset source, whose files
    /// (its buffers and images) can only be read on the game thread.
    Bytes(Vec<u8>),
}

/// Streams the levels of an entity's [`LodGroup`] in from files as the
/// camera needs them: file `i` gives the mesh of level `i`'s first
/// renderer. Each frame the engine asks for the level the camera shows
/// (with [`Ctx::stream_mesh`]) and for the coarsest level; until a level
/// is in, the group shows the closest coarser level that is (or finer, if
/// none), so a model never pops out while it loads. Levels the camera
/// moved away from stop showing their meshes, so the
/// [mesh budget](Ctx::set_residency_budget) can evict them.
///
/// Spawn the group with any meshes (such as [`Mesh::NONE`]) in the
/// levels the files fill in.
///
/// ```
/// # use ponko::prelude::*;
/// # fn f(ctx: &mut Ctx) {
/// let levels = vec![
///     ponko::render::LodLevel::new(0.3, vec![ponko::render::LodRenderer::new(ponko::render::Mesh::NONE)]),
///     ponko::render::LodLevel::new(0.05, vec![ponko::render::LodRenderer::new(ponko::render::Mesh::NONE)]),
/// ];
/// ctx.world.spawn((
///     ponko::d3::Transform::default(),
///     ponko::render::LodGroup::new(10.0, levels),
///     ponko::d3::StreamedLod::new(&["models/tower_lod0.glb", "models/tower_lod1.glb"]),
/// ));
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamedLod {
    files: Arc<[String]>,
}

impl StreamedLod {
    /// Levels from `files`, most detailed first, one per [`LodGroup`]
    /// level (extra files are ignored; levels without a file keep their
    /// meshes).
    #[must_use]
    pub fn new(files: &[&str]) -> Self {
        Self {
            files: files.iter().map(|f| (*f).to_string()).collect(),
        }
    }

    /// The files, most detailed level first.
    #[must_use]
    pub fn files(&self) -> &[String] {
        &self.files
    }
}

impl Ctx {
    /// Mesh file `path`, streamed in without waiting: `None` the first time
    /// (it starts reading and decoding on a worker thread) and until it is
    /// uploaded, within the [upload budget](crate::gfx::Graphics::set_upload_budget), at
    /// the start of a later frame; then the same [`Mesh`] as
    /// [`resident_mesh`](Self::resident_mesh) gives, under the same
    /// [mesh budget](Self::set_residency_budget). Ask for it each tick or
    /// frame you need it. An evicted mesh streams back in under the same
    /// handle.
    ///
    /// Files that refer to other files (a `.gltf` with its `.bin` and
    /// images) from added [asset sources](crate::Assets::add_source) are
    /// decoded on the game thread: those sources can only be read there. A
    /// failed load gives [`AssetState::Failed`] and isn't retried by
    /// asking again; `resident_mesh` loads it again.
    ///
    /// ```
    /// # use ponko::prelude::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     assert_eq!(ctx.stream_mesh("models/missing.glb"), None);
    ///     ctx.finish_uploads();
    ///     assert_eq!(ctx.gfx.asset_state(ponko::residency::ResidencyKind::Mesh, "models/missing.glb"), ponko::streaming::AssetState::Failed);
    /// });
    /// ```
    pub fn stream_mesh(&mut self, path: &str) -> Option<Mesh> {
        if let Some(mesh) = self.gfx.live_resident_mesh(path) {
            let now = self.residency_now();
            self.gfx.residency.set_mut(ResidencyKind::Mesh).touch(path, now);
            return Some(mesh);
        }
        if self.gfx.streaming.meshes.contains_key(path) {
            return None;
        }
        self.gfx.residency.set_mut(ResidencyKind::Mesh).forget(path);
        let lookup = self
            .gfx
            .asset_sources
            .lookup(path, |origin| self.gfx.assets3d.loaded.get(origin).copied());
        let job = match lookup {
            // The game loaded it already: it holds the handle.
            Ok(assets::Lookup::Cached(origin, mesh)) => {
                self.add_resident_mesh(path, mesh, origin, true);
                return Some(mesh);
            }
            Ok(assets::Lookup::Found(found)) => {
                let origin = found.origin.clone();
                let name = path.to_string();
                match origin {
                    Origin::File(_) if self.gfx.asset_sources.is_empty() => Job::spawn(move || {
                        let bytes = found.into_bytes().map_err(|e| e.with_path(&name))?;
                        let mut read = |file: &str| assets::Sources::default().read(file);
                        let loaded = gltf::load(&name, &bytes, &mut read, &|_| false)
                            .map_err(|e| crate::Error::decode(&name, e))?;
                        Ok((origin, LoadedFile::Parsed(Box::new(loaded))))
                    }),
                    _ => {
                        let bytes = found.into_bytes().map_err(|e| e.with_path(path));
                        Job::spawn(move || Ok((origin, LoadedFile::Bytes(bytes?))))
                    }
                }
            }
            Err(e) => Job::spawn(move || Err(e)),
        };
        self.gfx.streaming.meshes.insert(
            path.to_string(),
            StreamedMesh {
                job: Some(job),
                ready: None,
                state: AssetState::Loading,
            },
        );
        self.gfx.streaming.queue_mesh(path);
        None
    }

    /// One upload of streamed mesh `name`: making it on the GPU.
    pub(crate) fn mesh_step(&mut self, name: &str, room: u64, first: bool) -> crate::streaming::Step {
        use crate::streaming::Step;
        let Some(m) = self.gfx.streaming.meshes.get_mut(name) else {
            return Step::Dropped;
        };
        if m.ready.is_none() {
            let mut job = m.job.take();
            let decoded = self.take_decoded(&mut job);
            let m = self.gfx.streaming.meshes.get_mut(name).expect("still there");
            m.job = job;
            match decoded {
                None => return Step::NotReady,
                Some(Err(e)) => {
                    m.state = AssetState::Failed;
                    self.gfx.streaming.fail(ResidencyKind::Mesh, name, e);
                    return Step::Dropped;
                }
                Some(Ok(ready)) => {
                    m.ready = Some(ready);
                    m.state = AssetState::Uploading;
                }
            }
        }
        let m = self.gfx.streaming.meshes.get(name).expect("still there");
        let bytes = match &m.ready {
            Some((_, LoadedFile::Parsed(loaded))) => {
                super::mesh_gpu_bytes(&loaded.data)
                    + loaded
                        .images
                        .iter()
                        .map(|(w, h, _)| crate::inventory::rgba8_bytes(*w, *h))
                        .sum::<u64>()
            }
            Some((_, LoadedFile::Bytes(b))) => b.len() as u64,
            None => 0,
        };
        if !first && bytes > room {
            return Step::NotReady;
        }
        let (origin, file) = self
            .gfx
            .streaming
            .meshes
            .remove(name)
            .and_then(|m| m.ready)
            .expect("decoded");
        // Loaded by the game meanwhile: it holds that one.
        if let Some(&mesh) = self.gfx.assets3d.loaded.get(&origin) {
            self.add_resident_mesh(name, mesh, origin, true);
            return Step::Uploaded { bytes: 0, again: false };
        }
        let made = self.load_reserved(name, |ctx| {
            let (loaded, image_keys) = match file {
                LoadedFile::Parsed(loaded) => {
                    let keys = ctx.gfx.image_keys(&loaded);
                    (*loaded, keys)
                }
                LoadedFile::Bytes(bytes) => ctx.gfx.decode_gltf(name, &bytes)?,
            };
            Ok(ctx.gfx.mesh_from_loaded(name, origin.clone(), loaded, image_keys))
        });
        match made {
            Ok(mesh) => {
                self.add_resident_mesh(name, mesh, origin, false);
                Step::Uploaded { bytes, again: false }
            }
            Err(e) => {
                self.gfx.streaming.meshes.insert(
                    name.to_string(),
                    StreamedMesh {
                        job: None,
                        ready: None,
                        state: AssetState::Failed,
                    },
                );
                self.gfx.streaming.fail(ResidencyKind::Mesh, name, e);
                Step::Dropped
            }
        }
    }

    /// Asks for the levels each [`StreamedLod`] group needs and shows what
    /// is in (`pump_streaming`).
    pub(crate) fn stream_lod_levels(&mut self) {
        let mut groups = std::mem::take(&mut self.gfx.streaming.lod_scratch);
        groups.clear();
        {
            let (camera, settings) = (&self.camera3d, &self.lod);
            self.world
                .query::<(Entity, &Transform3D, &LodGroup, &StreamedLod)>()
                .for_each(|(e, t, g, s)| {
                    let n = g.levels.len().min(s.files.len()).min(64);
                    if n == 0 {
                        return;
                    }
                    let mut wanted = 1u64 << (n - 1);
                    match g.pick_for(t, camera, settings) {
                        LodPick::Level(i) if i < n => wanted |= 1 << i,
                        LodPick::Fade { from, to, .. } => {
                            for i in [Some(from), to].into_iter().flatten() {
                                if i < n {
                                    wanted |= 1 << i;
                                }
                            }
                        }
                        _ => {}
                    }
                    groups.push((e, s.files.clone(), wanted));
                });
        }
        for (entity, files, wanted) in groups.drain(..) {
            let n = (u64::BITS - wanted.leading_zeros()) as usize;
            let mut meshes = [None; 64];
            for (i, mesh) in meshes.iter_mut().enumerate().take(n) {
                if wanted & (1 << i) != 0 {
                    *mesh = self.stream_mesh(&files[i]);
                }
            }
            // Each level shows its mesh if in, else the closest coarser
            // one, else the closest finer one.
            let shown = |i: usize| {
                meshes[i]
                    .or_else(|| meshes[i + 1..n].iter().find_map(|m| *m))
                    .or_else(|| meshes[..i].iter().rev().find_map(|m| *m))
                    .unwrap_or(Mesh::NONE)
            };
            let Some(group) = self.world.get::<LodGroup>(entity) else {
                continue;
            };
            let current = |i: usize| group.levels[i].renderers.first().map(|r| r.mesh);
            if (0..n).all(|i| current(i).is_none_or(|m| m == shown(i))) {
                continue;
            }
            let group = self.world.get_mut::<LodGroup>(entity).expect("just read");
            for i in 0..n {
                if let Some(r) = group.levels[i].renderers.first_mut() {
                    r.mesh = shown(i);
                }
            }
        }
        self.gfx.streaming.lod_scratch = groups;
    }
}

impl crate::gfx::Graphics {
    /// Where mesh `name` is (`asset_state`).
    pub(crate) fn mesh_state(&self, name: &str) -> AssetState {
        if self.live_resident_mesh(name).is_some() {
            return AssetState::Resident;
        }
        if let Some(m) = self.streaming.meshes.get(name) {
            return m.state;
        }
        if self.residency.reserved_meshes.contains_key(name) {
            return AssetState::Evicted;
        }
        AssetState::Unknown
    }

    /// Adds the streamed and resident meshes to `out` (`streamed_assets`).
    pub(crate) fn list_streamed_meshes(&self, out: &mut Vec<StreamedAsset>) {
        let mut names: Vec<&String> = self.streaming.meshes.keys().collect();
        names.extend(self.residency.reserved_meshes.keys());
        names.extend(self.residency.set(ResidencyKind::Mesh).names());
        names.sort_unstable();
        names.dedup();
        for name in names {
            let id = self
                .live_resident_mesh(name)
                .map(|m| m.id)
                .or_else(|| self.residency.reserved_meshes.get(name).copied());
            out.push(StreamedAsset {
                kind: ResidencyKind::Mesh,
                name: name.clone(),
                state: self.mesh_state(name),
                id,
                size: [0, 0],
            });
        }
    }
}
