//! Query filters through the C API: `with`, `without`, OR over arms and AND
//! groups, their limits and errors, and that a C filter visits the entities
//! the typed Rust filter does.
use ponko_ecs::{Or, QueryFilter, With, Without, World};
use ponko_ffi::*;
use std::ffi::{CStr, c_void};
use std::ptr::{null, null_mut};
use std::sync::Mutex;

use PonkoStatus::*;

struct Player;
struct Enemy;
struct Dead;
struct Ghost;
struct Tag;
struct Label(u32);

/// Component ids in a C world, in the order of `NAMES`.
const NAMES: [&CStr; 5] = [c"Player", c"Enemy", c"Dead", c"Ghost", c"Tag"];
const PLAYER: usize = 0;
const ENEMY: usize = 1;
const DEAD: usize = 2;
const GHOST: usize = 3;
const TAG: usize = 4;

/// The same entities in both worlds: for each of the 32 combinations of the
/// five markers, one entity labelled with the combination (bit `i` is marker
/// `i`).
struct Worlds {
    c: *mut PonkoWorld,
    ids: [u32; 5],
    label: u32,
    rust: World,
}

impl Worlds {
    fn new() -> Self {
        let c = ponko_world_new();
        let mut ids = [0; 5];
        let mut label = 0;
        unsafe {
            for (id, name) in ids.iter_mut().zip(NAMES) {
                assert_eq!(ponko_component_register(c, name.as_ptr(), 4, 4, id), PONKO_OK);
            }
            assert_eq!(
                ponko_component_register(c, c"Label".as_ptr(), 4, 4, &mut label),
                PONKO_OK
            );
        }
        let mut rust = World::new();
        for bits in 0..32u32 {
            let marker = |i: usize| bits >> i & 1 != 0;
            let mut values = vec![PonkoValue {
                component: label,
                data: (&raw const bits).cast(),
                size: 4,
            }];
            for i in (0..5).filter(|&i| marker(i)) {
                values.push(PonkoValue {
                    component: ids[i],
                    data: (&raw const bits).cast(),
                    size: 4,
                });
            }
            let mut e = 0;
            assert_eq!(
                unsafe { ponko_spawn(c, values.as_ptr(), values.len(), &mut e) },
                PONKO_OK
            );
            let e = rust.spawn((Label(bits),));
            if marker(PLAYER) {
                rust.insert(e, Player);
            }
            if marker(ENEMY) {
                rust.insert(e, Enemy);
            }
            if marker(DEAD) {
                rust.insert(e, Dead);
            }
            if marker(GHOST) {
                rust.insert(e, Ghost);
            }
            if marker(TAG) {
                rust.insert(e, Tag);
            }
        }
        Worlds { c, ids, label, rust }
    }

    /// The labels the typed query with filter `F` visits, sorted.
    fn typed<F: QueryFilter>(&mut self) -> Vec<u32> {
        let mut out: Vec<u32> = self.rust.query::<&Label>().filter::<F>().iter().map(|l| l.0).collect();
        out.sort();
        out
    }

    /// The labels `ponko_query_filtered` visits with `filter`, sorted.
    unsafe fn through_c(&self, filter: *const PonkoFilter, options: *const PonkoQueryOptions) -> Vec<u32> {
        let mut seen = Seen::default();
        let status =
            unsafe { ponko_query_filtered(self.c, &self.label, 1, filter, options, Some(labels), user(&mut seen)) };
        assert_eq!(status, PONKO_OK, "{}", last_error());
        seen.labels.sort();
        seen.labels
    }
}

impl Drop for Worlds {
    fn drop(&mut self) {
        unsafe { ponko_world_free(self.c) };
    }
}

#[derive(Default)]
struct Seen {
    labels: Vec<u32>,
}
fn user<T>(t: &mut T) -> *mut c_void {
    (t as *mut T).cast()
}
unsafe extern "C" fn labels(chunk: *const PonkoChunk, user: *mut c_void) {
    unsafe {
        let chunk = &*chunk;
        let column = std::slice::from_raw_parts((*chunk.columns).cast::<u32>(), chunk.count);
        (*user.cast::<Seen>()).labels.extend_from_slice(column);
    }
}
fn last_error() -> String {
    unsafe { CStr::from_ptr(ponko_last_error()) }
        .to_string_lossy()
        .into_owned()
}

