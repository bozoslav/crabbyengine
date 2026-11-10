//! Change filters and removal lists in view queries: `Added`/`Changed`
//! through a `WorldView`, checked against its declaration; removals; and two
//! views filtering by change on two threads.

use std::panic::{AssertUnwindSafe, catch_unwind};

use ponko_ecs::*;

#[derive(Debug, PartialEq)]
struct Pos(f32);
#[derive(Debug, PartialEq)]
struct Vel(f32);
#[derive(Debug, PartialEq)]
struct Health(i32);
struct Frozen;
/// Never spawned or registered.
struct Never;

/// `a` and `b` exist before `seen`; after it `c` is spawned and `a`'s
/// position changes. The world's window starts at `seen`.
fn world() -> (World, [Entity; 3], Tick) {
    let mut w = World::new();
    w.track_changes::<Pos>();
    w.track_changes::<Health>();
    let a = w.spawn((Pos(0.0), Vel(1.0), Health(3)));
    let b = w.spawn((Pos(1.0), Vel(1.0), Health(3), Frozen));
    let seen = w.advance_change_tick();
    let c = w.spawn((Pos(2.0), Vel(1.0), Health(3)));
    w.get_mut::<Pos>(a).unwrap().0 = 5.0;
    w.set_changes_since(seen);
    (w, [a, b, c], seen)
}

fn sorted(mut v: Vec<Entity>) -> Vec<Entity> {
    v.sort_by_key(|e| e.to_bits());
    v
}

#[test]
fn added_and_changed_pick_rows_in_for_each_iter_count_and_get() {
    let (mut w, [a, b, c], _) = world();
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>());
    state.run(&mut w, |mut view| {
        let added: Vec<_> = view.query::<Entity>().filter::<Added<Pos>>().iter().collect();
        assert_eq!(added, [c]);
        let mut changed = Vec::new();
        view.query::<Entity>()
            .filter::<Changed<Pos>>()
            .for_each(|e| changed.push(e));
        assert_eq!(sorted(changed), sorted(vec![a, c]));
        assert_eq!(view.query::<&Pos>().filter::<Changed<Pos>>().count(), 2);
        assert_eq!(view.query::<&Pos>().count(), 3, "no filter: every row");
        let mut q = view.query::<&Pos>().filter::<Changed<Pos>>();
        assert_eq!(q.get(a), Some(&Pos(5.0)));
        assert_eq!(q.get(b), None, "b didn't change");
        // Or and tuples combine as in world queries.
        let either = view
            .query::<Entity>()
            .filter::<Or<(Added<Pos>, With<Frozen>)>>()
            .iter()
            .collect::<Vec<_>>();
        assert_eq!(sorted(either), sorted(vec![b, c]));
        let both = view
            .query::<Entity>()
            .filter::<(Changed<Pos>, Without<Frozen>)>()
            .count();
        assert_eq!(both, 2);
    });
}

#[test]
fn since_overrides_the_worlds_window() {
    let (mut w, [a, b, c], _) = world();
    let now = w.change_tick();
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>());
    state.run(&mut w, |mut view| {
        let all = view
            .query::<Entity>()
            .filter::<Changed<Pos>>()
            .since(Tick::default())
            .count();
        assert_eq!(all, 3, "since the beginning, everything was added");
        let none = view.query::<Entity>().filter::<Changed<Pos>>().since(now).count();
        assert_eq!(none, 0);
        let _ = (a, b, c);
    });
}

