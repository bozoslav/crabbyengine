//! Change detection through the C API: tracking, ticks, write masks,
//! change filters, removals, and per-system windows in a game.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::sync::Mutex;

use CrabbyStatus::*;

const NONE: [u64; 0] = [];
fn bits(e: &CrabbyEntity) -> u64 {
    (e.generation as u64) << 32 | e.index as u64
}
fn user<T>(t: &mut T) -> *mut c_void {
    (t as *mut T).cast()
}

unsafe fn register(w: *mut CrabbyWorld, name: &std::ffi::CStr) -> u32 {
    let mut id = 0;
    assert_eq!(
        unsafe { crabby_component_register(w, name.as_ptr(), 4, 4, &mut id) },
        CRABBY_OK
    );
    id
}
unsafe fn spawn(w: *mut CrabbyWorld, values: &[(u32, u32)]) -> u64 {
    let raw: Vec<CrabbyValue> = values
        .iter()
        .map(|(id, value)| CrabbyValue {
            component: *id,
            data: (value as *const u32).cast(),
            size: 4,
        })
        .collect();
    let mut e = 0;
    assert_eq!(unsafe { crabby_spawn(w, raw.as_ptr(), raw.len(), &mut e) }, CRABBY_OK);
    e
}
unsafe fn set(w: *mut CrabbyWorld, e: u64, id: u32, value: u32) -> CrabbyStatus {
    unsafe { crabby_set(w, e, id, (&raw const value).cast(), 4) }
}
unsafe fn advance(w: *mut CrabbyWorld) -> u64 {
    let mut tick = 0;
    assert_eq!(unsafe { crabby_world_advance_change_tick(w, &mut tick) }, CRABBY_OK);
    tick
}

/// What a query visited: entities in order, and each chunk's `first`.
#[derive(Default)]
struct Seen {
    entities: Vec<u64>,
    firsts: Vec<usize>,
}
unsafe extern "C" fn record(chunk: *const CrabbyChunk, user: *mut c_void) {
    unsafe {
        let (chunk, seen) = (&*chunk, &mut *user.cast::<Seen>());
        seen.firsts.push(chunk.first);
        seen.entities
            .extend(std::slice::from_raw_parts(chunk.entities, chunk.count).iter().map(bits));
    }
}
fn options(writes: u32, changed: u32, added: u32, since: u64) -> CrabbyQueryOptions {
    CrabbyQueryOptions {
        writes,
        changed,
        added,
        since,
        ..crabby_query_options_default()
    }
}
unsafe fn query(w: *mut CrabbyWorld, ids: &[u32], options: CrabbyQueryOptions) -> Vec<u64> {
    let mut seen = Seen::default();
    assert_eq!(
        unsafe { crabby_query_with(w, ids.as_ptr(), ids.len(), &options, Some(record), user(&mut seen)) },
        CRABBY_OK
    );
    seen.entities
}
unsafe fn removed(w: *mut CrabbyWorld, id: u32, since: u64) -> Vec<u64> {
    let mut n = 0;
    assert_eq!(
        unsafe { crabby_removed(w, id, since, null_mut(), 0, &mut n) },
        CRABBY_OK
    );
    let mut out = vec![0u64; n];
    assert_eq!(
        unsafe { crabby_removed(w, id, since, out.as_mut_ptr(), n, &mut n) },
        CRABBY_OK
    );
    assert_eq!(n, out.len());
    out
}

