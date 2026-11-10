//! The control endpoint: lets tools (agents through `ponko-mcp`, the
//! inspector, scripts) look inside a running game and drive it.
//!
//! Off by default. Turned on with [`Game::control_endpoint`](crate::Game::control_endpoint)
//! or the `PONKO_CONTROL=1` environment variable, so tools can enable it
//! for any game without code changes.
//!
//! Protocol: TCP on `127.0.0.1`, one JSON object per line each way.
//! Request: `{"id": 1, "token": "...", "method": "entity.get", "params": {...}}`.
//! Reply: `{"id": 1, "result": ...}` or `{"id": 1, "error": "..."}`.
//! The token is in the instance's registry file, readable only by its user.
//!
//! Requests are applied by the game loop between frames (or between ticks
//! when headless), so they never race the simulation or each other.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use glam::Vec2;
use ponko_ecs::Entity;
use serde_json::{Value, json};

use crate::ctx::Ctx;
// `crate::units::Health` → `Health`, for archetype listings.
use crate::diagnostics::short_name as short;
use crate::reflect::WorldReflect;

mod capabilities;
mod inspector;
mod query;
use query::{Condition, FieldPath, FieldStats};

/// Environment variable that turns the endpoint on (`1`).
pub(crate) const ENV_CONTROL: &str = "PONKO_CONTROL";
/// Environment variable that lets tools start input scenarios (`1`).
pub(crate) const ENV_INPUT_INJECTION: &str = "PONKO_INPUT_INJECTION";
/// Environment variable that starts the simulation paused (`1`).
pub(crate) const ENV_START_PAUSED: &str = "PONKO_START_PAUSED";
/// Environment variable overriding where instances register themselves.
pub(crate) const ENV_INSTANCES_DIR: &str = "PONKO_INSTANCES_DIR";
/// Environment variable a launching tool sets; copied into the registry file.
pub(crate) const ENV_LAUNCH_ID: &str = "PONKO_LAUNCH_ID";

/// How long a connection waits for the game to answer before giving up
/// (a game stuck in a long frame, or paused in a debugger).
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);

/// How often the accept loop checks whether the game has shut down.
const ACCEPT_POLL: Duration = Duration::from_millis(20);

/// Upper bound for `entities.list`, so one request can't produce gigabytes.
const MAX_LIST: usize = 10_000;

pub(crate) fn env_flag(name: &str) -> bool {
    crate::environment::var(name).is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// The directory where running instances register: `$PONKO_INSTANCES_DIR`,
/// else `~/.crabby/instances` (`%LOCALAPPDATA%\crabby\instances` on Windows).
pub(crate) fn instances_dir() -> Option<PathBuf> {
    if let Some(dir) = crate::environment::var_os(ENV_INSTANCES_DIR) {
        return Some(PathBuf::from(dir));
    }
    if cfg!(windows) {
        crate::environment::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("crabby").join("instances"))
    } else {
        crate::environment::var_os("HOME").map(|d| PathBuf::from(d).join(".crabby").join("instances"))
    }
}

/// A random token. The keys of `RandomState` come from the OS's random
/// source, which is plenty for telling local tools apart from strangers.
fn random_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for _ in 0..2 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
        );
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

struct Request {
    method: String,
    params: Value,
    reply: Sender<Result<Value, String>>,
}

/// A running endpoint. Dropping it stops accepting connections and removes
/// the registry file.
pub(crate) struct Control {
    requests: Receiver<Request>,
    addr: SocketAddr,
    registry_file: Option<PathBuf>,
    shutdown: Arc<AtomicBool>,
    /// Keeps stdout and stderr for `logs.get` while the endpoint is open.
    _logs: Option<crate::logs::Capture>,
}

impl Control {
    /// Opens the endpoint on a free localhost port and registers the instance.
    pub(crate) fn start(name: &str, headless: bool) -> std::io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let token: Arc<str> = random_token().into();
        let (tx, requests) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));

        let registry_file = instances_dir().and_then(|dir| {
            let info = json!({
                "name": name,
                "pid": std::process::id(),
                "port": addr.port(),
                "token": &*token,
                "headless": headless,
                "cwd": std::env::current_dir().ok(),
                "exe": std::env::current_exe().ok(),
                "started": SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()),
                "engine_version": env!("CARGO_PKG_VERSION"),
                // Set by tools that launch games, to find the instance they started.
                "launch_id": crate::environment::var(ENV_LAUNCH_ID).ok(),
            });
            match write_registry(&dir, addr.port(), &info) {
                Ok(path) => Some(path),
                Err(e) => {
                    eprintln!("ponko: control endpoint could not register in {}: {e}", dir.display());
                    None
                }
            }
        });

        let stop = shutdown.clone();
        thread::Builder::new()
            .name("ponko-control".into())
            .spawn(move || accept_loop(listener, token, tx, stop))?;
        let logs = crate::logs::Capture::start();
        eprintln!("ponko: control endpoint on {addr}");
        Ok(Self {
            requests,
            addr,
            registry_file,
            shutdown,
            _logs: logs,
        })
    }

    pub(crate) fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Handles every request that has arrived. Called between frames/ticks.
    pub(crate) fn poll(&self, ctx: &mut Ctx) {
        ctx.events.collect_feed(&ctx.world);
        while let Ok(req) = self.requests.try_recv() {
            answer(ctx, req);
        }
    }

    /// Waits up to `timeout` for a request, then handles everything queued.
    /// Used by a paused headless game, which has nothing else to do.
    pub(crate) fn wait(&self, ctx: &mut Ctx, timeout: Duration) {
        ctx.events.collect_feed(&ctx.world);
        match self.requests.recv_timeout(timeout) {
            Ok(req) => answer(ctx, req),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return,
        }
        self.poll(ctx);
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(path) = &self.registry_file {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn write_registry(dir: &Path, port: u16, info: &Value) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}-{port}.json", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600);
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    let mut file = options.open(&path)?;
    file.write_all(serde_json::to_string_pretty(info)?.as_bytes())?;
    Ok(path)
}

fn accept_loop(listener: TcpListener, token: Arc<str>, tx: Sender<Request>, shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                let (token, tx) = (token.clone(), tx.clone());
                let _ = thread::Builder::new()
                    .name("ponko-control-conn".into())
                    .spawn(move || serve_connection(stream, &token, &tx));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => thread::sleep(ACCEPT_POLL),
            Err(_) => thread::sleep(ACCEPT_POLL),
        }
    }
}

fn serve_connection(stream: TcpStream, token: &str, tx: &Sender<Request>) {
    static NEXT_CONNECTION: AtomicU64 = AtomicU64::new(1);
    let connection = NEXT_CONNECTION.fetch_add(1, Ordering::Relaxed);
    let mut bound_scenario = false;
    serve_lines(stream, token, tx, connection, &mut bound_scenario);
    if bound_scenario {
        // The client that started the scenario is gone: let go of its input.
        let (reply, _) = mpsc::channel();
        let _ = tx.send(Request {
            method: "input.scenario.cancel".into(),
            params: json!({ "reason": "the client that started it disconnected", "connection": connection }),
            reply,
        });
    }
}

