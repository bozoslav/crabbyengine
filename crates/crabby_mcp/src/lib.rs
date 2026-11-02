//! `crabby-mcp`: a [Model Context Protocol](https://modelcontextprotocol.io)
//! server that lets AI agents inspect and drive running Crabby Engine games.
//!
//! Agents launch it over stdio. It finds games through the instance registry
//! (`~/.crabby/instances`) that every game with the control endpoint on
//! writes to, and talks to each game's endpoint over localhost TCP. Every
//! tool takes an `instance`, so several agents, each with its own
//! `crabby-mcp`, can work with several games at once.
//!
//! The protocol logic is in [`Server::handle`], which maps one JSON-RPC
//! message to its reply, so it can be tested without stdio.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

/// MCP protocol versions this server speaks, newest first.
const PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// How long to wait for a TCP connection to an instance.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// How long a single endpoint call may take.
const CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Where games register: `$CRABBY_INSTANCES_DIR`, else `~/.crabby/instances`
/// (`%LOCALAPPDATA%\crabby\instances` on Windows). Matches the engine.
///
/// ```
/// let dir = crabby_mcp::default_instances_dir();
/// assert!(dir.is_none_or(|d| d.ends_with("instances") || std::env::var_os("CRABBY_INSTANCES_DIR").is_some()));
/// ```
pub fn default_instances_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("CRABBY_INSTANCES_DIR") {
        return Some(PathBuf::from(dir));
    }
    crabby_home().map(|h| h.join("instances"))
}

fn crabby_home() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("crabby"))
    } else {
        std::env::var_os("HOME").map(|d| PathBuf::from(d).join(".crabby"))
    }
}

/// Server settings.
///
/// ```
/// let mut config = crabby_mcp::Config::from_env();
/// config.read_only = true; // only tools that don't change games
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    /// Where to look for running instances.
    pub instances_dir: PathBuf,
    /// Where `launch` writes game logs.
    pub logs_dir: PathBuf,
    /// Hide every tool that changes a game (spawn, set, step, launch, ...).
    pub read_only: bool,
}

impl Config {
    /// The default directories, writable tools on.
    pub fn from_env() -> Self {
        let home = crabby_home().unwrap_or_else(|| std::env::temp_dir().join("crabby"));
        Self {
            instances_dir: default_instances_dir().unwrap_or_else(|| home.join("instances")),
            logs_dir: home.join("logs"),
            read_only: false,
        }
    }
}

/// One running game, from its registry file.
#[derive(Debug, Clone)]
struct Instance {
    info: Value,
    file: PathBuf,
}

impl Instance {
    fn name(&self) -> &str {
        self.info["name"].as_str().unwrap_or("")
    }
    fn pid(&self) -> u64 {
        self.info["pid"].as_u64().unwrap_or(0)
    }
    /// `<pid>-<port>`: unique even when one process runs several games.
    fn id(&self) -> String {
        format!("{}-{}", self.pid(), self.port())
    }
    fn port(&self) -> u16 {
        self.info["port"].as_u64().unwrap_or(0) as u16
    }
    fn addr(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.port()))
    }
    /// The registry entry without the token: what agents get to see.
    fn public(&self) -> Value {
        let mut v = self.info.clone();
        if let Some(o) = v.as_object_mut() {
            o.remove("token");
            o.insert("id".into(), json!(self.id()));
        }
        v
    }

    fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let unreachable = |e: std::io::Error| format!("instance `{}` is not reachable: {e}", self.name());
        let stream = TcpStream::connect_timeout(&self.addr(), CONNECT_TIMEOUT).map_err(unreachable)?;
        stream.set_read_timeout(Some(CALL_TIMEOUT)).map_err(unreachable)?;
        let mut writer = stream.try_clone().map_err(unreachable)?;
        let msg = json!({ "id": 1, "token": self.info["token"], "method": method, "params": params });
        writeln!(writer, "{msg}").map_err(unreachable)?;
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).map_err(unreachable)?;
        let reply: Value = serde_json::from_str(&line).map_err(|e| format!("bad reply from the game: {e}"))?;
        match reply.get("error") {
            Some(e) => Err(e.as_str().unwrap_or("unknown error").to_string()),
            None => Ok(reply["result"].clone()),
        }
    }
}

/// Reads every registry file. Entries whose endpoint refuses connections
/// belong to games that crashed; their files are removed.
fn discover(dir: &Path) -> Vec<Instance> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Some(info) = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        else {
            continue; // being written right now, or not ours
        };
        let inst = Instance { info, file: path };
        if TcpStream::connect_timeout(&inst.addr(), Duration::from_millis(300)).is_err() {
            let _ = std::fs::remove_file(&inst.file);
            continue;
        }
        out.push(inst);
    }
    out.sort_by_key(|i| i.info["started"].as_u64().unwrap_or(0));
    out
}