#[test]
fn spawn_set_and_mark_are_seen_by_changed_and_added_filters() {
    unsafe {
        let w = crabby_world_new();
        let (hp, speed) = (register(w, c"Hp"), register(w, c"Speed"));
        assert_eq!(crabby_component_track_changes(w, hp), CRABBY_OK);
        let a = spawn(w, &[(hp, 1), (speed, 1)]);
        let b = spawn(w, &[(hp, 2), (speed, 2)]);
        let c = spawn(w, &[(hp, 3)]);

        // Everything is new for a reader that starts at 0.
        assert_eq!(query(w, &[hp], options(0, 0, 1, 0)), [a, b, c]);
        let t1 = advance(w);
        assert_eq!(query(w, &[hp], options(0, 1, 0, t1)), NONE);
        assert_eq!(query(w, &[hp], options(0, 0, 1, t1)), NONE);

        // Replacing a value changes it; it isn't added again.
        assert_eq!(set(w, b, hp, 20), CRABBY_OK);
        assert_eq!(query(w, &[hp], options(0, 1, 0, t1)), [b]);
        assert_eq!(query(w, &[hp], options(0, 0, 1, t1)), NONE);
        // Reading doesn't.
        let mut out = 0u32;
        assert_eq!(crabby_get(w, a, hp, (&raw mut out).cast(), 4), CRABBY_OK);
        assert_eq!(query(w, &[hp], options(0, 1, 0, t1)), [b]);

        let t2 = advance(w);
        // A new component on an old entity is added and changed.
        assert_eq!(crabby_remove(w, c, hp), CRABBY_OK);
        assert_eq!(set(w, c, hp, 30), CRABBY_OK);
        assert_eq!(query(w, &[hp], options(0, 0, 1, t2)), [c]);
        assert_eq!(crabby_mark_changed(w, a, hp), CRABBY_OK);
        let mut changed = query(w, &[hp], options(0, 1, 0, t2));
        changed.sort();
        assert_eq!(changed, [a, c]);
        // The older reader still sees b's change too.
        assert_eq!(query(w, &[hp], options(0, 1, 0, t1)).len(), 3);

        // Marking needs a live entity that has the component.
        assert_eq!(crabby_mark_changed(w, c, speed), CRABBY_NOT_FOUND);
        assert_eq!(crabby_despawn(w, a), CRABBY_OK);
        assert_eq!(crabby_mark_changed(w, a, hp), CRABBY_NOT_FOUND);
        // An untracked component: fine, and nothing to record.
        assert_eq!(crabby_mark_changed(w, b, speed), CRABBY_OK);
        crabby_world_free(w);
    }
}

unsafe extern "C" fn add_one(chunk: *const CrabbyChunk, _: *mut c_void) {
    unsafe {
        let chunk = &*chunk;
        let values = std::slice::from_raw_parts_mut((*chunk.columns).cast::<u32>(), chunk.count);
        values.iter_mut().for_each(|v| *v += 1);
    }
}
unsafe extern "C" fn nothing(_: *const CrabbyChunk, _: *mut c_void) {}

#[test]
fn only_columns_in_the_write_mask_count_as_changed() {
    unsafe {
        let w = crabby_world_new();
        let (hp, speed) = (register(w, c"Hp"), register(w, c"Speed"));
        crabby_component_track_changes(w, hp);
        crabby_component_track_changes(w, speed);
        let a = spawn(w, &[(hp, 1), (speed, 1)]);
        let b = spawn(w, &[(hp, 2), (speed, 2)]);
        let ids = [hp, speed];

        // Reading both columns changes nothing.
        let t = advance(w);
        assert_eq!(query(w, &ids, options(0, 0, 0, 0)), [a, b]);
        assert_eq!(query(w, &ids, options(0, 0b11, 0, t)), NONE);
        assert_eq!(
            query(
                w,
                &ids,
                CrabbyQueryOptions {
                    any: true,
                    ..options(0, 0b11, 0, t)
                }
            ),
            NONE
        );

        // Writing column 0 (hp) changes hp only.
        let o = options(0b01, 0, 0, 0);
        assert_eq!(
            crabby_query_with(w, ids.as_ptr(), 2, &o, Some(add_one), null_mut()),
            CRABBY_OK
        );
        assert_eq!(query(w, &[hp], options(0, 1, 0, t)), [a, b]);
        assert_eq!(query(w, &[speed], options(0, 1, 0, t)), NONE);
        // Both changed? No. Either changed? Yes.
        assert_eq!(query(w, &ids, options(0, 0b11, 0, t)), NONE);
        assert_eq!(
            query(
                w,
                &ids,
                CrabbyQueryOptions {
                    any: true,
                    ..options(0, 0b11, 0, t)
                }
            ),
            [a, b]
        );
        let mut value = 0u32;
        crabby_get(w, b, hp, (&raw mut value).cast(), 4);
        assert_eq!(value, 3);

        // The plain queries may write every column, so all of them change;
        // null options mean the same.
        for run in 0..3 {
            let t = advance(w);
            let status = match run {
                0 => crabby_query(w, ids.as_ptr(), 2, Some(nothing), null_mut()),
                1 => crabby_query_par(w, ids.as_ptr(), 2, Some(nothing), null_mut()),
                _ => crabby_query_with(w, ids.as_ptr(), 2, null(), Some(nothing), null_mut()),
            };
            assert_eq!(status, CRABBY_OK);
            assert_eq!(query(w, &ids, options(0, 0b11, 0, t)), [a, b], "run {run}");
        }
        crabby_world_free(w);
    }
}

