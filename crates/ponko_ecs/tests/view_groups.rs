//! `ViewGroup`: checked sets of view states that share one world, run on
//! several threads or in order with the same result, and views with their
//! own change windows.

use std::panic::{AssertUnwindSafe, catch_unwind};

use ponko_ecs::*;

#[derive(Debug, PartialEq)]
struct Pos(f32);
#[derive(Debug, PartialEq)]
struct Vel(f32);
#[derive(Debug, PartialEq)]
struct Health(i32);
struct Gravity(f32);
#[derive(Debug, PartialEq)]
struct Score(u32);
#[derive(Debug, PartialEq)]
struct Steps(u32);

/// Miri interprets every instruction: keep its worlds small.
const ENTITIES: usize = if cfg!(miri) { 12 } else { 400 };

fn world() -> World {
    let mut w = World::new();
    w.insert_resource(Gravity(-2.0));
    w.insert_resource(Score(0));
    w.insert_resource(Steps(0));
    for i in 0..ENTITIES {
        w.spawn((Pos(i as f32), Vel(0.0), Health(10)));
    }
    w
}

/// What `f` gets next to the view: which of the three jobs in `work` to
/// do, and how often it ran.
struct Rest {
    job: u8,
    ran: u32,
}

struct Item {
    state: ViewState,
    rest: Rest,
}

fn items() -> Vec<Item> {
    let accesses = [
        ViewAccess::new().write::<Vel>().read_resource::<Gravity>(),
        ViewAccess::new()
            .write::<Health>()
            .read::<Pos>()
            .write_resource::<Score>(),
        ViewAccess::new().read::<Pos>().write_resource::<Steps>(),
    ];
    (accesses.into_iter().enumerate())
        .map(|(i, a)| Item {
            state: ViewState::new(a),
            rest: Rest { job: i as u8, ran: 0 },
        })
        .collect()
}

fn parts(item: &mut Item) -> (&mut ViewState, &mut Rest) {
    (&mut item.state, &mut item.rest)
}

fn work(rest: &mut Rest, mut view: WorldView<'_>) {
    rest.ran += 1;
    match rest.job {
        0 => {
            let g = view.resource::<Gravity>().0;
            view.query::<&mut Vel>().for_each(|v| v.0 += g);
        }
        1 => {
            let mut n = 0;
            view.query::<(&mut Health, &Pos)>().for_each(|(h, p)| {
                h.0 -= 1;
                n += u32::from(p.0 >= 0.0);
            });
            view.resource_mut::<Score>().0 += n;
        }
        _ => {
            let n = view.query::<&Pos>().count() as u32;
            view.resource_mut::<Steps>().0 += n;
        }
    }
}

/// The world's observable state after the tasks ran.
fn outcome(w: &mut World) -> (u32, u32, Vec<(f32, f32, i32)>) {
    let mut rows: Vec<(f32, f32, i32)> = w
        .query::<(&Pos, &Vel, &Health)>()
        .iter()
        .map(|(p, v, h)| (p.0, v.0, h.0))
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    (w.resource::<Score>().0, w.resource::<Steps>().0, rows)
}

// ---- checking -------------------------------------------------------------------------------

#[test]
fn a_group_accepts_states_that_do_not_overlap() {
    let items = items();
    let group = ViewGroup::new(items.iter().map(|i| &i.state)).unwrap();
    assert_eq!(group.len(), 3);
    assert!(!group.is_empty());
    assert!(ViewGroup::new(std::iter::empty()).unwrap().is_empty());
}

#[test]
fn a_group_names_the_first_two_states_that_overlap() {
    let a = ViewState::new(ViewAccess::new().write::<Pos>());
    let b = ViewState::new(ViewAccess::new().write::<Vel>());
    let reads_pos = ViewState::new(ViewAccess::new().read::<Pos>());
    assert_eq!(ViewGroup::new([&a, &b, &reads_pos]).unwrap_err(), (0, 2));
    let score = ViewState::new(ViewAccess::new().write_resource::<Score>());
    let reads_score = ViewState::new(ViewAccess::new().read_resource::<Score>());
    assert_eq!(ViewGroup::new([&b, &score, &reads_score]).unwrap_err(), (1, 2));
}

#[test]
fn a_group_refuses_the_same_state_twice() {
    // Two views of one state would share its lent resources.
    let nothing = ViewState::new(ViewAccess::new());
    assert_eq!(ViewGroup::new([&nothing, &nothing]).unwrap_err(), (0, 1));
}

// ---- running --------------------------------------------------------------------------------

#[test]
fn a_group_runs_every_item_once_and_returns_lent_resources() {
    let mut w = world();
    let mut items = items();
    let group = ViewGroup::new(items.iter().map(|i| &i.state)).unwrap();
    group.run(&mut w, &mut items, true, parts, work);
    assert!(items.iter().all(|i| i.rest.ran == 1 && !i.state.is_prepared()));
    let (score, steps, rows) = outcome(&mut w);
    assert_eq!((score, steps), (ENTITIES as u32, ENTITIES as u32));
    assert!(rows.iter().all(|r| r.1 == -2.0 && r.2 == 9));
}

