//! Runs the steps and the reference interaction, and writes the bundle.

use crate::bundle::{Assertion, Bundle, CommandSpec, CommandStatus, Redactor, run_command, unavailable};
use crate::identity;
use crate::launched::{self, Interaction, Launch};
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
    /// Find the commit and compiler when they aren't supplied (`git`,
    /// `rustc --version`). The command line turns this on.
    pub capture_identity: bool,
    /// Build the game, start it as its own process and drive it through
    /// the MCP tools, instead of running the interaction in this process.
    pub launch: Option<Launch>,
    /// A scenario file (the engine's input-scenario JSON) to put a launched
    /// game of your own through, instead of the reference interaction: its
    /// check steps are the assertions. Needs `launch`.
    pub scenario: Option<Value>,
    /// Profile the in-process run and keep the report. (A launched run
    /// always asks the game's performance tools.)
    pub profile: bool,
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
            capture_identity: false,
            launch: None,
            scenario: None,
            profile: false,
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
        ("accessibility", cfg!(feature = "accessibility")),
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
    // A scenario file's game has no variant and takes its seed its own way.
    if o.scenario.is_none() {
        argv.extend([
            "--variant".into(),
            o.variant.name().into(),
            "--seed".into(),
            o.seed.to_string(),
        ]);
    }
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
    if o.profile {
        argv.push("--perf".into());
    }
    if let Some(launch) = &o.launch {
        match &launch.program {
            Some(program) => argv.extend(["--launch-program".into(), program.clone()]),
            None => argv.push("--launch".into()),
        }
        for (flag, value) in [
            ("--launch-cwd", &launch.cwd),
            ("--launch-example", &launch.example),
            ("--launch-bin", &launch.bin),
        ] {
            // The default example is implied by `--launch`.
            if let Some(v) = value.as_ref().filter(|v| *v != "validation_bundle") {
                argv.extend([flag.to_string(), v.clone()]);
            }
        }
        if launch.release {
            argv.push("--launch-release".into());
        }
        if launch.windowed {
            argv.push("--windowed".into());
        }
        for arg in launch.args.iter().flatten() {
            argv.extend(["--launch-arg".into(), arg.clone()]);
        }
        for (name, _) in &launch.env {
            argv.extend(["--launch-env".into(), format!("{name}=<value>")]);
        }
    }
    if o.scenario.is_some() {
        argv.extend(["--scenario".into(), "<BUNDLE>/scenario.json".into()]);
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

    if o.scenario.is_some() && o.launch.is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a scenario file needs a launched game (--launch or --launch-program)",
        ));
    }
    let interaction = match &o.scenario {
        None => Interaction::Reference,
        Some(file) => {
            // Checked here, so a typo is an error before anything is built.
            let parsed = ponko::scenario::InputScenario::from_value(file)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            bundle.json("scenario.json", "scenario", file);
            bundle.section(
                "scenario",
                json!({
                    "id": file["id"],
                    "source": "supplied file, kept as scenario.json",
                    "file": "scenario.json",
                    "ticks": parsed.ticks(),
                    "steps": file["steps"].as_array().map_or(0, Vec::len),
                    "seed": unavailable("a game of your own takes its seed its own way: pass it with --launch-arg or --launch-env"),
                }),
            );
            Interaction::Scenario(file.clone())
        }
    };
    if o.scenario.is_none() {
        bundle.section(
            "scenario",
            json!({
                "id": reference::SCENARIO_ID,
                "seed": o.seed,
                "ticks": reference::TICKS,
                "steps": reference::STEPS.iter().map(|&(tick, pressed)| json!({"tick": tick, "event": "key", "key": "Space", "pressed": pressed})).collect::<Vec<_>>(),
            }),
        );
    }
    bundle.section(
        "settings",
        json!({
            "variant": if o.scenario.is_some() { Value::Null } else { json!(o.variant.name()) },
            "headless": o.launch.as_ref().is_none_or(|l| !l.windowed),
            "mode": if o.launch.is_some() { "launched" } else { "in-process" },
        }),
    );

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

    // The interaction: in this process, or in a launched game.
    let mut screenshots = unavailable(
        "headless run draws nothing; screenshots need a windowed run (--launch --windowed) and an inspected image",
    );
    let mut performance = unavailable("not collected: pass --perf (in-process) or --launch");
    let mut reported_features = None;
    let mut trace = None;
    let mut scenario_report = None;
    let mut log = match &o.scenario {
        None => vec![reference::banner(o.variant, o.seed)],
        Some(file) => vec![format!("scenario file: id={}", file["id"])],
    };
    if failure.is_some() {
        // A failed step: there is nothing to interact with.
    } else if let Some(launch) = &o.launch {
        // A game of your own gets only the arguments you gave it, not the
        // reference game's.
        let mut launch = launch.clone();
        if o.scenario.is_some() {
            launch.args.get_or_insert_default();
        }
        let seen = launched::observe(&launch, &interaction, o.variant, o.seed, &mut bundle, &mut commands)?;
        bundle.section("instance", seen.instance);
        (failure, trace, reported_features) = (seen.failure, seen.trace, seen.features);
        scenario_report = seen.scenario_report;
        (screenshots, performance) = (seen.screenshots, seen.performance);
        // The same log the in-process run writes, from the game's trace.
        log.extend(reference::jump_lines(trace.as_deref().unwrap_or_default()));
    } else {
        let run = reference::run(o.variant, o.seed, o.profile);
        // The run's own log already holds its assertion lines.
        log = run.log;
        log.truncate(log.len() - run.assertions.len());
        if let Some(report) = run.perf {
            bundle.text("performance/report.json", "performance", &(report + "\n"));
            performance = json!({
                "profile_headless": { "status": "collected", "artifact": "performance/report.json" },
                "note": "wall-clock measurements of this run on this machine; they differ between runs and are not compared",
            });
        }
        trace = Some(run.trace);
    }
    bundle.section("commands", Value::Array(commands));

    let features = o
        .features
        .clone()
        .or(reported_features.clone())
        .unwrap_or_else(compiled_features);
    let features_source = if o.features.is_some() {
        "supplied"
    } else if reported_features.is_some() {
        "reported by the launched game"
    } else {
        "compiled into the validation example"
    };
    let captured = |supplied: &Option<String>, flag: &str, find: &dyn Fn() -> Value| match supplied {
        Some(value) => json!(value),
        None if o.capture_identity => find(),
        None => unavailable(&format!("not supplied ({flag})")),
    };
    let build_dir = o.launch.as_ref().and_then(|l| l.cwd.clone());
    bundle.section(
        "build",
        json!({
            "engine_version": env!("CARGO_PKG_VERSION"),
            "commit": captured(&o.commit, "--commit", &|| identity::commit(build_dir.as_deref())),
            "features": features,
            "features_source": features_source,
            "rustc": captured(&o.rustc, "--rustc", &identity::rustc),
            "target": format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
            "debug_assertions": cfg!(debug_assertions),
        }),
    );

    let (assertions, ticks_run): (Vec<Assertion>, u64) = if let Some(trace) = &trace {
        let assertions = reference::check(trace);
        log.extend(reference::assertion_lines(&assertions));
        bundle.text("logs/run.log", "run-log", &(log.join("\n") + "\n"));
        let mut csv = String::from("tick,pressed,down,y,vy,jumps\n");
        for r in trace.iter().take(MAX_TRACE_ROWS) {
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                r.tick, r.pressed, r.down, r.y, r.vy, r.jumps
            ));
        }
        bundle.text("trace.csv", "trace", &csv);
        if failure.is_none()
            && let Some(bad) = assertions.iter().find(|a| !a.passed)
        {
            failure = Some(json!({
                "kind": "assertion",
                "assertion": bad.id,
                "tick": bad.tick,
                "log": "logs/run.log",
                "trace": "trace.csv",
            }));
        }
        let ticks = trace.len() as u64;
        (assertions, ticks)
    } else if let Some(report) = &scenario_report {
        // A scenario file: its checks are the assertions, and the game's
        // report says which one failed, where, and what it found instead.
        let bad = &report["failure"];
        let passed = report["status"] == "passed";
        let checks = Assertion {
            id: "scenario-checks".into(),
            description: "every check step of the scenario file holds on its tick".into(),
            passed,
            tick: bad["tick"].as_u64(),
            expected: if bad.is_object() {
                format!(
                    "step {} ({}): {}",
                    bad["step"],
                    bad["check"].as_str().unwrap_or("?"),
                    bad["expected"]
                )
            } else {
                "all checks hold".into()
            },
            actual: if bad.is_object() {
                bad["actual"].to_string()
            } else {
                format!("{} checks held; status {}", report["checks_passed"], report["status"])
            },
        };
        log.push(format!(
            "scenario: status={} ticks_run={} checks_passed={}",
            report["status"], report["ticks_run"], report["checks_passed"]
        ));
        log.extend(reference::assertion_lines(std::slice::from_ref(&checks)));
        bundle.text("logs/run.log", "run-log", &(log.join("\n") + "\n"));
        if failure.is_none() && !passed {
            failure = Some(json!({
                "kind": "assertion",
                "assertion": checks.id,
                "tick": checks.tick,
                "step": bad["step"],
                "message": bad["message"],
                "log": "logs/run.log",
                "report": "scenario-report.json",
            }));
        }
        (vec![checks], report["ticks_run"].as_u64().unwrap_or(0))
    } else {
        (Vec::new(), 0)
    };
    bundle.section(
        "assertions",
        if trace.is_none() && scenario_report.is_none() {
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
            "screenshots": screenshots,
            "performance": performance,
            "wall_time": unavailable("omitted so that identical runs produce identical bundles"),
        }),
    );

    let manifest = bundle.write(&o.out)?;
    Ok(Report {
        passed: manifest["result"]["outcome"] == "passed",
        manifest,
    })
}
