//! The launched flow: build the game, start it as its own process and drive
//! it through the MCP server's tools (`launch`, `run_input_scenario`,
//! `run_ticks`, `get_resource`, `read_logs`, `system_timings`,
//! `perf_report`, `screenshot`, `quit_instance`), exactly as an agent would.
//!
//! The server runs in this process ([`ponko_mcp::Server`]) and is spoken to
//! in MCP's JSON-RPC, so every call here is one an MCP client could make.
//! Each failure on the way (the build, the start, the scenario) ends the
//! run with a bounded artifact that says what failed.

use crate::bundle::{Bundle, CommandSpec, CommandStatus, bounded_text, run_command, unavailable};
use crate::reference::{self, Row, Variant};
use ponko_mcp::{Config, Server};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How the game is built and started.
#[derive(Clone, Debug)]
pub struct Launch {
    /// An executable that is already built. `None`: `cargo run`.
    pub program: Option<String>,
    /// The cargo project (or the directory `program` starts in).
    pub cwd: Option<String>,
    /// `cargo run --example <name>` (the default: this example).
    pub example: Option<String>,
    /// `cargo run --bin <name>`.
    pub bin: Option<String>,
    pub release: bool,
    /// Arguments for the game. `None`: serve the reference game.
    pub args: Option<Vec<String>>,
    /// Extra environment for the game. Only the names are recorded.
    pub env: Vec<(String, String)>,
    /// Build with `cargo build` first, so a build failure is told apart
    /// from a game that doesn't start. Only without `program`.
    pub build: bool,
    pub build_timeout: Duration,
    pub launch_timeout: Duration,
    /// In a window (a screenshot is taken) instead of headless.
    pub windowed: bool,
    /// Kept bytes of the build's and the game's output.
    pub output_limit: usize,
}

impl Launch {
    /// `cargo run --example validation_bundle`, headless.
    pub fn cargo() -> Self {
        Self {
            program: None,
            cwd: None,
            example: Some("validation_bundle".into()),
            bin: None,
            release: false,
            args: None,
            env: Vec::new(),
            build: true,
            build_timeout: Duration::from_secs(1800),
            launch_timeout: Duration::from_secs(600),
            windowed: false,
            output_limit: 64 * 1024,
        }
    }

    /// An executable that is already built.
    pub fn program(path: impl Into<String>) -> Self {
        Self {
            program: Some(path.into()),
            example: None,
            build: false,
            launch_timeout: Duration::from_secs(60),
            ..Self::cargo()
        }
    }

    fn game_args(&self, variant: Variant, seed: u64) -> Vec<String> {
        self.args.clone().unwrap_or_else(|| {
            ["--serve", "--variant", variant.name(), "--seed", &seed.to_string()]
                .map(String::from)
                .to_vec()
        })
    }

    /// The command line that repeats the launch, for the manifest.
    pub fn describe(&self, variant: Variant, seed: u64) -> Vec<String> {
        let mut argv = self.cargo_argv("run");
        if self.program.is_none() {
            argv.push("--".into());
        }
        argv.extend(self.game_args(variant, seed));
        argv
    }

    fn cargo_argv(&self, verb: &str) -> Vec<String> {
        if let Some(program) = &self.program {
            return vec![program.clone()];
        }
        let mut argv = vec!["cargo".to_string(), verb.to_string()];
        if self.release {
            argv.push("--release".into());
        }
        for (flag, value) in [("--example", &self.example), ("--bin", &self.bin)] {
            if let Some(v) = value {
                argv.extend([flag.to_string(), v.clone()]);
            }
        }
        argv
    }
}

/// What the launched game is put through.
#[derive(Clone, Debug)]
pub enum Interaction {
    /// The reference game's hold-jump scenario; its `Trace` resource is
    /// read back and checked.
    Reference,
    /// A scenario file for a game of your own (any language): its input
    /// steps are fed in and its check steps are the assertions.
    Scenario(Value),
}

/// What the launched run produced.
pub struct Observed {
    /// The reference game's per-tick trace, when the run got that far.
    pub trace: Option<Vec<Row>>,
    /// The game's report on a scenario file, when one ran to its end or to
    /// a failed check.
    pub scenario_report: Option<Value>,
    pub failure: Option<Value>,
    /// The manifest's `metrics.screenshots` and `metrics.performance`.
    pub screenshots: Value,
    pub performance: Value,
    /// What the game said about itself (`instance`), for the manifest.
    pub instance: Value,
    /// Features the launched game reported, when it did.
    pub features: Option<Vec<String>>,
}