fn serve_lines(stream: TcpStream, token: &str, tx: &Sender<Request>, connection: u64, bound_scenario: &mut bool) {
    let _ = stream.set_nonblocking(false);
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else {
            return;
        };
        if line.trim().is_empty() {
            continue;
        }
        let reply = handle_line(&line, token, tx, connection);
        if reply["result"]["bound_to_connection"] == true {
            *bound_scenario = true;
        }
        if writeln!(writer, "{reply}").and_then(|()| writer.flush()).is_err() {
            return;
        }
    }
}

fn handle_line(line: &str, token: &str, tx: &Sender<Request>, connection: u64) -> Value {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return json!({ "id": null, "error": format!("invalid JSON: {e}") }),
    };
    let id = msg.get("id").cloned().unwrap_or(Value::Null);
    let reply = |r: Result<Value, String>| match r {
        Ok(result) => json!({ "id": id, "result": result }),
        Err(error) => json!({ "id": id, "error": error }),
    };
    if msg.get("token").and_then(Value::as_str) != Some(token) {
        return reply(Err("missing or wrong token (see the instance's registry file)".into()));
    }
    let Some(method) = msg.get("method").and_then(Value::as_str) else {
        return reply(Err("missing `method`".into()));
    };
    if method == "logs.get" {
        // Answered right here, not by the game loop: logs matter most when
        // the game is stuck in a long frame.
        return reply(logs_get(msg.get("params").unwrap_or(&Value::Null)));
    }
    let (reply_tx, reply_rx) = mpsc::channel();
    let mut params = msg.get("params").cloned().unwrap_or(Value::Null);
    if let Some(map) = params.as_object_mut() {
        // Set by the server, never by the client: which connection is asking.
        map.insert("_connection".into(), json!(connection));
    }
    let req = Request {
        method: method.to_string(),
        params,
        reply: reply_tx,
    };
    if tx.send(req).is_err() {
        return reply(Err("the game is shutting down".into()));
    }
    reply(match reply_rx.recv_timeout(REPLY_TIMEOUT) {
        Ok(r) => r,
        Err(_) => Err("the game did not answer in time".into()),
    })
}

/// The JSON of the components an entity was matched on.
#[derive(Clone, Copy)]
struct Found<'a> {
    names: &'a [String],
    jsons: &'a [Value],
}

impl<'a> Found<'a> {
    fn get(&self, component: &str) -> Option<&'a Value> {
        self.names.iter().position(|n| n == component).map(|i| &self.jsons[i])
    }
}

/// Calls `f` for every entity that has the `with` components, the ones the
/// `fields` name, and passes every `where` condition. `f` gets the entity,
/// its archetype's component names, and a lookup of the JSON of the
/// components named in conditions and fields.
fn for_each_matching(
    world: &ponko_ecs::World,
    params: &Value,
    fields: &[FieldPath],
    mut f: impl FnMut(Entity, &[&str], Found) -> Result<(), String>,
) -> Result<(), String> {
    let mut with: Vec<String> = match params.get("with") {
        None | Some(Value::Null) => Vec::new(),
        Some(v) => serde_json::from_value(v.clone()).map_err(|_| "`with` must be a list of component names")?,
    };
    let conditions = query::list(params, "where", Condition::parse)?;
    // The components whose JSON is needed, each once.
    let mut needed: Vec<String> = Vec::new();
    for c in conditions.iter().map(|c| &c.path).chain(fields) {
        if !needed.contains(&c.component) {
            needed.push(c.component.clone());
        }
    }
    let reflected = world.reflected_components();
    if let Some(n) = needed.iter().find(|n| !reflected.contains(&n.as_str())) {
        return Err(format!(
            "`{n}` is not a reflected component (reflected: {})",
            reflected.join(", ")
        ));
    }
    with.extend(needed.iter().cloned());
    let mut jsons: Vec<Value> = Vec::with_capacity(needed.len());
    for a in world.raw_ref().archetypes().iter().filter(|a| !a.is_empty()) {
        let names: Vec<&str> = a
            .component_ids()
            .iter()
            .map(|&id| short(world.component_info(id).name()))
            .collect();
        if !with.iter().all(|w| names.contains(&w.as_str())) {
            continue;
        }
        for &e in a.entities() {
            jsons.clear();
            for n in &needed {
                jsons.push(world.get_json(e, n).map_err(|e| e.to_string())?);
            }
            let found = Found {
                names: &needed,
                jsons: &jsons,
            };
            if conditions
                .iter()
                .all(|c| found.get(&c.path.component).is_some_and(|j| c.holds(j)))
            {
                f(e, &names, found)?;
            }
        }
    }
    Ok(())
}

fn logs_get(params: &Value) -> Result<Value, String> {
    let since = params.get("since").and_then(Value::as_u64);
    let max = params.get("lines").and_then(Value::as_u64).unwrap_or(100) as usize;
    crate::logs::get(since, max)
}

fn answer(ctx: &mut Ctx, req: Request) {
    if req.method == "screenshot" && !ctx.headless {
        // Answered after the next frame is drawn.
        match screenshot_path(ctx, &req.params) {
            Ok(path) => {
                let max_size = req.params.get("max_size").and_then(Value::as_u64).unwrap_or(0) as u32;
                ctx.gfx.queue_screenshot(path, max_size, Some(req.reply));
            }
            Err(e) => {
                let _ = req.reply.send(Err(e));
            }
        }
        return;
    }
    let result = handle(ctx, &req.method, &req.params);
    let _ = req.reply.send(result);
}

fn camera_json(ctx: &Ctx) -> Value {
    let c = &ctx.camera;
    let r = c.visible_rect();
    json!({
        "position": [c.position.x, c.position.y],
        "zoom": c.zoom,
        "center": [c.center().x, c.center().y],
        "visible": { "min": [r.min.x, r.min.y], "max": [r.max.x, r.max.y] },
    })
}

/// The `path` param, or a fresh file in the temp dir.
fn screenshot_path(ctx: &Ctx, params: &Value) -> Result<PathBuf, String> {
    match params.get("path") {
        None | Some(Value::Null) => Ok(std::env::temp_dir().join(format!(
            "ponko-screenshot-{}-{}.png",
            std::process::id(),
            ctx.time.frame()
        ))),
        Some(Value::String(p)) => Ok(PathBuf::from(p)),
        Some(_) => Err("`path` must be a string".into()),
    }
}

// ------------------------------------------------------------------ methods

