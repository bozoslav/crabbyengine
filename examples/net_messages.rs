//! Byte messages between two games. Run with
//! `cargo run --example net_messages --no-default-features`.
use ponko::net::{Lane, MessageEvent, NetMessages, PeerId};
use ponko::prelude::*;
use std::time::Duration;

// ANCHOR: messages
/// The host answers every message. The engine polls `ctx.net.messages`
/// before the systems run, so a system only reads events and sends.
fn host(ctx: &mut Ctx) {
    let messages = ctx.net.messages.as_mut().expect("Game::messages");
    let mut done = false;
    while let Some(event) = messages.next_event() {
        match event {
            MessageEvent::Connected(peer) => println!("host: {peer:?} joined"),
            MessageEvent::Message { peer, bytes, .. } => {
                println!("host: received {}", String::from_utf8_lossy(&bytes));
                messages.send_serde(peer, Lane::Reliable, &42u32).unwrap();
            }
            MessageEvent::Disconnected(peer, reason) => {
                println!("host: {peer:?} left ({reason})");
                done = true;
            }
        }
    }
    if done {
        ctx.quit();
    }
}

/// The client says hello once connected and leaves when the answer is in.
fn client(ctx: &mut Ctx) {
    let messages = ctx.net.messages.as_mut().expect("Game::messages");
    let mut answer = None;
    while let Some(event) = messages.next_event() {
        match &event {
            MessageEvent::Connected(host) => messages.send(*host, Lane::Reliable, b"hello").unwrap(),
            MessageEvent::Message { .. } => answer = event.decode::<u32>(),
            MessageEvent::Disconnected(_, reason) => panic!("{reason}"),
        }
    }
    if let Some(answer) = answer {
        println!("client: received {answer}");
        messages.disconnect(PeerId::HOST).unwrap();
        ctx.world.insert_resource(answer);
        ctx.quit();
    }
}

fn main() -> std::io::Result<()> {
    let endpoint = NetMessages::host("127.0.0.1:0", 8)?;
    let address = endpoint.local_addr().expect("a UDP endpoint");
    let hosting = std::thread::spawn(move || play(endpoint, host));
    let world = play(NetMessages::join(address)?, client);
    // ANCHOR_END: messages
    hosting.join().expect("the host");
    match world.get_resource::<u32>() {
        Some(42) => Ok(()),
        _ => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "no answer")),
    }
}

// ANCHOR: play
/// One game with a message endpoint. Headless here, so the example needs no
/// window; a windowed game is the same with `.run(..)`.
fn play(endpoint: NetMessages, system: fn(&mut Ctx)) -> World {
    Game::new("messages", 1, 1)
        .messages(endpoint)
        .sim_system(system)
        // A headless run doesn't wait for the display: don't spin.
        .sim_system(|_: &mut Ctx| std::thread::sleep(Duration::from_millis(1)))
        .run_headless(5000, |_| {})
}
// ANCHOR_END: play
