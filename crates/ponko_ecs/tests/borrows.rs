//! Borrowing several things at once: resources together, resources next to
//! a query, several entities, and other entities while a query loops.

mod common;
use common::*;
use ponko_ecs::*;

struct Speed(f32);
struct Score(u32);
struct Log(Vec<String>);

struct Health(i32);
struct Target(Entity);
struct Damage(i32);

// ---- several resources at once ------------------------------------------------

#[test]
fn resources_mut_borrows_different_resources_together() {
    let mut w = World::new();
    w.insert_resource(Score(1));
    w.insert_resource(Log(Vec::new()));
    w.insert_resource(Speed(2.0));
    let (score, log, speed) = w.resources_mut::<(Score, Log, Speed)>();
    score.0 += speed.0 as u32;
    log.0.push(format!("score {}", score.0));
    speed.0 = 0.0;
    assert_eq!(w.resource::<Score>().0, 3);
    assert_eq!(w.resource::<Log>().0, ["score 3"]);
    assert_eq!(w.resource::<Speed>().0, 0.0);
}

#[test]
fn resources_mut_keeps_each_resource_in_place() {
    let mut w = World::new();
    w.insert_resource(Score(1));
    w.insert_resource(Log(Vec::new()));
    let before = (w.resource::<Score>() as *const Score, w.resource::<Log>() as *const Log);
    let (score, log) = w.resources_mut::<(Score, Log)>();
    assert_eq!((score as *const Score, log as *const Log), before);
    let (one,) = w.resources_mut::<(Score,)>();
    assert_eq!(one as *const Score, before.0);
}

#[test]
#[should_panic(expected = "is named twice")]
fn resources_mut_rejects_the_same_resource_twice() {
    let mut w = World::new();
    w.insert_resource(Score(1));
    let _ = w.resources_mut::<(Score, Score)>();
}

#[test]
#[should_panic(expected = "insert_resource")]
fn resources_mut_names_the_missing_resource() {
    let mut w = World::new();
    w.insert_resource(Score(1));
    let _ = w.resources_mut::<(Score, Log)>();
}

// ---- resources next to a query ---------------------------------------------------

#[test]
fn a_query_reads_and_writes_resources_in_its_loop() {
    let mut w = World::new();
    w.insert_resource(Speed(2.0));
    w.insert_resource(Score(0));
    w.spawn((Pos(1.0, 0.0),));
    w.spawn((Pos(2.0, 0.0), Frozen));
    w.spawn((Pos(3.0, 0.0),));
    w.query::<&mut Pos>()
        .without::<Frozen>()
        .with_resources::<(&Speed, &mut Score)>()
        .for_each(|p, (speed, score)| {
            p.0 *= speed.0;
            score.0 += 1;
        });
    assert_eq!(w.resource::<Score>().0, 2);
    let mut xs: Vec<f32> = w.query::<&Pos>().iter().map(|p| p.0).collect();
    xs.sort_by(f32::total_cmp);
    assert_eq!(xs, [2.0, 2.0, 6.0]);
}

#[test]
fn a_single_resource_needs_no_tuple() {
    let mut w = World::new();
    w.insert_resource(Score(0));
    w.spawn_batch((0..5).map(|i| (Pos(i as f32, 0.0),)));
    w.query::<&Pos>()
        .with_resources::<&mut Score>()
        .for_each(|p, score| score.0 += p.0 as u32);
    assert_eq!(w.resource::<Score>().0, 10);
}

#[test]
fn carried_resources_are_back_after_the_query_and_keep_their_place() {
    let mut w = World::new();
    w.insert_resource(Log(vec!["start".into()]));
    w.insert_resource(Score(7));
    w.spawn((Pos(0.0, 0.0),));
    let before = w.resource::<Log>() as *const Log;
    {
        let mut q = w.query::<&Pos>().with_resources::<(&mut Log, &Score)>();
        q.for_each(|_, (log, score)| log.0.push(format!("{}", score.0)));
        // Work before or after the loop uses the same borrow.
        let (log, _) = q.resources();
        log.0.push("end".into());
    }
    assert_eq!(w.resource::<Log>().0, ["start", "7", "end"]);
    assert_eq!(w.resource::<Log>() as *const Log, before);
    assert_eq!(w.resource::<Score>().0, 7);
}

