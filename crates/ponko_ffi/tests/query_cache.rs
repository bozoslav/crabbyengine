//! The archetype index from C: the per-query hint, the world's policy and
//! its stats. Hints and policies change how a query finds its archetypes,
//! never the chunks it gets.
use std::ffi::{CString, c_void};

use ponko_ffi::*;

unsafe extern "C" fn collect(chunk: *const PonkoChunk, user: *mut c_void) {
    // SAFETY: the engine passes a live chunk with one `u32` column; the
    // test passes a `Vec<u32>`.
    unsafe {
        let chunk = &*chunk;
        let out = &mut *user.cast::<Vec<u32>>();
        let labels = (*chunk.columns).cast::<u32>();
        for i in 0..chunk.count {
            out.push(*labels.add(i));
        }
    }
}

/// A world with one entity per subset of `markers` markers, labelled with
/// the subset; returns the label's id and the markers' ids.
unsafe fn fragmented(w: *mut PonkoWorld, markers: usize) -> (u32, Vec<u32>) {
    // SAFETY: a live world; values point at live `u32`s of the registered size.
    unsafe {
        let mut label = 0;
        assert_eq!(
            ponko_component_register(w, c"Label".as_ptr(), 4, 4, &mut label),
            PonkoStatus::PONKO_OK
        );
        let ids: Vec<u32> = (0..markers)
            .map(|i| {
                let (name, mut id) = (CString::new(format!("Marker{i}")).unwrap(), 0);
                assert_eq!(
                    ponko_component_register(w, name.as_ptr(), 4, 4, &mut id),
                    PonkoStatus::PONKO_OK
                );
                id
            })
            .collect();
        for bits in 0..1u32 << markers {
            let value = |component| PonkoValue {
                component,
                data: (&raw const bits).cast(),
                size: 4,
            };
            let mut values = vec![value(label)];
            values.extend((0..markers).filter(|i| bits >> i & 1 != 0).map(|i| value(ids[i])));
            let mut e = 0;
            assert_eq!(
                ponko_spawn(w, values.as_ptr(), values.len(), &mut e),
                PonkoStatus::PONKO_OK
            );
        }
        (label, ids)
    }
}

unsafe fn labels(w: *mut PonkoWorld, label: u32, filter: *const PonkoFilter, cache: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut options = ponko_query_options_default();
    options.writes = 0;
    // SAFETY: a live world and filter; `out` outlives the call.
    let status = unsafe {
        ponko_query_cached(
            w,
            &label,
            1,
            filter,
            &options,
            cache,
            Some(collect),
            (&raw mut out).cast(),
        )
    };
    assert_eq!(status, PonkoStatus::PONKO_OK);
    out
}

unsafe fn stats(w: *mut PonkoWorld) -> PonkoQueryCacheStats {
    let mut stats = PonkoQueryCacheStats::default();
    // SAFETY: a live world and a writable output.
    assert_eq!(
        unsafe { ponko_world_query_cache_stats(w, &mut stats) },
        PonkoStatus::PONKO_OK
    );
    stats
}

#[test]
fn every_hint_visits_the_same_rows_in_the_same_order() {
    let w = ponko_world_new();
    // SAFETY: live handles throughout; freed at the end.
    unsafe {
        let (label, ids) = fragmented(w, 7);
        let (with, without) = (ponko_filter_with(ids[0]), ponko_filter_without(ids[3]));
        let either = ponko_filter_any(
            [
                ponko_filter_with(ids[1]).cast_const(),
                ponko_filter_with(ids[2]).cast_const(),
            ]
            .as_ptr(),
            2,
        );
        let filter = ponko_filter_all(
            [with.cast_const(), without.cast_const(), either.cast_const()].as_ptr(),
            3,
        );
        let expected: Vec<u32> = (0..128).filter(|b| b & 1 != 0 && b & 8 == 0 && b & 6 != 0).collect();

        let scanned = labels(w, label, filter, PONKO_QUERY_CACHE_NEVER);
        assert_eq!(stats(w).signatures, 0, "a query that opts out leaves the index alone");
        let mut sorted = scanned.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, expected);
        for _ in 0..3 {
            assert_eq!(labels(w, label, filter, PONKO_QUERY_CACHE_AUTO), scanned);
        }
        let auto = stats(w);
        assert_eq!(
            (auto.signatures, auto.indexed, auto.indices, auto.hits),
            (1, 1, expected.len(), 2)
        );
        assert_eq!(labels(w, label, filter, PONKO_QUERY_CACHE_ALWAYS), scanned);
        assert_eq!(labels(w, label, std::ptr::null(), PONKO_QUERY_CACHE_ALWAYS).len(), 128);
        assert_eq!(stats(w).indexed, 2);
        for f in [with, without, either, filter] {
            ponko_filter_free(f);
        }
        ponko_world_free(w);
    }
}

