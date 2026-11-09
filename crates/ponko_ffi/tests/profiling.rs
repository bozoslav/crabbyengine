//! Profiling from C: scopes, the performance report, budgets.
use ponko_ffi::*;
use std::ffi::{CStr, c_char, c_void};
use std::ptr::null_mut;

use CrabbyStatus::*;

/// Reads text from a C function that fills a buffer.
fn read(fill: impl Fn(*mut c_char, usize, *mut usize) -> CrabbyStatus) -> String {
    let mut len = 0;
    assert_eq!(fill(null_mut(), 0, &mut len), CRABBY_OK);
    let mut buf = vec![1u8; len + 1];
    assert_eq!(fill(buf.as_mut_ptr().cast(), buf.len(), &mut len), CRABBY_OK);
    assert_eq!(buf[len], 0);
    String::from_utf8(buf[..len].to_vec()).unwrap()
}

unsafe extern "C" fn work(_ctx: *mut CrabbyCtx, _user: *mut c_void) {
    unsafe {
        // What `CRABBY_PROFILE_SCOPE` does: look the scope up once, then begin/end.
        static SCOPE: std::sync::atomic::AtomicPtr<CrabbyProfileScope> = std::sync::atomic::AtomicPtr::new(null_mut());
        let mut scope = SCOPE.load(std::sync::atomic::Ordering::Relaxed);
        if scope.is_null() {
            scope = crabby_profile_scope(c"c_work/inner".as_ptr()).cast_mut();
            SCOPE.store(scope, std::sync::atomic::Ordering::Relaxed);
        }
        let token = crabby_profile_begin(scope);
        std::hint::black_box((0..5000).map(|i| (i as f32).sqrt()).sum::<f32>());
        crabby_profile_end(scope, token);
    }
}

fn profile(ticks: u64) -> String {
    unsafe {
        let g = crabby_game_new(c"p".as_ptr(), 1, 1);
        assert_eq!(
            crabby_game_sim_system(g, c"c_work".as_ptr(), 1, Some(work), null_mut()),
            CRABBY_OK
        );
        let mut world = null_mut();
        assert_eq!(
            crabby_game_profile_headless(g, 5, ticks, None, null_mut(), &mut world),
            CRABBY_OK
        );
        assert_eq!(crabby_world_free(world), CRABBY_OK);
    }
    read(|b, c, n| unsafe { crabby_last_perf_report(b, c, n) })
}

#[test]
fn a_headless_profile_has_the_c_scope_and_system() {
    let json = profile(40);
    let text = read(|b, c, n| unsafe {
        crabby_perf_report_text(
            CStr::from_bytes_with_nul(format!("{json}\0").as_bytes())
                .unwrap()
                .as_ptr(),
            b,
            c,
            n,
        )
    });
    assert!(text.contains("40 frames"), "{text}");
    assert!(text.contains("c_work/inner"), "{text}");
    assert!(text.contains("c_work"), "{text}");
}

#[test]
fn budgets_and_comparisons_come_back_as_text() {
    let json = std::ffi::CString::new(profile(30)).unwrap();
    let check = |budget: &CStr| read(|b, c, n| unsafe { crabby_perf_check(json.as_ptr(), budget.as_ptr(), b, c, n) });
    assert_eq!(check(c"{\"frame_p99_ms\": 1000}"), "", "within budget: empty");
    let broken = check(c"{\"scopes\": {\"c_work/inner\": 0.0}, \"systems\": {\"nope\": 1}}");
    assert!(
        broken.contains("scope `c_work/inner`") && broken.contains("no such system"),
        "{broken}"
    );
    let mut len = 0;
    let bad = unsafe { crabby_perf_check(json.as_ptr(), c"[1".as_ptr(), null_mut(), 0, &mut len) };
    assert_eq!(bad, CRABBY_INVALID_ARGUMENT);
    // The same report is never slower than itself.
    let same = read(|b, c, n| unsafe { crabby_perf_compare(json.as_ptr(), json.as_ptr(), 0.1, b, c, n) });
    assert_eq!(same, "");
}

#[test]
fn scopes_are_free_when_profiling_is_off() {
    unsafe {
        let scope = crabby_profile_scope(c"c_off/scope".as_ptr());
        assert!(!scope.is_null() && scope == crabby_profile_scope(c"c_off/scope".as_ptr()));
        assert!(crabby_profile_scope(null_mut()).is_null());
        // Null scopes are ignored rather than crashing.
        crabby_profile_end(std::ptr::null(), crabby_profile_begin(std::ptr::null()));
    }
}
