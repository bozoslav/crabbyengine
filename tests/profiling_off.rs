//! Profiling costs nothing until something turns it on. This file has a
//! single test so that no other test can turn it on at the same time.

use ponko::prelude::*;
#[allow(unused_imports)]
use ponko::scope::ProfileScope;

#[test]
fn scopes_do_nothing_while_profiling_is_off() {
    static SCOPE: ProfileScope = ProfileScope::new("off/scope");
    assert_eq!(SCOPE.begin(), 0, "no clock read while off");
    {
        profile_scope!("off/macro");
        let _g = SCOPE.enter();
    }
    Game::new("t", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            drop(ctx.perf.scope("off/named"));
            assert!(!ctx.perf.profiling());
            assert!(ctx.perf.scope_timings().is_empty());
        })
        .run_headless(3, |_| {});
}
