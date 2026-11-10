//! Restricted world views: only declared components and resources, checked
//! on every fetch; written resources lent out and returned; non-overlapping
//! views sharing one world from two threads.

use std::panic::{AssertUnwindSafe, catch_unwind};

use ponko_ecs::*;

#[derive(Debug, PartialEq)]
struct Pos(f32);
#[derive(Debug, PartialEq)]
struct Vel(f32);
struct Health(i32);
struct Frozen;
struct Gravity(f32);
#[derive(Debug, PartialEq)]
struct Score(u32);

fn world() -> World {
    let mut w = World::new();
    w.insert_resource(Gravity(-1.0));
    w.insert_resource(Score(0));
    w.spawn((Pos(0.0), Vel(1.0), Health(3)));
    w.spawn((Pos(10.0), Vel(2.0), Frozen));
    w
}

// ---- components -------------------------------------------------------------------

#[test]
fn a_view_writes_declared_components_through_queries() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new().write::<Pos>().read::<Vel>());
    state.run(&mut w, |mut view| {
        view.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| p.0 += v.0);
    });
    let mut xs: Vec<f32> = w.query::<&Pos>().iter().map(|p| p.0).collect();
    xs.sort_by(f32::total_cmp);
    assert_eq!(xs, [1.0, 12.0]);
}

#[test]
fn view_queries_filter_count_iterate_and_get() {
    let mut w = world();
    let frozen = w.query::<(Entity, &Frozen)>().iter().next().unwrap().0;
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>());
    state.run(&mut w, |mut view| {
        assert_eq!(view.query::<&Pos>().count(), 2);
        assert_eq!(view.query::<&Pos>().with::<Frozen>().count(), 1);
        assert_eq!(view.query::<&Pos>().without::<Frozen>().iter().next(), Some(&Pos(0.0)));
        assert_eq!(view.query::<(Entity, &Pos)>().get(frozen).map(|(_, p)| p.0), Some(10.0));
        assert!(view.query::<&Pos>().without::<Frozen>().get(frozen).is_none());
        // A filter on a component no entity ever had matches nothing.
        struct Never;
        assert_eq!(view.query::<&Pos>().with::<Never>().count(), 0);
        assert_eq!(view.query::<&Pos>().with::<Never>().iter().count(), 0);
        assert!(view.query::<&Pos>().with::<Never>().get(frozen).is_none());
        assert_eq!(view.query::<&Pos>().without::<Never>().count(), 2);
        assert_eq!(view.len(), 2);
        assert!(!view.is_empty());
        assert!(view.is_alive(frozen));
        assert!(view.has::<Frozen>(frozen));
    });
}

#[test]
fn get_and_get_mut_follow_the_declaration() {
    let mut w = world();
    let e = w.query::<(Entity, &Health)>().iter().next().unwrap().0;
    let mut state = ViewState::new(ViewAccess::new().write::<Health>().read::<Pos>());
    state.run(&mut w, |mut view| {
        assert_eq!(view.get::<Pos>(e), Some(&Pos(0.0)));
        view.get_mut::<Health>(e).unwrap().0 -= 1;
        assert_eq!(view.get::<Health>(e).unwrap().0, 2, "a write also allows reading");
    });
    assert_eq!(w.get::<Health>(e).unwrap().0, 2);
}

#[test]
#[should_panic(expected = "doesn't declare that it may read component")]
fn querying_an_undeclared_component_panics() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>()).labeled("mover");
    state.run(&mut w, |mut view| view.query::<(&Pos, &Vel)>().for_each(|_| {}));
}

#[test]
#[should_panic(expected = "mover doesn't declare that it may write component")]
fn writing_a_read_only_component_panics() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>()).labeled("mover");
    state.run(&mut w, |mut view| view.query::<&mut Pos>().for_each(|_| {}));
}

#[test]
#[should_panic(expected = "uses a component its declared access doesn't name")]
fn querying_a_never_registered_component_panics() {
    struct Unseen;
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>());
    state.run(&mut w, |mut view| view.query::<&Unseen>().for_each(|_| {}));
}