/// Every method the endpoint understands, with a one-line description.
/// Returned by `methods`, so tools can discover them.
pub(crate) const METHODS: &[(&str, &str)] = &[
    ("methods", "list the methods this endpoint understands"),
    (
        "capabilities",
        "what this instance can do: engine version, headless or windowed, compiled features, subsystems (renderer, physics, audio, networking...) with why one is off, each method's availability with a reason when refused, and reflected components/resources with their schema (or `opaque` when not described)",
    ),
    (
        "logs.get",
        "the game's recent stdout and stderr lines (answered even mid-frame); params: lines? (100), since? (the `next` of a previous read: only newer lines)",
    ),
    (
        "info",
        "name, pid, tick, sim time, paused, speed, fps, entity count, render stats (sprites, draw calls, CPU and GPU ms)",
    ),
    (
        "world.summary",
        "entity count, allocated bytes, reflected components, archetypes with their sizes and bytes per entity",
    ),
    (
        "entities.list",
        "entities with their reflected components as JSON and the names of the others; params: with? (component names), where? (like [\"Health.current < 5\"]), limit? (100), offset?",
    ),
    (
        "entities.stats",
        "count, min, max, mean of reflected fields (per axis for vectors; value counts for names and enums); params: fields (like [\"Health.current\", \"Team\"]), with?, where?",
    ),
    ("entity.get", "one entity's reflected components; params: entity"),
    ("entity.spawn", "spawn from {component: value}; params: components"),
    ("entity.despawn", "params: entity"),
    (
        "component.set",
        "write a component (object fields merge); params: entity, component, value",
    ),
    ("component.remove", "params: entity, component"),
    (
        "resources.list",
        "every reflected resource with its value as JSON (null when it's not in the world)",
    ),
    ("resource.get", "one reflected resource as JSON; params: resource"),
    (
        "resource.set",
        "write a reflected resource (object fields merge; inserted if missing); params: resource, value",
    ),
    (
        "world.save",
        "save every entity with reflected components, and reflected resources, to a JSON file; params: path. Returns counts",
    ),
    (
        "world.load",
        "replace every entity with a save file's (same ids); params: path. Checked first: on error nothing changes",
    ),
    (
        "scene.save",
        "save the world as a scene in assets/ (what ctx.load_scene and the editor read); params: path (relative to assets/)",
    ),
    (
        "scene.load",
        "replace the world with a scene from assets/, loading the images its sprites name; params: path",
    ),
    (
        "prefab.spawn",
        "spawn a prefab from assets/ at a point; params: path, at [x, y]. Returns the entity",
    ),
    (
        "prefab.save",
        "save an entity's reflected components (without its position) as a prefab in assets/; params: entity, path",
    ),
    ("sim.pause", "pause simulation systems"),
    ("sim.resume", "resume simulation systems"),
    ("sim.step", "run extra simulation ticks; params: ticks? (default 1)"),
    ("sim.speed", "set simulation speed; params: speed"),
    (
        "input.scenario.load",
        "start a tick-indexed input scenario (versioned JSON: input steps fed through the real input path, and checks), replacing a running one; its tick 0 is the next simulation tick; params: scenario (the object), bind_connection? (cancel it when this connection closes), keep_input_when_done? (by default the keys it holds are released when it ends; with this they stay held, owned by the scenario's owner until cancelled), max_ticks? (refuse a longer scenario before touching the running one), queue_ticks? (paused games: queue the scenario's own ticks as steps; a failed check or a cancel takes back only those that haven't run). Refused unless the game allows input injection (PONKO_INPUT_INJECTION=1 or Game::input_injection). Returns the report",
    ),
    (
        "input.scenario.status",
        "the latest input scenario's report (status pending/running/passed/failed/cancelled, ticks run, checks passed, the failing step and tick with expected and actual), or null",
    ),
    (
        "input.scenario.cancel",
        "end the running input scenario and release the keys, buttons and axes it holds; params: reason? Returns the report",
    ),
    ("systems.timings", "each system's last and average time in ms"),
    (
        "ecs.diagnostics",
        "the ECS as stable, versioned data, built only when asked: entities and allocated bytes; archetypes (index, full component names, entities, bytes_per_entity, allocated_bytes); components (name, short_name, size, align, archetypes, entities, bytes); systems in run order (name, phase, order, sim, every, offset, access, last_ms, avg_ms). params: with? (component names: adds `matching`, the archetypes and entities a query for them visits), limit? (archetypes listed, non-empty ones in world order, default 100; `archetypes_total` counts all)",
    ),
    (
        "events.list",
        "reflected event types (ctx.events.reflect, or C events whose type is a reflected component) and how many are kept now",
    ),
    (
        "events.subscribe",
        "start collecting new events, between frames, newest kept; params: types? (names, default all), capacity? (1000)",
    ),
    (
        "events.take",
        "events collected since the last take, oldest first: [{tick, type, data}], plus how many were dropped for space",
    ),
    ("events.unsubscribe", "stop collecting events"),
    (
        "camera.get",
        "the camera: position (world point at the window's top-left), zoom, center, visible rect",
    ),
    (
        "camera.set",
        "move the camera; params: position? [x, y], zoom?, look_at? [x, y] (applied after zoom). Returns camera.get",
    ),
    (
        "screenshot",
        "save the next frame as a PNG (windowed games only); params: path? (default: a temp file), max_size? (scale down to fit, 0 = full size). Returns path, width, height",
    ),
    (
        "shaders.status",
        "the game's custom shaders (material shaders, screen effects, compute passes): each one's last compile or hot reload result (compiled, reloaded, failed, reload_failed: the last good version stays), reload count, frame, and its errors with line and column in the game's own WGSL, cause and GPU variant",
    ),
    (
        "perf.get",
        "frame timing over the last few seconds (fps; avg, p50, p95, p99, max ms; CPU split into systems, render and waiting on the display; GPU ms), GPU ms per pass, what was drawn (draw calls; 3D models and triangles for the camera and the shadows; render CPU split into collect, record, submit), the slowest systems, live asset handles by class with estimated bytes (`assets`: textures, meshes, materials, sounds, ...; with what the GPU holds), residency budgets per asset kind (`residency`: budget, resident and pinned counts and bytes, loads, reloads, evictions, evicted bytes), streaming (`streaming`: the upload queue's budget, pending count, bytes and uploads last frame and in total, oversized steps, decode waits, uploads held by a budget, failures, last and longest step ms; and `assets`, every streamed or resident asset with kind, name, state (loading, uploading, streaming, resident, evicted, failed), id and GPU size); params: meshes? (list the n meshes drawing the most triangles, default 0), profiling? (true turns on scope timing and allocation counting, false turns it off). While profiling is on, the answer also has `report`: frame percentiles, every system and profile_scope with time and allocations per frame, the slowest frames and what was in them, and warnings with advice. Always has `render_inspector` (schema 1, also when headless): the last frame's CPU time split into extract, prepare, encode, submit and other ms; GPU ms per frame and per pass; draw calls, models, triangles, occluded models, pipelines built; upload and memory bytes; the active graphics quality preset and settings. A metric that can't be measured (no timestamp queries, headless, not counted yet) is null and `unavailable` gives the reason; GPU times are never guessed",
    ),
    (
        "perf.trace",
        "record the next frames as a Chrome/Perfetto trace (systems, render, display wait, GPU passes; windowed games only); params: path? (default: a temp file), frames? (120). Returns the path; it's written when the frames are done",
    ),
    (
        "perf.hud",
        "show or hide the on-screen performance overlay; params: on (bool)",
    ),
    ("game.quit", "ask the game to exit"),
];

/// `perf.get`'s `render_inspector`: the last frame's rendering cost in a
/// versioned schema (see `control/inspector.rs`).
fn render_inspector_json(ctx: &Ctx) -> Value {
    let source = match (ctx.headless, ctx.perf.last_render_detail) {
        (true, _) => inspector::Source::Headless,
        (false, None) => inspector::Source::NotDrawn,
        (false, Some(detail)) => inspector::Source::Drawn(detail),
    };
    #[cfg(feature = "3d")]
    let quality = Some(inspector::quality_json(&ctx.gfx.quality(&ctx.post, &ctx.lighting)));
    #[cfg(not(feature = "3d"))]
    let quality = None;
    inspector::render_inspector(&inspector::Inputs {
        source,
        stats: ctx.perf.render_stats(),
        passes: ctx.perf.gpu_passes(),
        quality,
        pipelines: ctx.gfx.pipeline_stats(),
    })
}