/// A filter as data, to build in C and to say again in Rust types.
#[derive(Clone)]
enum S {
    With(usize),
    Without(usize),
    All(Vec<S>),
    Any(Vec<S>),
}
use S::{All, Any, With as Has, Without as Lacks};

/// Owns every handle a `build` made, so the test frees them at the end.
#[derive(Default)]
struct Arena(Vec<*mut PonkoFilter>);
impl Arena {
    fn keep(&mut self, f: *mut PonkoFilter) -> *mut PonkoFilter {
        self.0.push(f);
        f
    }
    unsafe fn build(&mut self, spec: &S, ids: &[u32; 5]) -> *mut PonkoFilter {
        unsafe {
            match spec {
                S::With(i) => self.keep(ponko_filter_with(ids[*i])),
                S::Without(i) => self.keep(ponko_filter_without(ids[*i])),
                S::All(parts) if parts.is_empty() => self.keep(ponko_filter_always()),
                S::Any(parts) if parts.is_empty() => self.keep(ponko_filter_never()),
                S::All(parts) | S::Any(parts) => {
                    let built: Vec<*const PonkoFilter> =
                        parts.iter().map(|p| self.build(p, ids).cast_const()).collect();
                    let f = if matches!(spec, S::All(_)) {
                        ponko_filter_all(built.as_ptr(), built.len())
                    } else {
                        ponko_filter_any(built.as_ptr(), built.len())
                    };
                    self.keep(f)
                }
            }
        }
    }
}
impl Drop for Arena {
    fn drop(&mut self) {
        for &f in &self.0 {
            unsafe { ponko_filter_free(f) };
        }
    }
}

/// Runs `spec` in C and `F` in Rust; both must visit the same entities.
/// Returns how many.
fn same<F: QueryFilter>(w: &mut Worlds, spec: S) -> usize {
    let want = w.typed::<F>();
    let mut arena = Arena::default();
    unsafe {
        let filter = arena.build(&spec, &w.ids);
        assert!(!filter.is_null(), "{}", last_error());
        assert_eq!(w.through_c(filter, null()), want, "serial, defaults");
        // Parallel runs see the same rows (in any order).
        let options = PonkoQueryOptions {
            parallel: true,
            writes: 0,
            ..ponko_query_options_default()
        };
        let mut seen: Mutex<Vec<u32>> = Mutex::new(Vec::new());
        unsafe extern "C" fn collect(chunk: *const PonkoChunk, user: *mut c_void) {
            let mut one = Seen::default();
            unsafe { labels(chunk, (&raw mut one).cast()) };
            unsafe { &*user.cast::<Mutex<Vec<u32>>>() }
                .lock()
                .unwrap()
                .extend(one.labels);
        }
        let status = ponko_query_filtered(w.c, &w.label, 1, filter, &options, Some(collect), user(&mut seen));
        assert_eq!(status, PONKO_OK, "{}", last_error());
        let mut parallel = std::mem::take(seen.get_mut().unwrap());
        parallel.sort();
        assert_eq!(parallel, want, "parallel");
    }
    want.len()
}

