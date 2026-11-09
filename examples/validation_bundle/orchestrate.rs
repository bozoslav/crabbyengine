//! Runs the steps and the reference interaction, and writes the bundle.

use crate::bundle::{Assertion, Bundle, CommandSpec, CommandStatus, Redactor, run_command, unavailable};
use crate::reference::{self, Variant};
use serde_json::{Value, json};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Longest trace kept, in ticks (a run's failure is near its start or end).
const MAX_TRACE_ROWS: usize = 1_000;

pub struct Options {
    /// Where the bundle goes. Must be empty or missing.
    pub out: PathBuf,
    pub variant: Variant,
    pub seed: u64,
    /// Supplied by the caller (CI knows it; a bare checkout may not).
    pub commit: Option<String>,
    /// Feature list supplied by the caller; otherwise the example's own.
    pub features: Option<Vec<String>>,
    pub rustc: Option<String>,
    /// Commands run before the interaction, in order; the first failure
    /// ends the run (a failed build has nothing to interact with).
    pub steps: Vec<CommandSpec>,
    pub step_timeout: Duration,
    /// Kept bytes per output stream of a step.
    pub step_output_limit: usize,
    /// Replace the working directory, home directory and bundle directory
    /// in everything written.
    pub redact_user_paths: bool,
    /// More `(prefix, placeholder)` pairs to replace.
    pub redact_paths: Vec<(String, String)>,
}

impl Options {
    pub fn new(out: impl Into<PathBuf>, variant: Variant) -> Self {
        Self {
            out: out.into(),
            variant,
            seed: 7,
            commit: None,
            features: None,
            rustc: None,
            steps: Vec::new(),
            step_timeout: Duration::from_secs(300),
            step_output_limit: 64 * 1024,
            redact_user_paths: false,
            redact_paths: Vec::new(),
        }
    }
}

pub struct Report {
    pub passed: bool,
    pub manifest: Value,
}

/// Features this example was compiled with, i.e. the ones the engine under
/// test has.
pub fn compiled_features() -> Vec<String> {
    let all = [
        ("3d", cfg!(feature = "3d")),
        ("audio", cfg!(feature = "audio")),
        ("gamepad", cfg!(feature = "gamepad")),
        ("hotpatch", cfg!(feature = "hotpatch")),
        ("inspector", cfg!(feature = "inspector")),
        ("navmesh", cfg!(feature = "navmesh")),
        ("physics", cfg!(feature = "physics")),
        ("physics-determinism", cfg!(feature = "physics-determinism")),
        ("physics3d", cfg!(feature = "physics3d")),
        ("profiling", cfg!(feature = "profiling")),
        ("steam", cfg!(feature = "steam")),
        ("window", cfg!(feature = "window")),
    ];
    all.iter().filter(|(_, on)| *on).map(|(n, _)| n.to_string()).collect()
}

/// The command that repeats the interaction (no output directory: it is
/// where the bundle goes, not part of what is run).
fn reproduce_argv(o: &Options) -> Vec<String> {
    let mut argv: Vec<String> = ["cargo", "run", "--example", "validation_bundle", "--", "--out", "<DIR>"]
        .map(String::from)
        .to_vec();
    argv.extend([
        "--variant".into(),
        o.variant.name().into(),
        "--seed".into(),
        o.seed.to_string(),
    ]);
    for (flag, value) in [("--commit", &o.commit), ("--rustc", &o.rustc)] {
        if let Some(v) = value {
            argv.extend([flag.to_string(), v.clone()]);
        }
    }
    if let Some(f) = &o.features {
        argv.extend(["--features".to_string(), f.join(",")]);
    }
    for step in &o.steps {
        argv.extend(["--step".into(), step.argv.join(" ")]);
    }
    argv
}

fn redactor(o: &Options) -> Redactor {
    let mut r = Redactor::default();
    for (prefix, placeholder) in &o.redact_paths {
        r = r.path(prefix, placeholder);
    }
    if o.redact_user_paths {
        let abs = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        if let Ok(cwd) = std::env::current_dir() {
            r = r.path(&cwd.to_string_lossy(), "<CWD>");
        }
        r = r
            .path(&abs(&o.out).to_string_lossy(), "<BUNDLE>")
            .path(&o.out.to_string_lossy(), "<BUNDLE>");
        for var in ["HOME", "USERPROFILE"] {
            if let Some(home) = std::env::var_os(var) {
                r = r.path(&home.to_string_lossy(), "<HOME>");
            }
        }
    }
    r
}

pub fn run(o: &Options) -> io::Result<Report> {
    let mut bundle = Bundle::new(redactor(o));
    let features = o.features.clone().unwrap_or_else(compiled_features);

    bundle.section(
        "build",
        json!({
            "engine_version": env!("CARGO_PKG_VERSION"),
            "commit": o.commit.as_ref().map_or_else(|| unavailable("not supplied (--commit)"), |c| json!(c)),
            "features": features,
            "features_source": if o.features.is_some() { "supplied" } else { "compiled into the validation example" },
            "rustc": o.rustc.as_ref().map_or_else(|| unavailable("not supplied (--rustc)"), |c| json!(c)),
            "target": format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
            "debug_assertions": cfg!(debug_assertions),
        }),
    );
    bundle.section(
        "scenario",
        json!({
            "id": reference::SCENARIO_ID,
            "seed": o.seed,
            "ticks": reference::TICKS,
            "steps": reference::STEPS.iter().map(|&(tick, pressed)| json!({"tick": tick, "event": "key", "key": "Space", "pressed": pressed})).collect::<Vec<_>>(),
        }),
    );
    bundle.section("settings", json!({ "variant": o.variant.name(), "headless": true }));

    // Commands: the way to repeat the run, then each step with its outcome.
    let mut commands = vec![json!({ "role": "reproduce", "argv": reproduce_argv(o) })];
    let mut failure: Option<Value> = None;
    for (i, step) in o.steps.iter().enumerate() {
        let n = i + 1;
        let out = run_command(step, o.step_timeout, o.step_output_limit);
        let mut log = format!("$ {}\n", step.argv.join(" "));
        log.push_str(&format!(
            "--- stdout ---\n{}\n--- stderr ---\n{}\n",
            out.stdout, out.stderr
        ));
        if let CommandStatus::LaunchFailed(e) = &out.status {
            log.push_str(&format!("--- could not start: {e} ---\n"));
        }
        let path = format!("logs/step-{n}.log");
        bundle.text(&path, "step-log", &log);
        commands.push(json!({
            "role": "step",
            "argv": step.argv,
            "env_names": step.env.iter().map(|(k, _)| k).collect::<Vec<_>>(),
            "status": out.status.name(),
            "exit_code": match &out.status { CommandStatus::Failed(c) => json!(c), _ => Value::Null },
            "timeout_secs": o.step_timeout.as_secs(),
            "omitted_output_bytes": out.omitted_bytes,
            "log": path,
        }));
        if out.status != CommandStatus::Passed {
            failure = Some(json!({
                "kind": if out.status == CommandStatus::TimedOut { "step_timeout" } else { "step_failed" },
                "step": n,
                "status": out.status.name(),
                "log": path,
            }));
            break;
        }
    }
    bundle.section("commands", Value::Array(commands));

    let (assertions, ticks_run): (Vec<Assertion>, u64) = if failure.is_none() {
        let run = reference::run(o.variant, o.seed);
        bundle.text("logs/run.log", "run-log", &(run.log.join("\n") + "\n"));
        let mut csv = String::from("tick,pressed,down,y,vy,jumps\n");
        for r in run.trace.iter().take(MAX_TRACE_ROWS) {
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                r.tick, r.pressed, r.down, r.y, r.vy, r.jumps
            ));
        }
        bundle.text("trace.csv", "trace", &csv);
        let ticks = run.trace.len() as u64;
        if let Some(bad) = run.assertions.iter().find(|a| !a.passed) {
            failure = Some(json!({
                "kind": "assertion",
                "assertion": bad.id,
                "tick": bad.tick,
                "log": "logs/run.log",
                "trace": "trace.csv",
            }));
        }
        (run.assertions, ticks)
    } else {
        (Vec::new(), 0)
    };
    bundle.section(
        "assertions",
        if failure.as_ref().is_some_and(|f| f["kind"] != "assertion") {
            unavailable("not run: an earlier step failed")
        } else {
            Value::Array(assertions.iter().map(Assertion::to_json).collect())
        },
    );

    bundle.section(
        "result",
        json!({
            "outcome": if failure.is_some() { "failed" } else { "passed" },
            "failure": failure,
            "ticks_run": ticks_run,
        }),
    );
    bundle.section(
        "metrics",
        json!({
            "screenshots": unavailable("headless run draws nothing; screenshots need a windowed run and an inspected image"),
            "performance": unavailable("no performance report collected in this stage (see PerfReport / profile_headless)"),
            "wall_time": unavailable("omitted so that identical runs produce identical bundles"),
        }),
    );

    let manifest = bundle.write(&o.out)?;
    Ok(Report {
        passed: manifest["result"]["outcome"] == "passed",
        manifest,
    })
}