#[test]
fn a_mutable_fetch_with_a_change_filter_marks_only_the_rows_it_visits() {
    let (mut w, [a, _, c], _) = world();
    let before = w.advance_change_tick();
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>().write::<Health>());
    state.run(&mut w, |mut view| {
        view.query::<&mut Health>()
            .filter::<Changed<Pos>>()
            .since(Tick::default())
            .for_each(|h| h.0 -= 1);
    });
    state.run(&mut w, |mut view| {
        let hurt: Vec<_> = view
            .query::<Entity>()
            .filter::<Changed<Health>>()
            .since(before)
            .iter()
            .collect();
        assert_eq!(hurt.len(), 3, "since the beginning every Pos counts as changed");
    });
    let mid = w.advance_change_tick();
    w.set_changes_since(mid);
    w.get_mut::<Pos>(c).unwrap().0 = 9.0;
    state.run(&mut w, |mut view| {
        view.query::<(Entity, &mut Health)>()
            .filter::<Changed<Pos>>()
            .for_each(|(e, h)| {
                assert_eq!(e, c);
                h.0 = 0;
            });
        let mut q = view.query::<&mut Health>().filter::<Changed<Pos>>();
        assert!(q.get(a).is_none(), "a get the filter rejects marks nothing");
    });
    let hurt: Vec<_> = w
        .query::<Entity>()
        .filter::<Changed<Health>>()
        .since(mid)
        .iter()
        .collect();
    assert_eq!(hurt, [c]);
}

#[test]
fn removals_are_listed_through_the_view() {
    let (mut w, [a, b, _], seen) = world();
    w.remove::<Health>(a);
    w.despawn(b);
    let mut state = ViewState::new(ViewAccess::new().read::<Health>());
    state.run(&mut w, |view| {
        assert_eq!(view.removed::<Health>(), [a, b]);
        assert_eq!(view.removed_since::<Health>(seen), [a, b]);
        assert!(view.removed_since::<Health>(view.change_tick()).is_empty());
    });
}

#[test]
fn components_no_entity_ever_had_resolve_without_registering() {
    let (mut w, [a, b, c], _) = world();
    let components = w.components().len();
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>());
    state.run(&mut w, |mut view| {
        assert_eq!(view.query::<Entity>().filter::<With<Never>>().count(), 0);
        assert_eq!(view.query::<Entity>().filter::<Without<Never>>().count(), 3);
        let frozen = view
            .query::<Entity>()
            .filter::<Or<(With<Never>, With<Frozen>)>>()
            .iter()
            .collect::<Vec<_>>();
        assert_eq!(frozen, [b]);
        let all = view
            .query::<Entity>()
            .filter::<Or<(Without<Never>, With<Frozen>)>>()
            .count();
        assert_eq!(all, 3, "a `Without` of an unknown component always holds");
        assert_eq!(view.query::<Entity>().filter::<Or<(With<Never>,)>>().count(), 0);
        // An always-true clause next to a real one.
        let changed = view
            .query::<Entity>()
            .filter::<(Or<(Without<Never>, With<Frozen>)>, Changed<Pos>)>()
            .iter()
            .collect::<Vec<_>>();
        assert_eq!(sorted(changed), sorted(vec![a, c]));
    });
    assert_eq!(w.components().len(), components, "nothing was registered");
}

#[test]
fn a_world_that_tracks_nothing_filters_by_archetype_only() {
    let mut w = World::new();
    w.spawn((Pos(0.0), Frozen));
    w.spawn((Pos(1.0),));
    let mut state = ViewState::new(ViewAccess::new().write::<Pos>());
    state.run(&mut w, |mut view| {
        let mut q = view.query::<&mut Pos>().filter::<Or<(With<Frozen>, With<Never>)>>();
        assert_eq!(q.count(), 1);
        q.for_each(|p| p.0 = 7.0);
    });
    assert_eq!(w.query::<&Pos>().with::<Frozen>().iter().next(), Some(&Pos(7.0)));
}

#[test]
#[should_panic(expected = "mover doesn't declare that it may read component")]
fn an_undeclared_change_filter_panics_naming_the_view() {
    let (mut w, _, _) = world();
    let mut state = ViewState::new(ViewAccess::new().write::<Vel>()).labeled("mover");
    state.run(&mut w, |mut view| {
        let _ = view.query::<&mut Vel>().filter::<Changed<Pos>>();
    });
}