/// `perf.get`: the profiler's numbers as JSON.
fn perf_json(ctx: &Ctx, meshes: usize) -> Value {
    if ctx.headless {
        let slowest = slowest_systems(ctx);
        return json!({ "headless": true, "systems": slowest, "residency": residency_json(ctx), "streaming": streaming_json(ctx), "render_inspector": render_inspector_json(ctx) });
    }
    let f = ctx.perf.frame_stats();
    let r = ctx.perf.render_stats();
    let passes: Vec<Value> = {
        let mut p: Vec<_> = ctx.perf.gpu_passes().to_vec();
        p.sort_by(|a, b| b.ms.total_cmp(&a.ms));
        p.iter().map(|p| json!({ "name": p.name, "ms": p.ms })).collect()
    };
    #[allow(unused_mut)]
    let mut out = json!({
        "frames": f.frames,
        "fps": f.fps,
        "frame_ms": { "avg": f.avg_ms, "p50": f.p50_ms, "p95": f.p95_ms, "p99": f.p99_ms, "max": f.max_ms },
        "cpu_ms": { "systems": f.systems_ms, "render": f.render_cpu_ms, "display_wait": f.display_wait_ms },
        "gpu_ms": f.gpu_ms,
        "gpu_passes": passes,
        "render": {
            "draw_calls": r.draw_calls,
            "sprites": r.sprites,
            "collect_ms": r.collect_ms,
            "record_ms": r.record_ms,
            "submit_ms": r.submit_ms,
            "models": r.instances,
            "triangles": r.triangles,
            "shadow_models": r.shadow_instances,
            "shadow_triangles": r.shadow_triangles,
            "uploads": r.uploads.json(),
        },
        "systems": slowest_systems(ctx),
        "residency": residency_json(ctx),
        "streaming": streaming_json(ctx),
        "render_inspector": render_inspector_json(ctx),
        "hint": "frame time is about max(CPU, GPU): when display_wait is large the GPU (or vsync) is the limit; gpu_passes says which pass",
    });
    #[cfg(feature = "3d")]
    if meshes > 0 {
        out["meshes"] = ctx
            .gfx
            .mesh_costs()
            .iter()
            .take(meshes)
            .map(|c| {
                json!({
                    "name": c.name,
                    "models": c.models,
                    "triangles": c.triangles,
                    "shadow_models": c.shadow_models,
                    "shadow_triangles": c.shadow_triangles,
                })
            })
            .collect();
    }
    #[cfg(not(feature = "3d"))]
    let _ = meshes;
    out
}

/// `Graphics::shader_status` as JSON.
#[cfg(feature = "3d")]
pub(crate) fn shader_status_json(gfx: &crate::gfx::Graphics) -> Value {
    use crate::render::ShaderKind;
    use crate::render::ShaderOutcome;
    let shaders: Vec<Value> = gfx
        .shader_status()
        .iter()
        .map(|s| {
            let kind = match s.kind {
                ShaderKind::Material => "material",
                ShaderKind::Effect => "effect",
                _ => "compute",
            };
            let outcome = match s.outcome {
                ShaderOutcome::Compiled => "compiled",
                ShaderOutcome::Reloaded => "reloaded",
                ShaderOutcome::Failed => "failed",
                _ => "reload_failed",
            };
            let diagnostics: Vec<Value> = s
                .diagnostics
                .iter()
                .map(|d| {
                    json!({
                        "line": d.line,
                        "column": d.column,
                        "in_engine_code": d.in_engine_code,
                        "message": d.message,
                        "label": d.label,
                        "notes": d.notes,
                        "source_line": d.source_line,
                        "variant": d.variant,
                        "text": d.to_string(),
                    })
                })
                .collect();
            json!({
                "shader": s.shader,
                "kind": kind,
                "outcome": outcome,
                "ok": s.is_ok(),
                "reloads": s.reloads,
                "frame": s.frame,
                "diagnostics": diagnostics,
            })
        })
        .collect();
    json!({ "shaders": shaders })
}

/// Without the `3d` feature a game has no custom shaders.
#[cfg(not(feature = "3d"))]
fn shader_status_json(_gfx: &crate::gfx::Graphics) -> Value {
    json!({ "shaders": [], "note": "built without the `3d` feature: no custom shaders" })
}

/// `ecs.diagnostics`: [`Ctx::ecs_diagnostics`] as JSON, with the
/// archetype list capped and an optional query match count.
fn ecs_diagnostics_json(ctx: &Ctx, params: &Value) -> Result<Value, String> {
    let limit = usize_param(params, "limit", 100)?.min(MAX_LIST);
    let with: Option<Vec<String>> = match params.get("with") {
        None | Some(Value::Null) => None,
        Some(v) => Some(serde_json::from_value(v.clone()).map_err(|_| "`with` must be a list of component names")?),
    };
    let mut d = ctx.ecs_diagnostics();
    let matching = match &with {
        Some(with) => Some(d.matching(&with.iter().map(String::as_str).collect::<Vec<_>>())?),
        None => None,
    };
    let total = d.archetypes.len();
    d.archetypes.retain(|a| a.entities > 0);
    d.archetypes.truncate(limit);
    let mut out = serde_json::to_value(&d).map_err(|e| e.to_string())?;
    out["archetypes_total"] = json!(total);
    if let (Some(with), Some(m)) = (with, matching) {
        out["matching"] = json!({ "with": with, "archetypes": m.archetypes, "entities": m.entities });
    }
    Ok(out)
}

/// The upload queue and every streamed asset's state.
fn streaming_json(ctx: &Ctx) -> Value {
    let u = ctx.gfx.upload_stats();
    let assets: Vec<Value> = ctx
        .gfx
        .streamed_assets()
        .into_iter()
        .map(|a| {
            json!({
                "kind": a.kind.name(),
                "name": a.name,
                "state": a.state.name(),
                "id": a.id,
                "size": a.size,
            })
        })
        .collect();
    json!({
        "uploads": {
            "budget": u.budget,
            "pending": u.pending,
            "frame_bytes": u.frame_bytes,
            "frame_uploads": u.frame_uploads,
            "total_bytes": u.total_bytes,
            "total_uploads": u.total_uploads,
            "oversized": u.oversized,
            "decode_waits": u.decode_waits,
            "held_by_budget": u.held_by_budget,
            "failures": u.failures,
            "step_ms": u.step_ms,
            "max_step_ms": u.max_step_ms,
        },
        "assets": assets,
    })
}

/// Residency budgets and counters, by kind name.
fn residency_json(ctx: &Ctx) -> Value {
    crate::residency::ResidencyKind::ALL
        .iter()
        .map(|&kind| {
            let r = ctx.gfx.residency(kind);
            let value = json!({
                "budget": r.budget,
                "resident": r.resident,
                "resident_bytes": r.resident_bytes,
                "over_budget_bytes": r.over_budget_bytes(),
                "pinned": r.pinned,
                "pinned_bytes": r.pinned_bytes,
                "loads": r.loads,
                "reloads": r.reloads,
                "evictions": r.evictions,
                "evicted_bytes": r.evicted_bytes,
            });
            (kind.name().to_string(), value)
        })
        .collect::<serde_json::Map<_, _>>()
        .into()
}