#[test]
fn running_on_threads_and_in_order_gives_the_same_world() {
    let run = |parallel: bool| {
        let mut w = world();
        let mut items = items();
        let group = ViewGroup::new(items.iter().map(|i| &i.state)).unwrap();
        for _ in 0..3 {
            group.run(&mut w, &mut items, parallel, parts, work);
        }
        outcome(&mut w)
    };
    assert_eq!(run(true), run(false));
}

#[test]
fn a_panicking_item_still_returns_every_lent_resource() {
    let mut w = world();
    let mut items = items();
    let group = ViewGroup::new(items.iter().map(|i| &i.state)).unwrap();
    for parallel in [false, true] {
        let result = catch_unwind(AssertUnwindSafe(|| {
            group.run(&mut w, &mut items, parallel, parts, |rest, view| {
                if rest.job == 1 {
                    panic!("job 1 failed");
                }
                work(rest, view);
            });
        }));
        let message = *result.unwrap_err().downcast::<&str>().unwrap();
        assert_eq!(message, "job 1 failed");
        assert!(items.iter().all(|i| !i.state.is_prepared()));
        // Written resources were lent out; they are back.
        assert!(w.get_resource::<Score>().is_some());
        assert!(w.get_resource::<Steps>().is_some());
    }
}

#[test]
#[should_panic(expected = "the group checked 3 states, not 2")]
fn a_group_refuses_a_different_number_of_items() {
    let mut w = world();
    let mut items = items();
    let group = ViewGroup::new(items.iter().map(|i| &i.state)).unwrap();
    items.pop();
    group.run(&mut w, &mut items, false, parts, work);
}

#[test]
#[should_panic(expected = "item 0 isn't the state this group checked")]
fn a_group_refuses_states_it_did_not_check() {
    let mut w = world();
    let mut items = items();
    let group = ViewGroup::new(items.iter().map(|i| &i.state)).unwrap();
    // Same declarations, other states: nothing proves these don't overlap.
    items.swap(0, 1);
    group.run(&mut w, &mut items, false, parts, work);
}

#[test]
fn a_refused_run_leaves_every_state_unprepared() {
    let mut w = world();
    let mut items = items();
    let group = ViewGroup::new(items.iter().map(|i| &i.state)).unwrap();
    items.swap(1, 2);
    let refused = catch_unwind(AssertUnwindSafe(|| {
        group.run(&mut w, &mut items, false, parts, work);
    }));
    assert!(refused.is_err());
    assert!(items.iter().all(|i| !i.state.is_prepared()));
    assert!(w.get_resource::<Score>().is_some());
}

// ---- change windows -------------------------------------------------------------------------

#[test]
fn each_view_state_has_its_own_change_window() {
    let mut w = World::new();
    w.track_changes::<Health>();
    let old = w.spawn((Health(1), Pos(0.0)));
    let first = w.advance_change_tick();
    let new = w.spawn((Health(2), Pos(1.0)));
    let second = w.advance_change_tick();
    // The world's own window would show everything.
    assert_eq!(w.changes_since(), Tick::default());

    struct Seen {
        state: ViewState,
        entities: Vec<Entity>,
    }
    let state = |since| {
        let mut s = ViewState::new(ViewAccess::new().read::<Health>());
        s.set_changes_since(since);
        s
    };
    let mut items: Vec<Seen> = [None, Some(first), Some(second)]
        .into_iter()
        .map(|since| Seen {
            state: state(since),
            entities: Vec::new(),
        })
        .collect();
    let group = ViewGroup::new(items.iter().map(|i| &i.state)).unwrap();
    group.run(
        &mut w,
        &mut items,
        true,
        |i| (&mut i.state, &mut i.entities),
        |seen, mut view| {
            *seen = view.query::<Entity>().filter::<Added<Health>>().iter().collect();
        },
    );
    assert_eq!(items[0].entities, [old, new], "no window of its own: the world's");
    assert_eq!(items[1].entities, [new]);
    assert!(items[2].entities.is_empty());
}

#[test]
fn a_view_states_window_also_limits_removed() {
    let mut w = World::new();
    w.track_changes::<Health>();
    let a = w.spawn((Health(1),));
    let b = w.spawn((Health(2),));
    w.remove::<Health>(a);
    let seen = w.advance_change_tick();
    w.remove::<Health>(b);
    let mut state = ViewState::new(ViewAccess::new().read::<Health>());
    assert_eq!(state.run(&mut w, |view| view.removed::<Health>().to_vec()), [a, b]);
    state.set_changes_since(Some(seen));
    assert_eq!(state.run(&mut w, |view| view.removed::<Health>().to_vec()), [b]);
    state.set_changes_since(None);
    assert_eq!(state.run(&mut w, |view| view.removed::<Health>().len()), 2);
}

// ---- panics ---------------------------------------------------------------------------------