/// An MCP client for the in-process server: `tools/call` requests in
/// JSON-RPC, and a record of every call for the manifest.
struct Mcp {
    server: Server,
    next: u64,
    calls: Vec<Value>,
}

impl Mcp {
    fn tool(&mut self, name: &str, arguments: Value) -> Result<Value, String> {
        self.next += 1;
        let request = json!({
            "jsonrpc": "2.0", "id": self.next, "method": "tools/call",
            "params": { "name": name, "arguments": arguments },
        });
        let reply = self
            .server
            .handle(&request)
            .ok_or_else(|| format!("{name}: the MCP server sent no reply"))?;
        let result = &reply["result"];
        let content = result["content"].as_array().cloned().unwrap_or_default();
        let text = content
            .iter()
            .find(|c| c["type"] == "text")
            .and_then(|c| c["text"].as_str())
            .unwrap_or("")
            .to_string();
        let ok = !reply["error"].is_object() && result["isError"] != true;
        // What was asked, without the parts that are recorded elsewhere or
        // differ on every run (the scenario, the instance's process id).
        let mut shown = arguments;
        if let Some(map) = shown.as_object_mut() {
            if map.contains_key("instance") {
                map.insert("instance".into(), json!("<launched>"));
            }
            if map.contains_key("scenario") {
                map.insert("scenario".into(), json!("<the manifest's scenario>"));
            }
            // Values may be secrets, and this run's own directory is gone
            // when it ends: names only.
            if let Some(env) = map.get("env").and_then(Value::as_object) {
                let names: Vec<&String> = env.keys().collect();
                map.insert("env".into(), json!({ "names": names }));
            }
            if let Some(file) = map.get("path").and_then(Value::as_str) {
                let name = Path::new(file)
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                map.insert("path".into(), json!(format!("<scratch>/{name}")));
            }
        }
        self.calls
            .push(json!({ "role": "mcp", "tool": name, "arguments": shown, "ok": ok }));
        if !ok {
            let message = if text.is_empty() {
                reply["error"]["message"].as_str().unwrap_or("failed").to_string()
            } else {
                text
            };
            return Err(message);
        }
        // Tools answer with JSON as text; an image tool with a line of prose.
        Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
    }
}

