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