#[test]
#[should_panic(expected = "doesn't declare that it may write component")]
fn get_mut_of_a_read_component_panics() {
    let mut w = world();
    let e = w.query::<Entity>().iter().next().unwrap();
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>());
    state.run(&mut w, |mut view| {
        view.get_mut::<Pos>(e);
    });
}

#[test]
#[should_panic(expected = "doesn't declare that it may read component")]
fn get_of_an_undeclared_component_panics() {
    let mut w = world();
    let e = w.query::<Entity>().iter().next().unwrap();
    let mut state = ViewState::new(ViewAccess::new());
    state.run(&mut w, |view| {
        let _ = view.get::<Pos>(e);
    });
}

#[test]
#[should_panic(expected = "that would alias")]
fn a_view_query_that_aliases_panics() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new().write::<Pos>());
    state.run(&mut w, |mut view| view.query::<(&mut Pos, &Pos)>().for_each(|_| {}));
}

#[test]
#[should_panic(expected = "doesn't declare that it may read component")]
fn an_undeclared_optional_read_panics() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>());
    state.run(&mut w, |mut view| {
        view.query::<(&Pos, Option<&Health>)>().for_each(|_| {})
    });
}

#[test]
#[should_panic(expected = "doesn't declare that it may write component")]
fn an_optional_write_of_a_read_only_component_panics() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>().read::<Health>());
    state.run(&mut w, |mut view| {
        view.query::<(&Pos, Option<&mut Health>)>().for_each(|_| {})
    });
}

// ---- resources ----------------------------------------------------------------------

#[test]
fn written_resources_are_lent_out_and_returned() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new().write_resource::<Score>().read_resource::<Gravity>());
    state.prepare(&mut w);
    assert!(state.is_prepared());
    assert!(w.get_resource::<Score>().is_none(), "lent out while viewed");
    assert!(w.get_resource::<Gravity>().is_some(), "read resources stay");
    // SAFETY: nothing else uses the world while the view lives.
    let mut view = unsafe { state.view(&w) };
    assert_eq!(view.resource::<Gravity>().0, -1.0);
    view.resource_mut::<Score>().0 += 5;
    assert_eq!(view.resource::<Score>(), &Score(5));
    state.finish(&mut w);
    assert!(!state.is_prepared());
    assert_eq!(w.resource::<Score>(), &Score(5));
    state.finish(&mut w); // finishing twice does nothing
}

#[test]
fn a_view_can_insert_and_remove_its_written_resources() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new().write_resource::<Score>().write_resource::<String>());
    state.run(&mut w, |mut view| {
        assert_eq!(view.remove_resource::<Score>(), Some(Score(0)));
        assert!(view.get_resource::<Score>().is_none());
        assert!(view.get_resource_mut::<String>().is_none());
        view.insert_resource(String::from("hi"));
    });
    assert!(w.get_resource::<Score>().is_none());
    assert_eq!(w.resource::<String>(), "hi");
}

#[test]
#[should_panic(expected = "doesn't declare that it may read resource")]
fn reading_an_undeclared_resource_panics() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new());
    state.run(&mut w, |view| {
        let _ = view.resource::<Score>();
    });
}

#[test]
#[should_panic(expected = "doesn't declare that it may write resource")]
fn writing_a_read_resource_panics() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new().read_resource::<Score>());
    state.run(&mut w, |mut view| view.resource_mut::<Score>().0 += 1);
}

#[test]
#[should_panic(expected = "no resource of type")]
fn a_missing_declared_resource_panics_like_the_world() {
    let mut w = World::new();
    let mut state = ViewState::new(ViewAccess::new().write_resource::<Score>());
    state.run(&mut w, |mut view| view.resource_mut::<Score>().0 += 1);
}

#[test]
fn resources_come_back_when_the_view_panics() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new().write_resource::<Score>());
    let r = catch_unwind(AssertUnwindSafe(|| {
        state.run(&mut w, |mut view| {
            view.resource_mut::<Score>().0 = 9;
            panic!("system failed");
        })
    }));
    assert!(r.is_err());
    assert!(!state.is_prepared());
    assert_eq!(w.resource::<Score>(), &Score(9));
}

// ---- state misuse ---------------------------------------------------------------------