#[test]
fn c_queries_use_the_automatic_policy_without_a_hint() {
    unsafe extern "C" fn count(chunk: *const PonkoChunk, user: *mut c_void) {
        // SAFETY: a live chunk and the test's counter.
        unsafe { *user.cast::<usize>() += (*chunk).count };
    }
    let w = ponko_world_new();
    // SAFETY: live handles throughout.
    unsafe {
        let (label, ids) = fragmented(w, 7);
        // `ponko_query`, `ponko_query_par`, `ponko_query_with` and
        // `ponko_query_filtered`: no hint anywhere.
        let two = [label, ids[0]];
        let filter = ponko_filter_with(ids[1]);
        for round in 0..3 {
            let mut rows = [0usize; 4];
            assert_eq!(
                ponko_query(w, two.as_ptr(), 2, Some(count), (&raw mut rows[0]).cast()),
                PonkoStatus::PONKO_OK
            );
            let three = [label, ids[0], ids[2]];
            assert_eq!(
                ponko_query_with(
                    w,
                    three.as_ptr(),
                    3,
                    std::ptr::null(),
                    Some(count),
                    (&raw mut rows[1]).cast()
                ),
                PonkoStatus::PONKO_OK
            );
            assert_eq!(
                ponko_query_filtered(
                    w,
                    two.as_ptr(),
                    2,
                    filter,
                    std::ptr::null(),
                    Some(count),
                    (&raw mut rows[2]).cast()
                ),
                PonkoStatus::PONKO_OK
            );
            assert_eq!(rows[..3], [64, 32, 32], "round {round}");
        }
        let s = stats(w);
        // Three signatures, each indexed on its second run and used twice.
        assert_eq!((s.signatures, s.indexed, s.hits), (3, 3, 6));
        // The parallel entry point shares the first signature.
        unsafe extern "C" fn nothing(_: *const PonkoChunk, _: *mut c_void) {}
        assert_eq!(
            ponko_query_par(w, two.as_ptr(), 2, Some(nothing), std::ptr::null_mut()),
            PonkoStatus::PONKO_OK
        );
        assert_eq!((stats(w).signatures, stats(w).hits), (3, 7));
        ponko_filter_free(filter);
        ponko_world_free(w);
    }
}

#[test]
fn a_small_world_never_touches_the_index() {
    let w = ponko_world_new();
    // SAFETY: live handles throughout.
    unsafe {
        let (label, ids) = fragmented(w, 3);
        let filter = ponko_filter_with(ids[0]);
        for cache in [
            PONKO_QUERY_CACHE_AUTO,
            PONKO_QUERY_CACHE_ALWAYS,
            PONKO_QUERY_CACHE_NEVER,
        ] {
            for _ in 0..3 {
                assert_eq!(labels(w, label, filter, cache), [1, 3, 5, 7]);
            }
        }
        assert_eq!(stats(w), PonkoQueryCacheStats::default());
        ponko_filter_free(filter);
        ponko_world_free(w);
    }
}