#[test]
fn a_panic_does_not_stop_the_other_items_on_threads_or_in_order() {
    for parallel in [false, true] {
        let mut w = world();
        let mut items = items();
        let group = ViewGroup::new(items.iter().map(|i| &i.state)).unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| {
            group.run(&mut w, &mut items, parallel, parts, |rest, view| {
                if rest.job == 0 {
                    panic!("job 0 failed");
                }
                work(rest, view);
            });
        }));
        assert!(result.is_err());
        // The items after the one that panicked ran too, and their work is
        // in the world: the same on threads and in order.
        assert_eq!(items[1].rest.ran, 1, "parallel: {parallel}");
        assert_eq!(items[2].rest.ran, 1, "parallel: {parallel}");
        let (score, steps, rows) = outcome(&mut w);
        assert_eq!((score, steps), (ENTITIES as u32, ENTITIES as u32));
        assert!(rows.iter().all(|r| r.1 == 0.0 && r.2 == 9));
    }
}

#[test]
fn the_first_panic_in_order_is_the_one_passed_on() {
    for parallel in [false, true] {
        let mut w = world();
        let mut items = items();
        let group = ViewGroup::new(items.iter().map(|i| &i.state)).unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| {
            group.run(&mut w, &mut items, parallel, parts, |rest, _view| {
                if rest.job == 2 {
                    panic!("job 2 failed");
                }
                if rest.job == 0 {
                    // Let the later item panic first when on threads.
                    if !cfg!(miri) {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    panic!("job 0 failed");
                }
            });
        }));
        let message = *result.unwrap_err().downcast::<&str>().unwrap();
        assert_eq!(message, "job 0 failed", "parallel: {parallel}");
        assert!(items.iter().all(|i| !i.state.is_prepared()));
    }
}

// ---- a view's own change tick ---------------------------------------------------------------

#[test]
fn views_with_their_own_change_ticks_stamp_and_report_them() {
    let mut w = World::new();
    w.track_changes::<Pos>();
    w.track_changes::<Vel>();
    let e = w.spawn((Pos(0.0), Vel(0.0)));
    // Three ticks for three views: the world moves past them first.
    let ticks = [
        w.advance_change_tick(),
        w.advance_change_tick(),
        w.advance_change_tick(),
    ];
    let now = w.change_tick();
    let (first, second) = (ticks[0], ticks[1]);

    struct Rest {
        reported: Option<Tick>,
        entity: Entity,
    }
    struct Writer {
        state: ViewState,
        rest: Rest,
    }
    let rest = || Rest {
        reported: None,
        entity: e,
    };
    let state = |access: ViewAccess, tick| {
        let mut s = ViewState::new(access);
        s.set_change_tick(tick);
        s
    };
    let mut items = vec![
        // A query write in the first tick, a `get_mut` in the second, and
        // a view with no tick of its own.
        Writer {
            state: state(ViewAccess::new().write::<Pos>(), Some(ticks[0])),
            rest: rest(),
        },
        Writer {
            state: state(ViewAccess::new().write::<Vel>(), Some(ticks[1])),
            rest: rest(),
        },
        Writer {
            state: state(ViewAccess::new(), None),
            rest: rest(),
        },
    ];
    let group = ViewGroup::new(items.iter().map(|i| &i.state)).unwrap();
    group.run(
        &mut w,
        &mut items,
        true,
        |i| (&mut i.state, &mut i.rest),
        |rest, mut view| {
            rest.reported = Some(view.change_tick());
            if view.change_tick() == first {
                view.query::<&mut Pos>().for_each(|p| p.0 += 1.0);
            } else if view.change_tick() == second {
                view.get_mut::<Vel>(rest.entity).unwrap().0 += 1.0;
            }
        },
    );
    assert_eq!(items[0].rest.reported, Some(ticks[0]));
    assert_eq!(items[1].rest.reported, Some(ticks[1]));
    assert_eq!(items[2].rest.reported, Some(now), "no tick of its own: the world's");

    let changed_pos = |w: &mut World, since| w.query::<&Pos>().filter::<Changed<Pos>>().since(since).count();
    let changed_vel = |w: &mut World, since| w.query::<&Vel>().filter::<Changed<Vel>>().since(since).count();
    // `Pos` was written in tick 0 of the three: seen by a reader that
    // stopped before it, not by one that stopped at it.
    assert_eq!(changed_pos(&mut w, Tick::default()), 1);
    assert_eq!(changed_pos(&mut w, ticks[0]), 0);
    // `Vel` in the second: after the first, not after the second.
    assert_eq!(changed_vel(&mut w, ticks[0]), 1);
    assert_eq!(changed_vel(&mut w, ticks[1]), 0);
}

#[test]
fn a_change_tick_after_the_worlds_counts_as_the_worlds() {
    let mut w = World::new();
    w.track_changes::<Pos>();
    w.spawn((Pos(0.0),));
    let now = w.change_tick();
    let mut state = ViewState::new(ViewAccess::new().write::<Pos>());
    state.set_change_tick(Some(Tick::from_bits(now.to_bits() + 5)));
    assert_eq!(state.run(&mut w, |view| view.change_tick()), now);
    state.set_change_tick(None);
    assert_eq!(state.run(&mut w, |view| view.change_tick()), now);
}
