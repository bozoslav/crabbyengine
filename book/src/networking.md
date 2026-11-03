# Multiplayer: deterministic lockstep

Lockstep is how RTS and simulation games do multiplayer: every player runs
the whole simulation, and only **inputs** cross the network. As long as
everyone applies the same inputs in the same order to the same start,
everyone has the same world, whether there are 10 units or 100,000. A
tick costs the same bandwidth either way: about 50 bytes per player.

Crabby's simulation is built for it already: a fixed clock, seeded
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

When something slips through, Crabby tells you. Every `checksum_every`
ticks (default 60) the peers compare a hash of every
[reflected](reflection.md) component and resource; on a mismatch,
`ctx.net.desync()` reports the tick, the player and **which components
differ**, so you know where to look:

```text
crabby lockstep: desync with player 1 at tick 119: Unit differ
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
crabbyengine = { version = "1", features = ["steam"] }
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
and C++, build crabby_ffi with `--features steam`.

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

Measured with `cargo run --release -p crabby_bench -- net`: 10,000 ticks of
a small input-driven sim take 2 ms offline and 8.5 ms as two loopback
peers in two threads (under 1 µs extra per tick, mostly waiting for the
other thread). The guest sends about 48 bytes per tick. At 60 ticks per
second that's under 3 KB/s per player, whatever the size of the world.

Try it: `cargo run --example lockstep -- host 7777` in one terminal and
`cargo run --example lockstep -- join 127.0.0.1:7777` in another.
