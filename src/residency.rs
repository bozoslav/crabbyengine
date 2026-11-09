//! Residency budgets: an opt-in byte budget per kind of asset, and the set
//! of assets loaded by name under it.
//!
//! A game asks for an asset by name each time it needs it
//! ([`Ctx::resident_mesh`](crate::Ctx::resident_mesh)); the asset loads if
//! it isn't resident, and when the resident bytes of its kind go over the
//! kind's budget, the least recently used assets that nothing needs are
//! evicted until they fit. Evicted assets load again (from their asset
//! source) the next time they are asked for.
//!
//! Which cells of a world to ask for is up to the game, for example with a
//! [`CellStreamer`](crate::CellStreamer); the budget then caps the memory
//! however many cells are near.

// Only meshes use the sets so far, and meshes need the `3d` feature.
#![cfg_attr(not(feature = "3d"), allow(dead_code, unused_variables))]

use std::collections::BTreeMap;
use std::fmt;

/// A kind of asset with its own residency budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResidencyKind {
    /// Meshes loaded from files with [`Ctx::resident_mesh`](crate::Ctx::resident_mesh):
    /// their vertex and index buffers.
    Mesh,
}

impl ResidencyKind {
    /// Every kind, in a fixed order.
    pub const ALL: [ResidencyKind; 1] = [ResidencyKind::Mesh];

    /// Its name in reports: `"mesh"`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            ResidencyKind::Mesh => "mesh",
        }
    }

    fn index(self) -> usize {
        match self {
            ResidencyKind::Mesh => 0,
        }
    }
}

/// The budget, usage and eviction counters of one [`ResidencyKind`]
/// ([`Ctx::residency`](crate::Ctx::residency)). Bytes are the engine's
/// estimates of GPU memory.
///
/// ```
/// # use ponko::*;
/// Game::new("t", 1, 1).run_headless(0, |ctx| {
///     ctx.set_residency_budget(ResidencyKind::Mesh, Some(64 << 20));
///     let meshes = ctx.residency(ResidencyKind::Mesh);
///     assert_eq!(meshes.budget, Some(64 << 20));
///     assert_eq!((meshes.resident, meshes.evictions), (0, 0));
/// });
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResidencyStats {
    /// The budget in bytes; `None` (the default) for no limit.
    pub budget: Option<u64>,
    /// Assets resident now.
    pub resident: usize,
    /// Their bytes.
    pub resident_bytes: u64,
    /// Resident assets that are never evicted: pinned ones, and ones the
    /// game also loaded directly (it holds their handles itself).
    pub pinned: usize,
    /// Their bytes.
    pub pinned_bytes: u64,
    /// Assets loaded for the first time.
    pub loads: u64,
    /// Assets loaded again after they were evicted or freed.
    pub reloads: u64,
    /// Assets evicted to stay under the budget.
    pub evictions: u64,
    /// Bytes those evictions released.
    pub evicted_bytes: u64,
}

impl ResidencyStats {
    /// How far the resident bytes are over the budget; 0 when under it or
    /// without one. Only assets that can't be evicted keep it above 0:
    /// pinned ones, ones entities show, and ones asked for this tick.
    #[must_use]
    pub fn over_budget_bytes(&self) -> u64 {
        self.budget.map_or(0, |b| self.resident_bytes.saturating_sub(b))
    }
}

impl fmt::Display for ResidencyStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mib = |b: u64| b as f64 / (1024.0 * 1024.0);
        write!(f, "{} resident ({:.2} MiB", self.resident, mib(self.resident_bytes))?;
        match self.budget {
            Some(b) => write!(f, " of {:.2} MiB", mib(b))?,
            None => write!(f, ", no budget")?,
        }
        write!(
            f,
            "), {} pinned, {} loads, {} reloads, {} evictions ({:.2} MiB)",
            self.pinned,
            self.loads,
            self.reloads,
            self.evictions,
            mib(self.evicted_bytes)
        )?;
        let over = self.over_budget_bytes();
        if over > 0 {
            write!(f, ", {:.2} MiB over budget", mib(over))?;
        }
        Ok(())
    }
}

/// One resident asset.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    /// The asset's handle id (a mesh id).
    pub handle: u32,
    /// Where it was loaded from, as the load cache knows it.
    pub file: crate::assets::Origin,
    pub bytes: u64,
    /// When it was last asked for: a counter, so no two uses tie.
    last_use: u64,
    /// The frame and tick it was last asked for in: it stays resident for
    /// the rest of them, so handles given out then stay good.
    used_in: (u64, u64),
    pub pinned: bool,
    /// Loaded by the game directly before residency took it: the game holds
    /// the handle, so it is never evicted.
    pub shared: bool,
}

