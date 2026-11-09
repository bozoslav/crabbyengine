//! Meshes loaded under a residency budget ([`crate::residency`]).

use std::collections::HashSet;

use super::{LodGroup, LodShadow, Mesh, Model, ShadowMesh};
use crate::{Ctx, ResidencyKind};

impl Ctx {
    /// The mesh in file `path`, loaded now if it isn't resident, under the
    /// [mesh budget](Self::set_residency_budget). Ask for it each tick or
    /// frame you need it (cell streaming, a level's props): asking marks it
    /// used, and when resident meshes go over the budget, the least recently
    /// used ones are freed until they fit. A freed one loads again, from
    /// where [`load_mesh`](Self::load_mesh) finds it, the next time it's
    /// asked for, possibly as a different [`Mesh`]; its materials and
    /// textures stay loaded.
    ///
    /// Never evicted: meshes asked for in the current frame and tick (so the
    /// handle stays good while you use it), meshes a [`Model`],
    /// [`ShadowMesh`] or [`LodGroup`] shows, [pinned](Self::pin_resident_mesh)
    /// ones, and ones the game also loaded with `load_mesh` before asking
    /// here. Without a budget nothing is evicted.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     ctx.set_residency_budget(ResidencyKind::Mesh, Some(256 << 20));
    ///     let house = ctx.resident_mesh("models/house.glb");
    ///     ctx.world.spawn((Transform3D::default(), Model::new(house))); // kept while shown
    ///     println!("meshes: {}", ctx.residency(ResidencyKind::Mesh));
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
        self.residency.set_mut(ResidencyKind::Mesh).set_pinned(path, true);
        mesh
    }

    /// Lets a [pinned](Self::pin_resident_mesh) mesh be evicted again, and
    /// evicts what the budget needs gone. Does nothing for one that isn't
    /// resident.
    pub fn unpin_resident_mesh(&mut self, path: &str) {
        self.residency.set_mut(ResidencyKind::Mesh).set_pinned(path, false);
        self.enforce_residency(ResidencyKind::Mesh);
    }

    /// [`resident_mesh`](Self::resident_mesh) with errors as values, the
    /// same ones [`try_load_mesh`](Self::try_load_mesh) returns. A failed
    /// load changes nothing resident.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let hat = ctx.try_resident_mesh("models/hat.glb");
    ///     assert_eq!(hat.unwrap_err().kind(), ErrorKind::NotFound);
    ///     assert!(!ctx.is_resident(ResidencyKind::Mesh, "models/hat.glb"));
    /// });
    /// ```
    ///
    /// # Errors
    ///
    /// As `try_load_mesh`.
    pub fn try_resident_mesh(&mut self, path: &str) -> Result<Mesh, crate::Error> {
        let now = self.residency_now();
        if let Some(mesh) = self.live_resident_mesh(path) {
            self.residency.set_mut(ResidencyKind::Mesh).touch(path, now);
            return Ok(mesh);
        }
        // Freed by the game (or never loaded): load it, noting whether the
        // game already holds it from `load_mesh`.
        self.residency.set_mut(ResidencyKind::Mesh).forget(path);
        // Already in the load cache: the game holds it from `load_mesh`.
        let cached = self.assets3d.loaded.len();
        let (mesh, file) = self.load_mesh_from(path)?;
        let shared = self.assets3d.loaded.len() == cached;
        let bytes = self.assets3d.mesh_bytes.get(mesh.id as usize).copied().unwrap_or(0);
        self.residency
            .set_mut(ResidencyKind::Mesh)
            .insert(path, (mesh.id, file), bytes, shared, now);
        self.enforce_residency(ResidencyKind::Mesh);
        Ok(mesh)
    }

    /// The resident mesh called `path`, unless it was evicted or the game
    /// freed it (then the load cache no longer maps its file to it).
    pub(crate) fn live_resident_mesh(&self, path: &str) -> Option<Mesh> {
        let entry = self.residency.set(ResidencyKind::Mesh).get(path)?;
        let mesh = Mesh { id: entry.handle };
        (self.assets3d.loaded.get(&entry.file) == Some(&mesh)).then_some(mesh)
    }

    /// Frees the meshes the budget needs gone (`enforce_residency`).
    pub(crate) fn evict_meshes(&mut self, now: (u64, u64)) {
        let shown = self.shown_meshes();
        let evicted = self
            .residency
            .set_mut(ResidencyKind::Mesh)
            .evict(now, |id| shown.contains(&id));
        for (_, entry) in evicted {
            let mesh = Mesh { id: entry.handle };
            // Freed elsewhere since: its id may be another mesh's now.
            if self.assets3d.loaded.get(&entry.file) == Some(&mesh) {
                self.free_mesh(mesh);
            }
        }
    }

    /// Ids of the meshes entities show: models, shadow meshes, LOD levels.
    fn shown_meshes(&mut self) -> HashSet<u32> {
        let mut ids = HashSet::new();
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