#[test]
fn carried_resources_are_back_after_a_panic_in_the_loop() {
    let mut w = World::new();
    w.insert_resource(Score(1));
    w.spawn((Pos(0.0, 0.0),));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        w.query::<&Pos>().with_resources::<&mut Score>().for_each(|_, score| {
            score.0 = 5;
            panic!("stop");
        });
    }));
    assert!(caught.is_err());
    assert_eq!(w.resource::<Score>().0, 5);
}

#[test]
fn an_unused_query_with_resources_still_returns_them() {
    let mut w = World::new();
    w.insert_resource(Score(3));
    let _ = w.query::<&Pos>().with_resources::<&Score>();
    assert_eq!(w.resource::<Score>().0, 3);
}

#[test]
#[should_panic(expected = "is named twice")]
fn a_query_rejects_the_same_resource_twice() {
    let mut w = World::new();
    w.insert_resource(Score(1));
    let _ = w.query::<&Pos>().with_resources::<(&Score, &mut Score)>();
}

#[test]
fn a_missing_resource_panics_before_any_is_taken() {
    let mut w = World::new();
    w.insert_resource(Score(1));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = w.query::<&Pos>().with_resources::<(&mut Score, &Log)>();
    }));
    let message = *caught.unwrap_err().downcast::<String>().unwrap();
    assert!(
        message.contains("Log") && message.contains("insert_resource"),
        "{message}"
    );
    assert_eq!(w.resource::<Score>().0, 1);
}

#[test]
fn carried_resources_are_recorded_as_read_or_written() {
    let mut w = World::new();
    w.insert_resource(Speed(1.0));
    w.insert_resource(Score(0));
    w.record_access();
    w.query::<&Pos>()
        .with_resources::<(&Speed, &mut Score)>()
        .for_each(|_, _| {});
    // Recording exists in debug builds only.
    if let Some(log) = w.take_access_log() {
        let mut written: Vec<bool> = log.resources.iter().map(|r| r.2).collect();
        written.sort();
        assert_eq!(written, [false, true]);
    }
}

// ---- several entities at once ---------------------------------------------------

#[test]
fn get_many_mut_borrows_different_entities_together() {
    let mut w = World::new();
    let a = w.spawn((Health(10),));
    let b = w.spawn((Health(4), Tag)); // another archetype
    let c = w.spawn((Health(1),));
    let [x, y, z] = w.get_many_mut::<Health, 3>([a, b, c]).unwrap();
    x.0 -= y.0;
    y.0 += z.0;
    z.0 = 0;
    let all = [a, b, c].map(|e| w.get::<Health>(e).unwrap().0);
    assert_eq!(all, [6, 5, 0]);
}

#[test]
fn get_many_mut_refuses_the_same_entity_twice() {
    let mut w = World::new();
    let a = w.spawn((Health(10),));
    let b = w.spawn((Health(4),));
    assert!(w.get_many_mut::<Health, 2>([a, a]).is_none());
    assert!(w.get_many_mut::<Health, 3>([a, b, a]).is_none());
    assert!(w.get_many_mut::<Health, 2>([a, b]).is_some());
}

#[test]
fn get_many_mut_needs_every_entity_alive_and_with_the_component() {
    let mut w = World::new();
    let a = w.spawn((Health(10),));
    let bare = w.spawn((Tag,));
    let dead = w.spawn((Health(1),));
    w.despawn(dead);
    assert!(w.get_many_mut::<Health, 2>([a, bare]).is_none());
    assert!(w.get_many_mut::<Health, 2>([dead, a]).is_none());
    assert!(w.get_many_mut::<Pos, 1>([a]).is_none()); // never registered
    assert!(w.get_many_mut::<Health, 0>([]).is_some());
}

// ---- other entities while a query loops -------------------------------------------

#[test]
fn a_loop_changes_other_entities_through_a_lookup() {
    let mut w = World::new();
    let crab = w.spawn((Health(10),));
    let rock = w.spawn((Health(50), Tag));
    w.spawn((Target(crab), Damage(3)));
    w.spawn((Target(crab), Damage(4)));
    w.spawn((Target(rock), Damage(1)));
    w.query::<(&Target, &Damage)>()
        .with_lookup::<&mut Health>()
        .for_each(|(target, damage), others| {
            if let Some(health) = others.get_mut(target.0) {
                health.0 -= damage.0;
            }
        });
    assert_eq!(w.get::<Health>(crab).unwrap().0, 3);
    assert_eq!(w.get::<Health>(rock).unwrap().0, 49);
}

