//! Check configuration in subprocesses so tests never mutate shared process state.
use ponko::prelude::*;

use std::process::Command;

#[test]
fn environment_spellings_remain_interoperable() {
    for (primary, legacy, enabled) in [
        (None, "1", true),
        (Some("1"), "0", true),
        (Some("0"), "1", false),
        (Some(""), "1", false),
    ] {
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args(["--exact", "configuration_child", "--nocapture"])
            .env("PONKO_NAMING_CHILD", if enabled { "enabled" } else { "disabled" })
            .env("CRABBY_CONTROL", legacy)
            .env("CRABBY_BENCH", if enabled { "3" } else { "0" })
            .env_remove("PONKO_CONTROL")
            .env_remove("PONKO_BENCH")
            .env("PONKO_START_PAUSED", "0")
            .env_remove("PONKO_TRACE");
        if let Some(value) = primary {
            child
                .env("PONKO_CONTROL", value)
                .env("PONKO_BENCH", if enabled { "3" } else { value });
        }
        let result = child.output().unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn configuration_child() {
    let Ok(mode) = std::env::var("PONKO_NAMING_CHILD") else {
        return;
    };
    let enabled = mode == "enabled";
    Game::new("Naming compatibility", 1, 1).run_headless(1, |ctx| {
        assert_eq!(ctx.control_addr().is_some(), enabled);
    });
    assert_eq!(Color::PONKO_DARK, Color::CRABBY_DARK);
}
