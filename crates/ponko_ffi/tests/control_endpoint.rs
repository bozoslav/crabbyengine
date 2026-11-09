//! `crabby_game_control_endpoint`: the game opens a token-protected localhost
//! endpoint, registers itself in the instances directory, answers a tool and
//! removes the registration when it ends. Runs headless; the only socket is on
//! the loopback interface.
use ponko_ffi::*;
use std::ffi::c_void;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use CrabbyStatus::*;

static STOP: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn quit_when_told(ctx: *mut CrabbyCtx, _: *mut c_void) {
    if STOP.load(Ordering::SeqCst) {
        unsafe { crabby_ctx_quit(ctx) };
    }
}

fn instances(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|entries| entries.map(|e| e.unwrap().path()).collect())
        .unwrap_or_default()
}

fn ask(stream: &mut TcpStream, reader: &mut BufReader<TcpStream>, request: serde_json::Value) -> serde_json::Value {
    writeln!(stream, "{request}").unwrap();
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

fn run_game(endpoint: bool) {
    unsafe {
        let game = crabby_game_new(c"c-control".as_ptr(), 1, 1);
        assert_eq!(crabby_game_control_endpoint(game, endpoint), CRABBY_OK);
        assert_eq!(
            crabby_game_sim_system(game, c"quit".as_ptr(), 1, Some(quit_when_told), null_mut()),
            CRABBY_OK
        );
        let mut world = null_mut();
        assert_eq!(
            crabby_game_run_headless(game, u64::MAX, None, null_mut(), &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
}

#[test]
fn the_control_endpoint_opens_registers_answers_and_closes() {
    let dir = std::env::temp_dir().join(format!("ponko-ffi-control-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // SAFETY: this test binary has one test, and no other thread reads the environment yet.
    unsafe { std::env::set_var("PONKO_INSTANCES_DIR", &dir) };
    assert_eq!(
        unsafe { crabby_game_control_endpoint(null_mut(), true) },
        CRABBY_INVALID_ARGUMENT
    );

    // Off (the default): the game registers nowhere.
    STOP.store(true, Ordering::SeqCst);
    run_game(false);
    assert!(instances(&dir).is_empty(), "no endpoint, no registration");

    // On: a client finds the registration, talks to the game, and ends it.
    STOP.store(false, Ordering::SeqCst);
    let client_dir = dir.clone();
    let client = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        let file = loop {
            if let Some(file) = instances(&client_dir).pop() {
                break file;
            }
            assert!(Instant::now() < deadline, "the game never registered");
            std::thread::sleep(Duration::from_millis(10));
        };
        let info: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(info["name"], "c-control");
        assert_eq!(info["headless"], true);
        assert_eq!(info["pid"], std::process::id());
        let port = info["port"].as_u64().unwrap();
        let mut stream = TcpStream::connect(("127.0.0.1", port as u16)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let token = info["token"].as_str().unwrap();

        let refused = ask(
            &mut stream,
            &mut reader,
            serde_json::json!({"id": 1, "token": "wrong", "method": "info"}),
        );
        assert!(refused["error"].as_str().unwrap().contains("token"), "{refused}");
        let reply = ask(
            &mut stream,
            &mut reader,
            serde_json::json!({"id": 2, "token": token, "method": "info"}),
        );
        assert_eq!(reply["id"], 2);
        assert!(reply["result"]["tick"].is_number(), "{reply}");
        STOP.store(true, Ordering::SeqCst);
        file
    });
    run_game(true);
    let file = client.join().unwrap();
    assert!(!file.exists(), "the registration is removed when the game ends");
    let _ = std::fs::remove_dir_all(&dir);
}
