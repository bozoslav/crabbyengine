//! The control endpoint: lets tools (agents through `crabby-mcp`, the
//! inspector, scripts) look inside a running game and drive it.
//!
//! Off by default. Turned on with [`Game::control_endpoint`](crate::Game::control_endpoint)
//! or the `CRABBY_CONTROL=1` environment variable, so tools can enable it
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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crabby_ecs::Entity;
use glam::Vec2;
use serde_json::{Value, json};

use crate::ctx::Ctx;
use crate::reflect::WorldReflect;

mod query;
use query::{Condition, FieldPath, FieldStats};

/// Environment variable that turns the endpoint on (`1`).
pub(crate) const ENV_CONTROL: &str = "CRABBY_CONTROL";
/// Environment variable that starts the simulation paused (`1`).
pub(crate) const ENV_START_PAUSED: &str = "CRABBY_START_PAUSED";
/// Environment variable overriding where instances register themselves.
pub(crate) const ENV_INSTANCES_DIR: &str = "CRABBY_INSTANCES_DIR";
/// Environment variable a launching tool sets; copied into the registry file.
pub(crate) const ENV_LAUNCH_ID: &str = "CRABBY_LAUNCH_ID";

/// How long a connection waits for the game to answer before giving up
/// (a game stuck in a long frame, or paused in a debugger).
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);

/// How often the accept loop checks whether the game has shut down.
const ACCEPT_POLL: Duration = Duration::from_millis(20);

/// Upper bound for `entities.list`, so one request can't produce gigabytes.
const MAX_LIST: usize = 10_000;

pub(crate) fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// The directory where running instances register: `$CRABBY_INSTANCES_DIR`,
/// else `~/.crabby/instances` (`%LOCALAPPDATA%\crabby\instances` on Windows).
pub(crate) fn instances_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(ENV_INSTANCES_DIR) {
        return Some(PathBuf::from(dir));
    }
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("crabby").join("instances"))
    } else {
        std::env::var_os("HOME").map(|d| PathBuf::from(d).join(".crabby").join("instances"))
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
                "launch_id": std::env::var(ENV_LAUNCH_ID).ok(),
            });
            match write_registry(&dir, addr.port(), &info) {
                Ok(path) => Some(path),
                Err(e) => {
                    eprintln!(
                        "crabbyengine: control endpoint could not register in {}: {e}",
                        dir.display()
                    );
                    None
                }
            }
        });

        let stop = shutdown.clone();
        thread::Builder::new()
            .name("crabby-control".into())
            .spawn(move || accept_loop(listener, token, tx, stop))?;
        let logs = crate::logs::Capture::start();
        eprintln!("crabbyengine: control endpoint on {addr}");
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
                    .name("crabby-control-conn".into())
                    .spawn(move || serve_connection(stream, &token, &tx));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => thread::sleep(ACCEPT_POLL),
            Err(_) => thread::sleep(ACCEPT_POLL),
        }
    }
}

fn serve_connection(stream: TcpStream, token: &str, tx: &Sender<Request>) {
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
        let reply = handle_line(&line, token, tx);
        if writeln!(writer, "{reply}").and_then(|()| writer.flush()).is_err() {
            return;
        }
    }
}