#[test]
#[should_panic(expected = "is already prepared")]
fn preparing_twice_panics() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new());
    state.prepare(&mut w);
    state.prepare(&mut w);
}

#[test]
#[should_panic(expected = "was prepared for another world")]
fn a_state_prepared_for_one_world_cannot_view_another() {
    let mut a = world();
    let b = world();
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>());
    state.prepare(&mut a);
    // SAFETY: panics before creating the view.
    let _ = unsafe { state.view(&b) };
}

#[test]
#[should_panic(expected = "isn't prepared")]
fn viewing_without_preparing_panics() {
    let w = world();
    let mut state = ViewState::new(ViewAccess::new());
    // SAFETY: panics before creating the view.
    let _ = unsafe { state.view(&w) };
}

// ---- declarations ------------------------------------------------------------------------

#[test]
fn overlaps_only_when_one_side_writes() {
    let read_pos = ViewAccess::new().read::<Pos>().read_resource::<Gravity>();
    let write_vel = ViewAccess::new().write::<Vel>().write_resource::<Score>();
    assert!(!read_pos.overlaps(&read_pos.clone()), "readers share");
    assert!(!read_pos.overlaps(&write_vel), "disjoint");
    assert!(write_vel.overlaps(&ViewAccess::new().read::<Vel>()));
    assert!(write_vel.overlaps(&ViewAccess::new().read_resource::<Score>()));
    assert!(read_pos.overlaps(&ViewAccess::new().write_resource::<Gravity>()));
    let merged = ViewAccess::new().read::<Pos>().write::<Pos>();
    assert_eq!(merged.components().count(), 1);
    assert!(merged.components().all(|(_, _, w)| w));
    assert_eq!(merged, ViewAccess::new().write::<Pos>());
    assert_eq!(write_vel.resources().count(), 1);
    assert!(format!("{write_vel:?}").contains("Score"));
}

// ---- sharing -------------------------------------------------------------------------------

#[test]
fn non_overlapping_views_share_one_world_across_threads() {
    let mut w = World::new();
    w.insert_resource(Score(0));
    w.insert_resource(Gravity(-2.0));
    for i in 0..64 {
        w.spawn((Pos(i as f32), Vel(0.0), Health(10)));
    }
    let e = w.query::<Entity>().iter().next().unwrap();
    let mut fall = ViewState::new(ViewAccess::new().write::<Vel>().read_resource::<Gravity>());
    let mut hurt = ViewState::new(
        ViewAccess::new()
            .write::<Health>()
            .read::<Pos>()
            .write_resource::<Score>(),
    );
    assert!(!fall.access().overlaps(hurt.access()));
    fall.prepare(&mut w);
    hurt.prepare(&mut w);
    {
        let shared = &w;
        // SAFETY: the declarations don't overlap and nothing changes the
        // world while the views live.
        let (mut a, mut b) = unsafe { (fall.view(shared), hurt.view(shared)) };
        std::thread::scope(|s| {
            s.spawn(move || {
                let g = a.resource::<Gravity>().0;
                a.query::<&mut Vel>().for_each(|v| v.0 += g);
                a.get_mut::<Vel>(e).unwrap().0 += 100.0;
            });
            s.spawn(move || {
                let mut n = 0;
                b.query::<(&mut Health, &Pos)>().for_each(|(h, p)| {
                    h.0 -= 1;
                    n += u32::from(p.0 >= 0.0);
                });
                b.resource_mut::<Score>().0 += n;
            });
        });
    }
    fall.finish(&mut w);
    hurt.finish(&mut w);
    assert_eq!(w.resource::<Score>(), &Score(64));
    assert!(w.query::<&Health>().iter().all(|h| h.0 == 9));
    assert_eq!(w.get::<Vel>(e), Some(&Vel(98.0)));
    let others = w
        .query::<(Entity, &Vel)>()
        .iter()
        .filter(|(x, _)| *x != e)
        .all(|(_, v)| v.0 == -2.0);
    assert!(others);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "ViewState dropped while prepared")]
fn dropping_a_prepared_state_panics_in_debug_builds() {
    let mut w = world();
    let mut state = ViewState::new(ViewAccess::new().write_resource::<Score>()).labeled("scorer");
    state.prepare(&mut w);
    drop(state);
}
