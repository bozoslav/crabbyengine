//! Global resources.

use crabby_ecs::*;

#[test]
fn resources() {
    struct Score(u32);
    let mut w = World::new();
    assert!(w.get_resource::<Score>().is_none());
    w.insert_resource(Score(1));
    w.resource_mut::<Score>().0 += 1;
    assert_eq!(w.resource::<Score>().0, 2);
    w.insert_resource(Score(10));
    assert_eq!(w.remove_resource::<Score>().map(|s| s.0), Some(10));
    assert!(w.get_resource::<Score>().is_none());
}

#[test]
#[should_panic(expected = "insert_resource")]
fn missing_resource_panics_helpfully() {
    struct Missing;
    World::new().resource::<Missing>();
}