impl Entry {
    fn kept(&self) -> bool {
        self.pinned || self.shared
    }
}

/// The resident set of one kind.
#[derive(Default)]
pub(crate) struct ResidentSet {
    budget: Option<u64>,
    /// By name, so iteration (and so eviction among equals) is in a fixed order.
    entries: BTreeMap<String, Entry>,
    /// Names loaded at least once, so loading them again counts as a reload.
    seen: std::collections::BTreeSet<String>,
    resident_bytes: u64,
    loads: u64,
    reloads: u64,
    evictions: u64,
    evicted_bytes: u64,
    uses: u64,
}

impl ResidentSet {
    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.entries.get(name)
    }

    /// Marks `name` used now, in frame and tick `now`.
    pub fn touch(&mut self, name: &str, now: (u64, u64)) {
        self.uses += 1;
        if let Some(e) = self.entries.get_mut(name) {
            e.last_use = self.uses;
            e.used_in = now;
        }
    }

    /// Adds `name` as just loaded and used in `now`.
    pub fn insert(
        &mut self,
        name: &str,
        (handle, file): (u32, crate::assets::Origin),
        bytes: u64,
        shared: bool,
        now: (u64, u64),
    ) {
        if self.seen.contains(name) {
            self.reloads += 1;
        } else {
            self.seen.insert(name.to_string());
            self.loads += 1;
        }
        self.uses += 1;
        let entry = Entry {
            handle,
            file,
            bytes,
            last_use: self.uses,
            used_in: now,
            pinned: false,
            shared,
        };
        if let Some(old) = self.entries.insert(name.to_string(), entry) {
            self.resident_bytes -= old.bytes;
        }
        self.resident_bytes += bytes;
    }

    /// Drops `name` without counting an eviction (it was freed elsewhere).
    pub fn forget(&mut self, name: &str) -> Option<Entry> {
        let e = self.entries.remove(name)?;
        self.resident_bytes -= e.bytes;
        Some(e)
    }

    pub fn set_pinned(&mut self, name: &str, pinned: bool) {
        if let Some(e) = self.entries.get_mut(name) {
            e.pinned = pinned;
        }
    }

    pub fn set_budget(&mut self, budget: Option<u64>) {
        self.budget = budget;
    }

    /// Whether anything should be evicted: over budget with something that
    /// could go, ignoring what entities use (the caller finds that out only
    /// when this says yes, since it costs a world scan).
    pub fn wants_eviction(&self, now: (u64, u64)) -> bool {
        self.budget.is_some_and(|b| self.resident_bytes > b)
            && self.entries.values().any(|e| !e.kept() && e.used_in != now)
    }

    /// Removes and returns the assets to evict, least recently used first,
    /// until the set fits its budget or nothing else can go. Never pinned
    /// or shared ones, ones used in `now`, or ones `in_use` says are.
    pub fn evict(&mut self, now: (u64, u64), in_use: impl Fn(u32) -> bool) -> Vec<(String, Entry)> {
        let Some(budget) = self.budget else {
            return Vec::new();
        };
        let mut candidates: Vec<(u64, &String)> = self
            .entries
            .iter()
            .filter(|(_, e)| !e.kept() && e.used_in != now && !in_use(e.handle))
            .map(|(name, e)| (e.last_use, name))
            .collect();
        candidates.sort_unstable();
        let mut bytes = self.resident_bytes;
        let mut names = Vec::new();
        for (_, name) in candidates {
            if bytes <= budget {
                break;
            }
            bytes -= self.entries[name].bytes;
            names.push(name.clone());
        }
        names
            .into_iter()
            .map(|name| {
                let e = self.entries.remove(&name).expect("a candidate");
                self.resident_bytes -= e.bytes;
                self.evictions += 1;
                self.evicted_bytes += e.bytes;
                (name, e)
            })
            .collect()
    }

    pub fn stats(&self) -> ResidencyStats {
        let kept = self.entries.values().filter(|e| e.kept());
        ResidencyStats {
            budget: self.budget,
            resident: self.entries.len(),
            resident_bytes: self.resident_bytes,
            pinned: kept.clone().count(),
            pinned_bytes: kept.map(|e| e.bytes).sum(),
            loads: self.loads,
            reloads: self.reloads,
            evictions: self.evictions,
            evicted_bytes: self.evicted_bytes,
        }
    }
}

/// The resident sets of every kind (a `Ctx` field).
#[derive(Default)]
pub(crate) struct Residency {
    sets: [ResidentSet; ResidencyKind::ALL.len()],
}

impl Residency {
    pub fn set(&self, kind: ResidencyKind) -> &ResidentSet {
        &self.sets[kind.index()]
    }