#[test]
fn c_filters_visit_the_entities_the_typed_filters_do() {
    let mut w = Worlds::new();
    // 32 entities: every combination of five markers.
    assert_eq!(same::<With<Player>>(&mut w, Has(PLAYER)), 16);
    assert_eq!(same::<Without<Dead>>(&mut w, Lacks(DEAD)), 16);
    assert_eq!(same::<()>(&mut w, All(vec![])), 32);
    assert_eq!(same::<Or<()>>(&mut w, Any(vec![])), 0);
    assert_eq!(
        same::<(With<Player>, Without<Dead>)>(&mut w, All(vec![Has(PLAYER), Lacks(DEAD)])),
        8
    );
    assert_eq!(
        same::<Or<(With<Player>, With<Enemy>)>>(&mut w, Any(vec![Has(PLAYER), Has(ENEMY)])),
        24
    );
    assert_eq!(
        same::<(Or<(With<Player>, With<Enemy>)>, Without<Dead>)>(
            &mut w,
            All(vec![Any(vec![Has(PLAYER), Has(ENEMY)]), Lacks(DEAD)])
        ),
        12
    );
    // AND groups inside an OR.
    assert_eq!(
        same::<Or<((With<Player>, With<Enemy>), Without<Dead>)>>(
            &mut w,
            Any(vec![All(vec![Has(PLAYER), Has(ENEMY)]), Lacks(DEAD)])
        ),
        20
    );
    assert_eq!(
        same::<Or<((With<Player>, Without<Ghost>), (With<Enemy>, With<Tag>))>>(
            &mut w,
            Any(vec![
                All(vec![Has(PLAYER), Lacks(GHOST)]),
                All(vec![Has(ENEMY), Has(TAG)])
            ])
        ),
        14
    );
    // Nesting.
    assert_eq!(
        same::<Or<((With<Player>, Or<(With<Dead>, (With<Ghost>, With<Tag>))>), With<Enemy>)>>(
            &mut w,
            Any(vec![
                All(vec![Has(PLAYER), Any(vec![Has(DEAD), All(vec![Has(GHOST), Has(TAG)])])]),
                Has(ENEMY)
            ])
        ),
        21
    );
    // Empty arms, contradictions and repeats built through the filter tree.
    assert_eq!(
        same::<Or<((), With<Player>)>>(&mut w, Any(vec![All(vec![]), Has(PLAYER)])),
        32
    );
    assert_eq!(
        same::<(Or<()>, With<Player>)>(&mut w, All(vec![Any(vec![]), Has(PLAYER)])),
        0
    );
    assert_eq!(
        same::<(With<Player>, Or<(With<Player>, Without<Player>)>)>(
            &mut w,
            All(vec![Has(PLAYER), Any(vec![Has(PLAYER), Lacks(PLAYER)])])
        ),
        16
    );
    assert_eq!(
        same::<(With<Player>, Without<Player>)>(&mut w, All(vec![Has(PLAYER), All(vec![Lacks(PLAYER)])])),
        0
    );
    // Direct repeats and contradictions: one slot, or nothing, as in Rust.
    assert_eq!(
        same::<(With<Player>, With<Player>)>(&mut w, All(vec![Has(PLAYER), Has(PLAYER)])),
        16
    );
    assert_eq!(
        same::<(Without<Dead>, Without<Dead>)>(&mut w, All(vec![Lacks(DEAD), Lacks(DEAD)])),
        16
    );
    assert_eq!(
        same::<(With<Player>, Without<Player>)>(&mut w, All(vec![Has(PLAYER), Lacks(PLAYER)])),
        0
    );
    assert_eq!(
        same::<(Without<Player>, With<Player>)>(&mut w, All(vec![Lacks(PLAYER), Has(PLAYER)])),
        0
    );
    assert_eq!(
        same::<(With<Player>, Or<(With<Player>, With<Enemy>)>, With<Player>)>(
            &mut w,
            All(vec![Has(PLAYER), Any(vec![Has(PLAYER), Has(ENEMY)]), Has(PLAYER)])
        ),
        16
    );
}

