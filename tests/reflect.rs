//! Reflection: components read and written by name, as JSON.

use crabbyengine::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Health {
    current: f32,
    max: f32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Team(u8);

/// Not reflected: must never show up in JSON.
struct Secret;

fn world() -> World {
    let mut w = World::new();
    w.reflect::<Transform2D>();
    w.reflect::<Health>();
    w.reflect::<Team>();
    w
}

#[test]
fn components_round_trip_through_json() {
    let mut w = world();
    let e = w.spawn((
        Health {
            current: 3.0,
            max: 10.0,
        },
        Team(2),
    ));
    assert_eq!(w.get_json(e, "Health").unwrap(), json!({ "current": 3.0, "max": 10.0 }));
    assert_eq!(w.get_json(e, "Team").unwrap(), json!(2));
    w.set_json(e, "Team", json!(7)).unwrap();
    assert_eq!(w.get::<Team>(e), Some(&Team(7)));
}

#[test]
fn set_json_on_an_object_changes_only_given_fields() {
    let mut w = world();
    let e = w.spawn((Health {
        current: 3.0,
        max: 10.0,
    },));
    w.set_json(e, "Health", json!({ "current": 9.0 })).unwrap();
    assert_eq!(
        w.get::<Health>(e),
        Some(&Health {
            current: 9.0,
            max: 10.0
        })
    );
}

#[test]
fn set_json_adds_a_missing_component() {
    let mut w = world();
    let e = w.spawn((Team(1),));
    w.set_json(e, "Health", json!({ "current": 1.0, "max": 2.0 })).unwrap();
    assert_eq!(w.get::<Health>(e), Some(&Health { current: 1.0, max: 2.0 }));
    // Transform2D has serde defaults, so a partial value is enough.
    w.set_json(e, "Transform2D", json!({ "rotation": 1.5 })).unwrap();
    let t = w.get::<Transform2D>(e).unwrap();
    assert_eq!((t.position, t.rotation, t.scale), (Vec2::ZERO, 1.5, Vec2::ONE));
}

#[test]
fn entity_json_lists_only_reflected_components() {
    let mut w = world();
    let e = w.spawn((Team(4), Secret, Transform2D::at(vec2(1.0, 2.0))));
    let v = w.entity_json(e).unwrap();
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(|k| k.as_str()).collect();
    keys.sort();
    assert_eq!(keys, ["Team", "Transform2D"]);
}

#[test]
fn spawn_json_reverses_entity_json() {
    let mut w = world();
    let a = w.spawn((
        Team(4),
        Health { current: 1.0, max: 5.0 },
        Transform2D::at(vec2(7.0, 8.0)),
    ));
    let snapshot = w.entity_json(a).unwrap();
    let b = w.spawn_json(snapshot.clone()).unwrap();
    assert_ne!(a, b);
    assert_eq!(w.entity_json(b).unwrap(), snapshot);
}

#[test]
fn failed_spawn_json_leaves_nothing_behind() {
    let mut w = world();
    let before = w.len();
    let err = w
        .spawn_json(json!({ "Team": 1, "Health": "not a health" }))
        .unwrap_err();
    assert!(matches!(err, ReflectError::BadValue { .. }), "{err}");
    assert!(matches!(
        w.spawn_json(json!({ "Team": 1, "Nope": 0 })).unwrap_err(),
        ReflectError::UnknownComponent(n) if n == "Nope"
    ));
    assert!(w.spawn_json(json!({})).is_err());
    assert!(w.spawn_json(json!([1, 2])).is_err());
    assert_eq!(w.len(), before);
}

#[test]
fn errors_say_what_went_wrong() {
    let mut w = world();
    let e = w.spawn((Team(1),));
    assert!(matches!(
        w.get_json(e, "Health"),
        Err(ReflectError::MissingComponent { component, .. }) if component == "Health"
    ));
    assert!(matches!(
        w.get_json(e, "Secret"),
        Err(ReflectError::UnknownComponent(_))
    ));
    let msg = w.set_json(e, "Team", json!("seven")).unwrap_err().to_string();
    assert!(msg.starts_with("bad value for `Team`"), "{msg}");
    w.despawn(e);
    assert_eq!(w.get_json(e, "Team"), Err(ReflectError::NoSuchEntity(e)));
    assert_eq!(w.set_json(e, "Team", json!(1)), Err(ReflectError::NoSuchEntity(e)));
}

#[test]
fn remove_json_removes_and_reports() {
    let mut w = world();
    let e = w.spawn((Team(1), Health { current: 1.0, max: 1.0 }));
    assert_eq!(w.remove_json(e, "Team"), Ok(true));
    assert_eq!(w.remove_json(e, "Team"), Ok(false));
    assert!(w.has::<Health>(e));
}

#[test]
fn reflecting_twice_is_harmless() {
    let mut w = world();
    w.reflect::<Health>();
    assert_eq!(w.reflected_components(), ["Transform2D", "Health", "Team"]);
}

mod other {
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Health(pub f32);
}

#[test]
#[should_panic(expected = "would both be named `Health`")]
fn two_types_with_one_short_name_panic() {
    let mut w = world();
    w.reflect::<other::Health>();
}

#[test]
fn games_reflect_transform_by_default() {
    let world = Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.world.spawn((Transform2D::default(),));
    });
    assert_eq!(world.reflected_components(), ["Transform2D"]);
}

