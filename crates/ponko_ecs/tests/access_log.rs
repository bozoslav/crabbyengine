//! `World::record_access` / `take_access_log`: what a world reports handing out.
//! Recording exists only with debug assertions, so these tests skip otherwise.
#![cfg(debug_assertions)]

use ponko_ecs::*;

struct Pos(f32);
struct Vel(f32);
struct Score(u32);

fn world() -> World {
    let mut w = World::new();
    let e = w.spawn((Pos(0.0), Vel(1.0)));
    w.insert_resource(Score(0));
    // Touch everything once so ids exist.
    let _ = w.get::<Pos>(e);
    w
}

#[test]
fn not_recording_gives_no_log() {
    let mut w = world();
    w.query::<&mut Pos>().for_each(|p| p.0 += 1.0);
    assert_eq!(w.take_access_log(), None);
}

#[test]
fn queries_record_each_component_with_its_mutability() {
    let mut w = world();
    w.record_access();
    w.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| p.0 += v.0);
    let log = w.take_access_log().unwrap();
    let (pos, vel) = (w.component_id::<Pos>().unwrap(), w.component_id::<Vel>().unwrap());
    assert_eq!(log.components, [(pos, true), (vel, false)]);
    assert!(log.resources.is_empty() && !log.structural);
}

#[test]
fn a_write_covers_an_earlier_read() {
    let mut w = world();
    let e = w.query::<Entity>().iter().next().unwrap();
    w.record_access();
    let _ = w.get::<Pos>(e);
    w.get_mut::<Pos>(e).unwrap().0 = 2.0;
    let _ = w.get::<Pos>(e);
    let log = w.take_access_log().unwrap();
    assert_eq!(log.components, [(w.component_id::<Pos>().unwrap(), true)]);
}

#[test]
fn resources_record_reads_and_writes() {
    let mut w = world();
    w.record_access();
    let _ = w.resource::<Score>().0;
    let log = w.take_access_log().unwrap();
    assert_eq!(log.resources.len(), 1);
    assert!(!log.resources[0].2);
    assert!(log.resources[0].1.ends_with("Score"));

    w.record_access();
    w.resource_scope::<Score, _>(|_, s| s.0 += 1);
    assert!(w.take_access_log().unwrap().resources[0].2);
}

#[test]
fn spawning_and_despawning_are_structural() {
    let mut w = world();
    w.record_access();
    let e = w.spawn((Pos(1.0),));
    w.despawn(e);
    let log = w.take_access_log().unwrap();
    assert!(log.structural);
}

#[test]
fn raw_access_counts_as_writing() {
    let mut w = world();
    let pos = w.component_id::<Pos>().unwrap();
    w.record_access();
    w.for_each_chunk(&[pos], |_| {});
    assert_eq!(w.take_access_log().unwrap().components, [(pos, true)]);
}

#[test]
fn a_new_recording_starts_empty() {
    let mut w = world();
    w.record_access();
    w.query::<&Pos>().for_each(|_| {});
    w.record_access();
    let log = w.take_access_log().unwrap();
    assert!(log.is_empty());
}

#[test]
fn replacing_a_component_in_place_is_a_write_and_adding_one_is_structural() {
    let mut w = world();
    let e = w.query::<Entity>().iter().next().unwrap();
    let pos = w.component_id::<Pos>().unwrap();
    let fresh = w.register::<u64>();
    w.record_access();
    let value = Pos(5.0);
    // SAFETY: a valid `Pos`, moved into the world (no destructor to skip).
    assert!(unsafe { w.insert_dynamic(e, pos, (&raw const value).cast()) });
    let log = w.take_access_log().unwrap();
    assert_eq!(log.components, [(pos, true)]);
    assert!(!log.structural);
    w.record_access();
    let n = 1u64;
    // SAFETY: a valid u64.
    assert!(unsafe { w.insert_dynamic(e, fresh, (&raw const n).cast()) });
    assert!(w.take_access_log().unwrap().structural);
    w.record_access();
    // SAFETY: a dead entity: nothing is read.
    assert!(!unsafe { w.insert_dynamic(Entity::from_bits(u64::MAX), pos, (&raw const value).cast()) });
    assert!(w.take_access_log().unwrap().structural);
}