#[test]
fn a_filtered_query_gets_runs_of_rows_and_writes_only_those() {
    unsafe {
        let w = crabby_world_new();
        let hp = register(w, c"Hp");
        crabby_component_track_changes(w, hp);
        let es: Vec<u64> = (0..10).map(|i| spawn(w, &[(hp, i)])).collect();
        let t = advance(w);
        for i in [1, 2, 3, 7, 9] {
            assert_eq!(crabby_mark_changed(w, es[i], hp), CRABBY_OK);
        }
        let mut seen = Seen::default();
        let o = options(0, 1, 0, t);
        assert_eq!(
            crabby_query_with(w, &hp, 1, &o, Some(record), user(&mut seen)),
            CRABBY_OK
        );
        assert_eq!(seen.entities, [es[1], es[2], es[3], es[7], es[9]]);
        // Three runs; `first` counts the rows visited before each.
        assert_eq!(seen.firsts, [0, 3, 4]);

        // Writing through a filtered query marks only the rows it visited.
        let t2 = advance(w);
        let o = options(1, 1, 0, t);
        assert_eq!(crabby_query_with(w, &hp, 1, &o, Some(add_one), null_mut()), CRABBY_OK);
        assert_eq!(
            query(w, &[hp], options(0, 1, 0, t2)),
            [es[1], es[2], es[3], es[7], es[9]]
        );
        let mut value = 0u32;
        crabby_get(w, es[7], hp, (&raw mut value).cast(), 4);
        assert_eq!(value, 8);
        crabby_get(w, es[6], hp, (&raw mut value).cast(), 4);
        assert_eq!(value, 6);
        crabby_world_free(w);
    }
}

#[test]
fn a_filter_starts_tracking_its_component() {
    unsafe {
        let w = crabby_world_new();
        let hp = register(w, c"Hp");
        let a = spawn(w, &[(hp, 1)]);
        // Untracked: removals can't be listed.
        let mut n = 7;
        assert_eq!(crabby_removed(w, hp, 0, null_mut(), 0, &mut n), CRABBY_INVALID_ARGUMENT);
        // The first filter turns tracking on: everything counts as new.
        assert_eq!(query(w, &[hp], options(0, 1, 0, 0)), [a]);
        let t = advance(w);
        assert_eq!(query(w, &[hp], options(0, 1, 0, t)), NONE);
        assert_eq!(set(w, a, hp, 5), CRABBY_OK);
        assert_eq!(query(w, &[hp], options(0, 1, 0, t)), [a]);
        assert_eq!(removed(w, hp, 0), NONE);
        crabby_world_free(w);
    }
}

