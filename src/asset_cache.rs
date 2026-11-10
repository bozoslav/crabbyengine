//! The loaded-asset cache of a service that loads named files.
//!
//! `ctx.gfx` (textures, fonts) and `ctx.audio` (sounds, streams) each keep
//! one: what a name resolved to, by [`Origin`] and [`AssetKind`], so loading
//! it again returns the same handle and never reads a source again. Before
//! this owner, each service kept its own map plus a side vector of handles
//! indexed by the map's values, and rebuilt the same lookup and release
//! steps. The handles now live in the map, and lookup and release are here.
//!
//! The cache does not own the asset sources (`SharedSources`, added through
//! `ctx.assets`), the hot-reload watch list or the handle liveness sets;
//! those stay with the service or with `ctx.assets`, which pass them in.

use std::collections::HashMap;

use crate::assets::{Lookup, Origin, SharedSources};

/// Which loader a cached file went through.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum AssetKind {
    Texture,
    Font,
    Sound,
    /// A sound loaded with `load_stream`: the same name can also be a
    /// `load_sound`, decoded whole, and the two are different sounds.
    Stream,
}

/// Handles of loaded assets by where they came from and how they loaded.
pub(crate) struct AssetCache<H> {
    loaded: HashMap<(Origin, AssetKind), H>,
}

impl<H> Default for AssetCache<H> {
    fn default() -> Self {
        Self { loaded: HashMap::new() }
    }
}

impl<H: Copy> AssetCache<H> {
    /// What `origin` loaded as `kind`, if it did.
    pub(crate) fn get(&self, origin: &Origin, kind: AssetKind) -> Option<H> {
        self.loaded.get(&(origin.clone(), kind)).copied()
    }

    /// Records that `origin` loaded as `kind` and became `handle`.
    pub(crate) fn insert(&mut self, origin: Origin, kind: AssetKind, handle: H) {
        self.loaded.insert((origin, kind), handle);
    }

    /// Finds asset `name` in `sources` unless this cache already has it from
    /// where it would be found: [`SharedSources::lookup`] with this cache as
    /// the memory. Earlier sources win, so this matches what a fresh
    /// [`find`](SharedSources::find) would pick for anything still cached,
    /// and sources are only read for what isn't cached.
    ///
    /// # Errors
    /// As [`SharedSources::find`].
    pub(crate) fn lookup(
        &self,
        sources: &SharedSources,
        name: &str,
        kind: AssetKind,
    ) -> Result<Lookup<H>, crate::Error> {
        sources.lookup(name, |origin| self.get(origin, kind))
    }

    /// Forgets every entry of one of `kinds` whose handle `is` the freed
    /// asset, so loading its file again makes a new one.
    pub(crate) fn release(&mut self, kinds: &[AssetKind], is: impl Fn(&H) -> bool) {
        self.loaded
            .retain(|(_, kind), handle| !kinds.contains(kind) || !is(handle));
    }

    /// How many entries there are.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.loaded.len()
    }

    /// The cached origins, for tests of where assets came from.
    #[cfg(test)]
    pub(crate) fn origins(&self) -> Vec<Origin> {
        self.loaded.keys().map(|(origin, _)| origin.clone()).collect()
    }

    /// Whether `origin` loaded as `kind`.
    #[cfg(test)]
    pub(crate) fn contains(&self, origin: &Origin, kind: AssetKind) -> bool {
        self.loaded.contains_key(&(origin.clone(), kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn file(name: &str) -> Origin {
        Origin::File(PathBuf::from(name))
    }

    #[test]
    fn the_same_file_is_cached_per_kind() {
        let mut cache = AssetCache::<u32>::default();
        cache.insert(file("a"), AssetKind::Sound, 1);
        cache.insert(file("a"), AssetKind::Stream, 2);
        assert_eq!(cache.get(&file("a"), AssetKind::Sound), Some(1));
        assert_eq!(cache.get(&file("a"), AssetKind::Stream), Some(2));
        assert_eq!(cache.get(&file("a"), AssetKind::Texture), None);
        assert_eq!(cache.get(&file("b"), AssetKind::Sound), None);
    }

    #[test]
    fn release_forgets_only_the_freed_handle_of_the_named_kinds() {
        let mut cache = AssetCache::<u32>::default();
        cache.insert(file("a"), AssetKind::Sound, 1);
        cache.insert(file("a"), AssetKind::Stream, 1);
        cache.insert(file("b"), AssetKind::Sound, 2);
        cache.insert(file("c"), AssetKind::Texture, 1);
        cache.release(&[AssetKind::Sound, AssetKind::Stream], |&h| h == 1);
        assert_eq!(cache.len(), 2);
        assert!(cache.contains(&file("b"), AssetKind::Sound), "another sound stays");
        assert!(cache.contains(&file("c"), AssetKind::Texture), "another kind stays");
    }
}