    pub fn set_mut(&mut self, kind: ResidencyKind) -> &mut ResidentSet {
        &mut self.sets[kind.index()]
    }
}

impl crate::Ctx {
    /// Sets the byte budget for resident assets of `kind`; `None` (the
    /// default) for no limit. Lowering it evicts at once what it can.
    ///
    /// The budget counts the engine's estimates of the assets' GPU memory,
    /// and only assets loaded through residency (such as
    /// [`resident_mesh`](Self::resident_mesh)). It can be exceeded: assets
    /// that are pinned, shown by entities or asked for in the current tick
    /// are never evicted, and [`ResidencyStats::over_budget_bytes`] says by
    /// how much.
    pub fn set_residency_budget(&mut self, kind: ResidencyKind, bytes: Option<u64>) {
        self.residency.set_mut(kind).set_budget(bytes);
        self.enforce_residency(kind);
    }

    /// The budget, usage and eviction counters of `kind`.
    #[must_use]
    pub fn residency(&self, kind: ResidencyKind) -> ResidencyStats {
        self.residency.set(kind).stats()
    }

    /// Whether the asset of `kind` called `name` is resident (loaded through
    /// residency, and not evicted or freed since).
    #[must_use]
    pub fn is_resident(&self, kind: ResidencyKind, name: &str) -> bool {
        match kind {
            #[cfg(feature = "3d")]
            ResidencyKind::Mesh => self.live_resident_mesh(name).is_some(),
            #[cfg(not(feature = "3d"))]
            ResidencyKind::Mesh => false,
        }
    }

    /// The frame and tick now, which residency protects assets asked for in.
    pub(crate) fn residency_now(&self) -> (u64, u64) {
        (self.time.frame(), self.time.sim_tick())
    }

    /// Evicts what `kind`'s budget needs gone.
    pub(crate) fn enforce_residency(&mut self, kind: ResidencyKind) {
        let now = self.residency_now();
        if !self.residency.set(kind).wants_eviction(now) {
            return;
        }
        match kind {
            #[cfg(feature = "3d")]
            ResidencyKind::Mesh => self.evict_meshes(now),
            #[cfg(not(feature = "3d"))]
            ResidencyKind::Mesh => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Origin;

    const NOW: (u64, u64) = (0, 10);

    fn set(budget: u64, names: &[(&str, u64)]) -> ResidentSet {
        let mut s = ResidentSet::default();
        s.set_budget(Some(budget));
        for (i, &(name, bytes)) in names.iter().enumerate() {
            s.insert(name, (i as u32, Origin::File(name.into())), bytes, false, (0, i as u64));
        }
        s
    }

    #[test]
    fn evicts_least_recently_used_until_it_fits() {
        let mut s = set(250, &[("a", 100), ("b", 100), ("c", 100), ("d", 100)]);
        s.touch("a", (0, 5));
        let gone: Vec<String> = s.evict(NOW, |_| false).into_iter().map(|(n, _)| n).collect();
        assert_eq!(gone, ["b", "c"]);
        let st = s.stats();
        assert_eq!((st.resident_bytes, st.evictions, st.evicted_bytes), (200, 2, 200));
    }

    #[test]
    fn skips_pinned_shared_in_use_and_current() {
        let mut s = set(0, &[("a", 1), ("b", 1), ("c", 1), ("d", 1), ("e", 1)]);
        s.set_pinned("a", true);
        s.insert("b", (1, Origin::File("b".into())), 1, true, (0, 1));
        s.touch("d", NOW);
        let gone: Vec<String> = s.evict(NOW, |h| h == 2).into_iter().map(|(n, _)| n).collect();
        assert_eq!(gone, ["e"]);
        assert_eq!(s.stats().over_budget_bytes(), 4);
        assert_eq!(s.stats().pinned, 2);
    }

    #[test]
    fn no_budget_evicts_nothing() {
        let mut s = set(0, &[("a", 1)]);
        s.set_budget(None);
        assert!(!s.wants_eviction(NOW));
        assert!(s.evict(NOW, |_| false).is_empty());
    }

    #[test]
    fn loading_a_name_again_counts_as_a_reload() {
        let mut s = set(10, &[("a", 4)]);
        s.forget("a");
        s.insert("a", (0, Origin::File("a".into())), 4, false, NOW);
        let st = s.stats();
        assert_eq!((st.loads, st.reloads, st.resident_bytes), (1, 1, 4));
    }

    #[test]
    fn report_says_when_over_budget() {
        let s = set(1, &[("a", 3)]);
        let text = s.stats().to_string();
        assert!(text.contains("1 resident") && text.contains("over budget"), "{text}");
        assert!(!set(5, &[("a", 3)]).stats().to_string().contains("over"));
    }
}