#[test]
fn removals_list_removed_components_and_despawned_entities() {
    unsafe {
        let w = crabby_world_new();
        let (hp, speed) = (register(w, c"Hp"), register(w, c"Speed"));
        crabby_component_track_changes(w, hp);
        let a = spawn(w, &[(hp, 1), (speed, 1)]);
        let b = spawn(w, &[(hp, 2)]);
        let c = spawn(w, &[(speed, 3)]);
        assert_eq!(crabby_remove(w, a, hp), CRABBY_OK);
        let t = advance(w);
        assert_eq!(crabby_despawn(w, b), CRABBY_OK);
        // No hp on c: nothing to list.
        assert_eq!(crabby_despawn(w, c), CRABBY_OK);
        assert_eq!(removed(w, hp, 0), [a, b]);
        assert_eq!(removed(w, hp, t), [b]);
        assert_eq!(removed(w, hp, advance(w)), NONE);

        // A short buffer gets the oldest entries and the full count.
        let (mut out, mut n) = ([0u64; 1], 0);
        assert_eq!(crabby_removed(w, hp, 0, out.as_mut_ptr(), 1, &mut n), CRABBY_OK);
        assert_eq!((n, out[0]), (2, a));

        // CRABBY_SINCE_LAST_RUN reads the world's window.
        assert_eq!(removed(w, hp, CRABBY_SINCE_LAST_RUN), [a, b]);
        assert_eq!(crabby_world_set_changes_since(w, t), CRABBY_OK);
        assert_eq!(removed(w, hp, CRABBY_SINCE_LAST_RUN), [b]);
        crabby_world_free(w);
    }
}

#[test]
fn since_last_run_uses_the_world_window() {
    unsafe {
        let w = crabby_world_new();
        let hp = register(w, c"Hp");
        crabby_component_track_changes(w, hp);
        let a = spawn(w, &[(hp, 1)]);
        let default = CrabbyQueryOptions {
            writes: 0,
            changed: 1,
            ..crabby_query_options_default()
        };
        assert_eq!(default.since, CRABBY_SINCE_LAST_RUN);
        // The window starts at 0: everything.
        assert_eq!(query(w, &[hp], default), [a]);
        let t = advance(w);
        assert_eq!(crabby_world_set_changes_since(w, t), CRABBY_OK);
        let mut since = 0;
        assert_eq!(crabby_world_changes_since(w, &mut since), CRABBY_OK);
        assert_eq!(since, t);
        assert_eq!(query(w, &[hp], default), NONE);
        assert_eq!(set(w, a, hp, 2), CRABBY_OK);
        assert_eq!(query(w, &[hp], default), [a]);
        let mut now = 0;
        assert_eq!(crabby_world_change_tick(w, &mut now), CRABBY_OK);
        assert_eq!(now, t + 1);
        crabby_world_free(w);
    }
}

struct Par {
    rows: Mutex<Vec<(usize, u64)>>,
}
unsafe extern "C" fn par_rows(chunk: *const CrabbyChunk, user: *mut c_void) {
    unsafe {
        let (chunk, par) = (&*chunk, &*user.cast::<Par>());
        let values = std::slice::from_raw_parts_mut((*chunk.columns).cast::<u32>(), chunk.count);
        let entities = std::slice::from_raw_parts(chunk.entities, chunk.count);
        let mut rows = par.rows.lock().unwrap();
        for (i, (value, e)) in values.iter_mut().zip(entities).enumerate() {
            *value += 100;
            rows.push((chunk.first + i, bits(e)));
        }
    }
}

