//! Lockstep: the same small input-driven sim offline and as two loopback
//! peers (one thread each). The gap is what the session costs per tick;
//! bytes per tick is what it would send over the network.
use std::thread;

use ponko::{Ctx, Game, Net};

use crate::Bench;

pub const TICKS: u64 = 10_000;

struct Total(u64);

fn sim(ctx: &mut Ctx) {
    ctx.net.set_input(&(ctx.time.sim_tick() as u32, ctx.net.player() as u8));
    let mut sum = 0;
    for p in 0..ctx.net.players() {
        if let Some((tick, _)) = ctx.net.input::<(u32, u8)>(p) {
            sum += u64::from(tick);
        }
    }
    ctx.world.resource_mut::<Total>().0 += sum;
}

fn game() -> Game {
    Game::new("bench", 1, 1).sim_system(sim)
}

pub fn run(b: &mut Bench) {
    b.run(
        "net/offline",
        3,
        || (),
        |_| {
            let world = game().run_headless(TICKS, |ctx| ctx.world.insert_resource(Total(0)));
            std::hint::black_box(world.resource::<Total>().0);
        },
    );
    let mut bytes = 0;
    b.run(
        "net/loopback-2p",
        3,
        || (),
        |_| {
            let peers: Vec<_> = Net::loopback(2)
                .into_iter()
                .map(|net| {
                    thread::spawn(move || {
                        let world = game()
                            .lockstep(net)
                            .sim_system(|ctx: &mut Ctx| {
                                let sent = ctx.net.stats().bytes_sent;
                                ctx.world.insert_resource(Sent(sent));
                            })
                            .run_headless(TICKS, |ctx| ctx.world.insert_resource(Total(0)));
                        (world.resource::<Total>().0, world.resource::<Sent>().0)
                    })
                })
                .collect();
            let results: Vec<_> = peers.into_iter().map(|p| p.join().unwrap()).collect();
            assert_eq!(results[0].0, results[1].0, "same sum on both sides");
            bytes = results[1].1;
        },
    );
    println!(
        "net: the guest sent {:.1} bytes/tick ({} ticks)",
        bytes as f64 / TICKS as f64,
        TICKS
    );
}

struct Sent(u64);
