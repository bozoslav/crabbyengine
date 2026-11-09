//! Commands: changes recorded now, applied to the world later.

use std::mem::{MaybeUninit, size_of};
use std::ptr;

use crate::bundle::{Bundle, Component};
use crate::entity::Entity;
use crate::world::World;

/// One recorded command: where its closure sits in `Commands::bytes`, and the
/// function that reads it back out and runs it (`Some(world)`) or drops it
/// (`None`).
struct Op {
    call: unsafe fn(*mut MaybeUninit<u8>, Option<&mut World>),
    offset: usize,
}

/// Reads the `F` stored at `p` and runs it on `world`, or drops it.
///
/// # Safety
///
/// `p` must point at an initialized `F` written by `Commands::push`, and that
/// `F` must not be read again afterwards (it has been moved out).
unsafe fn call<F: FnOnce(&mut World)>(p: *mut MaybeUninit<u8>, world: Option<&mut World>) {
    // The byte buffer has no alignment, so the value is copied out unaligned.
    let f = unsafe { ptr::read_unaligned(p.cast::<F>()) };
    match world {
        Some(world) => f(world),
        None => drop(f),
    }
}

/// A list of world changes to apply later. Use it to spawn or despawn while a
/// query is running, since the query holds the world.
///
/// ```
/// # use ponko_ecs::*;
/// struct Health(i32);
///
/// let mut world = World::new();
/// world.spawn((Health(0),));
/// world.spawn((Health(5),));
///
/// let mut commands = Commands::new();
/// world.query::<(Entity, &Health)>().for_each(|(e, h)| {
///     if h.0 <= 0 {
///         commands.despawn(e);
///     }
/// });
/// world.apply(&mut commands);
/// assert_eq!(world.len(), 1);
/// ```
///
/// Commands run in the order they were recorded. Keep one `Commands` around
/// and reuse it: applying empties it without freeing its memory.
///
/// Recording doesn't allocate per command: closures are packed into one byte
/// buffer that is reused across applies.
#[derive(Default)]
pub struct Commands {
    ops: Vec<Op>,
    /// The recorded closures, packed back to back, unaligned.
    bytes: Vec<MaybeUninit<u8>>,
}

impl Commands {
    /// An empty command list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawns an entity with these components when applied.
    pub fn spawn<B: Bundle>(&mut self, bundle: B) {
        self.push(move |w| {
            w.spawn(bundle);
        });
    }

    /// Despawns `e` when applied (no-op if it's already gone by then).
    pub fn despawn(&mut self, e: Entity) {
        self.push(move |w| {
            w.despawn(e);
        });
    }

    /// Adds or replaces component `value` on `e` when applied.
    pub fn insert<T: Component>(&mut self, e: Entity, value: T) {
        self.push(move |w| {
            w.insert(e, value);
        });
    }

    /// Removes component `T` from `e` when applied.
    pub fn remove<T: Component>(&mut self, e: Entity) {
        self.push(move |w| {
            w.remove::<T>(e);
        });
    }

    /// Runs any code with full world access when applied.
    pub fn add(&mut self, f: impl FnOnce(&mut World) + Send + 'static) {
        self.push(f);
    }

    /// Moves `f` into the byte buffer and records how to run it.
    fn push<F: FnOnce(&mut World) + Send + 'static>(&mut self, f: F) {
        // Reserve both first, so nothing can fail between writing `f` and
        // recording it.
        self.ops.reserve(1);
        let offset = self.bytes.len();
        self.bytes.reserve(size_of::<F>());
        // SAFETY: the reserve above makes `offset..offset + size_of::<F>()`
        // in bounds of the allocation; the write is unaligned because the
        // buffer is bytes. Those bytes are now initialized, so the new length
        // covers only written memory. `Op::call` reads `F` back exactly once.
        unsafe {
            ptr::write_unaligned(self.bytes.as_mut_ptr().add(offset).cast::<F>(), f);
            self.bytes.set_len(offset + size_of::<F>());
        }
        self.ops.push(Op {
            call: call::<F>,
            offset,
        });
    }

    /// Number of recorded commands.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// `true` if nothing is recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub(crate) fn apply(&mut self, world: &mut World) {
        /// Drops the commands not yet run and empties the buffers, also when a
        /// command panics part way through.
        struct Rest<'a> {
            commands: &'a mut Commands,
            next: usize,
        }
        impl Drop for Rest<'_> {
            fn drop(&mut self) {
                let c = &mut *self.commands;
                let base = c.bytes.as_mut_ptr();
                for op in &c.ops[self.next..] {
                    // SAFETY: ops from `next` on were never read.
                    unsafe { (op.call)(base.add(op.offset), None) };
                }
                c.ops.clear();
                c.bytes.clear();
            }
        }

        let mut rest = Rest {
            commands: self,
            next: 0,
        };
        while let Some(op) = rest.commands.ops.get(rest.next) {
            let (run, offset) = (op.call, op.offset);
            // Count it as taken before it runs: if it panics, `Rest` must not
            // drop it again.
            rest.next += 1;
            // SAFETY: each op is read once, in order; the buffer isn't touched
            // while a command runs (it only gets the world).
            unsafe { run(rest.commands.bytes.as_mut_ptr().add(offset), Some(world)) };
        }
    }
}

impl Drop for Commands {
    fn drop(&mut self) {
        let base = self.bytes.as_mut_ptr();
        for op in &self.ops {
            // SAFETY: recorded commands that were never applied; each read once.
            unsafe { (op.call)(base.add(op.offset), None) };
        }
    }
}