#[test]
fn a_parallel_filtered_query_visits_each_passing_row_once() {
    unsafe {
        let w = crabby_world_new();
        let (hp, tag) = (register(w, c"Hp"), register(w, c"Tag"));
        crabby_component_track_changes(w, hp);
        // Two archetypes, enough rows to be split across threads.
        let es: Vec<u64> = (0..20_000u32)
            .map(|i| {
                if i % 2 == 0 {
                    spawn(w, &[(hp, i)])
                } else {
                    spawn(w, &[(hp, i), (tag, 0)])
                }
            })
            .collect();
        let t = advance(w);
        let marked: Vec<u64> = es.iter().copied().filter(|e| (e % 7) < 3).collect();
        for e in &marked {
            assert_eq!(crabby_mark_changed(w, *e, hp), CRABBY_OK);
        }
        let t2 = advance(w);
        let par = Par {
            rows: Mutex::new(Vec::new()),
        };
        let o = CrabbyQueryOptions {
            parallel: true,
            ..options(1, 1, 0, t)
        };
        assert_eq!(
            crabby_query_with(w, &hp, 1, &o, Some(par_rows), (&raw const par).cast_mut().cast()),
            CRABBY_OK
        );
        let mut rows = par.rows.into_inner().unwrap();
        rows.sort();
        // Row numbers are 0..n with no gaps or repeats.
        assert!(rows.iter().enumerate().all(|(i, (row, _))| *row == i));
        let mut got: Vec<u64> = rows.iter().map(|r| r.1).collect();
        let mut want = marked.clone();
        got.sort();
        want.sort();
        assert_eq!(got, want);
        // Only those rows were written and marked.
        let mut again = query(w, &[hp], options(0, 1, 0, t2));
        again.sort();
        assert_eq!(again, want);
        let mut value = 0u32;
        crabby_get(w, marked[0], hp, (&raw mut value).cast(), 4);
        assert!(value >= 100);

        // Commands are back on after a parallel query.
        let mut seen = Seen::default();
        let all = CrabbyQueryOptions {
            parallel: true,
            ..options(0, 0, 0, 0)
        };
        assert_eq!(
            crabby_query_with(w, &hp, 1, &all, Some(record_locked), user(&mut seen)),
            CRABBY_OK
        );
        crabby_world_free(w);
    }
}
unsafe extern "C" fn record_locked(_: *const CrabbyChunk, _: *mut c_void) {}

#[test]
fn bad_arguments_are_reported_not_panics() {
    unsafe {
        let w = crabby_world_new();
        let ids: Vec<u32> = (0..9)
            .map(|i| register(w, &std::ffi::CString::new(format!("C{i}")).unwrap()))
            .collect();
        let mut out = 0u64;
        let mut n = 0usize;
        let o = crabby_query_options_default();
        for status in [
            crabby_component_track_changes(null_mut(), 0),
            crabby_component_track_changes(w, 99),
            crabby_world_change_tick(w, null_mut()),
            crabby_world_change_tick(null_mut(), &mut out),
            crabby_world_advance_change_tick(w, null_mut()),
            crabby_world_changes_since(w, null_mut()),
            crabby_world_set_changes_since(null_mut(), 0),
            crabby_mark_changed(w, 1, 99),
            crabby_mark_changed(null_mut(), 1, 0),
            crabby_removed(w, 99, 0, null_mut(), 0, &mut n),
            crabby_removed(w, ids[0], 0, null_mut(), 0, null_mut()),
            crabby_removed(w, ids[0], 0, null_mut(), 4, &mut n),
            crabby_query_with(w, ids.as_ptr(), 1, &o, None, null_mut()),
            crabby_query_with(w, null(), 1, &o, Some(nothing), null_mut()),
            crabby_query_with(null_mut(), ids.as_ptr(), 1, &o, Some(nothing), null_mut()),
            crabby_query_with(w, [99u32].as_ptr(), 1, &o, Some(nothing), null_mut()),
            // More than 8 filters.
            crabby_query_with(w, ids.as_ptr(), 9, &options(0, 0x1ff, 0, 0), Some(nothing), null_mut()),
            crabby_query_with(w, ids.as_ptr(), 9, &options(0, 0x1f, 0xf, 0), Some(nothing), null_mut()),
        ] {
            assert_eq!(status, CRABBY_INVALID_ARGUMENT);
        }
        // Bits past the query's columns are ignored: 8 real filters are fine.
        assert_eq!(
            crabby_query_with(
                w,
                ids.as_ptr(),
                8,
                &options(0, u32::MAX, 0, 0),
                Some(nothing),
                null_mut()
            ),
            CRABBY_OK
        );
        // A tick the world hasn't reached reports everything; nothing breaks.
        crabby_component_track_changes(w, ids[0]);
        let e = spawn(w, &[(ids[0], 1)]);
        assert_eq!(query(w, &ids[..1], options(0, 1, 0, 1 << 40)), [e]);
        crabby_world_free(w);
    }
}