#[test]
fn the_policy_round_trips_and_off_opts_the_world_out() {
    let w = ponko_world_new();
    // SAFETY: live handles throughout.
    unsafe {
        let (label, ids) = fragmented(w, 7);
        let filter = ponko_filter_with(ids[0]);
        let mut policy = ponko_query_cache_policy_off();
        assert_eq!(ponko_world_query_cache_policy(w, &mut policy), PonkoStatus::PONKO_OK);
        assert_eq!(policy, ponko_query_cache_policy_default());

        let expected = labels(w, label, filter, PONKO_QUERY_CACHE_ALWAYS);
        assert_eq!(stats(w).indexed, 1);
        let off = ponko_query_cache_policy_off();
        assert_eq!(off.min_archetypes, usize::MAX);
        assert_eq!(ponko_world_set_query_cache_policy(w, &off), PonkoStatus::PONKO_OK);
        assert_eq!(ponko_world_query_cache_policy(w, &mut policy), PonkoStatus::PONKO_OK);
        assert_eq!(policy, off);
        let before = stats(w);
        assert_eq!((before.signatures, before.indices, before.bytes), (0, 0, 0));
        for cache in [PONKO_QUERY_CACHE_AUTO, PONKO_QUERY_CACHE_ALWAYS] {
            assert_eq!(labels(w, label, filter, cache), expected);
        }
        assert_eq!(stats(w), before, "nothing is indexed in a world that opted out");

        // Tuned: a bigger world before anything is indexed.
        policy = ponko_query_cache_policy_default();
        policy.min_archetypes = 1000;
        assert_eq!(ponko_world_set_query_cache_policy(w, &policy), PonkoStatus::PONKO_OK);
        assert_eq!(labels(w, label, filter, PONKO_QUERY_CACHE_ALWAYS), expected);
        assert_eq!(stats(w).signatures, 0);
        ponko_filter_free(filter);
        ponko_world_free(w);
    }
}

#[test]
fn bad_arguments_are_reported() {
    unsafe extern "C" fn nothing(_: *const PonkoChunk, _: *mut c_void) {}
    let w = ponko_world_new();
    let invalid = PonkoStatus::PONKO_INVALID_ARGUMENT;
    // SAFETY: a live world; null pointers are what is tested.
    unsafe {
        let mut policy = ponko_query_cache_policy_default();
        let mut stats = PonkoQueryCacheStats::default();
        assert_eq!(ponko_world_set_query_cache_policy(w, std::ptr::null()), invalid);
        assert_eq!(
            ponko_world_set_query_cache_policy(std::ptr::null_mut(), &policy),
            invalid
        );
        assert_eq!(ponko_world_query_cache_policy(w, std::ptr::null_mut()), invalid);
        assert_eq!(
            ponko_world_query_cache_policy(std::ptr::null_mut(), &mut policy),
            invalid
        );
        assert_eq!(ponko_world_query_cache_stats(w, std::ptr::null_mut()), invalid);
        assert_eq!(ponko_world_query_cache_stats(std::ptr::null_mut(), &mut stats), invalid);
        policy.reserved[18] = 1;
        assert_eq!(ponko_world_set_query_cache_policy(w, &policy), invalid);
        // More than every archetype is every archetype, as in Rust.
        policy.reserved[18] = 0;
        policy.max_match_percent = 101;
        assert_eq!(ponko_world_set_query_cache_policy(w, &policy), PonkoStatus::PONKO_OK);
        assert_eq!(ponko_world_query_cache_policy(w, &mut policy), PonkoStatus::PONKO_OK);
        assert_eq!(policy.max_match_percent, 100);
        let no_user = std::ptr::null_mut();
        assert_eq!(
            ponko_query_cached(
                w,
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                3,
                Some(nothing),
                no_user
            ),
            invalid
        );
        assert_eq!(
            ponko_query_cached(
                w,
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                PONKO_QUERY_CACHE_AUTO,
                None,
                no_user
            ),
            invalid
        );
        assert_eq!(
            ponko_query_cached(
                std::ptr::null_mut(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                Some(nothing),
                no_user
            ),
            invalid
        );
        ponko_world_free(w);
    }
}
