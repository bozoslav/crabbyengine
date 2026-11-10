//! Run with `cargo run --example net_messages --no-default-features`.
use ponko::net::{Lane, MessageEvent, NetMessages, PeerId};
use std::time::{Duration, Instant};

fn main() -> std::io::Result<()> {
    // ANCHOR: messages
    // The game owns the endpoint. In a game, call poll and drain events from
    // a frame system, including frames where the simulation is paused.
    let mut host = NetMessages::host("127.0.0.1:0", 8)?;
    let mut client = NetMessages::join(host.local_addr().unwrap())?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut received = false;
    while Instant::now() < deadline && !received {
        client.poll()?;
        let mut send_hello = false;
        for event in client.events() {
            match event {
                MessageEvent::Connected(_) => {
                    send_hello = true;
                }
                MessageEvent::Disconnected(_, reason) => eprintln!("{reason}"),
                _ => {}
            }
        }
        if send_hello {
            client.send(PeerId::HOST, Lane::Reliable, b"hello")?;
        }
        host.poll()?;
        // Collect actions separately because events() borrows the endpoint.
        let mut reply_to = None;
        for event in host.events() {
            if let MessageEvent::Message { peer, bytes, .. } = event {
                println!("received: {}", String::from_utf8_lossy(&bytes));
                reply_to = Some(peer);
                received = true;
            }
        }
        if let Some(peer) = reply_to {
            host.send_serde(peer, Lane::Unreliable, &42u32)?;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    // ANCHOR_END: messages
    if !received {
        return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "no message"));
    }
    Ok(())
}
