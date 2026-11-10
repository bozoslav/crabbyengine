# Multiplayer: deterministic lockstep

Lockstep is how RTS and simulation games do multiplayer: every player runs
the whole simulation, and only **inputs** cross the network. As long as
everyone applies the same inputs in the same order to the same start,
everyone has the same world, whether there are 10 units or 100,000. A
tick costs the same bandwidth either way: about 50 bytes per player.

Ponko's simulation is built for it already: a fixed clock, seeded
randomness per (seed, tick, system, entity), and deterministic queries and
physics. Lockstep adds the session: 2–8 players, plain UDP, no server to
run.

## Host and join

One player hosts, the others join by address:

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/lockstep.rs:session}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:lockstep_setup}}
```

</div>
</div>

The host is player 0; the others get 1, 2, ... in the order they join. The
game starts when everyone is in. Until then, and whenever a player's input
is late, the simulation waits: frames (drawing, UI) go on, ticks don't.

## Inputs out, inputs in

A player's input is whatever you send with `ctx.net.set_input(&value)`: a
serde type in Rust (a plain struct in C++, up to 1024 bytes). Set it in a
frame system from the keys and mouse; the session sends it. Then
simulation systems read **every** player's input for the tick and apply
it:

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/lockstep.rs:input}}

{{#include ../../examples/lockstep.rs:simulate}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:lockstep_input}}
```

</div>
</div>

The last input set before a tick goes into it and is then cleared, so set
it every frame for held keys. Without a session, `ctx.net` is offline: one
player whose input applies on the next tick. The same code runs alone and
online, so you can build the single-player game first.

## Input delay

Input set at tick `t` is used at tick `t + input_delay` on every machine
(default 3 ticks, 50 ms at 60 Hz). That's the time it has to reach the
others; as long as the round trip is shorter, nobody ever waits. A longer
delay hides a worse connection but makes controls feel laggier. Set it on
the host: `Net::host(addr, players)?.input_delay(5)`.

Packets get lost; nobody notices. Every packet repeats all inputs the other
side hasn't acknowledged yet, so the next one fills the gap.

## Staying in sync

Lockstep only works if the simulation really is deterministic. The rules:

- **Change the world only in simulation systems**, driven by inputs. Frame
  systems may read it and draw it, never change it.
- **Same start**: the same setup and the same `Game::seed` everywhere (a
  joining player with another seed is refused).
- **No outside state**: no wall clock, no `HashMap` iteration order, no
  thread-dependent results. Use `ctx.random()` for randomness;
  `par_for_each` and the spatial grid are deterministic.
- **Physics across platforms**: rapier gives the same result on the same
  platform. Players on different OSes or CPUs need the
  `physics-determinism` feature.

When something slips through, Ponko tells you. Every `checksum_every`
ticks (default 60) the peers compare a hash of every
[reflected](reflection.md) component and resource; on a mismatch,
`ctx.net.desync()` reports the tick, the player and **which components
differ**, so you know where to look:

```text
ponko lockstep: desync with player 1 at tick 119: Unit differ
```

Only reflected components are compared, so reflect the ones your
simulation changes.

## Status

`ctx.net.state()` says where the session is: `Connecting`, `Running`, or
`Lost` (a player was silent for longer than the timeout, 10 s by default,
or the host refused us; `ctx.net.error()` says why). `ctx.net.player()` and
`players()` say who's who; `stats()` counts packets and bytes sent.

## Over Steam

With the `steam` feature, Steam carries the packets: players meet in a
friends-only Steam lobby, invites come from the Steam overlay, and Steam's
relays get through any router, with no port forwarding and no addresses
to type. The lockstep itself is the same, so nothing else in the game changes.

```toml
ponko = { version = "0.0.1", features = ["steam"] }
```

