//! World views and the archetype index: a view shares the world, so it only
//! reads the index. What it scanned for is indexed the next time the world
//! is held exclusively, and several views read one index at once.

use ponko_ecs::*;

#[derive(Debug, PartialEq, Clone, Copy)]
struct Value(usize);
struct Total(usize);
struct Marker<const N: usize>;

/// Miri interprets every instruction: fewer rounds and a smaller world
/// there, with the index's threshold lowered to fit it.
const ROUNDS: usize = if cfg!(miri) { 2 } else { 24 };
/// Markers the archetypes are made of.
const BITS: usize = if cfg!(miri) { 3 } else { 7 };
/// Archetypes with a `Value`: every subset of the markers.
const ALL: usize = 1 << BITS;
/// Of those, the ones with marker 0 and without marker 1.
const SELECTED: usize = ALL / 4;

/// The policy of these worlds: the default one, from 4 archetypes on under
/// Miri.
fn policy() -> QueryCachePolicy {
    let mut policy = QueryCachePolicy::new();
    if cfg!(miri) {
        policy.min_archetypes = 4;
    }
    policy
}

/// `ALL` archetypes, one entity in each.
fn fragmented() -> World {
    let mut w = World::new();
    w.set_query_cache_policy(policy());
    for bits in 0..ALL {
        let e = w.spawn((Value(bits),));
        macro_rules! m {
            ($n:literal) => {
                if $n < BITS && bits & (1 << $n) != 0 {
                    w.insert(e, Marker::<$n>);
                }
            };
        }
        m!(0);
        m!(1);
        m!(2);
        m!(3);
        m!(4);
        m!(5);
        m!(6);
    }
    w
}

fn reads_values() -> ViewState {
    ViewState::new(ViewAccess::new().read::<Value>())
}

/// What a view's selective query visits, in order.
fn selective(view: &mut WorldView<'_>, hint: QueryCache) -> Vec<usize> {
    let q = view.query::<&Value>().with::<Marker<0>>().without::<Marker<1>>();
    let mut q = match hint {
        QueryCache::Never => q.uncached(),
        QueryCache::Always => q.cached(),
        _ => q,
    };
    let seen: Vec<usize> = q.iter().map(|v| v.0).collect();
    assert_eq!(q.count(), seen.len());
    let mut each = Vec::new();
    q.for_each(|v| each.push(v.0));
    assert_eq!(each, seen);
    seen
}

#[test]
fn a_view_query_is_indexed_once_the_world_is_held_again() {
    let mut w = fragmented();
    let mut state = reads_values();
    let scanned = state.run(&mut w, |mut view| selective(&mut view, QueryCache::Never));
    assert_eq!(scanned.len(), SELECTED);
    assert_eq!(
        w.query_cache_stats().signatures,
        0,
        "an opted-out view query leaves no trace"
    );

    // The view only reads the index: every run's demand is counted when the
    // next one is prepared.
    for _ in 0..4 {
        let seen = state.run(&mut w, |mut view| selective(&mut view, QueryCache::Auto));
        assert_eq!(seen, scanned);
    }
    let stats = w.query_cache_stats();
    assert_eq!((stats.signatures, stats.indexed, stats.indices), (1, 1, SELECTED));

    // The entry is the one an exclusive query of the same signature uses.
    let exclusive: Vec<usize> = w
        .query::<&Value>()
        .with::<Marker<0>>()
        .without::<Marker<1>>()
        .iter()
        .map(|v| v.0)
        .collect();
    assert_eq!(exclusive, scanned);
    assert_eq!(w.query_cache_stats().signatures, 1);
}

#[test]
fn an_indexed_view_query_sees_archetypes_added_between_runs() {
    let mut w = fragmented();
    let mut state = reads_values();
    for _ in 0..4 {
        state.run(&mut w, |mut view| selective(&mut view, QueryCache::Auto));
    }
    assert_eq!(w.query_cache_stats().indexed, 1);

    // A new archetype the query matches, and one it doesn't. Nothing
    // exclusive queries the world before the view does.
    w.spawn((Value(1000), Marker::<0>, Marker::<7>));
    w.spawn((Value(2000), Marker::<0>, Marker::<1>, Marker::<7>));
    for _ in 0..3 {
        let (seen, scanned) = state.run(&mut w, |mut view| {
            (
                selective(&mut view, QueryCache::Auto),
                selective(&mut view, QueryCache::Never),
            )
        });
        assert_eq!(seen, scanned);
        assert_eq!(seen.len(), SELECTED + 1);
        assert_eq!(seen.last(), Some(&1000));
    }
    assert_eq!(w.query_cache_stats().indices, SELECTED + 1);

    // Emptied archetypes stay indexed and yield nothing; refilled, they
    // are visited again.
    let e = w.spawn((Value(3000), Marker::<0>, Marker::<7>));
    w.despawn(e);
    w.clear();
    let seen = state.run(&mut w, |mut view| selective(&mut view, QueryCache::Auto));
    assert!(seen.is_empty());
    w.spawn((Value(7), Marker::<0>));
    let seen = state.run(&mut w, |mut view| selective(&mut view, QueryCache::Auto));
    assert_eq!(seen, [7]);
}