#[test]
#[should_panic(expected = "mover doesn't declare that it may read component")]
fn an_undeclared_change_term_in_an_or_panics() {
    let (mut w, _, _) = world();
    let mut state = ViewState::new(ViewAccess::new().write::<Vel>()).labeled("mover");
    state.run(&mut w, |mut view| {
        let _ = view.query::<&mut Vel>().filter::<Or<(With<Frozen>, Added<Health>)>>();
    });
}

#[test]
#[should_panic(expected = "a view can't start tracking")]
fn a_change_filter_on_an_untracked_component_panics() {
    let (mut w, _, _) = world();
    let mut state = ViewState::new(ViewAccess::new().read::<Vel>());
    state.run(&mut w, |mut view| {
        let _ = view.query::<&Vel>().filter::<Changed<Vel>>();
    });
}

#[test]
#[should_panic(expected = "doesn't declare that it may read component")]
fn reading_removals_of_an_undeclared_component_panics() {
    let (mut w, _, _) = world();
    let mut state = ViewState::new(ViewAccess::new().read::<Pos>());
    state.run(&mut w, |view| {
        let _ = view.removed::<Health>();
    });
}

#[test]
fn a_panicking_filter_still_returns_lent_resources() {
    struct Score(u32);
    let (mut w, _, _) = world();
    w.insert_resource(Score(1));
    let mut state = ViewState::new(ViewAccess::new().write_resource::<Score>());
    let r = catch_unwind(AssertUnwindSafe(|| {
        state.run(&mut w, |mut view| {
            let _ = view.query::<Entity>().filter::<Added<Pos>>();
        })
    }));
    assert!(r.is_err());
    assert_eq!(w.resource::<Score>().0, 1);
}

#[test]
fn two_views_filter_by_change_on_two_threads() {
    let mut w = World::new();
    w.track_changes::<Pos>();
    w.track_changes::<Health>();
    for i in 0..64 {
        w.spawn((Pos(i as f32), Vel(0.0), Health(10)));
    }
    let seen = w.advance_change_tick();
    let moved: Vec<Entity> = w.query::<Entity>().iter().step_by(4).collect();
    for &e in &moved {
        w.get_mut::<Pos>(e).unwrap().0 += 1.0;
    }
    let fresh = w.spawn((Pos(0.0), Vel(0.0), Health(1)));
    w.set_changes_since(seen);

    let mut push = ViewState::new(ViewAccess::new().read::<Pos>().write::<Vel>());
    let mut heal = ViewState::new(ViewAccess::new().write::<Health>());
    assert!(!push.access().overlaps(heal.access()));
    push.prepare(&mut w);
    heal.prepare(&mut w);
    let (pushed, healed) = {
        let shared = &w;
        // SAFETY: the declarations don't overlap and nothing changes the
        // world while the views live.
        let (mut a, mut b) = unsafe { (push.view(shared), heal.view(shared)) };
        std::thread::scope(|s| {
            let pushed = s.spawn(move || {
                let mut n = 0;
                a.query::<&mut Vel>().filter::<Changed<Pos>>().for_each(|v| {
                    v.0 = 1.0;
                    n += 1;
                });
                n
            });
            let healed = s.spawn(move || {
                let mut n = 0;
                b.query::<&mut Health>().filter::<Added<Health>>().for_each(|h| {
                    h.0 = 10;
                    n += 1;
                });
                n
            });
            (pushed.join().unwrap(), healed.join().unwrap())
        })
    };
    push.finish(&mut w);
    heal.finish(&mut w);
    assert_eq!(pushed, moved.len() + 1, "the moved ones and the fresh one");
    assert_eq!(healed, 1);
    assert_eq!(w.get::<Health>(fresh), Some(&Health(10)));
    let pushed_now = w.query::<&Vel>().iter().filter(|v| v.0 == 1.0).count();
    assert_eq!(pushed_now, moved.len() + 1);
}