/// The 10 systems with the most average time.
fn slowest_systems(ctx: &Ctx) -> Value {
    let mut t: Vec<_> = ctx
        .perf
        .system_timings()
        .iter()
        .filter(|t| t.sim || !ctx.headless)
        .collect();
    t.sort_by(|a, b| b.avg_ms.total_cmp(&a.avg_ms));
    t.iter()
        .take(10)
        .map(|t| json!({ "name": t.name, "avg_ms": t.avg_ms }))
        .collect()
}

fn param<'a>(params: &'a Value, name: &str) -> Result<&'a Value, String> {
    params.get(name).ok_or_else(|| format!("missing param `{name}`"))
}

fn entity_param(params: &Value) -> Result<Entity, String> {
    param(params, "entity")?
        .as_u64()
        .map(Entity::from_bits)
        .ok_or_else(|| "`entity` must be an entity id (a non-negative integer)".into())
}

fn str_param<'a>(params: &'a Value, name: &str) -> Result<&'a str, String> {
    param(params, name)?
        .as_str()
        .ok_or_else(|| format!("`{name}` must be a string"))
}

fn usize_param(params: &Value, name: &str, default: usize) -> Result<usize, String> {
    match params.get(name) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => v
            .as_u64()
            .map(|n| n as usize)
            .ok_or_else(|| format!("`{name}` must be a non-negative integer")),
    }
}

/// The `capabilities` answer as JSON text (see `Ctx::capabilities_json`).
pub(crate) fn capabilities_json(ctx: &Ctx) -> String {
    capabilities::report(ctx).to_string()
}