#[test]
fn a_view_query_takes_the_hints() {
    let mut w = fragmented();
    let mut state = reads_values();
    // Matches every archetype with a `Value`: scanned unless asked for.
    for _ in 0..4 {
        let n = state.run(&mut w, |mut view| view.query::<&Value>().count());
        assert_eq!(n, ALL);
    }
    assert_eq!(w.query_cache_stats().indexed, 0);
    for _ in 0..3 {
        let n = state.run(&mut w, |mut view| view.query::<&Value>().cached().count());
        assert_eq!(n, ALL);
    }
    let stats = w.query_cache_stats();
    assert_eq!((stats.indexed, stats.indices), (1, ALL));

    // A world that opted out: views scan, hinted or not.
    w.set_query_cache_policy(QueryCachePolicy::off());
    for _ in 0..3 {
        let seen = state.run(&mut w, |mut view| selective(&mut view, QueryCache::Always));
        assert_eq!(seen.len(), SELECTED);
    }
    assert_eq!(w.query_cache_stats().signatures, 0);
}

struct Item {
    state: ViewState,
    seen: Vec<Vec<usize>>,
}

fn parts(item: &mut Item) -> (&mut ViewState, &mut Vec<Vec<usize>>) {
    (&mut item.state, &mut item.seen)
}

/// Six queries per item: three signatures every item shares and three that
/// differ from item to item.
fn work(job: usize, seen: &mut Vec<Vec<usize>>, mut view: WorldView<'_>) {
    seen.clear();
    seen.push(selective(&mut view, QueryCache::Auto));
    seen.push(view.query::<&Value>().with::<Marker<2>>().iter().map(|v| v.0).collect());
    seen.push(
        view.query::<&Value>()
            .filter::<Or<(With<Marker<3>>, With<Marker<4>>)>>()
            .without::<Marker<5>>()
            .iter()
            .map(|v| v.0)
            .collect(),
    );
    macro_rules! own {
        ($($n:literal)*) => {
            match job % 7 {
                $($n => {
                    seen.push(view.query::<&Value>().with::<Marker<$n>>().with::<Marker<6>>().iter().map(|v| v.0).collect());
                    seen.push(view.query::<&Value>().without::<Marker<$n>>().with::<Marker<0>>().with::<Marker<1>>().iter().map(|v| v.0).collect());
                    seen.push(view.query::<&Value>().cached().without::<Marker<$n>>().iter().map(|v| v.0).collect());
                })*
                _ => unreachable!(),
            }
        };
    }
    own!(0 1 2 3 4 5 6);
    let total = view.get_resource::<Total>().map_or(0, |t| t.0);
    assert_eq!(total, ALL);
}

#[test]
fn views_on_several_threads_read_one_index_and_see_what_a_scan_sees() {
    let run = |parallel: bool, policy: QueryCachePolicy| {
        let mut w = fragmented();
        w.insert_resource(Total(ALL));
        w.set_query_cache_policy(policy);
        let mut items: Vec<(usize, Item)> = (0..8)
            .map(|job| {
                let access = ViewAccess::new().read::<Value>().read_resource::<Total>();
                (
                    job,
                    Item {
                        state: ViewState::new(access),
                        seen: Vec::new(),
                    },
                )
            })
            .collect();
        let group = ViewGroup::new(items.iter().map(|i| &i.1.state)).unwrap();
        let mut rounds = Vec::new();
        for round in 0..ROUNDS {
            group.run(
                &mut w,
                &mut items,
                parallel,
                |item| {
                    let job = item.0;
                    let (state, seen) = parts(&mut item.1);
                    // The job travels with what the item keeps.
                    seen.push(vec![job]);
                    (state, seen)
                },
                |seen, view| {
                    let job = seen.pop().unwrap()[0];
                    work(job, seen, view);
                },
            );
            rounds.push(items.iter().map(|i| i.1.seen.clone()).collect::<Vec<_>>());
            // The world changes between rounds: new archetypes, moves,
            // an emptied archetype.
            let e = w.spawn((Value(1000 + round), Marker::<0>, Marker::<8>));
            if round % 3 == 0 {
                w.insert(e, Marker::<6>);
            }
            if round % 4 == 1 {
                w.despawn(e);
            }
        }
        (rounds, w.query_cache_stats())
    };
    let (scanned, off) = run(false, QueryCachePolicy::off());
    assert_eq!(off.signatures, 0);
    for parallel in [false, true] {
        let (indexed, stats) = run(parallel, policy());
        assert_eq!(indexed, scanned, "parallel: {parallel}");
        assert!(stats.indexed > 0, "the views' queries were indexed: {stats:?}");
    }
}