fn handle_line(line: &str, token: &str, tx: &Sender<Request>) -> Value {
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
    let req = Request {
        method: method.to_string(),
        params: msg.get("params").cloned().unwrap_or(Value::Null),
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
    world: &crabby_ecs::World,
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
    for a in world.archetypes().iter().filter(|a| !a.is_empty()) {
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
                ctx.queue_screenshot(path, max_size, Some(req.reply));
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
        None | Some(Value::Null) => {
            Ok(std::env::temp_dir().join(format!("crabby-screenshot-{}-{}.png", std::process::id(), ctx.frame())))
        }
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
    ("systems.timings", "each system's last and average time in ms"),
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
    ("game.quit", "ask the game to exit"),
];

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

/// `crate::units::Health` → `Health`, for archetype listings.
fn short(type_name: &str) -> &str {
    let end = type_name.find('<').unwrap_or(type_name.len());
    type_name[..end].rfind("::").map_or(type_name, |i| &type_name[i + 2..])
}

pub(crate) fn handle(ctx: &mut Ctx, method: &str, params: &Value) -> Result<Value, String> {
    let err = |e: crate::ReflectError| e.to_string();
    Ok(match method {
        "methods" => Value::Array(
            METHODS
                .iter()
                .map(|(name, about)| json!({ "name": name, "about": about }))
                .collect(),
        ),
        "info" => json!({
            "pid": std::process::id(),
            "headless": ctx.headless,
            "tick": ctx.sim_tick(),
            "sim_time": ctx.sim_time(),
            "paused": ctx.is_paused(),
            "speed": ctx.sim_speed(),
            "fixed_dt": ctx.fixed_dt(),
            // Headless games have no frames, so no frame rate.
            "fps": if ctx.headless { Value::Null } else { json!(ctx.fps()) },
            "frame": if ctx.headless { Value::Null } else { json!(ctx.frame()) },
            "entities": ctx.world.len(),
            "render": if ctx.headless { Value::Null } else {
                let r = ctx.render_stats();
                json!({ "sprites": r.sprites, "draw_calls": r.draw_calls, "cpu_ms": r.cpu_ms, "gpu_ms": r.gpu_ms })
            },
        }),
        "logs.get" => logs_get(params)?,
        "world.summary" => {
            let world = &ctx.world;
            let archetypes: Vec<Value> = world
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
                    Err(crate::ReflectError::MissingResource(_)) => Value::Null,
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
                "tick": ctx.sim_tick(),
            })
        }
        "scene.save" => {
            let path = str_param(params, "path")?;
            ctx.save_scene(path)?;
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
            ctx.save_prefab(e, path)?;
            json!({ "path": path })
        }
        "world.load" => {
            let path = str_param(params, "path")?;
            let text = std::fs::read_to_string(path).map_err(|e| format!("could not read {path}: {e}"))?;
            let save: Value = serde_json::from_str(&text).map_err(|e| format!("{path} is not JSON: {e}"))?;
            ctx.load_scene_assets(&save)?;
            ctx.world.load(&save).map_err(err)?;
            json!({ "path": path, "entities": ctx.world.len() })
        }
        "sim.pause" => {
            ctx.pause();
            json!({ "paused": true, "tick": ctx.sim_tick() })
        }
        "sim.resume" => {
            ctx.resume();
            json!({ "paused": false, "tick": ctx.sim_tick() })
        }
        "sim.step" => {
            let ticks = usize_param(params, "ticks", 1)?;
            let ticks = u32::try_from(ticks).map_err(|_| "`ticks` is too large")?;
            for _ in 0..ticks {
                ctx.step();
            }
            json!({ "queued_ticks": ticks, "tick": ctx.sim_tick() })
        }
        "sim.speed" => {
            let speed = param(params, "speed")?
                .as_f64()
                .filter(|s| s.is_finite() && *s >= 0.0)
                .ok_or("`speed` must be a finite number >= 0")?;
            ctx.set_sim_speed(speed as f32);
            json!({ "speed": ctx.sim_speed() })
        }
        "systems.timings" => Value::Array(
            ctx.system_timings()
                .iter()
                // Headless games run only simulation systems.
                .filter(|t| t.sim || !ctx.headless)
                .map(|t| json!({ "name": t.name, "sim": t.sim, "last_ms": t.last_ms, "avg_ms": t.avg_ms }))
                .collect(),
        ),
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
                let (lo, hi) = crate::Camera2D::ZOOM_LIMITS;
                ctx.camera.zoom = z.clamp(lo, hi);
            }
            if let Some(p) = look_at {
                ctx.camera.look_at(p);
            }
            camera_json(ctx)
        }
        // Windowed games answer it after the next frame (see `answer`).
        "screenshot" => {
            return Err("headless games have no window to capture; launch the game with a window".into());
        }
        other => return Err(format!("unknown method `{other}` (call `methods` for the list)")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Transform2D;
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
                assert!(e.starts_with("missing param") || uncaptured, "{name}: {e}");
            }
        }
        assert!(
            handle(&mut ctx, "nope", &json!({}))
                .unwrap_err()
                .contains("unknown method")
        );
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
        assert!(ctx.is_paused());
        handle(&mut ctx, "sim.step", &json!({ "ticks": 3 })).unwrap();
        assert_eq!(ctx.take_pending_steps(), 3);
        handle(&mut ctx, "sim.speed", &json!({ "speed": 4.0 })).unwrap();
        assert_eq!(ctx.sim_speed(), 4.0);
        assert!(handle(&mut ctx, "sim.speed", &json!({ "speed": -1.0 })).is_err());
    }

    #[test]
    fn headless_games_report_no_frames_and_only_sim_timings() {
        let mut ctx = ctx();
        ctx.headless = true;
        ctx.timings = vec![
            crate::SystemTiming {
                name: "sim",
                sim: true,
                last_ms: 1.0,
                avg_ms: 1.0,
            },
            crate::SystemTiming {
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
        ctx.headless = false;
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
    fn short_names() {
        assert_eq!(short("a::b::Health"), "Health");
        assert_eq!(short("Health"), "Health");
    }
}