The host makes the lobby and invites; a friend either launches the game
from the invite (Steam passes `+connect_lobby`) or accepts it while the
game waits:

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/lockstep.rs:steam}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:steam_setup}}
```

</div>
</div>

While connecting, `ctx.net.steam_lobby()` is the lobby, and
`ctx.net.steam_invite()` opens the overlay's invite dialog (the example
does that on the I key). `steamworks` is re-exported, so the same `Client`
also gives you achievements, stats and the rest of Steam.

Shipping: the game needs Steam running, its Steam app id (480, Valve's
test app "Spacewar", works for development) and Valve's `libsteam_api`
library next to the executable (`libsteam_api.dylib`, `libsteam_api.so`
or `steam_api64.dll`). Cargo puts it there for `cargo run`, examples and
tests; when you package the game, copy it from `target/<profile>/`. For C
and C++, build ponko_ffi with `--features steam`.

Try it with two Steam accounts on two machines:
`cargo run --example lockstep --features steam -- steam` on one, press I
to invite, and `cargo run --example lockstep --features steam -- steam-join`
on the other, then accept the invite.

## Testing

`Net::loopback(players)` connects sessions in memory, so a test can run a
whole match headless, one thread per player, and check that every world
ends identical:

```rust
{{#include ../../tests/net.rs:loopback}}
```

## Cost

Measured with `cargo run --release -p ponko_bench -- net`: 10,000 ticks of
a small input-driven sim take 2 ms offline and 8.5 ms as two loopback
peers in two threads (under 1 µs extra per tick, mostly waiting for the
other thread). The guest sends about 48 bytes per tick. At 60 ticks per
second that's under 3 KB/s per player, whatever the size of the world.

Try it: `cargo run --example lockstep -- host 7777` in one terminal and
`cargo run --example lockstep -- join 127.0.0.1:7777` in another.


## Application messages (Rust)

For client/server games with their own packet format, own a `NetMessages`
endpoint and call `poll()` once per frame. This is independent of `ctx.net`:
there is no simulation barrier and the engine does not synchronize worlds.
A host accepts up to `max_clients` remote clients. Joining clients address
`PeerId::HOST`; hosts use the identifiers from `MessageEvent::Connected`.
The host can relay messages explicitly, or each P2P participant can host an
endpoint and join the others. Peer identifiers belong to one endpoint.

`Lane::Reliable` delivers complete messages once, in order, while the
connection survives. `Lane::Unreliable` never retries and discards messages
older than the most recently delivered message on that peer's lane.
Ordering is independent in each direction. Renet handles fragmentation and
reassembly for both lanes. Each application payload is limited to 1 MiB;
an unreliable fragmented message is lost if any fragment is missing.

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/net_messages.rs:messages}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:messages}}
```

</div>
</div>

Run the self-contained UDP example with
`cargo run --example net_messages --no-default-features`.
`send(peer, lane, bytes)` copies the payload into a bounded channel queue;
`send_serde` encodes JSON and `MessageEvent::decode::<T>()` decodes it.
`send_to_all` targets connected peers and can partially succeed if a later
peer's queue is full. The application remains responsible for its packet
schema, authority and version negotiation. The [replication
helpers](#replicating-state-over-messages) below cover the common pieces
without dictating a format.

Drain `events()` after each poll, then apply any sends after dropping the
iterator (it borrows the endpoint). Events include connections, complete
messages, and disconnections with diagnostic reasons. The default silence
and connection timeout is ten seconds; configure it with `.timeout(duration)`.
`disconnect(peer)` discards pending messages, sends three best-effort leave
notifications and reports a local event. If those notifications are lost,
the remote detects the timeout. Dropping an endpoint is detected by timeout.
A full host refuses new connections with a disconnection event.

Each Renet lane has a 4 MiB budget per direction per peer; a full send queue
returns `WouldBlock` so the application can retry or drop its update.
Incoming event storage is bounded to about 5 MiB and 4096 message events,
with a small allowance for connection events during a poll. Polling returns
`WouldBlock` if events have not been drained; continued receive pressure can
exhaust a reliable receive channel and disconnect that peer. Polling handles
at most 256 incoming datagrams per call. `stats()` counts datagrams and bytes,
including retransmissions and control traffic. Buffers are reused for framing,
but owned messages and Renet packet batches allocate; this API does not promise
allocation-free frames.

`loopback(n)` builds an in-memory host and clients, intentionally reversing
packet arrival to exercise reordering. `loopback_lossy(n, drop_one_in)` also
drops every Nth datagram. `poll_elapsed(duration)` advances an explicit clock
for deterministic tests without sleeping.

UDP sessions use nonblocking `std::net::UdpSocket` on the calling thread;
there is no async runtime or worker thread. They are **unauthenticated and
unencrypted**. Session nonces reject delayed traffic from earlier connections,
but do not authenticate users. Use a trusted network or a secure tunnel;
public Internet authentication, encryption and admission policy belong in a
secure transport. This message API currently supports UDP and memory only.
With feature `steam`, `NetMessages` uses Steam Networking Messages on a
separate channel from lockstep. Steam sends unreliable packets without Nagle;
renet supplies delivery, ordering and fragmentation. Each transport packet is
at most 1,500 bytes, below Steam's message limit. `Lane::ReliableUnordered`
(`PONKO_LANE_RELIABLE_UNORDERED` in C++) retries without ordering messages.

Create one message endpoint per Steam client. The host creates a friends-only
lobby (1–249 clients plus host); join with its lobby id. Poll every frame and
drain the same events as UDP. Peer ids remain session-local; `steam_peer`
returns the authenticated Steam identity. Hosts do not automatically forward
client messages. The protocol metadata is checked before joining the message
handshake; applications should check their own game version before accepting
application messages. Host migration requires creating a new endpoint.

<div class="lang-pair">
<div class="lang-rust">

```rust,no_run
use ponko::prelude::*;
let client = ponko::net::steam::Client::init_app(480).unwrap();
let lobby_id = ponko::net::steam_lobby_from_args();
let mut net = match lobby_id {
    Some(id) => ponko::net::NetMessages::steam_join(&client, id)?,
    None => ponko::net::NetMessages::steam_host(&client, 3)?,
};
net.poll()?;
if lobby_id.is_none() {
    if let Some(lobby) = net.steam_lobby().filter(|l| l.id().is_some()) {
        lobby.set_metadata("version", "1")?;
        lobby.invite()?;
    }
}
# Ok::<(), std::io::Error>(())
```

</div>
<div class="lang-cpp">

```cpp
{{#include ../../examples/cpp/guide.cpp:steam_messages}}
```

</div>
</div>

For a lobby without a message endpoint, use `SteamLobby`. Visibility can be
friends-only, public or private; capacity includes the owner (1–250).
Creation and joining are asynchronous: `poll` runs callbacks and reports
errors, and `id` remains absent until ready. Metadata writes require ownership;
reads use Steam's callback-updated cache. Drop/free leaves the lobby, including
when a pending request completes after the handle was dropped. The existing
`steam_lobby_from_args` and `steam_wait_for_invite` helpers cover overlay invites
accepted before launch or while running. Steam is initialized only by the
application (or the C API's first call with an app id), never by fake relay tests.

<div class="lang-pair">
<div class="lang-rust">

```rust,no_run
use ponko::prelude::*;
let client = ponko::net::steam::Client::init_app(480).unwrap();
let mut lobby = ponko::net::SteamLobby::create(&client, ponko::net::SteamLobbyVisibility::Public, 4)?;
lobby.poll()?;
if lobby.id().is_some() {
    lobby.set_metadata("version", "1")?;
    let members = lobby.members();
    let version = lobby.metadata("version");
    lobby.invite()?;
    lobby.leave()?;
}
# Ok::<(), std::io::Error>(())
```

</div>
<div class="lang-cpp">

```cpp
{{#include ../../examples/cpp/guide.cpp:general_steam_lobby}}
```

</div>
</div>


The protocol layer uses [Renet](https://docs.rs/renet/2.0.0/renet/), whose
transport-independent channels provide retransmission, ordered delivery and
fragmentation without Bevy or an async runtime. `renet_steam` was evaluated,
but its Steamworks 0.12.2 dependency differs from this engine's 0.13.1, and
the engine instead adapts the existing renet packet protocol directly.

## Replicating state over messages

Client/server and co-op games usually don't run lockstep: each player owns
their own motion (and the vehicle they drive), the host owns the rest, and
state travels as snapshots on an unreliable lane. The engine moves the bytes
(`NetMessages`) and gives you small helpers for the parts every such game
rewrites, **without imposing a wire format**: they take decoded values and
return decisions. You keep your own packet codec. None of them touches the
network, so they're plain data structures that work (and test) without a
session, in Rust and from C and C++.

### Showing remote things smoothly: snapshot interpolation

Packets arrive unevenly, out of order, twice, or not at all; drawing each one
as it comes makes remote entities jitter. Instead, buffer the snapshots and
draw the entity a little in the past (**100 ms** by default), always between
two snapshots you already have.

A `SnapshotStream` is one remote entity's buffer plus an estimate of the
sender's clock. For each decoded packet, `push` the sender's timestamp, your
own clock and the state; every frame, `sample` your clock:

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/net_replication.rs:snapshots}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:snapshots}}
```

</div>
</div>

- **Pose:** a position, a rotation and an optional velocity. Position is
  interpolated linearly, or along a cubic Hermite curve when both
  neighbouring snapshots carry a velocity (smoother through turns, and at low
  send rates). Rotation uses slerp. Rust's `SnapshotStream<T>` and
  `SnapshotBuffer<T>` work for any state you implement `Interpolate` for
  (`f32`, `Vec2`, `Vec3` and `Quat` already are); C and C++ have the pose.
- **Late packets:** when the buffer runs dry, the entity is **extrapolated**
  from the newest snapshot (by its velocity, or the speed between the last two
  snapshots) for at most **250 ms**, then held where the prediction ended,
  never snapped back. `sample` says which one you got (`Interpolated`,
  `Extrapolated`, `Held`), which is handy for showing a "connection problem".
  Rotation is held, not predicted.
- **Out of order and duplicates:** snapshots are stored by timestamp, so a
  late one still lands in the right place and a second copy is ignored
  (`push` tells you which).
- **The clock:** timestamps are on the sender's clock, which is none of
  yours. `ClockSync` learns the offset from the packets alone, no round
  trips: it follows the fastest of the recent packets (the 4th smallest of
  the last 32, so jitter and late packets don't move it, and neither do a few
  freakishly fast ones) and smooths that, so it also follows clock drift.
  A clock that jumps (the sender restarted) is noticed after 8 packets and
  the estimate starts over. Use any clock you like for yours (`ctx.time.elapsed()`),
  the same one for `push` and `sample`.
- **Teleports:** `clear()` the stream when something respawns or teleports,
  or it slides across the map. `reset()` on a new connection.
- **Tuning:** `SnapshotConfig` has `delay` (make it more than the gap between
  your packets plus their jitter), `max_extrapolation` and `capacity`
  (snapshots kept; at least twice `delay` × your send rate).

With the `3d` feature, the `NetPose` component holds a stream and the system
`systems::apply_net_poses` sets the `d3::Transform` position and rotation of every entity
that has one, from `ctx.time.elapsed()`; add it with `Game::system` so entities move
every frame:

```rust
# let mut game = Game::new("t", 800, 600);
game.system(systems::apply_net_poses);
// from the message handler:
//   world.get_mut::<NetPose>(entity).unwrap().stream.push(sender_time, ctx.time.elapsed(), pose);
```

### Who may update what: authority epochs and sequence numbers

With mixed authority, a handoff (the guest takes the wheel of a car the host
was driving) races with updates still in flight from the old owner. An
`Authority` is an owner peer and an **epoch**; a handoff bumps the epoch, and
updates carry the epoch they were sent under, so a late packet from the
previous owner is recognised and dropped (`AuthorityCheck::Stale`) instead of
yanking the car back:

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/net_replication.rs:authority}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:authority}}
```

</div>
</div>

Whoever decides (usually the host) calls `handoff` and announces
`(owner, epoch)` on a **reliable** lane; the others `adopt` it. An update that
overtakes its announcement is `Ahead`: drop it, or hold it until the
announcement arrives. If two peers hand the same entity off at once, `adopt`
breaks the tie in favour of the lower peer id, so everyone ends up with the
same owner.

For unreliable state updates, number them with a wrapping 16-bit sequence.
`seq_newer(a, b)` compares with wrap-around (65535 is followed by 0), and
`LatestWins` is the per-entity filter: it accepts an update only if its
`UpdateStamp { epoch, seq }` is newer than everything accepted before for the
same entity, which drops duplicates, reordered packets and updates from an old
epoch in one call (see `newest.accept(...)` in the snapshot example above).
Forget an entity with `forget` when it's gone, and after a long silence (half
the sequence range, 32,768 updates, is the limit for an unambiguous
comparison).

### Who needs what: interest by distance

Sending every entity to every peer doesn't scale. An `Interest` decides, per
peer, which entities to send and how often, from the position of that
peer's viewer(s) (the avatar, the camera) and the entities' positions. The
rings are `InterestConfig::tiers`; by default entities within 40 units are
sent every tick, within 120 units every 4th tick, and beyond that not at all.

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/net_replication.rs:interest}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:interest}}
```

</div>
</div>

Call `update` once per send tick with all viewers and entities. Then
`entries` (Rust: `due` for just the ones to send now) lists what each peer is
interested in, with the ring (`tier`), the distance and a `send` flag, and
`changes` reports entities that **entered** (spawn them on that peer) or
**left** (despawn them, also when the entity is gone from the world). An
entity that has just entered always has `send` set.

- **Hysteresis** (5 units by default): an entity moves to a closer ring only
  once it is that far inside it, and to a farther ring (or out of interest)
  only once it is that far outside. One standing on a boundary doesn't flicker
  in and out, and doesn't cost a spawn and a despawn every other tick.
- **Staggering:** the slow rings send on different ticks for different
  entities (by id), so a ring of 1,000 entities every 4th tick is 250 sends
  per tick, not 1,000 every 4th. Turn it off with `stagger: false`.
- **Scale:** the entities go into a spatial grid first, so a peer only looks
  at those near its viewers, and the buffers are reused. One `update` in a
  release build on a laptop: 1,000 entities and 4 peers took 28 µs, 5,000
  entities and 8 peers 65 µs, 20,000 entities and 8 peers 272 µs (about 350
  entries in range in total); with everything in range of everyone (5,000
  entities, 8 peers, 29,000 entries) 1.4 ms, about 50 ns per entry. A peer
  may have several viewers
  (split screen); an entity counts by its nearest. Positions are 3D; for a
  flat world, use a constant height.
- Peers that have no viewer in an `update` are forgotten, with their entities
  reported as left; call `remove_peer` when one disconnects.

### Testing it

Everything above is pure data, so tests need no sockets: feed `push` with
recorded or simulated packets and a made-up clock. The engine's own tests also
run two `NetMessages::loopback_lossy` endpoints with extra delay and jitter,
exchanging snapshots built with these helpers
(`tests/net_replication.rs`), and every helper has tests for jitter,
reordering, packet loss, the extrapolation cap, handoff races, sequence
wrap-around and interest hysteresis.