fn random_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
    );
    format!("{:016x}", h.finish())
}

// --------------------------------------------------------------------- tools

struct Tool {
    name: &'static str,
    description: &'static str,
    /// Changes the game (hidden with `--read-only`).
    writes: bool,
    /// JSON schema properties, besides `instance`.
    params: fn() -> Value,
    required: &'static [&'static str],
}

const INSTANCE_DOC: &str =
    "Which game: its id, pid, port or name from list_instances. Optional when exactly one game is running.";

fn no_params() -> Value {
    json!({})
}

fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list_instances",
            description: "List running Crabby Engine games that have the control endpoint on (started with CRABBY_CONTROL=1 or launched with `launch`). Shows id, name, pid, port, headless, cwd, start time.",
            writes: false,
            params: no_params,
            required: &[],
        },
        Tool {
            name: "instance_info",
            description: "Live status of a game: simulation tick and time, paused, speed, fps, entity count.",
            writes: false,
            params: no_params,
            required: &[],
        },
        Tool {
            name: "world_summary",
            description: "Overview of a game's world: entity count, archetypes (which component sets exist and how many entities each has), and which components are reflected (readable/writable as JSON).",
            writes: false,
            params: no_params,
            required: &[],
        },
        Tool {
            name: "list_entities",
            description: "List entities with their reflected components as JSON and the names of their other components. Filter with `with` (component names); page with `limit`/`offset`.",
            writes: false,
            params: || {
                json!({
                    "with": { "type": "array", "items": { "type": "string" }, "description": "Only entities that have all these components, e.g. [\"Transform2D\", \"Health\"]" },
                    "limit": { "type": "integer", "minimum": 0, "description": "Max entities to return (default 100, max 10000)" },
                    "offset": { "type": "integer", "minimum": 0, "description": "Skip this many matching entities first" }
                })
            },
            required: &[],
        },
        Tool {
            name: "get_entity",
            description: "All reflected components of one entity, as JSON keyed by component name.",
            writes: false,
            params: || json!({ "entity": { "type": "integer", "description": "Entity id from list_entities" } }),
            required: &["entity"],
        },
        Tool {
            name: "system_timings",
            description: "How long each system took (last frame and smoothed average, in ms), simulation systems first. Use it to find what's slow.",
            writes: false,
            params: no_params,
            required: &[],
        },
        Tool {
            name: "read_logs",
            description: "The last lines of a game's stdout/stderr. Works for games started with `launch`.",
            writes: false,
            params: || json!({ "lines": { "type": "integer", "minimum": 1, "description": "How many lines from the end (default 100)" } }),
            required: &[],
        },
        Tool {
            name: "launch",
            description: "Build and start a game or example with `cargo run`, with the control endpoint on, and wait until it is ready. Starts headless and paused by default, so you can inspect it and step it with run_ticks. Returns the new instance.",
            writes: true,
            params: || {
                json!({
                    "cwd": { "type": "string", "description": "Cargo project directory (default: this server's working directory)" },
                    "example": { "type": "string", "description": "Run `--example <name>`" },
                    "bin": { "type": "string", "description": "Run `--bin <name>`" },
                    "package": { "type": "string", "description": "Run `-p <package>`" },
                    "release": { "type": "boolean", "description": "Release build (default true; debug is 10-50x slower)" },
                    "headless": { "type": "boolean", "description": "Set HEADLESS=1, which the engine's examples read (default true)" },
                    "paused": { "type": "boolean", "description": "Start with the simulation paused (default true)" },
                    "env": { "type": "object", "additionalProperties": { "type": "string" }, "description": "Extra environment variables, e.g. {\"BOIDS\": \"100000\"}" },
                    "args": { "type": "array", "items": { "type": "string" }, "description": "Arguments passed to the game" },
                    "timeout_secs": { "type": "integer", "minimum": 1, "description": "How long to wait for build + start (default 600)" }
                })
            },
            required: &[],
        },
        Tool {
            name: "run_ticks",
            description: "Run exactly N simulation ticks (the game should be paused) and wait for them to finish. Returns the new status. Use it to advance a simulation step by step and check results.",
            writes: true,
            params: || {
                json!({
                    "ticks": { "type": "integer", "minimum": 1, "description": "How many ticks" },
                    "timeout_secs": { "type": "integer", "minimum": 1, "description": "How long to wait (default 120)" }
                })
            },
            required: &["ticks"],
        },
        Tool {
            name: "pause",
            description: "Pause the simulation (rendering and input keep running in windowed games).",
            writes: true,
            params: no_params,
            required: &[],
        },
        Tool {
            name: "resume",
            description: "Resume the simulation.",
            writes: true,
            params: no_params,
            required: &[],
        },
        Tool {
            name: "set_speed",
            description: "Set simulation speed: 1 real time, 4 four times as fast, 0.5 half. Results don't depend on speed.",
            writes: true,
            params: || json!({ "speed": { "type": "number", "minimum": 0 } }),
            required: &["speed"],
        },
        Tool {
            name: "spawn_entity",
            description: "Spawn an entity from reflected components, e.g. {\"Transform2D\": {\"position\": [10, 20]}, \"Health\": {\"current\": 5, \"max\": 5}}. Returns its id.",
            writes: true,
            params: || json!({ "components": { "type": "object", "description": "Component name -> JSON value" } }),
            required: &["components"],
        },
        Tool {
            name: "despawn_entity",
            description: "Despawn an entity.",
            writes: true,
            params: || json!({ "entity": { "type": "integer" } }),
            required: &["entity"],
        },
        Tool {
            name: "set_component",
            description: "Write a reflected component of an entity (adds it if missing). For an existing component, object fields you give are changed and the rest keep their values, e.g. {\"current\": 1}. Returns the new value.",
            writes: true,
            params: || {
                json!({
                    "entity": { "type": "integer" },
                    "component": { "type": "string", "description": "Component name, e.g. \"Transform2D\"" },
                    "value": { "description": "JSON value (or partial object)" }
                })
            },
            required: &["entity", "component", "value"],
        },
        Tool {
            name: "remove_component",
            description: "Remove a reflected component from an entity.",
            writes: true,
            params: || json!({ "entity": { "type": "integer" }, "component": { "type": "string" } }),
            required: &["entity", "component"],
        },
        Tool {
            name: "quit_instance",
            description: "Ask a game to exit.",
            writes: true,
            params: no_params,
            required: &[],
        },
    ]
}

