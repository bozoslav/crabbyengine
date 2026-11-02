//! Entity ids and the allocator that hands them out.

use std::fmt;

/// A handle to a thing in the [`World`](crate::World). Cheap to copy and store.
///
/// Handles are generational: after an entity is despawned, its old handle stays
/// dead even if the slot is reused, so `world.get(old)` returns `None` instead
/// of someone else's data.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl Entity {
    /// Packs the handle into 64 bits (for saving, or passing to C/C++).
    pub fn to_bits(self) -> u64 {
        (self.generation as u64) << 32 | self.index as u64
    }

    /// Inverse of [`to_bits`](Self::to_bits).
    pub fn from_bits(bits: u64) -> Self {
        Entity { index: bits as u32, generation: (bits >> 32) as u32 }
    }

    /// Slot number. Slots are reused after despawn; use the whole `Entity` as an id.
    pub fn index(self) -> u32 {
        self.index
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
        self.alive += 1;
        if let Some(index) = self.free.pop() {
            Entity { index, generation: self.slots[index as usize].generation }
        } else {
            let index = u32::try_from(self.slots.len()).ok().filter(|&i| i != u32::MAX).expect("too many entities");
            self.slots.push(Slot { generation: 0, loc: Location { archetype: DEAD, row: 0 } });
            Entity { index, generation: 0 }
        }
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
