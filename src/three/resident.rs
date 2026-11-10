//! Meshes loaded under a residency budget ([`crate::residency`]).

use std::collections::HashSet;

use super::{LodGroup, LodShadow, Mesh, Model, ShadowMesh};
use crate::Ctx;
use crate::residency::ResidencyKind;

impl Ctx {
    /// The mesh in file `path`, loaded now if it isn't resident, under the
    /// [mesh budget](Self::set_residency_budget). Ask for it each tick or
    /// frame you need it (cell streaming, a level's props): asking marks it
    /// used, and when resident meshes go over the budget, the least recently
    /// used ones are freed until they fit. A freed one loads again, from
    /// where [`load_mesh`](crate::gfx::Graphics::load_mesh) finds it, the next time it's
    /// asked for, as the same [`Mesh`] (its id is kept for it while it is
    /// out: a handle the game kept reads as not [live](Self::is_live) and
    /// models showing it draw nothing until then); its materials and
    /// textures stay loaded. [Freeing](Self::free) such a kept handle
    /// releases the id; the file then loads as a new mesh.
    ///
    /// The budget is checked when a mesh loads and at the start of every
    /// frame (so what entities stop showing can go). Never evicted: meshes
    /// asked for in this or the previous frame or tick (so the handle stays
    /// good while you use it, and what you ask for every frame or every
    /// fixed-step tick stays between asks), meshes a [`Model`],
    /// [`ShadowMesh`] or [`LodGroup`] shows, [pinned](Self::pin_resident_mesh)
    /// ones, and ones the game also loaded with `load_mesh` before asking
    /// here. Without a budget nothing is evicted.
    ///
    /// ```
    /// # use ponko::prelude::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     ctx.set_residency_budget(ponko::residency::ResidencyKind::Mesh, Some(256 << 20));
    ///     let house = ctx.resident_mesh("models/house.glb");
    ///     ctx.world.spawn((ponko::d3::Transform::default(), ponko::render::Model::new(house))); // kept while shown
    ///     println!("meshes: {}", ctx.gfx.residency(ponko::residency::ResidencyKind::Mesh));
    /// });
    /// ```
    ///
    /// # Panics
    ///
    /// With the loading error if the file is missing or can't be read, as
    /// `load_mesh` does.
    pub fn resident_mesh(&mut self, path: &str) -> Mesh {
        self.try_resident_mesh(path).unwrap_or_else(|e| panic!("{e}"))
    }

    /// Like [`resident_mesh`](Self::resident_mesh), and never evicted until
    /// [`unpin_resident_mesh`](Self::unpin_resident_mesh). For meshes that
    /// must not hitch when they come back (the player's), or that the game
    /// keeps handles to for longer than a tick.
    ///
    /// # Panics
    ///
    /// As `resident_mesh`.
    pub fn pin_resident_mesh(&mut self, path: &str) -> Mesh {
        let mesh = self.resident_mesh(path);
        self.gfx.residency.set_mut(ResidencyKind::Mesh).set_pinned(path, true);
        mesh
    }

    /// Lets a [pinned](Self::pin_resident_mesh) mesh be evicted again, and
    /// evicts what the budget needs gone. Does nothing for one that isn't
    /// resident.
    pub fn unpin_resident_mesh(&mut self, path: &str) {
        self.gfx.residency.set_mut(ResidencyKind::Mesh).set_pinned(path, false);
        self.enforce_residency(ResidencyKind::Mesh);
    }

    /// [`resident_mesh`](Self::resident_mesh) with errors as values, the
    /// same ones [`try_load_mesh`](crate::gfx::Graphics::try_load_mesh) returns. A failed
    /// load changes nothing resident.
    ///
    /// ```
    /// # use ponko::prelude::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let hat = ctx.try_resident_mesh("models/hat.glb");
    ///     assert_eq!(hat.unwrap_err().kind(), ErrorKind::NotFound);
    ///     assert!(!ctx.gfx.is_resident(ponko::residency::ResidencyKind::Mesh, "models/hat.glb"));
    /// });
    /// ```
    ///
    /// # Errors
    ///
    /// As `try_load_mesh`.
    pub fn try_resident_mesh(&mut self, path: &str) -> Result<Mesh, crate::Error> {
        let now = self.residency_now();
        if let Some(mesh) = self.gfx.live_resident_mesh(path) {
            self.gfx.residency.set_mut(ResidencyKind::Mesh).touch(path, now);
            return Ok(mesh);
        }
        // Freed by the game (or never loaded): load it, noting whether the
        // game already holds it from `load_mesh`.
        self.gfx.residency.set_mut(ResidencyKind::Mesh).forget(path);
        // Already in the load cache: the game holds it from `load_mesh`.
        let cached = self.gfx.assets3d.loaded.len();
        let (mesh, file) = self.load_reserved(path, |ctx| ctx.gfx.load_mesh_from(path))?;
        let shared = self.gfx.assets3d.loaded.len() == cached;
        self.add_resident_mesh(path, mesh, file, shared);
        Ok(mesh)
    }