// -------------------------------------------------------------- save/load

/// A component that refers to another entity: the reason ids are kept.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Follows(Entity);

fn save_world() -> (World, Entity, Entity) {
    let mut w = world();
    w.reflect::<Follows>();
    // Churn the allocator so ids have gaps and generations.
    for _ in 0..5 {
        let tmp = w.spawn((Team(0),));
        w.despawn(tmp);
    }
    let leader = w.spawn((Team(1), Transform2D::at(vec2(3.0, 4.0))));
    let follower = w.spawn((Team(1), Follows(leader), Health { current: 2.0, max: 9.0 }));
    w.spawn((Secret,)); // not reflected: not saved
    (w, leader, follower)
}

#[test]
fn save_then_load_restores_entities_with_their_ids() {
    let (w, leader, follower) = save_world();
    let save = w.save();
    assert_eq!(save["entities"].as_array().unwrap().len(), 2);
    assert_eq!(save["not_saved"], json!({ "Secret": 1 }));

    let mut fresh = World::new();
    fresh.reflect::<Transform2D>();
    fresh.reflect::<Health>();
    fresh.reflect::<Team>();
    fresh.reflect::<Follows>();
    fresh.load(&save).unwrap();
    assert_eq!(fresh.len(), 2);
    // The reference still points at the leader.
    let target = fresh.get::<Follows>(follower).unwrap().0;
    assert_eq!(target, leader);
    assert_eq!(fresh.get::<Transform2D>(target).unwrap().position, vec2(3.0, 4.0));
    assert_eq!(fresh.get::<Health>(follower), Some(&Health { current: 2.0, max: 9.0 }));
    // Saving again gives the same entities (the unreflected one was never saved).
    assert_eq!(fresh.save()["entities"], save["entities"]);
    // New entities don't collide with loaded ids.
    let new = fresh.spawn((Team(9),));
    assert!(new != leader && new != follower);
}

#[test]
fn loading_into_the_same_world_rolls_it_back() {
    let (mut w, leader, follower) = save_world();
    let save = w.save();
    w.despawn(leader);
    w.get_mut::<Health>(follower).unwrap().current = 0.0;
    w.spawn((Team(5),));
    w.load(&save).unwrap();
    assert_eq!(w.save()["entities"], save["entities"]);
    // Entities that weren't in the save (nothing reflected) are gone too.
    assert_eq!(w.save()["not_saved"], json!({}));
    assert!(w.is_alive(leader));
    assert_eq!(w.get::<Health>(follower).unwrap().current, 2.0);
}

#[test]
fn a_bad_save_leaves_the_world_untouched() {
    let (mut w, _, _) = save_world();
    let before = w.save();
    let good = before.clone();
    let mut cases = Vec::new();
    let mut bad_value = good.clone();
    bad_value["entities"][1]["components"]["Health"] = json!("lots");
    cases.push(("bad value", bad_value));
    let mut unknown = good.clone();
    unknown["entities"][0]["components"]["Mystery"] = json!(1);
    cases.push(("unknown component", unknown));
    let mut dup = good.clone();
    dup["entities"][1]["entity"] = dup["entities"][0]["entity"].clone();
    cases.push(("duplicate id", dup));
    let mut version = good.clone();
    version["version"] = json!(99);
    cases.push(("version", version));
    cases.push(("not a save", json!({ "hello": 1 })));
    for (what, save) in cases {
        assert!(w.load(&save).is_err(), "{what} should fail");
        assert_eq!(w.save(), before, "{what} changed the world");
    }
}
