//! Commands: changes recorded now, applied to the world later.

use crate::bundle::{Bundle, Component};
use crate::entity::Entity;
use crate::world::World;

enum Op {
    Despawn(Entity),
    Run(Box<dyn FnOnce(&mut World) + Send>),
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
#[derive(Default)]
pub struct Commands {
    ops: Vec<Op>,
}

impl Commands {
    /// An empty command list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawns an entity with these components when applied.
    pub fn spawn<B: Bundle>(&mut self, bundle: B) {
        self.ops.push(Op::Run(Box::new(move |w| {
            w.spawn(bundle);
        })));
    }

    /// Despawns `e` when applied (no-op if it's already gone by then).
    pub fn despawn(&mut self, e: Entity) {
        self.ops.push(Op::Despawn(e));
    }

    /// Adds or replaces component `value` on `e` when applied.
    pub fn insert<T: Component>(&mut self, e: Entity, value: T) {
        self.ops.push(Op::Run(Box::new(move |w| {
            w.insert(e, value);
        })));
    }

    /// Removes component `T` from `e` when applied.
    pub fn remove<T: Component>(&mut self, e: Entity) {
        self.ops.push(Op::Run(Box::new(move |w| {
            w.remove::<T>(e);
        })));
    }

    /// Runs any code with full world access when applied.
    pub fn add(&mut self, f: impl FnOnce(&mut World) + Send + 'static) {
        self.ops.push(Op::Run(Box::new(f)));
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
        for op in self.ops.drain(..) {
            match op {
                Op::Despawn(e) => {
                    world.despawn(e);
                }
                Op::Run(f) => f(world),
            }
        }
    }
}