/// Two C systems in a game: each sees what changed since it last ran.
struct Game {
    hp: u32,
    target: u64,
    /// Per tick: how many changed rows the reader saw, and removals.
    reader: Vec<(usize, usize)>,
    writer_saw_own: Vec<usize>,
}
unsafe extern "C" fn count_rows(chunk: *const CrabbyChunk, user: *mut c_void) {
    unsafe { *user.cast::<usize>() += (*chunk).count }
}
unsafe fn changed_rows(w: *mut CrabbyWorld, hp: u32) -> usize {
    let mut rows = 0usize;
    let o = CrabbyQueryOptions {
        writes: 0,
        changed: 1,
        ..crabby_query_options_default()
    };
    assert_eq!(
        unsafe { crabby_query_with(w, &hp, 1, &o, Some(count_rows), user(&mut rows)) },
        CRABBY_OK
    );
    rows
}
unsafe extern "C" fn game_setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let (game, w) = (&mut *user.cast::<Game>(), crabby_ctx_world(ctx));
        game.hp = register(w, c"Hp");
        assert_eq!(crabby_component_track_changes(w, game.hp), CRABBY_OK);
        game.target = spawn(w, &[(game.hp, 0)]);
        spawn(w, &[(game.hp, 0)]);
        spawn(w, &[(game.hp, 0)]);
    }
}
unsafe extern "C" fn game_reader(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let (game, w) = (&mut *user.cast::<Game>(), crabby_ctx_world(ctx));
        let rows = changed_rows(w, game.hp);
        let mut gone = 0;
        assert_eq!(
            crabby_removed(w, game.hp, CRABBY_SINCE_LAST_RUN, null_mut(), 0, &mut gone),
            CRABBY_OK
        );
        game.reader.push((rows, gone));
    }
}
unsafe extern "C" fn game_writer(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let (game, w) = (&mut *user.cast::<Game>(), crabby_ctx_world(ctx));
        // Its own writes of last tick aren't news to it.
        game.writer_saw_own.push(changed_rows(w, game.hp));
        match crabby_ctx_sim_tick(ctx) {
            1 => assert_eq!(set(w, game.target, game.hp, 9), CRABBY_OK),
            3 => assert_eq!(crabby_despawn(w, game.target), CRABBY_OK),
            _ => {}
        }
    }
}

#[test]
fn c_systems_see_each_change_once_whichever_side_of_the_writer_they_run() {
    for reader_first in [true, false] {
        let mut game = Game {
            hp: 0,
            target: 0,
            reader: Vec::new(),
            writer_saw_own: Vec::new(),
        };
        let ptr = user(&mut game);
        unsafe {
            let g = crabby_game_new(c"changes".as_ptr(), 1, 1);
            let add = |name: &std::ffi::CStr, f: unsafe extern "C" fn(*mut CrabbyCtx, *mut c_void)| {
                assert_eq!(crabby_game_sim_system(g, name.as_ptr(), 1, Some(f), ptr), CRABBY_OK);
            };
            if reader_first {
                add(c"reader", game_reader);
                add(c"writer", game_writer);
            } else {
                add(c"writer", game_writer);
                add(c"reader", game_reader);
            }
            let mut world = null_mut();
            assert_eq!(
                crabby_game_run_headless(g, 6, Some(game_setup), ptr, &mut world),
                CRABBY_OK
            );
            crabby_world_free(world);
        }
        // Tick 0: the three spawns. Tick 1's write and tick 3's despawn show
        // up once: the same tick behind the writer, the next tick before it.
        let late = usize::from(reader_first);
        let mut want = vec![(0, 0); 6];
        want[0] = (3, 0);
        want[1 + late] = (1, 0);
        want[3 + late] = (0, 1);
        assert_eq!(game.reader, want, "reader_first {reader_first}");
        assert_eq!(game.writer_saw_own, [3, 0, 0, 0, 0, 0]);
    }
}
