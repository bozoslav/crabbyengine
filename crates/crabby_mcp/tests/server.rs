//! The MCP server end to end: real headless games with the control endpoint
//! on, driven through `Server::handle` the way an agent's client would.

use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crabby_mcp::{Config, Server};
use crabbyengine::*;
use serde_json::{Value, json};

#[derive(serde::Serialize, serde::Deserialize)]
struct Health {
    current: f32,
    max: f32,
}

#[derive(serde::Serialize)]
struct Regen {
    tick: u64,
}

fn regen(ctx: &mut Ctx) {
    ctx.world
        .query::<&mut Health>()
        .for_each(|h| h.current = (h.current + 1.0).min(h.max));
    let tick = ctx.sim_tick();
    ctx.events.send(Regen { tick });
}

/// Starts a paused headless game; returns its port and the thread.
fn start_game(name: &'static str) -> (u16, thread::JoinHandle<World>) {
    let (tx, rx) = mpsc::channel();
    let h = thread::spawn(move || {
        Game::new(name, 1, 1)
            .control_endpoint(true)
            .sim_system(regen)
            .run_headless(u64::MAX, move |ctx| {
                ctx.world.reflect::<Health>();
                ctx.events.reflect::<Regen>();
                ctx.pause();
                tx.send(ctx.control_addr().unwrap().port()).unwrap();
            })
    });
    (rx.recv_timeout(Duration::from_secs(10)).unwrap(), h)
}

struct Agent {
    server: Server,
    next: u64,
}

impl Agent {
    fn new(dir: &Path, read_only: bool) -> Self {
        let server = Server::new(Config {
            instances_dir: dir.to_path_buf(),
            logs_dir: dir.join("logs"),
            read_only,
        });
        Self { server, next: 0 }
    }

    fn rpc(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let reply = self
            .server
            .handle(&json!({ "jsonrpc": "2.0", "id": self.next, "method": method, "params": params }))
            .unwrap();
        assert_eq!(reply["id"], self.next);
        reply
    }

    /// Calls a tool; `Ok(json)` or `Err(message)`.
    fn tool(&mut self, name: &str, args: Value) -> Result<Value, String> {
        let r = self.rpc("tools/call", json!({ "name": name, "arguments": args }))["result"].clone();
        let text = r["content"][0]["text"].as_str().unwrap().to_string();
        if r["isError"] == true {
            Err(text)
        } else {
            Ok(serde_json::from_str(&text).unwrap())
        }
    }

