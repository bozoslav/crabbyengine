//! `Ctx::ecs_diagnostics`: archetypes, component storage and the system
//! schedule as stable, machine-readable data, read headlessly.

use std::cell::RefCell;
use std::rc::Rc;

#[allow(unused_imports)]
use ponko::diagnostics::EcsDiagnostics;
#[allow(unused_imports)]
use ponko::input::Input;
use ponko::prelude::*;

struct Pos(#[allow(dead_code)] [f32; 2]);
struct Vel(#[allow(dead_code)] [f32; 2]);
struct Tag;

fn early(_: &mut Ctx) {}
fn moving(_: &mut Ctx) {}
fn rare(_: &mut Ctx) {}

/// Runs `ticks` headless ticks and returns the diagnostics read from a
/// `SimLast` system on the final tick.
fn diagnose(game: Game, ticks: u64, setup: impl FnOnce(&mut Ctx) + 'static) -> EcsDiagnostics {
    let out = Rc::new(RefCell::new(None));
    let sink = out.clone();
    let _world = game
        .add(Phase::SimLast, move |ctx: &mut Ctx| {
            *sink.borrow_mut() = Some(ctx.ecs_diagnostics());
        })
        .run_headless(ticks, setup);
    out.take().expect("the SimLast system ran")
}

#[test]
fn archetypes_list_their_components_entities_and_storage() {
    let d = diagnose(Game::new("t", 1, 1), 1, |ctx| {
        for _ in 0..3 {
            ctx.world.spawn((Pos([0.0; 2]), Vel([1.0; 2])));
        }
        ctx.world.spawn((Pos([0.0; 2]),));
        ctx.world.spawn((Tag,));
    });
    assert_eq!(d.version, ponko::diagnostics::ECS_DIAGNOSTICS_VERSION);
    assert_eq!(d.entities, 5);
    let with = |names: &[&str]| {
        d.archetypes
            .iter()
            .find(|a| {
                a.components.len() == names.len() && names.iter().all(|n| a.components.iter().any(|c| c.ends_with(n)))
            })
            .unwrap_or_else(|| panic!("no archetype {names:?} in {:#?}", d.archetypes))
    };
    let moving = with(&["::Pos", "::Vel"]);
    assert_eq!(moving.entities, 3);
    assert!(moving.bytes_per_entity >= 16 + 8, "two [f32; 2] plus the handle");
    assert!(moving.allocated_bytes >= 3 * 16);
    assert_eq!(with(&["::Pos"]).entities, 1);
    assert_eq!(with(&["::Tag"]).entities, 1);
    for (i, a) in d.archetypes.iter().enumerate() {
        assert_eq!(a.index, i, "indices are the world's archetype indices");
    }
    let total: usize = d.archetypes.iter().map(|a| a.entities).sum();
    assert_eq!(total, d.entities);
    assert!(d.allocated_bytes >= moving.allocated_bytes);

    let pos = d.components.iter().find(|c| c.name.ends_with("::Pos")).unwrap();
    assert_eq!(pos.short_name, "Pos");
    assert_eq!((pos.size, pos.align), (8, 4));
    assert_eq!((pos.archetypes, pos.entities), (2, 4));
    assert_eq!(pos.bytes, 4 * 8);
    let tag = d.components.iter().find(|c| c.short_name == "Tag").unwrap();
    assert_eq!((tag.size, tag.entities, tag.bytes), (0, 1, 0));
}

#[test]
fn the_schedule_lists_systems_in_run_order_with_phases() {
    let game = Game::new("t", 1, 1)
        .sim_system(moving)
        .add(Phase::Input, early)
        .sim_system_every(4, rare)
        .system(|_| {});
    let d = diagnose(game, 3, |_| {});
    let sim: Vec<_> = d.systems.iter().filter(|s| s.sim).collect();
    let phases: Vec<_> = sim.iter().map(|s| s.phase.as_str()).collect();
    assert_eq!(phases, ["Input", "AfterPhysics", "AfterPhysics", "SimLast"]);
    assert!(sim[0].name.ends_with("::early"));
    assert!(sim[1].name.ends_with("::moving"));
    assert!(sim[2].name.ends_with("::rare"));
    let rare = sim.iter().find(|s| s.name.ends_with("::rare")).unwrap();
    assert_eq!(rare.every, 4);
    assert_eq!(sim[0].every, 1);
    for (i, s) in d.systems.iter().enumerate() {
        assert_eq!(s.order, i, "order is the position in the schedule");
        assert_eq!(s.access, "exclusive", "every system gets &mut Ctx today");
    }
    // Frame and draw systems are listed (they don't run headlessly).
    let frame = d.systems.iter().find(|s| s.phase == "Frame").unwrap();
    assert!(!frame.sim);
    assert!(
        d.systems
            .iter()
            .any(|s| s.phase == "Draw" && s.name.ends_with("draw_sprites"))
    );
    let draw_at = d.systems.iter().position(|s| s.phase == "Draw").unwrap();
    assert!(d.systems[draw_at..].iter().all(|s| s.phase == "Draw"), "Draw is last");
}

#[test]
fn diagnostics_round_trip_through_json_with_stable_field_names() {
    let d = diagnose(Game::new("t", 1, 1).sim_system(moving), 1, |ctx| {
        ctx.world.spawn((Pos([0.0; 2]),));
    });
    let json: serde_json::Value = serde_json::to_value(&d).unwrap();
    for key in [
        "version",
        "entities",
        "allocated_bytes",
        "archetypes",
        "components",
        "systems",
    ] {
        assert!(json.get(key).is_some(), "missing `{key}` in {json}");
    }
    for key in ["index", "components", "entities", "bytes_per_entity", "allocated_bytes"] {
        assert!(json["archetypes"][0].get(key).is_some(), "archetype `{key}`");
    }
    for key in ["name", "short_name", "size", "align", "archetypes", "entities", "bytes"] {
        assert!(json["components"][0].get(key).is_some(), "component `{key}`");
    }
    for key in [
        "name", "phase", "order", "sim", "every", "offset", "access", "last_ms", "avg_ms",
    ] {
        assert!(json["systems"][0].get(key).is_some(), "system `{key}`");
    }
    let back: EcsDiagnostics = serde_json::from_value(json).unwrap();
    assert_eq!(back, d);
}

#[test]
fn matching_counts_archetypes_and_entities_for_component_names() {
    let d = diagnose(Game::new("t", 1, 1), 1, |ctx| {
        ctx.world.spawn((Pos([0.0; 2]), Vel([0.0; 2])));
        ctx.world.spawn((Pos([0.0; 2]), Vel([0.0; 2]), Tag));
        ctx.world.spawn((Pos([0.0; 2]),));
    });
    let m = d.matching(&["Pos", "Vel"]).unwrap();
    assert_eq!((m.archetypes, m.entities), (2, 2));
    assert_eq!(d.matching(&["Pos"]).unwrap().entities, 3);
    assert_eq!(d.matching(&[]).unwrap().entities, 3);
    let err = d.matching(&["Nope"]).unwrap_err();
    assert!(err.contains("Nope"), "{err}");
}