#[test]
fn a_loop_reads_other_entities_while_it_writes_its_own() {
    let mut w = World::new();
    let leader = w.spawn((Pos(5.0, 1.0),));
    let follower = w.spawn((Pos(0.0, 0.0), Vel(0.0, 0.0), Target(leader)));
    // The query reads `Pos` too: two readers are fine.
    w.query::<(&Pos, &mut Vel, &Target)>()
        .with_lookup::<&Pos>()
        .for_each(|(pos, vel, target), others| {
            let Some(goal) = others.get(target.0) else { return };
            *vel = Vel(goal.0 - pos.0, goal.1 - pos.1);
        });
    assert_eq!(w.get::<Vel>(follower), Some(&Vel(5.0, 1.0)));
}

#[test]
fn a_read_only_lookup_hands_out_several_items_at_once() {
    let mut w = World::new();
    let a = w.spawn((Pos(1.0, 0.0),));
    let b = w.spawn((Pos(4.0, 0.0), Tag));
    w.spawn((Target(a), Health(0)));
    let mut gaps = Vec::new();
    w.query::<&Target>()
        .with_lookup::<(Entity, &Pos, Option<&Tag>)>()
        .for_each(|target, others| {
            let (Some(first), Some(second)) = (others.get(target.0), others.get(b)) else {
                return;
            };
            assert_eq!((first.0, second.0), (a, b));
            assert!(first.2.is_none() && second.2.is_some());
            gaps.push(second.1.0 - first.1.0);
        });
    assert_eq!(gaps, [3.0]);
}

#[test]
fn a_lookup_reaches_the_entity_being_visited() {
    let mut w = World::new();
    let e = w.spawn((Pos(1.0, 2.0), Health(1)));
    w.query::<(Entity, &Pos)>()
        .with_lookup::<&mut Health>()
        .for_each(|(me, pos), others| {
            others.get_mut(me).unwrap().0 += pos.1 as i32;
        });
    assert_eq!(w.get::<Health>(e).unwrap().0, 3);
}

#[test]
fn a_lookup_ignores_the_querys_filters_and_misses_cleanly() {
    let mut w = World::new();
    let frozen = w.spawn((Health(1), Frozen));
    let bare = w.spawn((Tag,));
    let dead = w.spawn((Health(1),));
    w.despawn(dead);
    w.spawn((Target(frozen),));
    let mut seen = 0;
    w.query::<&Target>()
        .without::<Frozen>()
        .with_lookup::<&mut Health>()
        .for_each(|target, others| {
            assert!(others.contains(target.0));
            assert!(others.get_mut(target.0).is_some());
            assert!(!others.contains(bare) && others.get_mut(bare).is_none());
            assert!(!others.contains(dead) && others.get_mut(dead).is_none());
            seen += 1;
        });
    assert_eq!(seen, 1);
}

#[test]
#[should_panic(expected = "both access")]
fn a_lookup_cannot_read_what_the_query_writes() {
    let mut w = World::new();
    let _ = w.query::<&mut Pos>().with_lookup::<&Pos>();
}

#[test]
#[should_panic(expected = "both access")]
fn a_lookup_cannot_write_what_the_query_reads() {
    let mut w = World::new();
    let _ = w.query::<(&Vel, Option<&Pos>)>().with_lookup::<(&Vel, &mut Pos)>();
}

#[test]
#[should_panic(expected = "twice, once mutably")]
fn a_lookup_cannot_alias_itself() {
    let mut w = World::new();
    let _ = w.query::<&Vel>().with_lookup::<(&mut Pos, &Pos)>();
}

#[test]
fn a_lookup_is_recorded_like_a_query() {
    let mut w = World::new();
    w.spawn((Pos(0.0, 0.0), Vel(0.0, 0.0)));
    w.record_access();
    w.query::<&Pos>().with_lookup::<&mut Vel>().for_each(|_, _| {});
    if let Some(log) = w.take_access_log() {
        let pos = w.component_id::<Pos>().unwrap();
        let vel = w.component_id::<Vel>().unwrap();
        assert!(log.components.contains(&(pos, false)));
        assert!(log.components.contains(&(vel, true)));
    }
}