/// A directory of this run's own (the instance registry, the game's output,
/// the screenshot before it is copied), removed when the run ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> std::io::Result<Self> {
        // Two runs in one process (tests run side by side) can start within
        // the clock's resolution, so the time alone doesn't tell them apart.
        static RUNS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let run = RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        let dir = std::env::temp_dir().join(format!("ponko-validation-{}-{nanos}-{run}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        Ok(Self(std::fs::canonicalize(&dir).unwrap_or(dir)))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The first line of a tool's error, bounded: the rest is in the log.
fn headline(message: &str) -> String {
    let line = message.lines().next().unwrap_or("");
    line.chars().take(300).collect()
}

/// Everything the game process wrote (stdout and stderr), bounded.
fn process_output(logs: &Path, limit: usize) -> Option<(String, u64)> {
    let mut all = Vec::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(logs).ok()?.flatten().map(|e| e.path()).collect();
    files.sort();
    for file in files {
        all.extend(std::fs::read(file).unwrap_or_default());
    }
    Some(bounded_text(&all, limit))
}

/// Builds, launches and drives the game. Files go into `bundle`, the calls
/// made into `commands`.
pub fn observe(
    launch: &Launch,
    interaction: &Interaction,
    variant: Variant,
    seed: u64,
    bundle: &mut Bundle,
    commands: &mut Vec<Value>,
) -> std::io::Result<Observed> {
    let mut seen = Observed {
        trace: None,
        scenario_report: None,
        failure: None,
        screenshots: unavailable("the run ended before a frame could be captured"),
        performance: unavailable("the run ended before a report could be collected"),
        instance: unavailable("the game did not start"),
        features: None,
    };

    // 1. The build, apart from the launch: its failure is its own kind.
    if launch.build && launch.program.is_none() {
        let mut spec = CommandSpec::from_argv(launch.cargo_argv("build"));
        if let Some(cargo) = std::env::var_os("CARGO") {
            spec.argv[0] = cargo.to_string_lossy().into_owned();
        }
        spec.cwd = launch.cwd.clone();
        let out = run_command(&spec, launch.build_timeout, launch.output_limit);
        let mut log = format!("$ {}\n", launch.cargo_argv("build").join(" "));
        log.push_str(&format!(
            "--- stdout ---\n{}\n--- stderr ---\n{}\n",
            out.stdout, out.stderr
        ));
        if let CommandStatus::LaunchFailed(e) = &out.status {
            log.push_str(&format!("--- could not start: {e} ---\n"));
        }
        bundle.text("logs/build.log", "step-log", &log);
        commands.push(json!({
            "role": "build",
            "argv": launch.cargo_argv("build"),
            "cwd": launch.cwd,
            "status": out.status.name(),
            "exit_code": match &out.status { CommandStatus::Failed(c) => json!(c), _ => Value::Null },
            "timeout_secs": launch.build_timeout.as_secs(),
            "omitted_output_bytes": out.omitted_bytes,
            "log": "logs/build.log",
        }));
        if out.status != CommandStatus::Passed {
            seen.failure = Some(json!({
                "kind": if out.status == CommandStatus::TimedOut { "build_timeout" } else { "build_failed" },
                "status": out.status.name(),
                "log": "logs/build.log",
            }));
            return Ok(seen);
        }
    }

    // 2. The launch, through the MCP tool, with a registry and a log
    // directory of this run's own.
    let scratch = Scratch::new()?;
    let logs = scratch.0.join("logs");
    let mut mcp = Mcp {
        server: Server::new(Config {
            instances_dir: scratch.0.join("instances"),
            logs_dir: logs.clone(),
            read_only: false,
        }),
        next: 0,
        calls: Vec::new(),
    };
    let mut arguments = json!({
        "release": launch.release,
        "headless": !launch.windowed,
        "paused": true,
        "args": launch.game_args(variant, seed),
        "timeout_secs": launch.launch_timeout.as_secs().max(1),
    });
    if !launch.env.is_empty() {
        arguments["env"] = Value::Object(launch.env.iter().map(|(k, v)| (k.clone(), json!(v))).collect());
    }
    for (key, value) in [
        ("program", &launch.program),
        ("cwd", &launch.cwd),
        ("example", &launch.example),
        ("bin", &launch.bin),
    ] {
        if let Some(v) = value {
            arguments[key] = json!(v);
        }
    }
    let launched = mcp.tool("launch", arguments);
    let result = match launched {
        Ok(instance) => drive(&mut mcp, &instance, launch, interaction, &scratch.0, bundle, &mut seen),
        Err(message) => {
            let kind = if message.starts_with("timed out") {
                "launch_timeout"
            } else {
                "launch_failed"
            };
            seen.failure = Some(json!({
                "kind": kind,
                "message": headline(&message),
                "timeout_secs": launch.launch_timeout.as_secs(),
                "log": "logs/process-output.log",
            }));
            Ok(())
        }
    };
    // 3. What the process wrote, whatever happened: after the game is gone,
    // so the file is complete.
    let (text, omitted) = process_output(&logs, launch.output_limit).unwrap_or_default();
    bundle.text(
        "logs/process-output.log",
        "process-output",
        &format!("$ {}\n{text}\n", launch.describe(variant, seed).join(" ")),
    );
    commands.push(json!({
        "role": "launch",
        "argv": launch.describe(variant, seed),
        "cwd": launch.cwd,
        "env_names": launch.env.iter().map(|(k, _)| k).collect::<Vec<_>>(),
        "headless": !launch.windowed,
        "timeout_secs": launch.launch_timeout.as_secs(),
        "omitted_output_bytes": omitted,
        "log": "logs/process-output.log",
    }));
    commands.append(&mut mcp.calls);
    if let Err(message) = result
        && seen.failure.is_none()
    {
        seen.failure = Some(json!({
            "kind": "tool_failed",
            "message": headline(&message),
            "log": "logs/process-output.log",
        }));
    }
    Ok(seen)
}

/// The run itself, on a started game. Always asks the game to quit.
fn drive(
    mcp: &mut Mcp,
    instance: &Value,
    launch: &Launch,
    interaction: &Interaction,
    scratch: &Path,
    bundle: &mut Bundle,
    seen: &mut Observed,
) -> Result<(), String> {
    let id = instance["id"].as_str().unwrap_or("").to_string();
    let result = interact(mcp, &id, instance, launch, interaction, scratch, bundle, seen);
    let _ = mcp.tool("quit_instance", json!({ "instance": id }));
    // Its output file is complete once it is gone.
    let registry = scratch.join("instances");
    let gone = Instant::now() + Duration::from_secs(10);
    while Instant::now() < gone && std::fs::read_dir(&registry).is_ok_and(|mut entries| entries.next().is_some()) {
        std::thread::sleep(Duration::from_millis(20));
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn interact(
    mcp: &mut Mcp,
    id: &str,
    instance: &Value,
    launch: &Launch,
    interaction: &Interaction,
    scratch: &Path,
    bundle: &mut Bundle,
    seen: &mut Observed,
) -> Result<(), String> {
    let at = |extra: Value| {
        let mut args = json!({ "instance": id });
        if let (Some(args), Some(extra)) = (args.as_object_mut(), extra.as_object()) {
            args.extend(extra.clone());
        }
        args
    };
    // Who answered: what identifies the build, not the process.
    seen.instance = json!({
        "name": instance["name"],
        "engine_version": instance["engine_version"],
        "headless": instance["headless"],
        "exe": instance["exe"],
        "cwd": instance["cwd"],
    });
    if let Ok(capabilities) = mcp.tool("instance_capabilities", at(json!({}))) {
        let features = capabilities["features"].as_object().map(|map| {
            let mut on: Vec<String> = map
                .iter()
                .filter(|(_, v)| **v == json!(true))
                .map(|(k, _)| k.clone())
                .collect();
            on.sort();
            on
        });
        seen.features = features;
    }

    // The scenario, through the game's real input path.
    let scenario = match interaction {
        Interaction::Reference => reference::scenario().to_json(),
        Interaction::Scenario(file) => file.clone(),
    };
    let ran = mcp.tool(
        "run_input_scenario",
        at(json!({ "scenario": scenario, "timeout_secs": 120 })),
    )?;
    bundle.json("scenario-report.json", "scenario-report", &ran["report"]);
    match interaction {
        // A scenario file's checks are the assertions: the report is the result.
        Interaction::Scenario(_) => seen.scenario_report = Some(ran["report"].clone()),
        Interaction::Reference => {
            if ran["passed"] != true {
                seen.failure = Some(json!({
                    "kind": "scenario_failed",
                    "status": ran["report"]["status"],
                    "report": "scenario-report.json",
                }));
                return Ok(());
            }
            // The rest of the run, then what the game observed.
            let tick = mcp.tool("instance_info", at(json!({})))?["tick"].as_u64().unwrap_or(0);
            if tick < reference::TICKS {
                mcp.tool(
                    "run_ticks",
                    at(json!({ "ticks": reference::TICKS - tick, "timeout_secs": 120 })),
                )?;
            }
            let trace = mcp.tool("get_resource", at(json!({ "resource": "Trace" })))?;
            let rows: Vec<Row> = serde_json::from_value(trace["rows"].clone())
                .map_err(|e| format!("the game's Trace resource isn't a list of rows: {e}"))?;
            seen.trace = Some(rows);
        }
    }
    // What it printed.
    match mcp.tool("read_logs", at(json!({ "lines": 2000 }))) {
        Ok(logs) => bundle.text(
            "logs/game.log",
            "game-log",
            &format!("{}\n", logs["text"].as_str().unwrap_or("")),
        ),
        Err(e) => bundle.text(
            "logs/game.log",
            "game-log",
            &format!("[read_logs failed: {}]\n", headline(&e)),
        ),
    }

    // Performance: what the tools report, kept as they gave it.
    let mut performance = serde_json::Map::new();
    for (tool, path) in [
        ("system_timings", "performance/system-timings.json"),
        ("perf_report", "performance/report.json"),
    ] {
        match mcp.tool(tool, at(json!({}))) {
            Ok(report) => {
                bundle.json(path, "performance", &report);
                performance.insert(tool.into(), json!({ "status": "collected", "artifact": path }));
            }
            Err(e) => {
                performance.insert(tool.into(), unavailable(&headline(&e)));
            }
        }
    }
    performance.insert(
        "note".into(),
        json!("wall-clock measurements of this run on this machine; they differ between runs and are not compared"),
    );
    seen.performance = Value::Object(performance);

    // A frame, when there is a window to draw one.
    seen.screenshots = if launch.windowed {
        let file = scratch.join("final.png");
        match mcp.tool("screenshot", at(json!({ "path": file }))) {
            Ok(_) => match std::fs::read(&file) {
                Ok(png) => {
                    bundle.bytes("screenshots/final.png", "screenshot", png);
                    json!({
                        "status": "captured",
                        "images": ["screenshots/final.png"],
                        "inspection": "required: an image is evidence only after someone has looked at it (see the inspect command); it is never compared pixel by pixel",
                    })
                }
                Err(e) => unavailable(&format!("the game reported a screenshot, but it can't be read: {e}")),
            },
            Err(e) => unavailable(&headline(&e)),
        }
    } else {
        // Asked anyway, so the bundle holds the game's own answer.
        match mcp.tool("screenshot", at(json!({ "path": scratch.join("final.png") }))) {
            Ok(_) => unavailable("a headless game reported a frame; not kept, because headless runs draw nothing"),
            Err(e) => unavailable(&format!("headless run: {}", headline(&e))),
        }
    };
    Ok(())
}
