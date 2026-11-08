//! Deferred commands.

mod common;

use common::*;
use ponko_ecs::*;

#[test]
fn commands_apply_in_order() {
    let mut w = World::new();
    let es: Vec<_> = (0..10).map(|i| w.spawn((Pos(i as f32, 0.0),))).collect();
    let mut cmd = Commands::new();
    w.query::<(Entity, &Pos)>().for_each(|(e, p)| {
        if p.0 < 5.0 {
            cmd.despawn(e);
        } else {
            cmd.insert(e, Tag);
        }
    });
    cmd.spawn((Pos(100.0, 0.0),));
    cmd.remove::<Tag>(es[9]);
    cmd.add(|w| w.insert_resource(42u32));
    assert_eq!(cmd.len(), 13);
    w.apply(&mut cmd);
    assert!(cmd.is_empty());
    assert_eq!(w.len(), 6);
    assert_eq!(w.query::<&Tag>().count(), 4);
    assert_eq!(*w.resource::<u32>(), 42);
}
