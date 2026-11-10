//! Replication helpers on top of byte messages: snapshot interpolation,
//! authority epochs and interest by distance. Run with
//! `cargo run --example net_replication --no-default-features`.
#[allow(unused_imports)]
use ponko::net::{
    Authority, AuthorityCheck, Interest, InterestConfig, InterestEntity, InterestViewer, LatestWins, NetMessages,
    PeerId, Pose, SnapshotState, SnapshotStream, UpdateStamp,
};
use ponko::prelude::*;

fn main() {
    // ANCHOR: snapshots
    // One stream per remote entity: it holds the snapshots received and learns
    // the sender's clock from their timestamps. The wire format is yours.
    let mut car = SnapshotStream::<Pose>::new(); // shown 100 ms in the past
    let mut newest = LatestWins::new(); // drops duplicate and late packets
    for n in 0u16..20 {
        // In a game this is a decoded packet from `NetMessages::events()`:
        let (entity, stamp) = (7u64, UpdateStamp { epoch: 0, seq: n });
        let sender_time = 5000.0 + f64::from(n) * 0.05; // on the sender's clock
        let arrived = f64::from(n) * 0.05 + 0.03; // on yours, e.g. ctx.time.elapsed()
        let pose = Pose::at(vec3(n as f32, 0.0, 0.0)).moving(vec3(20.0, 0.0, 0.0));
        if newest.accept(entity, stamp) {
            car.push(sender_time, arrived, pose);
        }
    }
    // Every frame: where to draw it. Between snapshots it is interpolated;
    // when packets are late it is extrapolated for 250 ms, then held.
    let shown = car.sample(1.0).unwrap();
    println!("{:?} at {:?}", shown.state, shown.value.position);
    // ANCHOR_END: snapshots
    assert_eq!(shown.state, SnapshotState::Interpolated);
    assert!((shown.value.position.x - 17.4).abs() < 0.05);
    assert!(!newest.accept(7, UpdateStamp { epoch: 0, seq: 3 }));

    // ANCHOR: authority
    let (host, guest) = (PeerId::HOST, PeerId::from_bits(1));
    let mut vehicle = Authority::new(host); // the host owns it...
    let epoch = vehicle.handoff(guest); // ...until the guest takes the wheel.
    // Announce (guest, epoch) on a reliable lane; every other peer calls
    // `vehicle.adopt(guest, epoch)` when it arrives. State updates carry the
    // epoch they were sent under, and the receiver checks it:
    assert_eq!(vehicle.check(host, 0), AuthorityCheck::Stale); // late, from before the handoff
    assert_eq!(vehicle.check(guest, epoch), AuthorityCheck::Accept);
    assert!(ponko::net::seq_newer(2, 65535)); // sequence numbers wrap around
    // ANCHOR_END: authority

    // ANCHOR: interest
    // Near entities every tick, mid ones every 4th, far ones never.
    let mut interest = Interest::new(InterestConfig::default());
    let viewers = [InterestViewer {
        peer: PeerId::HOST,
        position: vec3(0.0, 0.0, 0.0),
    }];
    let world = [
        InterestEntity {
            id: 1,
            position: vec3(10.0, 0.0, 0.0),
        },
        InterestEntity {
            id: 2,
            position: vec3(500.0, 0.0, 0.0),
        },
    ];
    let mut sent = 0;
    for tick in 0..8u64 {
        interest.update(tick, &viewers, &world);
        for change in interest.changes() {
            println!("{:?} {:?} for peer {:?}", change.kind, change.entity, change.peer);
        }
        for entry in interest.due(PeerId::HOST) {
            sent += 1; // send entity `entry.entity` to this peer now
            let _ = entry;
        }
    }
    // ANCHOR_END: interest
    assert_eq!(sent, 8);
}
