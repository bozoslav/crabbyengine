//! The `PONKO_*` environment settings, checked in subprocesses so tests never
//! mutate shared process state.
use ponko::prelude::*;

use std::process::Command;

/// Runs `environment_child` in a fresh process with the given settings and
/// expects it to pass.
fn run_child(expect_control: bool, control: Option<&str>, bench: Option<&str>) {
    let registry = std::env::temp_dir().join(format!("ponko-env-test-{}-{expect_control}", std::process::id()));
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args(["--exact", "environment_child", "--nocapture"])
        .env("PONKO_ENV_CHILD", if expect_control { "control" } else { "no-control" })
        .env("PONKO_INSTANCES_DIR", &registry)
        .env("PONKO_START_PAUSED", "0")
        .env_remove("PONKO_CONTROL")
        .env_remove("PONKO_BENCH")
        .env_remove("PONKO_TRACE");
    if let Some(value) = control {
        child.env("PONKO_CONTROL", value);
    }
    if let Some(value) = bench {
        child.env("PONKO_BENCH", value);
    }
    let result = child.output().unwrap();
    let _ = std::fs::remove_dir_all(&registry);
    assert!(
        result.status.success(),
        "control {control:?}, bench {bench:?}\n{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn control_one_opens_the_control_port() {
    // Release builds may gate the environment switch, so this only holds for
    // the debug builds tests normally run in.
    if cfg!(debug_assertions) {
        run_child(true, Some("1"), None);
    }
}

#[test]
fn control_zero_or_empty_or_unset_leaves_the_port_closed() {
    run_child(false, Some("0"), None);
    run_child(false, Some(""), None);
    run_child(false, None, None);
}

#[test]
fn a_benchmark_setting_does_not_stop_a_headless_run() {
    // `PONKO_BENCH` only drives the windowed loop; a headless run must ignore it.
    run_child(false, None, Some("3"));
}

#[test]
fn environment_child() {
    let Ok(mode) = std::env::var("PONKO_ENV_CHILD") else {
        return;
    };
    let expect_control = mode == "control";
    Game::new("Environment settings", 1, 1).run_headless(1, |ctx| {
        assert_eq!(ctx.control_addr().is_some(), expect_control);
    });
}

#[test]
fn a_smoke_setting_runs_the_game_headless_and_returns() {
    // `PONKO_SMOKE=<ticks>`: `Game::run` opens no window, runs the ticks with
    // the frame systems, says so and returns (what `ponko package` checks).
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "smoke_child", "--nocapture"])
        .env("PONKO_SMOKE_CHILD", "1")
        .env("PONKO_SMOKE", "5")
        .env_remove("PONKO_CONTROL")
        .env_remove("PONKO_BENCH")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        result.status.success(),
        "{}\n{stderr}",
        String::from_utf8_lossy(&result.stdout)
    );
    assert!(stderr.contains("ponko: smoke run passed: 5 ticks"), "{stderr}");
}

#[test]
fn smoke_child() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SIM: AtomicU64 = AtomicU64::new(0);
    static FRAMES: AtomicU64 = AtomicU64::new(0);
    if std::env::var_os("PONKO_SMOKE_CHILD").is_none() {
        return;
    }
    Game::new("Smoke setting", 1, 1)
        .sim_system(|_: &mut Ctx| {
            SIM.fetch_add(1, Ordering::Relaxed);
        })
        .system(|_: &mut Ctx| {
            FRAMES.fetch_add(1, Ordering::Relaxed);
        })
        .run(|_| {});
    assert_eq!(SIM.load(Ordering::Relaxed), 5);
    assert_eq!(FRAMES.load(Ordering::Relaxed), 5);
}