/// Every filter of up to two atoms, and every OR of two AND pairs, from a
/// small set: the C and Rust paths agree on all of them.
#[test]
fn c_filters_agree_with_typed_filters_on_every_small_combination() {
    let mut w = Worlds::new();
    macro_rules! atoms {
        ($($t:ty => $spec:expr),*) => {{
            let mut cases: Vec<(Vec<u32>, S)> = Vec::new();
            $(
                cases.push((w.typed::<$t>(), $spec));
            )*
            cases
        }};
    }
    let mut arena = Arena::default();
    let cases = atoms!(
        With<Player> => Has(PLAYER),
        Without<Player> => Lacks(PLAYER),
        With<Enemy> => Has(ENEMY),
        Without<Enemy> => Lacks(ENEMY),
        With<Dead> => Has(DEAD),
        Without<Ghost> => Lacks(GHOST),
        With<Tag> => Has(TAG),
        Without<Tag> => Lacks(TAG)
    );
    // Evaluate sets directly from the combination bits, as an independent
    // oracle: C result == set arithmetic over the atom results.
    let atom = |spec: &S| -> Vec<u32> { cases.iter().find(|(_, s)| same_atom(s, spec)).unwrap().0.clone() };
    fn same_atom(a: &S, b: &S) -> bool {
        match (a, b) {
            (S::With(x), S::With(y)) | (S::Without(x), S::Without(y)) => x == y,
            _ => false,
        }
    }
    let specs: Vec<S> = cases.iter().map(|(_, s)| s.clone()).collect();
    let union = |a: &[u32], b: &[u32]| -> Vec<u32> { (0..32).filter(|x| a.contains(x) || b.contains(x)).collect() };
    let inter = |a: &[u32], b: &[u32]| -> Vec<u32> { (0..32).filter(|x| a.contains(x) && b.contains(x)).collect() };
    let run = |arena: &mut Arena, w: &Worlds, spec: &S| unsafe {
        let f = arena.build(spec, &w.ids);
        assert!(!f.is_null(), "{}", last_error());
        w.through_c(f, null())
    };
    let mut checked = 0;
    for a in &specs {
        for b in &specs {
            let (ra, rb) = (atom(a), atom(b));
            assert_eq!(run(&mut arena, &w, &Any(vec![a.clone(), b.clone()])), union(&ra, &rb));
            // Repeats and contradictions too: one slot, or nothing, as in Rust.
            assert_eq!(run(&mut arena, &w, &All(vec![a.clone(), b.clone()])), inter(&ra, &rb));
            checked += 2;
            for c in &specs {
                for d in &specs {
                    let (rc, rd) = (atom(c), atom(d));
                    let want = union(&inter(&ra, &rb), &inter(&rc, &rd));
                    let spec = Any(vec![All(vec![a.clone(), b.clone()]), All(vec![c.clone(), d.clone()])]);
                    // Some of these exceed the clause budget, or repeat a
                    // component in a group: those are errors, never wrong rows.
                    unsafe {
                        let f = arena.build(&spec, &w.ids);
                        if f.is_null() {
                            continue;
                        }
                        let mut seen = Seen::default();
                        let status = ponko_query_filtered(w.c, &w.label, 1, f, null(), Some(labels), user(&mut seen));
                        if status == PONKO_OK {
                            seen.labels.sort();
                            assert_eq!(seen.labels, want);
                            checked += 1;
                        } else {
                            assert_eq!(status, PONKO_INVALID_ARGUMENT);
                        }
                    }
                }
            }
        }
    }
    assert!(checked > 1000, "{checked}");
}

#[test]
fn a_filter_is_checked_against_the_world_when_a_query_uses_it() {
    let w = Worlds::new();
    unsafe {
        let unknown = ponko_filter_with(9999);
        assert!(!unknown.is_null(), "ids are looked up by the query, not when building");
        assert_eq!(ponko_filter_check(w.c, unknown), PONKO_INVALID_ARGUMENT);
        assert!(last_error().contains("not registered"), "{}", last_error());
        let mut seen = Seen::default();
        let status = ponko_query_filtered(w.c, &w.label, 1, unknown, null(), Some(labels), user(&mut seen));
        assert_eq!(status, PONKO_INVALID_ARGUMENT);
        assert!(seen.labels.is_empty(), "nothing ran");

        // The same unknown id deep inside a group, an `any` and a `without`.
        let bad = ponko_filter_without(9999);
        let ok = ponko_filter_with(w.ids[PLAYER]);
        let arms = [ok.cast_const(), bad.cast_const()];
        let any = ponko_filter_any(arms.as_ptr(), 2);
        let all = ponko_filter_all([any.cast_const()].as_ptr(), 1);
        assert_eq!(ponko_filter_check(w.c, all), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_filter_check(w.c, ok), PONKO_OK);
        assert_eq!(ponko_filter_check(w.c, null()), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_filter_check(null_mut(), ok), PONKO_INVALID_ARGUMENT);
        for f in [unknown, bad, ok, any, all] {
            ponko_filter_free(f);
        }
    }
}

