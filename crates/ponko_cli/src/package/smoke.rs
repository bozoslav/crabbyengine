//! The smoke run: the packaged game, copied to a folder whose path has a
//! space and a non-ASCII letter, started from another, empty folder with
//! nothing of Cargo's or the engine's in the environment, headless for a
//! number of ticks (`PONKO_SMOKE`, read by `Game::run`). It must exit
//! normally within the time limit; that shows it starts, finds its assets
//! next to itself and runs its systems on a machine without a display.

use super::layout::Plan;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// What the engine prints when a smoke run finishes.
pub const PASSED: &str = "ponko: smoke run passed";

/// Removes a folder when dropped.
struct Remove(PathBuf);
impl Drop for Remove {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The environment without anything that could point the game back at the
/// project (`CARGO_MANIFEST_DIR` is one of the asset folders) or change how
/// the engine runs.
pub fn clean_env(vars: impl Iterator<Item = (String, String)>) -> Vec<(String, String)> {
    vars.filter(|(k, _)| {
        let k = k.to_ascii_uppercase();
        !["CARGO", "PONKO", "RUST"].iter().any(|p| k.starts_with(p))
    })
    .collect()
}

/// Reads a pipe to the end on a thread, so a chatty program never blocks.
fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut text = Vec::new();
        if let Some(mut p) = pipe {
            let _ = p.read_to_end(&mut text);
        }
        String::from_utf8_lossy(&text).into_owned()
    })
}

/// Runs the package's program below `out`; returns a line for the report.
pub fn run(out: &Path, plan: &Plan, ticks: u64, timeout: Duration) -> Result<String, String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let work = std::env::temp_dir().join(format!("ponko smoke ü {}-{stamp}", std::process::id()));
    let _work = Remove(work.clone());
    let unpacked = work.join("unpacked");
    super::copy_tree(&out.join(&plan.folder), &unpacked.join(&plan.folder))?;
    let elsewhere = work.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).map_err(|e| e.to_string())?;
    let program = unpacked.join(&plan.program);

    let mut child = Command::new(&program)
        .current_dir(&elsewhere)
        .env_clear()
        .envs(clean_env(std::env::vars()))
        .env("PONKO_SMOKE", ticks.to_string())
        .env("PONKO_DATA_DIR", work.join("data"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("smoke run: can't start {}: {e}", program.display()))?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break Some(status);
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let (stdout, stderr) = (stdout.join().unwrap_or_default(), stderr.join().unwrap_or_default());
    let tail = |text: &str| {
        let lines: Vec<&str> = text.lines().collect();
        lines[lines.len().saturating_sub(40)..].join("\n")
    };
    match status {
        None => Err(format!(
            "smoke run: {} did not exit within {} s (does it start through Game::run?)\n{}",
            plan.program.display(),
            timeout.as_secs(),
            tail(&stderr)
        )),
        Some(s) if !s.success() => Err(format!(
            "smoke run: the packaged game failed ({s}), started from {}:\n{}{}",
            elsewhere.display(),
            tail(&stderr),
            tail(&stdout)
        )),
        Some(_) if stderr.contains(PASSED) => Ok(format!(
            "passed, {ticks} ticks headless from an unrelated folder in {:.1} s",
            start.elapsed().as_secs_f64()
        )),
        Some(_) => Ok(format!(
            "the program exited normally, but not through the engine's smoke run \
             (it doesn't start with Game::run, so PONKO_SMOKE={ticks} did not apply)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_smoke_environment_has_nothing_of_cargo_or_the_engine() {
        let vars = [
            ("PATH", "/bin"),
            ("CARGO_MANIFEST_DIR", "/game"),
            ("cargo_home", "/c"),
            ("PONKO_DATA_DIR", "/d"),
            ("RUST_LOG", "debug"),
            ("HOME", "/h"),
        ];
        let kept = clean_env(vars.iter().map(|(k, v)| (k.to_string(), v.to_string())));
        let names: Vec<&str> = kept.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(names, ["PATH", "HOME"]);
    }
}