pub(crate) fn handle(ctx: &mut Ctx, method: &str, params: &Value) -> Result<Value, String> {
    let err = |e: crate::reflect::ReflectError| e.to_string();
    if let Some(reason) = capabilities::unavailable(ctx, method) {
        return Err(reason);
    }
    Ok(match method {
        "capabilities" => capabilities::report(ctx),
        "methods" => Value::Array(
            METHODS
                .iter()
                .map(|(name, about)| json!({ "name": name, "about": about }))
                .collect(),
        ),
        "info" => json!({
            "pid": std::process::id(),
            "headless": ctx.headless,
            "tick": ctx.time.sim_tick(),
            "sim_time": ctx.time.sim_time(),
            "paused": ctx.time.is_paused(),
            "speed": ctx.time.speed(),
            "fixed_dt": ctx.time.fixed_dt(),
            // Headless games have no frames, so no frame rate.
            "fps": if ctx.headless { Value::Null } else { json!(ctx.time.fps()) },
            "frame": if ctx.headless { Value::Null } else { json!(ctx.time.frame()) },
            "entities": ctx.world.len(),
            "render": if ctx.headless { Value::Null } else {
                let r = ctx.perf.render_stats();
                json!({ "sprites": r.sprites, "draw_calls": r.draw_calls, "cpu_ms": r.cpu_ms, "gpu_ms": r.gpu_ms })
            },
        }),
        "logs.get" => logs_get(params)?,
        "world.summary" => {
            let world = &ctx.world;
            let archetypes: Vec<Value> = world
                .raw_ref()
                .archetypes()
                .iter()
                .filter(|a| !a.is_empty())
                .map(|a| {
                    let names: Vec<&str> = a
                        .component_ids()
                        .iter()
                        .map(|&id| short(world.component_info(id).name()))
                        .collect();
                    json!({ "components": names, "entities": a.len(), "bytes_per_entity": a.bytes_per_entity() })
                })
                .collect();
            json!({
                "entities": world.len(),
                "allocated_bytes": world.allocated_bytes(),
                "reflected_components": world.reflected_components(),
                "reflected_resources": world.reflected_resources(),
                "reflected_events": ctx.events.reflected_json(world),
                "archetypes": archetypes,
            })
        }
        "entities.list" => {
            let limit = usize_param(params, "limit", 100)?.min(MAX_LIST);
            let offset = usize_param(params, "offset", 0)?;
            let world = &ctx.world;
            let reflected = world.reflected_components();
            let (mut total, mut out) = (0, Vec::new());
            for_each_matching(world, params, &[], |e, names, _| {
                if total >= offset && out.len() < limit {
                    let components = world.entity_json(e).map_err(err)?;
                    let other: Vec<&str> = names.iter().copied().filter(|n| !reflected.contains(n)).collect();
                    out.push(json!({ "entity": e.to_bits(), "components": components, "other_components": other }));
                }
                total += 1;
                Ok(())
            })?;
            json!({ "total": total, "offset": offset, "entities": out })
        }
        "entities.stats" => {
            let fields = query::list(params, "fields", FieldPath::parse)?;
            if fields.is_empty() {
                return Err("missing param `fields` (like [\"Health.current\", \"Team\"])".into());
            }
            let mut stats: Vec<FieldStats> = fields.iter().map(|_| FieldStats::default()).collect();
            let mut count = 0u64;
            for_each_matching(&ctx.world, params, &fields, |_, _, found| {
                count += 1;
                for (f, s) in fields.iter().zip(&mut stats) {
                    if let Some(v) = found.get(&f.component).and_then(|c| f.get(c)) {
                        s.add(v);
                    }
                }
                Ok(())
            })?;
            let by_field: serde_json::Map<String, Value> = fields
                .iter()
                .zip(&stats)
                .map(|(f, s)| (f.text().to_string(), s.to_json()))
                .collect();
            json!({ "entities": count, "fields": by_field })
        }
        "entity.get" => ctx.world.entity_json(entity_param(params)?).map_err(err)?,
        "entity.spawn" => {
            let components = param(params, "components")?.clone();
            ctx.load_scene_assets(&components)?;
            let e = ctx.world.spawn_json(components).map_err(err)?;
            json!({ "entity": e.to_bits() })
        }
        "entity.despawn" => json!(ctx.world.despawn(entity_param(params)?)),
        "component.set" => {
            let e = entity_param(params)?;
            let name = str_param(params, "component")?;
            let value = param(params, "value")?.clone();
            ctx.load_scene_assets(&json!({ name: &value }))?;
            ctx.world.set_json(e, name, value).map_err(err)?;
            ctx.world.get_json(e, name).map_err(err)?
        }
        "component.remove" => {
            let e = entity_param(params)?;
            json!(ctx.world.remove_json(e, str_param(params, "component")?).map_err(err)?)
        }
        "resources.list" => {
            let world = &ctx.world;
            let mut out = serde_json::Map::new();
            for name in world.reflected_resources() {
                let value = match world.resource_json(name) {
                    Ok(v) => v,
                    Err(crate::reflect::ReflectError::MissingResource(_)) => Value::Null,
                    Err(e) => return Err(err(e)),
                };
                out.insert(name.to_string(), value);
            }
            Value::Object(out)
        }
        "resource.get" => ctx.world.resource_json(str_param(params, "resource")?).map_err(err)?,
        "resource.set" => {
            let name = str_param(params, "resource")?;
            ctx.world
                .set_resource_json(name, param(params, "value")?.clone())
                .map_err(err)?;
            ctx.world.resource_json(name).map_err(err)?
        }
        "world.save" => {
            let path = str_param(params, "path")?;
            let save = ctx.world.save();
            let text = serde_json::to_string(&save).map_err(|e| e.to_string())?;
            std::fs::write(path, &text).map_err(|e| format!("could not write {path}: {e}"))?;
            json!({
                "path": path,
                "entities": save["entities"].as_array().map_or(0, Vec::len),
                "resources": save["resources"].as_object().map_or(0, |r| r.len()),
                "bytes": text.len(),
                "not_saved": save["not_saved"],
                "tick": ctx.time.sim_tick(),
            })
        }
        "scene.save" => {
            let path = str_param(params, "path")?;
            ctx.assets.save_scene(&ctx.world, path)?;
            json!({ "path": path, "entities": ctx.world.len() })
        }
        "scene.load" => {
            let path = str_param(params, "path")?;
            ctx.load_scene(path)?;
            json!({ "path": path, "entities": ctx.world.len() })
        }
        "prefab.spawn" => {
            let path = str_param(params, "path")?;
            let at: [f32; 2] = serde_json::from_value(param(params, "at")?.clone())
                .map_err(|e| format!("`at` must be [x, y]: {e}"))?;
            let e = ctx.spawn_prefab(path, crate::vec2(at[0], at[1]))?;
            json!({ "entity": e.to_bits() })
        }
        "prefab.save" => {
            let e = entity_param(params)?;
            let path = str_param(params, "path")?;
            ctx.assets.save_prefab(&ctx.world, e, path)?;
            json!({ "path": path })
        }
        "world.load" => {
            let path = str_param(params, "path")?;
            let text = std::fs::read_to_string(path).map_err(|e| format!("could not read {path}: {e}"))?;
            let save: Value = serde_json::from_str(&text).map_err(|e| format!("{path} is not JSON: {e}"))?;
            ctx.replace_world(&save)?;
            json!({ "path": path, "entities": ctx.world.len() })
        }
        "sim.pause" => {
            ctx.time.pause();
            json!({ "paused": true, "tick": ctx.time.sim_tick() })
        }
        "sim.resume" => {
            ctx.time.resume();
            json!({ "paused": false, "tick": ctx.time.sim_tick() })
        }
        "sim.step" => {
            let ticks = usize_param(params, "ticks", 1)?;
            let ticks = u32::try_from(ticks).map_err(|_| "`ticks` is too large")?;
            for _ in 0..ticks {
                ctx.time.step();
            }
            json!({ "queued_ticks": ticks, "tick": ctx.time.sim_tick() })
        }
        "sim.speed" => {
            let speed = param(params, "speed")?
                .as_f64()
                .filter(|s| s.is_finite() && *s >= 0.0)
                .ok_or("`speed` must be a finite number >= 0")?;
            ctx.time.set_speed(speed as f32);
            json!({ "speed": ctx.time.speed() })
        }
        "input.scenario.load" => {
            let mut scenario =
                crate::scenario::InputScenario::from_value(param(params, "scenario")?).map_err(|e| e.to_string())?;
            let flag = |name: &str| -> Result<bool, String> {
                match params.get(name) {
                    None | Some(Value::Null) => Ok(false),
                    Some(v) => v.as_bool().ok_or_else(|| format!("`{name}` must be true or false")),
                }
            };
            let (bind, keep, queue) = (
                flag("bind_connection")?,
                flag("keep_input_when_done")?,
                flag("queue_ticks")?,
            );
            // Checked before anything is installed: a refused scenario leaves a running one alone.
            let ticks = scenario.ticks();
            if ticks == 0 {
                return Err("the scenario has no steps, so there is nothing to run".into());
            }
            if let Some(max) = params.get("max_ticks").filter(|v| !v.is_null()) {
                let max = max.as_u64().ok_or("`max_ticks` must be a whole number")?;
                if ticks > max {
                    return Err(format!(
                        "the scenario runs {ticks} ticks, more than `max_ticks` ({max})"
                    ));
                }
            }
            if queue && !ctx.time.is_paused() {
                return Err("`queue_ticks` needs a paused game (call sim.pause first)".into());
            }
            // A tool's scenario lets go of its input when it ends, unless told to keep it.
            scenario.release_unless_kept(keep);
            let owner = params.get("_connection").and_then(Value::as_u64).filter(|_| bind);
            let start = ctx.time.sim_tick();
            ctx.input.install_scenario(
                &mut ctx.world,
                &mut ctx.time,
                scenario,
                start,
                crate::scenario::RunOrigin {
                    owner,
                    tool: true,
                    queue_ticks: queue,
                },
            );
            json!({ "report": ctx.input.scenario_report(&ctx.world), "bound_to_connection": owner.is_some() })
        }
        "input.scenario.status" => json!(ctx.input.scenario_report(&ctx.world)),
        "input.scenario.cancel" => {
            let reason = params
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("cancelled by a tool");
            let owner = params.get("connection").and_then(Value::as_u64);
            let cancelled = ctx
                .input
                .cancel_scenario_for(&mut ctx.world, &mut ctx.time, reason, owner);
            json!({ "cancelled": cancelled, "report": ctx.input.scenario_report(&ctx.world) })
        }
        "systems.timings" => Value::Array(
            ctx.perf
                .system_timings()
                .iter()
                // Headless games run only simulation systems.
                .filter(|t| t.sim || !ctx.headless)
                .map(|t| json!({ "name": t.name, "sim": t.sim, "last_ms": t.last_ms, "avg_ms": t.avg_ms }))
                .collect(),
        ),
        "ecs.diagnostics" => ecs_diagnostics_json(ctx, params)?,
        "perf.get" => {
            if let Some(on) = params.get("profiling") {
                ctx.perf
                    .set_profiling(on.as_bool().ok_or("`profiling` must be true or false")?);
            }
            let mut out = perf_json(ctx, usize_param(params, "meshes", 0)?);
            out["assets"] = serde_json::to_value(ctx.asset_inventory()).unwrap_or_default();
            if ctx.perf.profiling() {
                let report = ctx.perf.report();
                out["report"] = serde_json::to_value(&report).unwrap_or_default();
                out["report_text"] = json!(report.to_string());
            }
            out
        }
        "perf.trace" => {
            let frames = usize_param(params, "frames", 120)?.clamp(1, 10_000);
            let path = match params.get("path").and_then(Value::as_str) {
                Some(p) => PathBuf::from(p),
                None => {
                    std::env::temp_dir().join(format!("ponko-trace-{}-{}.json", std::process::id(), ctx.time.frame()))
                }
            };
            ctx.perf.record_trace(path.clone(), frames);
            json!({ "path": path, "frames": frames, "written_after_frame": ctx.time.frame() + frames as u64 })
        }
        "perf.hud" => {
            let on = param(params, "on")?.as_bool().ok_or("`on` must be true or false")?;
            ctx.perf.set_hud(on);
            json!(on)
        }
        "game.quit" => {
            ctx.quit();
            json!(true)
        }
        "events.list" => Value::Array(ctx.events.reflected_json(&ctx.world)),
        "events.subscribe" => {
            let types: Vec<String> = match params.get("types") {
                None | Some(Value::Null) => Vec::new(),
                Some(v) => {
                    serde_json::from_value(v.clone()).map_err(|_| "`types` must be a list of event type names")?
                }
            };
            let capacity = match params.get("capacity") {
                None | Some(Value::Null) => 1000,
                Some(v) => v.as_u64().ok_or("`capacity` must be a whole number")?.min(100_000) as usize,
            };
            ctx.events.subscribe(&ctx.world, types.clone(), capacity);
            json!({ "types": types, "capacity": capacity })
        }
        "events.take" => {
            let feed = ctx
                .events
                .feed
                .as_mut()
                .ok_or("not subscribed: call events.subscribe first")?;
            let events: Vec<Value> = feed.items.drain(..).collect();
            json!({ "events": events, "dropped": std::mem::take(&mut feed.dropped) })
        }
        "events.unsubscribe" => json!(ctx.events.feed.take().is_some()),
        "camera.get" => camera_json(ctx),
        "shaders.status" => shader_status_json(&ctx.gfx),
        "camera.set" => {
            let vec = |name: &str| -> Result<Option<Vec2>, String> {
                match params.get(name) {
                    None | Some(Value::Null) => Ok(None),
                    Some(v) => serde_json::from_value::<[f32; 2]>(v.clone())
                        .map(|[x, y]| Some(Vec2::new(x, y)))
                        .map_err(|_| format!("`{name}` must be [x, y]")),
                }
            };
            let (position, look_at) = (vec("position")?, vec("look_at")?);
            let zoom = match params.get("zoom") {
                None | Some(Value::Null) => None,
                Some(v) => Some(
                    v.as_f64()
                        .filter(|z| *z > 0.0)
                        .ok_or("`zoom` must be a positive number")? as f32,
                ),
            };
            if let Some(p) = position {
                ctx.camera.position = p;
            }
            if let Some(z) = zoom {
                let (lo, hi) = crate::d2::Camera::ZOOM_LIMITS;
                ctx.camera.zoom = z.clamp(lo, hi);
            }
            if let Some(p) = look_at {
                ctx.camera.look_at(p);
            }
            camera_json(ctx)
        }
        // Windowed games answer it after the next frame (see `answer`);
        // headless ones were refused above.
        "screenshot" => return Err("screenshots are taken after the next frame".into()),
        other => return Err(format!("unknown method `{other}` (call `methods` for the list)")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::d2::Transform as Transform2D;
    use glam::vec2;

    fn ctx() -> Ctx {
        Ctx::new(1.0 / 60.0, glam::Vec2::ZERO)
    }

    #[test]
    fn every_listed_method_is_handled() {
        let mut ctx = ctx();
        for (name, _) in METHODS {
            if *name == "game.quit" || *name == "screenshot" {
                continue;
            }
            let r = handle(&mut ctx, name, &json!({}));
            if let Err(e) = r {
                let uncaptured = *name == "logs.get" && e.contains("captured");
                let refused = *name == "input.scenario.load" && e.contains("input injection is off");
                assert!(e.starts_with("missing param") || uncaptured || refused, "{name}: {e}");
            }
        }
        assert!(
            handle(&mut ctx, "nope", &json!({}))
                .unwrap_err()
                .contains("unknown method")
        );
    }

    #[cfg(all(feature = "3d", feature = "window"))]
    #[test]
    fn shader_status_reports_errors_in_the_games_lines() {
        let mut ctx = ctx();
        assert_eq!(
            handle(&mut ctx, "shaders.status", &json!({})).unwrap()["shaders"],
            json!([])
        );
        let _ = ctx.gfx.try_create_shader(
            "red.wgsl",
            "// red\nfn surface(s: Surface) -> Surface {\n    return 1.0;\n}",
        );
        let r = handle(&mut ctx, "shaders.status", &json!({})).unwrap();
        let s = &r["shaders"][0];
        assert_eq!(
            (s["shader"].clone(), s["kind"].clone()),
            (json!("red.wgsl"), json!("material"))
        );
        assert_eq!((s["outcome"].clone(), s["ok"].clone()), (json!("failed"), json!(false)));
        let d = &s["diagnostics"][0];
        assert_eq!((d["line"].clone(), d["column"].clone()), (json!(3), json!(12)), "{d}");
        assert!(d["text"].as_str().unwrap().starts_with("red.wgsl:3:12: error:"), "{d}");
    }

    #[test]
    fn entity_methods_round_trip() {
        let mut ctx = ctx();
        let id = handle(
            &mut ctx,
            "entity.spawn",
            &json!({ "components": { "Transform2D": { "position": [1.0, 2.0] } } }),
        )
        .unwrap()["entity"]
            .as_u64()
            .unwrap();
        let set = handle(
            &mut ctx,
            "component.set",
            &json!({ "entity": id, "component": "Transform2D", "value": { "rotation": 2.0 } }),
        )
        .unwrap();
        assert_eq!(set["position"], json!([1.0, 2.0]));
        assert_eq!(set["rotation"], json!(2.0));
        let list = handle(&mut ctx, "entities.list", &json!({ "with": ["Transform2D"] })).unwrap();
        assert_eq!(list["total"], 1);
        assert_eq!(
            handle(&mut ctx, "entity.despawn", &json!({ "entity": id })).unwrap(),
            json!(true)
        );
        assert!(handle(&mut ctx, "entity.get", &json!({ "entity": id })).is_err());
    }

    #[test]
    fn list_pages_and_filters() {
        let mut ctx = ctx();
        struct Hidden;
        for i in 0..10 {
            ctx.world.spawn((Transform2D::at(vec2(i as f32, 0.0)),));
        }
        ctx.world.spawn((Hidden, Transform2D::default()));
        let page = handle(&mut ctx, "entities.list", &json!({ "limit": 3, "offset": 9 })).unwrap();
        assert_eq!(page["total"], 11);
        let got = page["entities"].as_array().unwrap();
        assert_eq!(got.len(), 2, "pages cross archetypes");
        assert_eq!(got[1]["other_components"], json!(["Hidden"]));
        assert!(got[1]["components"].get("Transform2D").is_some());
        let hidden = handle(&mut ctx, "entities.list", &json!({ "with": ["Hidden"] })).unwrap();
        assert_eq!(hidden["total"], 1, "filters accept unreflected names too");
        let none = handle(&mut ctx, "entities.list", &json!({ "with": ["Nope"] })).unwrap();
        assert_eq!(none["total"], 0);
    }

    #[test]
    fn sim_methods_drive_the_clock() {
        let mut ctx = ctx();
        handle(&mut ctx, "sim.pause", &json!({})).unwrap();
        assert!(ctx.time.is_paused());
        handle(&mut ctx, "sim.step", &json!({ "ticks": 3 })).unwrap();
        assert_eq!(ctx.take_pending_steps(), 3);
        handle(&mut ctx, "sim.speed", &json!({ "speed": 4.0 })).unwrap();
        assert_eq!(ctx.time.speed(), 4.0);
        assert!(handle(&mut ctx, "sim.speed", &json!({ "speed": -1.0 })).is_err());
    }

    #[test]
    fn perf_reports_list_the_frames_gpu_uploads_and_reallocations() {
        let mut ctx = ctx();
        ctx.perf.last_render_stats.uploads = crate::render::GpuUploads {
            instances: 640,
            uniforms: 80,
            buffers_created: 3,
            reallocations: 2,
            buffer_bytes_created: 4096,
            ..Default::default()
        };
        let perf = handle(&mut ctx, "perf.get", &json!({})).unwrap();
        let u = &perf["render"]["uploads"];
        assert_eq!(u["bytes"], 720, "{u}");
        assert_eq!(u["instances"], 640, "{u}");
        assert_eq!(u["buffers_created"], 3, "{u}");
        assert_eq!(u["reallocations"], 2, "{u}");
        assert_eq!(u["buffer_bytes_created"], 4096, "{u}");
        // Every count, by the name it has in Rust.
        assert_eq!(u.as_object().unwrap().len(), 14, "{u}");
    }

    #[test]
    fn perf_reports_put_the_slowest_systems_first() {
        let mut ctx = ctx();
        ctx.perf.timings = ["fast", "slow", "middle"]
            .iter()
            .zip([0.1, 3.0, 1.0])
            .map(|(&name, avg_ms)| crate::perf::SystemTiming {
                name,
                sim: false,
                last_ms: avg_ms,
                avg_ms,
            })
            .collect();
        let perf = handle(&mut ctx, "perf.get", &json!({})).unwrap();
        let names: Vec<_> = perf["systems"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["name"].clone())
            .collect();
        assert_eq!(names, [json!("slow"), json!("middle"), json!("fast")]);
        for key in [
            "fps",
            "frame_ms",
            "cpu_ms",
            "gpu_passes",
            "render",
            "assets",
            "render_inspector",
        ] {
            assert!(perf.get(key).is_some(), "{key} missing from {perf}");
        }
        // No renderer has drawn a frame: no made-up zeros.
        let inspector = &perf["render_inspector"];
        assert_eq!(inspector["schema"], 1);
        assert!(
            inspector["gpu_ms"].is_null() && inspector["unavailable"]["gpu_ms"].is_string(),
            "{inspector}"
        );
        assert!(handle(&mut ctx, "perf.hud", &json!({ "on": true })).unwrap() == json!(true));
        assert!(ctx.perf.hud());
        assert!(handle(&mut ctx, "perf.hud", &json!({})).is_err(), "`on` is required");
        assert!(perf.get("report").is_none(), "the deep report needs profiling on");
        let deep = handle(&mut ctx, "perf.get", &json!({ "profiling": true })).unwrap();
        assert!(ctx.perf.profiling() && deep["report"].is_object(), "{deep}");
        assert!(handle(&mut ctx, "perf.get", &json!({ "profiling": 3 })).is_err());
        ctx.perf.set_profiling(false);
    }

    #[test]
    fn headless_games_cant_trace_frames() {
        let mut ctx = ctx();
        ctx.set_headless(true);
        let err = handle(&mut ctx, "perf.trace", &json!({})).unwrap_err();
        assert!(err.contains("headless"), "{err}");
        let perf = handle(&mut ctx, "perf.get", &json!({})).unwrap();
        assert_eq!(perf["headless"], true);
        let inspector = &perf["render_inspector"];
        assert!(inspector["cpu_ms"].is_null(), "{inspector}");
        assert!(
            inspector["unavailable"]["cpu_ms"]
                .as_str()
                .unwrap()
                .contains("headless"),
            "{inspector}"
        );
        assert_eq!(inspector["quality"].is_object(), cfg!(feature = "3d"), "{inspector}");
        ctx.set_residency_budget(crate::residency::ResidencyKind::Mesh, Some(1024));
        let perf = handle(&mut ctx, "perf.get", &json!({})).unwrap();
        assert_eq!(perf["residency"]["mesh"]["budget"], 1024, "{perf}");
        assert_eq!(perf["residency"]["mesh"]["evictions"], 0, "{perf}");
        assert_eq!(perf["residency"]["texture"]["evictions"], 0, "{perf}");
        assert_eq!(perf["streaming"]["uploads"]["pending"], 0, "{perf}");
        ctx.stream_texture("missing.png");
        let perf = handle(&mut ctx, "perf.get", &json!({})).unwrap();
        let asset = &perf["streaming"]["assets"][0];
        assert_eq!(
            (asset["kind"].as_str(), asset["state"].as_str()),
            (Some("texture"), Some("loading")),
            "{perf}"
        );
        assert_eq!(perf["streaming"]["uploads"]["pending"], 1, "{perf}");
    }

    #[test]
    fn headless_games_report_no_frames_and_only_sim_timings() {
        let mut ctx = ctx();
        ctx.set_headless(true);
        ctx.perf.timings = vec![
            crate::perf::SystemTiming {
                name: "sim",
                sim: true,
                last_ms: 1.0,
                avg_ms: 1.0,
            },
            crate::perf::SystemTiming {
                name: "draw",
                sim: false,
                last_ms: 0.0,
                avg_ms: 0.0,
            },
        ];
        let info = handle(&mut ctx, "info", &json!({})).unwrap();
        assert_eq!((info["fps"].clone(), info["frame"].clone()), (Value::Null, Value::Null));
        let timings = handle(&mut ctx, "systems.timings", &json!({})).unwrap();
        assert_eq!(timings.as_array().unwrap().len(), 1);
        assert_eq!(timings[0]["name"], "sim");
        ctx.set_headless(false);
        assert!(handle(&mut ctx, "info", &json!({})).unwrap()["fps"].is_number());
        assert_eq!(
            handle(&mut ctx, "systems.timings", &json!({}))
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn ecs_diagnostics_list_archetypes_and_count_matches() {
        let mut ctx = ctx();
        ctx.world.spawn((Transform2D::at(vec2(1.0, 2.0)),));
        ctx.world.spawn((Transform2D::default(), 5u32));
        let d = handle(&mut ctx, "ecs.diagnostics", &json!({ "with": ["Transform2D"] })).unwrap();
        assert_eq!(d["version"], crate::diagnostics::ECS_DIAGNOSTICS_VERSION);
        assert_eq!(d["entities"], 2);
        assert_eq!(
            d["matching"],
            json!({ "with": ["Transform2D"], "archetypes": 2, "entities": 2 })
        );
        assert!(d["archetypes"].as_array().unwrap().iter().all(|a| a["entities"] != 0));
        assert!(d["archetypes_total"].as_u64().unwrap() >= 2);
        let one = handle(&mut ctx, "ecs.diagnostics", &json!({ "limit": 1 })).unwrap();
        assert_eq!(one["archetypes"].as_array().unwrap().len(), 1);
        assert!(one.get("matching").is_none());
        let err = handle(&mut ctx, "ecs.diagnostics", &json!({ "with": ["Nope"] })).unwrap_err();
        assert!(err.contains("unknown component `Nope`"), "{err}");
        assert!(handle(&mut ctx, "ecs.diagnostics", &json!({ "with": 3 })).is_err());
    }

    #[test]
    fn short_names() {
        assert_eq!(short("a::b::Health"), "Health");
        assert_eq!(short("Health"), "Health");
    }
}
