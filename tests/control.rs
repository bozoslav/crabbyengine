//! The control endpoint, end to end: a headless game driven over TCP, the
//! way `crabby-mcp` drives it.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crabbyengine::*;

fn registry_file(addr: SocketAddr) -> PathBuf {
    let dir = match std::env::var_os("CRABBY_INSTANCES_DIR") {
        Some(d) => PathBuf::from(d),
        None if cfg!(windows) => PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("crabby/instances"),
        None => PathBuf::from(std::env::var_os("HOME").unwrap()).join(".crabby/instances"),
    };
    dir.join(format!("{}-{}.json", std::process::id(), addr.port()))
}

struct Client {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    token: String,
    next_id: u64,
}

impl Client {
    fn connect(addr: SocketAddr, token: String) -> Self {
        let writer = TcpStream::connect(addr).unwrap();
        writer.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        let reader = BufReader::new(writer.try_clone().unwrap());
        Self {
            reader,
            writer,
            token,
            next_id: 0,
        }
    }

    fn raw(&mut self, msg: Value) -> Value {
        writeln!(self.writer, "{msg}").unwrap();
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        let reply = self.raw(json!({ "id": id, "token": self.token, "method": method, "params": params }));
        assert_eq!(reply["id"], id);
        match reply.get("error") {
            Some(e) => Err(e.as_str().unwrap().to_string()),
            None => Ok(reply["result"].clone()),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Counter(u32);

fn count(ctx: &mut Ctx) {
    ctx.world.query::<&mut Counter>().for_each(|c| c.0 += 1);
}

#[test]
fn a_paused_headless_game_is_driven_over_tcp() {
    let (addr_tx, addr_rx) = mpsc::channel();
    let game = thread::spawn(move || {
        Game::new("control-test", 1, 1)
            .control_endpoint(true)
            .sim_system(count)
            .run_headless(u64::MAX, move |ctx| {
                ctx.world.reflect::<Counter>();
                ctx.pause();
                addr_tx.send(ctx.control_addr().unwrap()).unwrap();
            })
    });
    let addr = addr_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(addr.ip().is_loopback());

    // The registry file says who we are and holds the token.
    let file = registry_file(addr);
    let info: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(info["name"], "control-test");
    assert_eq!(info["port"], addr.port());
    assert_eq!(info["headless"], true);
    let mut c = Client::connect(addr, info["token"].as_str().unwrap().to_string());

    // Wrong token: refused, and nothing happens.
    let reply = c.raw(json!({ "id": 7, "token": "nope", "method": "game.quit" }));
    assert!(reply["error"].as_str().unwrap().contains("token"), "{reply}");

    let status = c.call("info", json!({})).unwrap();
    assert_eq!(
        (status["paused"].clone(), status["tick"].clone()),
        (json!(true), json!(0))
    );

    let e = c
        .call("entity.spawn", json!({ "components": { "Counter": 0 } }))
        .unwrap()["entity"]
        .as_u64()
        .unwrap();

    // Paused: only the requested ticks run.
    c.call("sim.step", json!({ "ticks": 5 })).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while c.call("info", json!({})).unwrap()["tick"] != 5 {
        assert!(Instant::now() < deadline, "ticks never ran");
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        c.call("entity.get", json!({ "entity": e })).unwrap(),
        json!({ "Counter": 5 })
    );
    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        c.call("info", json!({})).unwrap()["tick"],
        5,
        "no ticks beyond the steps"
    );

    let summary = c.call("world.summary", json!({})).unwrap();
    assert_eq!(summary["entities"], 1);
    assert!(
        summary["reflected_components"]
            .as_array()
            .unwrap()
            .contains(&json!("Counter"))
    );
    assert!(summary["allocated_bytes"].as_u64().unwrap() > 0, "memory is reported");
    assert!(
        summary["archetypes"][0]["bytes_per_entity"].as_u64().unwrap() >= 8,
        "at least the entity handle"
    );

    let info = c.call("info", json!({})).unwrap();
    assert!(info["render"].is_null(), "headless games draw nothing");
    let cam = c
        .call("camera.set", json!({ "zoom": 2.0, "look_at": [100.0, 50.0] }))
        .unwrap();
    assert_eq!(cam["zoom"], 2.0);
    assert_eq!(cam["center"], json!([100.0, 50.0]));
    assert_eq!(c.call("camera.get", json!({})).unwrap(), cam);
    assert!(
        c.call("camera.set", json!({ "zoom": -1 }))
            .unwrap_err()
            .contains("positive")
    );

    let err = c.call("screenshot", json!({})).unwrap_err();
    assert!(err.contains("no window"), "{err}");

    let err = c.call("entity.get", json!({ "entity": 999_999 })).unwrap_err();
    assert!(err.contains("does not exist"), "{err}");
    assert!(
        c.call("no.such.method", json!({}))
            .unwrap_err()
            .contains("unknown method")
    );

    c.call("game.quit", json!(null)).unwrap();
    let mut world = game.join().unwrap();
    assert_eq!(world.query::<&Counter>().iter().next().unwrap().0, 5);
    assert!(!file.exists(), "registry file is removed on exit");
}

#[test]
fn two_instances_get_their_own_endpoints() {
    let start = || {
        let (tx, rx) = mpsc::channel();
        let h = thread::spawn(move || {
            Game::new("twin", 1, 1)
                .control_endpoint(true)
                .run_headless(u64::MAX, move |ctx| {
                    ctx.pause();
                    tx.send(ctx.control_addr().unwrap()).unwrap();
                })
        });
        (rx.recv_timeout(Duration::from_secs(10)).unwrap(), h)
    };
    let (a, ha) = start();
    let (b, hb) = start();
    assert_ne!(a.port(), b.port());
    for (addr, h) in [(a, ha), (b, hb)] {
        let info: Value = serde_json::from_str(&std::fs::read_to_string(registry_file(addr)).unwrap()).unwrap();
        let mut c = Client::connect(addr, info["token"].as_str().unwrap().into());
        c.call("game.quit", json!({})).unwrap();
        h.join().unwrap();
    }
}

#[test]
fn the_endpoint_is_off_by_default() {
    let mut addr = Some("127.0.0.1:1".parse().unwrap());
    Game::new("t", 1, 1).run_headless(1, |ctx| addr = ctx.control_addr());
    assert_eq!(addr, None);
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Score {
    red: u32,
    blue: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Weather(String);

#[test]
fn reflected_resources_are_read_and_written_over_tcp() {
    let (addr_tx, addr_rx) = mpsc::channel();
    let game = thread::spawn(move || {
        Game::new("resources-test", 1, 1)
            .control_endpoint(true)
            .sim_system(|ctx: &mut Ctx| ctx.world.resource_mut::<Score>().red += 1)
            .run_headless(u64::MAX, move |ctx| {
                ctx.world.reflect_resource::<Score>();
                ctx.world.reflect_resource::<Weather>();
                ctx.world.insert_resource(Score { red: 0, blue: 0 });
                ctx.pause();
                addr_tx.send(ctx.control_addr().unwrap()).unwrap();
            })
    });
    let addr = addr_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let info: Value = serde_json::from_str(&std::fs::read_to_string(registry_file(addr)).unwrap()).unwrap();
    let mut c = Client::connect(addr, info["token"].as_str().unwrap().to_string());

    assert_eq!(
        c.call("resources.list", json!({})).unwrap(),
        json!({ "Score": { "red": 0, "blue": 0 }, "Weather": null }),
        "missing resources are listed as null"
    );
    let summary = c.call("world.summary", json!({})).unwrap();
    assert_eq!(summary["reflected_resources"], json!(["Score", "Weather"]));

    // Partial objects merge; the game sees the change on its next tick.
    let score = c
        .call("resource.set", json!({ "resource": "Score", "value": { "blue": 7 } }))
        .unwrap();
    assert_eq!(score, json!({ "red": 0, "blue": 7 }));
    c.call("sim.step", json!({ "ticks": 3 })).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while c.call("info", json!({})).unwrap()["tick"] != 3 {
        assert!(Instant::now() < deadline, "ticks never ran");
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        c.call("resource.get", json!({ "resource": "Score" })).unwrap(),
        json!({ "red": 3, "blue": 7 })
    );

    // A missing resource is inserted.
    c.call("resource.set", json!({ "resource": "Weather", "value": "rain" }))
        .unwrap();
    assert_eq!(
        c.call("resource.get", json!({ "resource": "Weather" })).unwrap(),
        "rain"
    );

    let err = c.call("resource.get", json!({ "resource": "Gold" })).unwrap_err();
    assert!(err.contains("no reflected resource named `Gold`"), "{err}");
    let err = c
        .call(
            "resource.set",
            json!({ "resource": "Score", "value": { "red": "lots" } }),
        )
        .unwrap_err();
    assert!(err.contains("bad value for `Score`"), "{err}");

    c.call("game.quit", json!(null)).unwrap();
    let world = game.join().unwrap();
    assert_eq!(world.resource::<Score>().blue, 7);
    assert_eq!(world.resource::<Weather>().0, "rain");
}

#[derive(serde::Serialize)]
struct Ping {
    tick: u64,
}

fn wait_for_tick(c: &mut Client, tick: u64) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while c.call("info", json!({})).unwrap()["tick"] != tick {
        assert!(Instant::now() < deadline, "tick {tick} never came");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn subscribers_get_reflected_events_with_their_ticks() {
    let (addr_tx, addr_rx) = mpsc::channel();
    let game = thread::spawn(move || {
        Game::new("events-test", 1, 1)
            .control_endpoint(true)
            .sim_system(|ctx| {
                let tick = ctx.sim_tick();
                ctx.events.send(Ping { tick });
            })
            .run_headless(u64::MAX, move |ctx| {
                ctx.events.reflect::<Ping>();
                ctx.pause();
                addr_tx.send(ctx.control_addr().unwrap()).unwrap();
            })
    });
    let addr = addr_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let info: Value = serde_json::from_str(&std::fs::read_to_string(registry_file(addr)).unwrap()).unwrap();
    let mut c = Client::connect(addr, info["token"].as_str().unwrap().to_string());

    assert!(c.call("events.take", json!({})).unwrap_err().contains("subscribe"));
    let list = c.call("events.list", json!({})).unwrap();
    assert_eq!(list, json!([{ "name": "Ping", "kept": 0 }]));

    c.call("events.subscribe", json!({ "types": ["Ping"] })).unwrap();
    c.call("sim.step", json!({ "ticks": 5 })).unwrap();
    wait_for_tick(&mut c, 5);
    let got = c.call("events.take", json!({})).unwrap();
    let ticks: Vec<u64> = got["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["tick"].as_u64().unwrap())
        .collect();
    assert_eq!(ticks, [0, 1, 2, 3, 4]);
    assert_eq!(
        got["events"][2],
        json!({ "tick": 2, "type": "Ping", "data": { "tick": 2 } })
    );
    assert_eq!(got["dropped"], 0);
    assert_eq!(c.call("events.take", json!({})).unwrap()["events"], json!([]));

    // A small buffer keeps the newest and counts the rest.
    c.call("events.subscribe", json!({ "capacity": 2 })).unwrap();
    c.call("sim.step", json!({ "ticks": 5 })).unwrap();
    wait_for_tick(&mut c, 10);
    let got = c.call("events.take", json!({})).unwrap();
    let ticks: Vec<u64> = got["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["tick"].as_u64().unwrap())
        .collect();
    assert_eq!((ticks, got["dropped"].clone()), (vec![8, 9], json!(3)));

    assert_eq!(c.call("events.unsubscribe", json!({})).unwrap(), json!(true));
    assert!(c.call("events.take", json!({})).is_err());
    let summary = c.call("world.summary", json!({})).unwrap();
    assert_eq!(summary["reflected_events"][0]["name"], "Ping");
    c.call("game.quit", json!({})).unwrap();
    game.join().unwrap();
}

/// Lines this test printed. Other tests share the process, and the test
/// harness's progress dots (no newline) can end up in front of ours.
fn ours(logs: &Value) -> Vec<String> {
    let lines = logs["lines"].as_array().unwrap().iter().filter_map(|l| l.as_str());
    lines
        .filter_map(|l| l.find("logs-test").map(|i| l[i..].to_string()))
        .collect()
}

#[test]
fn the_game_output_is_kept_for_tools() {
    let (addr_tx, addr_rx) = mpsc::channel();
    let game = thread::spawn(move || {
        Game::new("logs-test", 1, 1)
            .control_endpoint(true)
            .sim_system(|ctx: &mut Ctx| {
                // Straight to the file descriptors, past the test harness's capture.
                writeln!(std::io::stdout(), "logs-test tick {}", ctx.sim_tick()).unwrap();
            })
            .run_headless(u64::MAX, move |ctx| {
                ctx.pause();
                writeln!(std::io::stderr(), "logs-test setup on stderr").unwrap();
                addr_tx.send(ctx.control_addr().unwrap()).unwrap();
            })
    });
    let addr = addr_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let info: Value = serde_json::from_str(&std::fs::read_to_string(registry_file(addr)).unwrap()).unwrap();
    let mut c = Client::connect(addr, info["token"].as_str().unwrap().to_string());

    let first = c.call("logs.get", json!({ "lines": 5000 })).unwrap();
    assert_eq!(ours(&first), ["logs-test setup on stderr"], "{first}");

    // Only what's new since the last read.
    c.call("sim.step", json!({ "ticks": 3 })).unwrap();
    let mut since = first["next"].as_u64().unwrap();
    let mut seen = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while seen.len() < 3 {
        assert!(Instant::now() < deadline, "saw only {seen:?}");
        let new = c.call("logs.get", json!({ "since": since })).unwrap();
        since = new["next"].as_u64().unwrap();
        seen.extend(ours(&new));
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(seen, ["logs-test tick 0", "logs-test tick 1", "logs-test tick 2"]);
    assert!(c.call("methods", json!({})).unwrap().to_string().contains("logs.get"));
    c.call("game.quit", json!({})).unwrap();
    game.join().unwrap();
}

/// With `CHILD_EXITS=1` this test is the child: a game with the endpoint on
/// prints more than a pipe holds and calls `std::process::exit` mid-tick,
/// skipping every destructor; the engine's exit hook passes the rest on.
#[test]
fn output_printed_right_before_exit_still_reaches_the_terminal() {
    if std::env::var_os("CHILD_EXITS").is_some() {
        Game::new("exit-test", 1, 1)
            .control_endpoint(true)
            .sim_system(|_: &mut Ctx| {
                for i in 0..20_000 {
                    writeln!(std::io::stdout(), "exit-test line {i}").unwrap();
                }
                std::process::exit(0);
            })
            .run_headless(1, |_| {});
        return;
    }
    let me = std::env::current_exe().unwrap();
    let out = std::process::Command::new(me)
        .args([
            "--exact",
            "output_printed_right_before_exit_still_reaches_the_terminal",
            "--nocapture",
        ])
        .env("CHILD_EXITS", "1")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("exit-test line 19999"), "lost the last lines: {text}");
}