#[test]
fn filters_over_the_limits_are_errors_not_panics() {
    let mut w = Worlds::new();
    // Nine more components than the limit of 8 `with` or `without`.
    let many: Vec<u32> = (0..9)
        .map(|i| {
            let name = std::ffi::CString::new(format!("Extra{i}")).unwrap();
            let mut id = 0;
            assert_eq!(
                unsafe { ponko_component_register(w.c, name.as_ptr(), 4, 4, &mut id) },
                PONKO_OK
            );
            id
        })
        .collect();
    let check = |w: &Worlds, f: *mut PonkoFilter| unsafe {
        assert!(!f.is_null(), "{}", last_error());
        let mut seen = Seen::default();
        let status = ponko_query_filtered(w.c, &w.label, 1, f, null(), Some(labels), user(&mut seen));
        assert_eq!(status, ponko_filter_check(w.c, f), "a query and the check agree");
        (status, last_error())
    };
    unsafe {
        let mut arena = Arena::default();
        for (name, build_one) in [
            ("with", ponko_filter_with as extern "C" fn(u32) -> *mut PonkoFilter),
            ("without", ponko_filter_without),
        ] {
            let parts: Vec<*const PonkoFilter> =
                many.iter().map(|&id| arena.keep(build_one(id)).cast_const()).collect();
            let eight = arena.keep(ponko_filter_all(parts.as_ptr(), 8));
            assert_eq!(check(&w, eight).0, PONKO_OK, "8 `{name}` components are fine");
            let nine = arena.keep(ponko_filter_all(parts.as_ptr(), 9));
            let (status, message) = check(&w, nine);
            assert_eq!(status, PONKO_INVALID_ARGUMENT, "9 `{name}` components");
            assert!(message.contains(&format!("at most 8 `{name}`")), "{message}");
        }

        // Five clauses: one `any` of one is a clause of one.
        let one: Vec<*const PonkoFilter> = (0..5)
            .map(|i| {
                let with = arena.keep(ponko_filter_with(w.ids[i]));
                arena
                    .keep(ponko_filter_any([with.cast_const()].as_ptr(), 1))
                    .cast_const()
            })
            .collect();
        let four = arena.keep(ponko_filter_all(one.as_ptr(), 4));
        let five = arena.keep(ponko_filter_all(one.as_ptr(), 5));
        assert_eq!(check(&w, four).0, PONKO_OK);
        let (status, message) = check(&w, five);
        assert_eq!(
            (status, message.as_str()),
            (PONKO_INVALID_ARGUMENT, "at most 4 `Or` filters per query")
        );

        // 16 predicates: four clauses of four fit, a fifth term does not.
        let wide = |arena: &mut Arena, n: usize| -> *const PonkoFilter {
            let terms: Vec<*const PonkoFilter> = (0..n)
                .map(|i| arena.keep(ponko_filter_with(w.ids[i])).cast_const())
                .collect();
            arena.keep(ponko_filter_any(terms.as_ptr(), n)).cast_const()
        };
        let (a, b) = (wide(&mut arena, 4), wide(&mut arena, 4));
        let fits = arena.keep(ponko_filter_all([a, a, a, b].as_ptr(), 4));
        assert_eq!(check(&w, fits).0, PONKO_OK);
        let c = wide(&mut arena, 5);
        let too_many = arena.keep(ponko_filter_all([a, a, a, c].as_ptr(), 4));
        let (status, message) = check(&w, too_many);
        assert_eq!(status, PONKO_INVALID_ARGUMENT);
        assert!(message.contains("at most 16 components"), "{message}");
    }
    // The typed path panics on exactly these.
    let typed_panics = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = w.rust.query::<&Label>().filter::<(
            Or<(With<Player>,)>,
            Or<(With<Enemy>,)>,
            Or<(With<Dead>,)>,
            Or<(With<Ghost>,)>,
            Or<(With<Tag>,)>,
        )>();
    }));
    assert!(typed_panics.is_err());
}