    fn tool_names(&mut self) -> Vec<String> {
        self.rpc("tools/list", json!({}))["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect()
    }
}

// One test function, because it sets an environment variable that the games
// read (where to register), and tests in a file run in parallel.
#[test]
fn agents_drive_several_games_through_mcp() {
    let dir = std::env::temp_dir().join(format!("crabby-mcp-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // SAFETY: set before any other thread of this test binary starts.
    unsafe { std::env::set_var("CRABBY_INSTANCES_DIR", &dir) };

    let mut agent = Agent::new(&dir, false);

    // Handshake.
    let init = agent.rpc("initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } }));
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "crabby-mcp");
    assert!(
        agent
            .server
            .handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .is_none()
    );
    assert_eq!(agent.rpc("nope", json!({}))["error"]["code"], -32601);

    // No games yet.
    assert_eq!(agent.tool("list_instances", json!({})).unwrap(), json!([]));
    assert!(
        agent
            .tool("instance_info", json!({}))
            .unwrap_err()
            .contains("no game is running")
    );

    let (port_a, game_a) = start_game("alpha");
    let (port_b, game_b) = start_game("beta");

    let list = agent.tool("list_instances", json!({})).unwrap();
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert!(
        list.iter().all(|i| i.get("token").is_none()),
        "tokens are never shown to agents"
    );
    let err = agent.tool("world_summary", json!({})).unwrap_err();
    assert!(err.contains("2 games are running"), "{err}");

    let cam = agent
        .tool(
            "set_camera",
            json!({ "instance": "alpha", "zoom": 4.0, "position": [10.0, 20.0] }),
        )
        .unwrap();
    assert_eq!(cam["position"], json!([10.0, 20.0]));
    assert_eq!(cam["zoom"], 4.0);

    let err = agent.tool("screenshot", json!({ "instance": "alpha" })).unwrap_err();
    assert!(err.contains("no window"), "headless games say why: {err}");

    // Drive alpha by name, beta by port.
    let e = agent
        .tool(
            "spawn_entity",
            json!({ "instance": "alpha", "components": { "Health": { "current": 1, "max": 10 } } }),
        )
        .unwrap()["entity"]
        .as_u64()
        .unwrap();
    let ran = agent
        .tool("run_ticks", json!({ "instance": "alpha", "ticks": 4 }))
        .unwrap();
    assert_eq!(ran["ticks_run"], 4);
    assert_eq!(
        agent
            .tool("get_entity", json!({ "instance": "alpha", "entity": e }))
            .unwrap(),
        json!({ "Health": { "current": 5.0, "max": 10.0 } })
    );
    let set = agent
        .tool(
            "set_component",
            json!({ "instance": "alpha", "entity": e, "component": "Health", "value": { "max": 6 } }),
        )
        .unwrap();
    assert_eq!(set, json!({ "current": 5.0, "max": 6.0 }));
    agent
        .tool("run_ticks", json!({ "instance": "alpha", "ticks": 3 }))
        .unwrap();
    assert_eq!(
        agent
            .tool("get_entity", json!({ "instance": "alpha", "entity": e }))
            .unwrap()["Health"]["current"],
        6.0
    );

    // Beta is untouched.
    let b = json!(port_b.to_string());
    assert_eq!(
        agent.tool("instance_info", json!({ "instance": b })).unwrap()["tick"],
        0
    );
    assert_eq!(
        agent.tool("world_summary", json!({ "instance": b })).unwrap()["entities"],
        0
    );

    // run_ticks refuses a running sim, so "exactly N" holds.
    agent.tool("resume", json!({ "instance": b })).unwrap();
    assert!(
        agent
            .tool("run_ticks", json!({ "instance": b, "ticks": 1 }))
            .unwrap_err()
            .contains("pause")
    );
    agent.tool("pause", json!({ "instance": b })).unwrap();

    // A second agent, read-only: sees the games, can't change them.
    let mut viewer = Agent::new(&dir, true);
    let names = viewer.tool_names();
    assert!(names.contains(&"list_entities".to_string()));
    assert!(
        !names
            .iter()
            .any(|n| n == "spawn_entity" || n == "launch" || n == "run_ticks")
    );
    let listed = viewer
        .tool("list_entities", json!({ "instance": port_a, "with": ["Health"] }))
        .unwrap();
    assert_eq!(listed["total"], 1);
    let err = viewer
        .tool("despawn_entity", json!({ "instance": port_a, "entity": e }))
        .unwrap_err();
    assert!(err.contains("read-only"), "{err}");

    // Errors from the game come through as tool errors.
    let err = agent
        .tool("get_entity", json!({ "instance": "alpha", "entity": 123_456 }))
        .unwrap_err();
    assert!(err.contains("does not exist"), "{err}");
    assert!(
        agent
            .tool("instance_info", json!({ "instance": "gamma" }))
            .unwrap_err()
            .contains("no running instance")
    );
    assert!(
        agent
            .tool("read_logs", json!({ "instance": "alpha" }))
            .unwrap_err()
            .contains("launch")
    );

    // Checkpoint, experiment, roll back.
    let checkpoint = dir.join("checkpoint.json");
    let saved = agent
        .tool("save_world", json!({ "instance": "alpha", "path": checkpoint }))
        .unwrap();
    assert_eq!(saved["entities"], 1);
    agent
        .tool("despawn_entity", json!({ "instance": "alpha", "entity": e }))
        .unwrap();
    assert_eq!(
        agent.tool("world_summary", json!({ "instance": "alpha" })).unwrap()["entities"],
        0
    );
    // The checkpoint sits in the registry directory on purpose: discovery
    // must never delete files that aren't registry entries.
    agent.tool("list_instances", json!({})).unwrap();
    assert!(checkpoint.exists(), "discovery deleted a file it doesn't own");
    agent
        .tool("load_world", json!({ "instance": "alpha", "path": checkpoint }))
        .unwrap();
    assert_eq!(
        agent
            .tool("get_entity", json!({ "instance": "alpha", "entity": e }))
            .unwrap()["Health"]["current"],
        6.0,
        "the same entity id is back with its saved values"
    );
    // The read-only agent may save (it changes nothing) but not load.
    assert!(
        viewer
            .tool(
                "save_world",
                json!({ "instance": "alpha", "path": dir.join("viewer.json") })
            )
            .is_ok()
    );
    assert!(
        viewer
            .tool("load_world", json!({ "instance": "alpha", "path": checkpoint }))
            .unwrap_err()
            .contains("read-only")
    );

    // Events: what happened while ticks ran.
    let kinds = agent.tool("list_events", json!({ "instance": "alpha" })).unwrap();
    assert_eq!(kinds[0]["name"], "Regen");
    viewer
        .tool("subscribe_events", json!({ "instance": "alpha", "types": ["Regen"] }))
        .unwrap();
    let start = agent.tool("instance_info", json!({ "instance": "alpha" })).unwrap()["tick"]
        .as_u64()
        .unwrap();
    agent
        .tool("run_ticks", json!({ "instance": "alpha", "ticks": 3 }))
        .unwrap();
    let got = viewer.tool("take_events", json!({ "instance": "alpha" })).unwrap();
    let ticks: Vec<u64> = (got["events"].as_array().unwrap().iter())
        .map(|e| e["data"]["tick"].as_u64().unwrap())
        .collect();
    assert_eq!(ticks, [start, start + 1, start + 2]);

    agent.tool("quit_instance", json!({ "instance": "alpha" })).unwrap();
    agent.tool("quit_instance", json!({ "instance": b })).unwrap();
    game_a.join().unwrap();
    game_b.join().unwrap();
    assert_eq!(agent.tool("list_instances", json!({})).unwrap(), json!([]));
    let _ = std::fs::remove_dir_all(&dir);
}