    /// Adds `mesh`, just loaded from `file` as `path`, to the resident
    /// meshes and evicts what the budget needs gone.
    pub(crate) fn add_resident_mesh(&mut self, path: &str, mesh: Mesh, file: crate::assets::Origin, shared: bool) {
        let now = self.residency_now();
        let bytes = self.gfx.assets3d.mesh_bytes.get(mesh.id as usize).copied().unwrap_or(0);
        self.gfx
            .residency
            .set_mut(ResidencyKind::Mesh)
            .insert(path, (mesh.id, file), bytes, shared, now);
        self.enforce_residency(ResidencyKind::Mesh);
    }

    /// Runs `load` (which makes mesh `path` at most once) so that a mesh it
    /// makes takes the id `path` had before it was evicted. The id is
    /// released if `load` found the file cached instead.
    pub(crate) fn load_reserved<T>(
        &mut self,
        path: &str,
        load: impl FnOnce(&mut Self) -> Result<T, crate::Error>,
    ) -> Result<T, crate::Error> {
        let reserved = self.gfx.residency.reserved_meshes.get(path).copied();
        self.gfx.assets3d.reuse_mesh_id = reserved;
        let result = load(self);
        let unused = self.gfx.assets3d.reuse_mesh_id.take();
        if result.is_ok()
            && let Some(id) = reserved
        {
            self.gfx.residency.reserved_meshes.remove(path);
            if unused.is_some() {
                self.gfx.assets3d.free_meshes.push(id);
            }
        }
        result
    }

    /// Frees the meshes the budget needs gone (`enforce_residency`).
    pub(crate) fn evict_meshes(&mut self, now: (u64, u64)) {
        let shown = self.shown_meshes();
        let evicted = self
            .gfx
            .residency
            .set_mut(ResidencyKind::Mesh)
            .evict(now, |id| shown.contains(&id));
        self.gfx.residency.shown = shown;
        for (name, entry) in evicted {
            let mesh = Mesh { id: entry.handle };
            // Freed elsewhere since: its id may be another mesh's now.
            if self.gfx.assets3d.loaded.get(&entry.file) == Some(&mesh) {
                self.gfx.free(mesh);
                // Keep the id for this name: a handle the game kept reads
                // as not live (models showing it draw nothing) until the
                // mesh is asked for again and comes back under it.
                if let Some(i) = self.gfx.assets3d.free_meshes.iter().rposition(|&id| id == mesh.id) {
                    self.gfx.assets3d.free_meshes.swap_remove(i);
                }
                self.gfx.residency.reserved_meshes.insert(name, mesh.id);
            }
        }
    }

    /// Ids of the meshes entities show (models, shadow meshes, LOD levels),
    /// in the reused set taken from `residency.shown` (put it back after
    /// use, so checking the budget each frame doesn't allocate).
    fn shown_meshes(&mut self) -> HashSet<u32> {
        let mut ids = std::mem::take(&mut self.gfx.residency.shown);
        ids.clear();
        self.world.query::<&Model>().for_each(|m| {
            ids.insert(m.mesh.id);
        });
        self.world.query::<&ShadowMesh>().for_each(|s| {
            if let Some(m) = s.0 {
                ids.insert(m.id);
            }
        });
        self.world.query::<&LodGroup>().for_each(|g| {
            for level in &g.levels {
                let casters = match &level.shadow {
                    LodShadow::Casters(c) => c.as_slice(),
                    LodShadow::Same => &[],
                };
                for r in level.renderers.iter().chain(casters) {
                    ids.insert(r.mesh.id);
                }
            }
        });
        ids
    }
}

impl crate::gfx::Graphics {
    /// Mesh `id` is being freed (`free_mesh`). If it is the id kept for an
    /// evicted resident mesh, the game let its handle go: release the id
    /// (once) and forget the name. Returns whether it did. A live resident
    /// mesh's entry is dropped from the budget too.
    pub(crate) fn release_resident_mesh_id(&mut self, id: u32, live: bool) -> bool {
        if live {
            let set = self.residency.set_mut(ResidencyKind::Mesh);
            let name = set
                .names()
                .find(|n| set.get(n).is_some_and(|e| e.handle == id))
                .cloned();
            if let Some(name) = name {
                set.forget(&name);
            }
            return false;
        }
        let reserved = &mut self.residency.reserved_meshes;
        let Some(name) = reserved.iter().find(|(_, r)| **r == id).map(|(n, _)| n.clone()) else {
            return false;
        };
        reserved.remove(&name);
        self.assets3d.free_meshes.push(id);
        true
    }

    /// The resident mesh called `path`, unless it was evicted or the game
    /// freed it (then the load cache no longer maps its file to it).
    pub(crate) fn live_resident_mesh(&self, path: &str) -> Option<Mesh> {
        let entry = self.residency.set(ResidencyKind::Mesh).get(path)?;
        let mesh = Mesh { id: entry.handle };
        (self.assets3d.loaded.get(&entry.file) == Some(&mesh)).then_some(mesh)
    }
}