#[test]
fn building_filters_rejects_empty_null_repeated_and_too_deep_parts() {
    unsafe {
        // Nothing to AND or OR: say `always` or `never`.
        for f in [ponko_filter_all(null(), 0), ponko_filter_any(null(), 0)] {
            assert!(f.is_null());
            assert!(last_error().contains("at least one part"), "{}", last_error());
        }
        let (always, never) = (ponko_filter_always(), ponko_filter_never());
        assert!(!always.is_null() && !never.is_null());

        // Null array, null part.
        assert!(ponko_filter_all(null(), 1).is_null());
        assert!(last_error().contains("null"));
        let parts = [null::<PonkoFilter>()];
        assert!(ponko_filter_any(parts.as_ptr(), 1).is_null());
        assert!(last_error().contains("null"));

        // Repeats and contradictions inside one group are built as in Rust
        // (the results are compared with the typed filters in
        // `c_filters_visit_the_entities_the_typed_filters_do`).
        let (a, not_a, b) = (ponko_filter_with(1), ponko_filter_without(1), ponko_filter_with(2));
        for pair in [[a, a], [a, not_a], [not_a, not_a]] {
            let pair = [pair[0].cast_const(), pair[1].cast_const()];
            let group = ponko_filter_all(pair.as_ptr(), 2);
            assert!(!group.is_null(), "{}", last_error());
            ponko_filter_free(group);
        }
        // The same component in different arms or groups is fine.
        let pair = [a.cast_const(), not_a.cast_const()];
        let either = ponko_filter_any(pair.as_ptr(), 2);
        assert!(!either.is_null());
        let nested = [a.cast_const(), either.cast_const(), b.cast_const()];
        let all = ponko_filter_all(nested.as_ptr(), 3);
        assert!(!all.is_null());

        // Nesting is bounded (16 levels), so evaluating it can't overflow the
        // stack. An empty group is a leaf, as in `DynFilter`: 16 wrappers
        // around `always` fit, 17 don't.
        let mut deepest = ponko_filter_always();
        let mut around_empty = vec![deepest];
        for _ in 0..16 {
            deepest = ponko_filter_any([deepest.cast_const()].as_ptr(), 1);
            assert!(!deepest.is_null(), "{}", last_error());
            around_empty.push(deepest);
        }
        assert!(ponko_filter_any([deepest.cast_const()].as_ptr(), 1).is_null());
        for f in around_empty {
            ponko_filter_free(f);
        }
        let mut deepest = ponko_filter_with(1);
        let mut chain = vec![deepest];
        for _ in 0..16 {
            deepest = ponko_filter_all([deepest.cast_const()].as_ptr(), 1);
            assert!(!deepest.is_null(), "{}", last_error());
            chain.push(deepest);
        }
        assert!(ponko_filter_all([deepest.cast_const()].as_ptr(), 1).is_null());
        assert!(last_error().contains("16 levels"), "{}", last_error());

        // A built filter owns its parts: freeing them first is fine.
        ponko_filter_free(a);
        ponko_filter_free(not_a);
        ponko_filter_free(b);
        let w = Worlds::new();
        assert_eq!(ponko_filter_check(w.c, all), PONKO_OK);
        for f in chain.into_iter().chain([always, never, either, all]) {
            ponko_filter_free(f);
        }
        ponko_filter_free(null_mut());
    }
}

#[test]
fn filtered_queries_combine_with_columns_change_filters_and_unfiltered_defaults() {
    let w = Worlds::new();
    unsafe {
        // A null filter is `ponko_query_with`, null options its defaults.
        let mut all = Seen::default();
        assert_eq!(
            ponko_query_filtered(w.c, &w.label, 1, null(), null(), Some(labels), user(&mut all)),
            PONKO_OK
        );
        let mut plain = Seen::default();
        assert_eq!(
            ponko_query_with(w.c, &w.label, 1, null(), Some(labels), user(&mut plain)),
            PONKO_OK
        );
        all.labels.sort();
        plain.labels.sort();
        assert_eq!((all.labels.len(), &all.labels), (32, &plain.labels));

        // A filter component can also be a column, and the columns still narrow.
        let ids = [w.label, w.ids[TAG]];
        let tagged = ponko_filter_without(w.ids[DEAD]);
        let mut seen = Seen::default();
        assert_eq!(
            ponko_query_filtered(w.c, ids.as_ptr(), 2, tagged, null(), Some(labels), user(&mut seen)),
            PONKO_OK
        );
        assert_eq!(seen.labels.len(), 8, "without `Dead`, and with a `Tag` column");

        // Change filters of the options apply to the filtered rows.
        let mut tick = 0;
        assert_eq!(ponko_component_track_changes(w.c, w.label), PONKO_OK);
        assert_eq!(ponko_world_advance_change_tick(w.c, &mut tick), PONKO_OK);
        let mut e = 0;
        let value = 7u32;
        let values = [
            PonkoValue {
                component: w.label,
                data: (&raw const value).cast(),
                size: 4,
            },
            PonkoValue {
                component: w.ids[PLAYER],
                data: (&raw const value).cast(),
                size: 4,
            },
        ];
        assert_eq!(ponko_spawn(w.c, values.as_ptr(), 2, &mut e), PONKO_OK);
        let options = PonkoQueryOptions {
            writes: 0,
            changed: 1,
            since: tick,
            ..ponko_query_options_default()
        };
        let (player, enemy) = (ponko_filter_with(w.ids[PLAYER]), ponko_filter_with(w.ids[ENEMY]));
        for (filter, rows) in [(player, 1), (enemy, 0)] {
            let mut seen = Seen::default();
            assert_eq!(
                ponko_query_filtered(w.c, &w.label, 1, filter, &options, Some(labels), user(&mut seen)),
                PONKO_OK
            );
            assert_eq!(seen.labels.len(), rows);
        }
        ponko_filter_free(tagged);
        ponko_filter_free(player);
        ponko_filter_free(enemy);
    }
}

