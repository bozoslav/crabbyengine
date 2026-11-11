//! What built this run, found without being told: the commit of the working
//! tree and the compiler on the path. Each answer says where it came from;
//! what can't be found is marked unavailable with the reason.

use crate::bundle::{CommandSpec, CommandStatus, run_command, unavailable};
use serde_json::{Value, json};
use std::time::Duration;

/// The first line a short command prints, or why there is none.
fn first_line(argv: &[&str], cwd: Option<&str>) -> Result<String, String> {
    let mut spec = CommandSpec::new(argv);
    spec.cwd = cwd.map(String::from);
    let out = run_command(&spec, Duration::from_secs(10), 4096);
    match out.status {
        CommandStatus::Passed => Ok(out.stdout.lines().next().unwrap_or("").trim().to_string()),
        CommandStatus::Failed(code) => Err(format!("`{}` exited with {code}", argv.join(" "))),
        CommandStatus::TimedOut => Err(format!("`{}` timed out", argv.join(" "))),
        CommandStatus::LaunchFailed(e) => Err(format!("`{}` could not start: {e}", argv.join(" "))),
    }
}

/// The checked-out commit of the repository at `cwd` (the current directory
/// when `None`), and whether the working tree has changes the commit doesn't
/// hold: a bundle from a dirty tree can't be rebuilt from its commit alone.
pub fn commit(cwd: Option<&str>) -> Value {
    match first_line(&["git", "rev-parse", "HEAD"], cwd) {
        Ok(sha) if !sha.is_empty() => {
            let dirty = first_line(&["git", "status", "--porcelain", "--untracked-files=no"], cwd)
                .map(|line| !line.is_empty())
                .ok();
            json!({ "sha": sha, "dirty": dirty, "source": "git rev-parse HEAD" })
        }
        Ok(_) => unavailable("git printed no commit"),
        Err(e) => unavailable(&format!("not supplied (--commit), and {e}")),
    }
}

/// The compiler on the path now. That is the one a `cargo run` of this tool
/// just used; for a game built earlier it may be another, which the source
/// line says.
pub fn rustc() -> Value {
    let program = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    match first_line(&[&program, "--version"], None) {
        Ok(version) if !version.is_empty() => {
            json!({ "version": version, "source": "the compiler on the path when the bundle was made" })
        }
        Ok(_) => unavailable("rustc printed no version"),
        Err(e) => unavailable(&format!("not supplied (--rustc), and {e}")),
    }
}