fn tool_schema(t: &Tool) -> Value {
    let mut props = (t.params)();
    if t.name != "list_instances" && t.name != "launch" {
        props.as_object_mut().unwrap().insert(
            "instance".into(),
            json!({ "type": ["string", "integer"], "description": INSTANCE_DOC }),
        );
    }
    json!({
        "name": t.name,
        "description": t.description,
        "inputSchema": { "type": "object", "properties": props, "required": t.required },
        "annotations": { "readOnlyHint": !t.writes, "destructiveHint": matches!(t.name, "despawn_entity" | "remove_component" | "quit_instance") },
    })
}

// -------------------------------------------------------------------- server

/// The MCP server state. One per agent connection.
///
/// ```
/// use crabby_mcp::{Config, Server};
/// use serde_json::json;
///
/// let mut server = Server::new(Config::from_env());
/// let reply = server.handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" })).unwrap();
/// assert!(reply["result"]["tools"].as_array().unwrap().iter().any(|t| t["name"] == "list_instances"));
/// ```
pub struct Server {
    config: Config,
    /// Log files of games this server launched, by launch id.
    launched: HashMap<String, PathBuf>,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            launched: HashMap::new(),
        }
    }

    /// Handles one JSON-RPC message. Returns the reply, or `None` for
    /// notifications (which get no reply).
    pub fn handle(&mut self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let result = match method {
            "initialize" => Ok(self.initialize(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": self.visible_tools().iter().map(tool_schema).collect::<Vec<_>>() })),
            "tools/call" => Ok(self.call_tool(&params)),
            m if m.starts_with("notifications/") => return None,
            m => Err((-32601, format!("method not found: {m}"))),
        };
        let id = id?; // a request without id is a notification: no reply
        Some(match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err((code, message)) => {
                json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
            }
        })
    }

    fn initialize(&self, params: &Value) -> Value {
        let asked = params["protocolVersion"].as_str().unwrap_or(PROTOCOL_VERSIONS[0]);
        let version = PROTOCOL_VERSIONS
            .iter()
            .find(|v| **v == asked)
            .unwrap_or(&PROTOCOL_VERSIONS[0]);
        json!({
            "protocolVersion": version,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "crabby-mcp", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Tools for Crabby Engine games. Start with list_instances; if none is running, use launch (headless and paused by default). Then world_summary and list_entities to look around, run_ticks to advance the simulation, and get_entity / set_component to check and change state. Every tool except list_instances and launch takes `instance`, optional when only one game runs. Components must be reflected in the game (world.reflect::<T>()) to be read or written as JSON."
        })
    }

    fn visible_tools(&self) -> Vec<Tool> {
        tools()
            .into_iter()
            .filter(|t| !(self.config.read_only && t.writes))
            .collect()
    }

    fn call_tool(&mut self, params: &Value) -> Value {
        let name = params["name"].as_str().unwrap_or("");
        let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
        let result = match self.visible_tools().iter().find(|t| t.name == name) {
            None if tools().iter().any(|t| t.name == name) => {
                Err(format!("`{name}` changes the game and this server is read-only"))
            }
            None => Err(format!("unknown tool `{name}`")),
            Some(_) => self.run_tool(name, &args),
        };
        match result {
            Ok(v) => json!({
                "content": [{ "type": "text", "text": serde_json::to_string_pretty(&v).unwrap() }],
                "isError": false,
            }),
            Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
        }
    }

    /// Finds the instance an agent means.
    fn instance(&self, args: &Value) -> Result<Instance, String> {
        let all = discover(&self.config.instances_dir);
        let wanted = match args.get("instance") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(v) => Some(v.to_string()),
        };
        let Some(wanted) = wanted else {
            return match all.len() {
                1 => Ok(all.into_iter().next().unwrap()),
                0 => Err("no game is running with the control endpoint on; start one with `launch`, or run it with CRABBY_CONTROL=1".into()),
                n => Err(format!(
                    "{n} games are running; pass `instance` (one of: {})",
                    all.iter().map(|i| format!("{} \"{}\"", i.id(), i.name())).collect::<Vec<_>>().join(", ")
                )),
            };
        };
        let mut matches: Vec<Instance> = all
            .iter()
            .filter(|i| i.id() == wanted || i.pid().to_string() == wanted || i.port().to_string() == wanted)
            .cloned()
            .collect();
        if matches.is_empty() {
            matches = all.iter().filter(|i| i.name() == wanted).cloned().collect();
        }
        match matches.len() {
            1 => Ok(matches.pop().unwrap()),
            0 => Err(format!("no running instance `{wanted}`; see list_instances")),
            _ => Err(format!(
                "several instances match `{wanted}`; pass the id from list_instances"
            )),
        }
    }

    fn run_tool(&mut self, name: &str, args: &Value) -> Result<Value, String> {
        let pass = |keys: &[&str]| {
            let mut m = Map::new();
            for k in keys {
                if let Some(v) = args.get(*k) {
                    m.insert((*k).into(), v.clone());
                }
            }
            Value::Object(m)
        };
        match name {
            "list_instances" => Ok(Value::Array(
                discover(&self.config.instances_dir)
                    .iter()
                    .map(Instance::public)
                    .collect(),
            )),
            "instance_info" => {
                let i = self.instance(args)?;
                let mut info = i.call("info", json!({}))?;
                info["name"] = json!(i.name());
                Ok(info)
            }
            "world_summary" => self.instance(args)?.call("world.summary", json!({})),
            "list_entities" => self
                .instance(args)?
                .call("entities.list", pass(&["with", "limit", "offset"])),
            "get_entity" => self.instance(args)?.call("entity.get", pass(&["entity"])),
            "system_timings" => self.instance(args)?.call("systems.timings", json!({})),
            "read_logs" => self.read_logs(args),
            "launch" => self.launch(args),
            "run_ticks" => self.run_ticks(args),
            "pause" => self.instance(args)?.call("sim.pause", json!({})),
            "resume" => self.instance(args)?.call("sim.resume", json!({})),
            "set_speed" => self.instance(args)?.call("sim.speed", pass(&["speed"])),
            "spawn_entity" => self.instance(args)?.call("entity.spawn", pass(&["components"])),
            "despawn_entity" => self.instance(args)?.call("entity.despawn", pass(&["entity"])),
            "set_component" => self
                .instance(args)?
                .call("component.set", pass(&["entity", "component", "value"])),
            "remove_component" => self
                .instance(args)?
                .call("component.remove", pass(&["entity", "component"])),
            "quit_instance" => self.instance(args)?.call("game.quit", json!({})),
            _ => Err(format!("unknown tool `{name}`")),
        }
    }

    fn run_ticks(&self, args: &Value) -> Result<Value, String> {
        let ticks = args["ticks"]
            .as_u64()
            .filter(|t| *t > 0)
            .ok_or("`ticks` must be a positive integer")?;
        let timeout = Duration::from_secs(args["timeout_secs"].as_u64().unwrap_or(120));
        let i = self.instance(args)?;
        let before = i.call("info", json!({}))?;
        if before["paused"] != json!(true) {
            return Err("the simulation is running; call `pause` first so exactly N ticks run".into());
        }
        let start = before["tick"].as_u64().unwrap_or(0);
        i.call("sim.step", json!({ "ticks": ticks }))?;
        let deadline = Instant::now() + timeout;
        let begun = Instant::now();
        loop {
            let info = i.call("info", json!({}))?;
            let now = info["tick"].as_u64().unwrap_or(0);
            if now >= start + ticks {
                return Ok(json!({
                    "ticks_run": now - start,
                    "seconds": begun.elapsed().as_secs_f64(),
                    "status": info,
                }));
            }
            if Instant::now() > deadline {
                return Err(format!("only {} of {ticks} ticks ran within the timeout", now - start));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn launch(&mut self, args: &Value) -> Result<Value, String> {
        let flag = |k: &str| args[k].as_bool().unwrap_or(true);
        let cwd = args["cwd"]
            .as_str()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let mut cmd = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
        cmd.arg("run").current_dir(&cwd);
        if flag("release") {
            cmd.arg("--release");
        }
        for (key, opt) in [("package", "-p"), ("example", "--example"), ("bin", "--bin")] {
            if let Some(v) = args[key].as_str() {
                cmd.args([opt, v]);
            }
        }
        if let Some(extra) = args["args"].as_array() {
            cmd.arg("--");
            cmd.args(extra.iter().filter_map(Value::as_str));
        }
        let launch_id = random_id();
        cmd.env("CRABBY_CONTROL", "1")
            .env("CRABBY_LAUNCH_ID", &launch_id)
            .env("CRABBY_INSTANCES_DIR", &self.config.instances_dir);
        if flag("paused") {
            cmd.env("CRABBY_START_PAUSED", "1");
        }
        if flag("headless") {
            cmd.env("HEADLESS", "1");
        }
        if let Some(env) = args["env"].as_object() {
            for (k, v) in env {
                cmd.env(k, v.as_str().map_or_else(|| v.to_string(), str::to_string));
            }
        }
        std::fs::create_dir_all(&self.config.logs_dir).map_err(|e| format!("can't create the logs dir: {e}"))?;
        let log_path = self.config.logs_dir.join(format!("{launch_id}.log"));
        let log = std::fs::File::create(&log_path).map_err(|e| format!("can't create the log file: {e}"))?;
        let log2 = log.try_clone().map_err(|e| e.to_string())?;
        let mut child = cmd
            .stdin(Stdio::null())
            .stdout(log)
            .stderr(log2)
            .spawn()
            .map_err(|e| format!("could not run cargo: {e}"))?;
        self.launched.insert(launch_id.clone(), log_path.clone());

        let deadline = Instant::now() + Duration::from_secs(args["timeout_secs"].as_u64().unwrap_or(600));
        loop {
            if let Some(i) = discover(&self.config.instances_dir)
                .into_iter()
                .find(|i| i.info["launch_id"] == json!(launch_id))
            {
                // Reap the cargo process in the background when it exits.
                std::thread::spawn(move || child.wait());
                let mut v = i.public();
                v["log_file"] = json!(log_path);
                return Ok(v);
            }
            if let Ok(Some(status)) = child.try_wait() {
                return Err(format!(
                    "the game exited ({status}) before its control endpoint opened. Last log lines:\n{}",
                    tail(&log_path, 40)
                ));
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                return Err(format!(
                    "timed out waiting for the game to start. Last log lines:\n{}",
                    tail(&log_path, 40)
                ));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn read_logs(&self, args: &Value) -> Result<Value, String> {
        let lines = args["lines"].as_u64().unwrap_or(100) as usize;
        let i = self.instance(args)?;
        let id = i.info["launch_id"].as_str().ok_or(
            "this game wasn't started with `launch`, so its output isn't captured; check the terminal it runs in",
        )?;
        let path = self
            .launched
            .get(id)
            .cloned()
            .unwrap_or_else(|| self.config.logs_dir.join(format!("{id}.log")));
        Ok(json!({ "log_file": path, "text": tail(&path, lines) }))
    }
}

fn tail(path: &Path, lines: usize) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// Runs the server on stdin/stdout until stdin closes.
///
/// ```no_run
/// crabby_mcp::serve_stdio(crabby_mcp::Config::from_env()).unwrap();
/// ```
pub fn serve_stdio(config: Config) -> std::io::Result<()> {
    let mut server = Server::new(config);
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => server.handle(&msg),
            Err(e) => Some(
                json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": format!("parse error: {e}") } }),
            ),
        };
        if let Some(reply) = reply {
            writeln!(stdout, "{reply}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}
