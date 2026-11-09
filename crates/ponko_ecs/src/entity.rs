//! Entity ids and the allocator that hands them out.

use std::fmt;

/// A handle to a thing in the [`World`](crate::World). Cheap to copy and store.
///
/// Handles are generational: after an entity is despawned, its old handle stays
/// dead even if the slot is reused, so `world.get(old)` returns `None` instead
/// of someone else's data.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)] // C/C++ read entity arrays directly: `{ uint32_t index, generation; }`
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl Entity {
    /// Packs the handle into 64 bits (for saving, or passing to C/C++).
    #[must_use]
    pub fn to_bits(self) -> u64 {
        (self.generation as u64) << 32 | self.index as u64
    }

    /// Inverse of [`to_bits`](Self::to_bits).
    #[must_use]
    pub fn from_bits(bits: u64) -> Self {
        Entity {
            index: bits as u32,
            generation: (bits >> 32) as u32,
        }
    }

    /// Slot number. Slots are reused after despawn; use the whole `Entity` as an id.
    #[must_use]
    pub fn index(self) -> u32 {
        self.index
    }
}

/// Saved as its 64-bit id ([`Entity::to_bits`]), so components that refer
/// to entities can be saved and loaded (with the `serde` feature).
///
/// ```
/// use ponko_ecs::Entity;
///
/// let e = Entity::from_bits(5 | 2 << 32);
/// let json = serde_json::to_string(&e).unwrap();
/// assert_eq!(json, (5u64 | 2 << 32).to_string());
/// assert_eq!(serde_json::from_str::<Entity>(&json).unwrap(), e);
/// ```
#[cfg(feature = "serde")]
impl serde::Serialize for Entity {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(self.to_bits())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Entity {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        u64::deserialize(d).map(Entity::from_bits)
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Entity({}v{})", self.index, self.generation)
    }
}

#[derive(Copy, Clone)]
pub(crate) struct Location {
    pub(crate) archetype: u32,
    pub(crate) row: u32,
}

#[derive(Copy, Clone)]
struct Slot {
    generation: u32,
    /// `archetype == DEAD` means the slot is free.
    loc: Location,
}

const DEAD: u32 = u32::MAX;

#[derive(Default)]
pub(crate) struct Entities {
    slots: Vec<Slot>,
    free: Vec<u32>,
    alive: usize,
}

impl Entities {
    pub(crate) fn alloc(&mut self) -> Entity {
        // The free list can hold slots that `alloc_at` claimed since; skip those.
        while let Some(index) = self.free.pop() {
            let slot = &self.slots[index as usize];
            if slot.loc.archetype == DEAD {
                self.alive += 1;
                return Entity {
                    index,
                    generation: slot.generation,
                };
            }
        }
        self.alive += 1;
        {
            let index = u32::try_from(self.slots.len())
                .ok()
                .filter(|&i| i != u32::MAX)
                .expect("too many entities");
            self.slots.push(Slot {
                generation: 0,
                loc: Location {
                    archetype: DEAD,
                    row: 0,
                },
            });
            Entity { index, generation: 0 }
        }
    }

    /// Claims exactly `e` (for restoring saved worlds). Fails if its slot is
    /// in use, or if `e` is older than the slot's generation (a handle that
    /// was despawned here must stay dead). The caller must then place the
    /// entity with [`set`](Self::set) before anything reads it.
    pub(crate) fn alloc_at(&mut self, e: Entity) -> bool {
        let index = e.index as usize;
        if e.index == u32::MAX {
            return false;
        }
        while self.slots.len() <= index {
            let i = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 0,
                loc: Location {
                    archetype: DEAD,
                    row: 0,
                },
            });
            if i != e.index {
                self.free.push(i);
            }
        }
        let slot = &mut self.slots[index];
        if slot.loc.archetype != DEAD || e.generation < slot.generation {
            return false;
        }
        // Mark it taken right away; `set` fills in the real location.
        slot.generation = e.generation;
        slot.loc = Location { archetype: 0, row: 0 };
        self.alive += 1;
        true
    }

    pub(crate) fn free(&mut self, e: Entity) {
        let slot = &mut self.slots[e.index as usize];
        slot.loc.archetype = DEAD;
        self.alive -= 1;
        // A slot whose generation would wrap is retired, so old handles can never come back to life.
        if let Some(g) = slot.generation.checked_add(1) {
            slot.generation = g;
            self.free.push(e.index);
        }
    }

    #[inline]
    pub(crate) fn get(&self, e: Entity) -> Option<Location> {
        let s = self.slots.get(e.index as usize)?;
        (s.generation == e.generation && s.loc.archetype != DEAD).then_some(s.loc)
    }

    #[inline]
    pub(crate) fn set(&mut self, e: Entity, loc: Location) {
        self.slots[e.index as usize].loc = loc;
    }

    #[inline]
    pub(crate) fn set_row(&mut self, e: Entity, row: usize) {
        self.slots[e.index as usize].loc.row = row as u32;
    }

    pub(crate) fn len(&self) -> usize {
        self.alive
    }
}