#[test]
fn a_lookup_works_across_many_archetypes_with_a_cached_query() {
    // More than 128 archetypes, so the cached query takes the cache path
    // while the lookup reads the same archetype list.
    struct Marker<const N: usize>;
    let mut w = World::new();
    let hub = w.spawn((Health(0),));
    for bits in 0..256u32 {
        let e = w.spawn((Target(hub), Damage(1)));
        macro_rules! mark {
            ($($n:literal)*) => {$(
                if bits & (1 << $n) != 0 {
                    w.insert(e, Marker::<$n>);
                }
            )*};
        }
        mark!(0 1 2 3 4 5 6 7);
    }
    for _ in 0..2 {
        w.query::<(&Target, &Damage)>()
            .cached()
            .with_lookup::<&mut Health>()
            .for_each(|(target, damage), others| {
                others.get_mut(target.0).unwrap().0 += damage.0;
            });
    }
    assert_eq!(w.get::<Health>(hub).unwrap().0, 512);
}

// ---- lookups and change detection ---------------------------------------------

/// Entities whose `Health` changed after `since`, in handle order.
fn changed_health(w: &mut World, since: Tick) -> Vec<Entity> {
    let mut out: Vec<Entity> = w
        .query::<Entity>()
        .filter::<Changed<Health>>()
        .since(since)
        .iter()
        .collect();
    out.sort_by_key(|e| e.to_bits());
    out
}

#[test]
fn writes_through_a_lookup_count_as_changes() {
    let mut w = World::new();
    w.track_changes::<Health>();
    let hit = w.spawn((Health(10),));
    let spared = w.spawn((Health(10),));
    w.spawn((Target(hit), Damage(3)));
    let seen = w.advance_change_tick();
    assert!(changed_health(&mut w, seen).is_empty());

    w.query::<(&Target, &Damage)>()
        .with_lookup::<&mut Health>()
        .for_each(|(target, damage), others| {
            if let Some(health) = others.get_mut(target.0) {
                health.0 -= damage.0;
            }
        });
    assert_eq!(w.get::<Health>(hit).unwrap().0, 7);
    assert_eq!(
        changed_health(&mut w, seen),
        vec![hit],
        "only the entity written through the lookup"
    );
    assert!(!changed_health(&mut w, seen).contains(&spared));

    // Reading through a lookup changes nothing, as with `World::get`.
    let seen = w.advance_change_tick();
    w.query::<&Target>()
        .with_lookup::<&Health>()
        .for_each(|target, others| {
            assert_eq!(others.get(target.0).map(|h| h.0), Some(7));
        });
    assert!(changed_health(&mut w, seen).is_empty());
}

#[test]
fn a_lookup_writing_an_untracked_component_still_works() {
    let mut w = World::new();
    w.track_changes::<Damage>(); // the world tracks something, but not Health
    let hit = w.spawn((Health(10),));
    w.spawn((Target(hit), Damage(4)));
    w.query::<(&Target, &Damage)>()
        .with_lookup::<&mut Health>()
        .for_each(|(target, damage), others| others.get_mut(target.0).unwrap().0 -= damage.0);
    assert_eq!(w.get::<Health>(hit).unwrap().0, 6);
}

#[test]
#[should_panic(expected = "which the query's `Added`/`Changed` filter reads")]
fn a_lookup_cannot_write_what_the_querys_changed_filter_reads() {
    let mut w = World::new();
    w.track_changes::<Health>();
    let other = w.spawn((Health(2),));
    w.spawn((Health(1), Target(other)));
    w.query::<&Target>()
        .filter::<Changed<Health>>()
        .with_lookup::<&mut Health>()
        .for_each(|_, _| {});
}

#[test]
#[should_panic(expected = "which the query's `Added`/`Changed` filter reads")]
fn a_lookup_cannot_write_what_an_added_filter_inside_an_or_reads() {
    let mut w = World::new();
    w.track_changes::<Health>();
    w.query::<&Target>()
        .filter::<Or<(Added<Health>, With<Damage>)>>()
        .with_lookup::<&mut Health>()
        .for_each(|_, _| {});
}

#[test]
fn a_lookup_may_read_what_the_querys_filter_reads() {
    let mut w = World::new();
    w.track_changes::<Health>();
    let a = w.spawn((Health(5),));
    w.get_mut::<Health>(a).unwrap();
    let seen = w.advance_change_tick();
    w.get_mut::<Health>(a).unwrap().0 = 6;
    let mut read = Vec::new();
    w.query::<Entity>()
        .filter::<Changed<Health>>()
        .since(seen)
        .with_lookup::<&Health>()
        .for_each(|e, others| read.push(others.get(e).unwrap().0));
    assert_eq!(read, vec![6]);
}