#[test]
fn filtered_queries_reject_bad_arguments_without_running() {
    let w = Worlds::new();
    unsafe {
        let filter = ponko_filter_with(w.ids[PLAYER]);
        let mut seen = Seen::default();
        // No callback, too many ids, a null world, duplicate columns, unknown columns.
        assert_eq!(
            ponko_query_filtered(w.c, &w.label, 1, filter, null(), None, null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        let ids = [w.label; 33];
        assert_eq!(
            ponko_query_filtered(w.c, ids.as_ptr(), 33, filter, null(), Some(labels), user(&mut seen)),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_query_filtered(null_mut(), &w.label, 1, filter, null(), Some(labels), user(&mut seen)),
            PONKO_INVALID_ARGUMENT
        );
        let twice = [w.label, w.label];
        assert_eq!(
            ponko_query_filtered(w.c, twice.as_ptr(), 2, filter, null(), Some(labels), user(&mut seen)),
            PONKO_INVALID_ARGUMENT
        );
        assert!(seen.labels.is_empty());
        ponko_filter_free(filter);
    }
}

#[test]
fn a_filter_is_reusable_across_worlds_and_after_its_parts_are_gone() {
    let one = Worlds::new();
    let two = Worlds::new();
    unsafe {
        // The same filter value, ids that agree in both worlds.
        let (player, enemy) = (ponko_filter_with(one.ids[PLAYER]), ponko_filter_with(one.ids[ENEMY]));
        let either = ponko_filter_any([player.cast_const(), enemy.cast_const()].as_ptr(), 2);
        ponko_filter_free(player);
        ponko_filter_free(enemy);
        let a = one.through_c(either, null());
        let b = two.through_c(either, null());
        assert_eq!((a.len(), &a), (24, &b));
        ponko_filter_free(either);
    }
}

#[test]
fn a_c_query_takes_32_columns_and_the_filter_adds_none() {
    let w = ponko_world_new();
    unsafe {
        let ids: Vec<u32> = (0..33)
            .map(|i| {
                let name = std::ffi::CString::new(format!("Wide{i}")).unwrap();
                let mut id = 0;
                assert_eq!(ponko_component_register(w, name.as_ptr(), 4, 4, &mut id), PONKO_OK);
                id
            })
            .collect();
        let values: Vec<u32> = (0..33).collect();
        let spawn: Vec<PonkoValue> = ids
            .iter()
            .zip(&values)
            .map(|(&component, value)| PonkoValue {
                component,
                data: (value as *const u32).cast(),
                size: 4,
            })
            .collect();
        let mut e = 0;
        assert_eq!(ponko_spawn(w, spawn.as_ptr(), 32, &mut e), PONKO_OK);
        assert_eq!(ponko_spawn(w, spawn.as_ptr(), 33, &mut e), PONKO_OK);
        unsafe extern "C" fn rows(chunk: *const PonkoChunk, user: *mut c_void) {
            unsafe { *user.cast::<usize>() += (*chunk).count };
        }
        let mut n = 0usize;
        let status = ponko_query_filtered(w, ids.as_ptr(), 32, null(), null(), Some(rows), (&raw mut n).cast());
        assert_eq!((status, n), (PONKO_OK, 2));
        let last = ponko_filter_with(ids[32]);
        n = 0;
        let status = ponko_query_filtered(w, ids.as_ptr(), 32, last, null(), Some(rows), (&raw mut n).cast());
        assert_eq!((status, n), (PONKO_OK, 1));
        let status = ponko_query_filtered(w, ids.as_ptr(), 33, last, null(), Some(rows), (&raw mut n).cast());
        assert_eq!(status, PONKO_INVALID_ARGUMENT, "33 columns are too many");
        ponko_filter_free(last);
        ponko_world_free(w);
    }
}
