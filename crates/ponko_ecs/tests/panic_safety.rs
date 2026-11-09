//! Recovery after user code unwinds at structural boundaries.
use ponko_ecs::{Commands, World};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Tracked {
    drops: Arc<AtomicUsize>,
    panic: bool,
}
impl Drop for Tracked {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
        assert!(!self.panic, "component drop panic");
    }
}
fn tracked(drops: &Arc<AtomicUsize>, panic: bool) -> Tracked {
    Tracked {
        drops: drops.clone(),
        panic,
    }
}

#[test]
fn despawn_drop_panic_repairs_swapped_row_and_drops_other_columns() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut w = World::new();
    let a = w.spawn((tracked(&drops, true), String::from("a")));
    let b = w.spawn((tracked(&drops, false), String::from("b")));
    assert!(catch_unwind(AssertUnwindSafe(|| w.despawn(a))).is_err());
    assert!(!w.is_alive(a));
    assert_eq!(w.query::<&Tracked>().count(), 1);
    assert_eq!(w.get::<String>(b).unwrap(), "b");
    assert!(w.despawn(b));
    drop(w);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn replacement_drop_panic_keeps_new_value_and_other_components() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut w = World::new();
    let a = w.spawn((tracked(&drops, true), 12usize));
    assert!(catch_unwind(AssertUnwindSafe(|| w.insert(a, tracked(&drops, false)))).is_err());
    assert!(!w.get::<Tracked>(a).unwrap().panic);
    assert_eq!(w.get::<usize>(a), Some(&12));
    drop(w);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn removed_value_drop_panic_preserves_archetype_move_and_command_cleanup() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut w = World::new();
    let a = w.spawn((tracked(&drops, true), 1usize));
    let b = w.spawn((tracked(&drops, false), 2usize));
    let mut commands = Commands::new();
    commands.remove::<Tracked>(a);
    commands.spawn((tracked(&drops, false),));
    assert!(catch_unwind(AssertUnwindSafe(|| w.apply(&mut commands))).is_err());
    assert!(commands.is_empty());
    assert!(w.get::<Tracked>(a).is_none());
    assert_eq!(w.get::<usize>(a), Some(&1));
    assert_eq!(w.get::<usize>(b), Some(&2));
    assert!(w.insert(a, String::from("moved")));
    assert_eq!(w.remove::<String>(a).as_deref(), Some("moved"));
    commands.spawn((3usize,));
    w.apply(&mut commands);
    assert_eq!(w.query::<&usize>().count(), 3);
    drop(w);
    assert_eq!(drops.load(Ordering::SeqCst), 3);
}

#[test]
fn spawn_iterator_panic_preserves_completed_entities() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut w = World::new();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            w.spawn_batch((0..4).map(|i| {
                assert_ne!(i, 2, "iterator panic");
                (tracked(&drops, false), i)
            }));
        }))
        .is_err()
    );
    assert_eq!(w.query::<&Tracked>().count(), 2);
    let a = w.spawn((9usize,));
    assert!(w.despawn(a));
    drop(w);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn dynamic_replacement_and_removal_publish_before_drop_panics() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut w = World::new();
    let id = w.register::<Tracked>();
    let a = w.spawn((tracked(&drops, true), String::from("kept")));
    let replacement = std::mem::ManuallyDrop::new(tracked(&drops, true));
    assert!(
        catch_unwind(AssertUnwindSafe(|| unsafe {
            // Ownership transfers even if dropping the previous value panics.
            w.insert_dynamic(a, id, (&*replacement as *const Tracked).cast())
        }))
        .is_err()
    );
    assert_eq!(w.get::<String>(a).unwrap(), "kept");
    assert!(catch_unwind(AssertUnwindSafe(|| w.remove_dynamic(a, id))).is_err());
    assert!(w.get::<Tracked>(a).is_none());
    assert_eq!(w.get::<String>(a).unwrap(), "kept");
    drop(w);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn rejected_duplicate_bundle_drops_inputs_without_creating_a_row() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut w = World::new();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            w.spawn((tracked(&drops, false), tracked(&drops, false)));
        }))
        .is_err()
    );
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    assert_eq!(w.query::<&Tracked>().count(), 0);
    let a = w.spawn((tracked(&drops, false),));
    assert!(w.despawn(a));
    assert_eq!(drops.load(Ordering::SeqCst), 3);
}
