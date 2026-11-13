# Parallel systems

Systems run one at a time unless you ask for more. With
`Game::parallel_systems(true)`, [view systems](system-access.md) whose
declarations don't conflict run at the same time, on several threads. It is
off by default, it never changes what the simulation computes, and it only
helps when the systems are big enough: [measurements](#when-it-doesnt-pay-off)
below.

What it does not do: `&mut Ctx` systems (`sim_system`, `system`, `add`
with a function of `&mut Ctx`, declared or not, and everything from C and
C++) always run alone, in registration order. They may touch anything, so nothing can run next to
them.

## Turning it on

Write each system as a function of `&mut SystemView`, declare what it uses
where you add it (`system.access(...)`), and turn the scheduler on:

<div class="lang-pair">
<div>

```rust
{{#include ../../tests/system_schedule.rs:parallel_systems}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:parallel_systems}}
```

</div>
</div>

`fall` and `regenerate` write different components, so they share a
**batch**. `movement` reads the `Vel` that `fall` writes, so it runs after
that batch. Nothing else is needed: there are no labels and no dependency
graph, only the declarations and the order the systems were added in.

C and C++ can turn the scheduler on and read the plan
(`ponko_game_parallel_systems`, `ponko_game_schedule_plan`), which matters
for a game that mixes Rust view systems with native ones. Their own systems
receive the whole `ctx` through a pointer, which can't be limited to a
declaration, so they stay alone.

## Stages and batches

Systems run phase after phase, and inside a phase in registration order.
The scheduler cuts that order into **stages**. A stage is either

- one system that runs alone, or
- a batch: view systems next to each other in the order, none of which
  [conflicts](system-access.md#when-two-systems-conflict) with an earlier
  one in the batch.

Stages run one after another, and a system never moves past another one.
So two systems that conflict are in different stages and keep their
registration order, and two systems never read and write the same data at
the same time.

`Game::schedule_plan()` shows the stages and why each one starts where it
does. It is the same plan whether the scheduler is on or off:

```rust
# use ponko::access::{Service, SystemAccess};
# use ponko::prelude::*;
# use ponko::system_view::SystemView;
{{#include ../../tests/system_schedule.rs:parallel_systems}}
# fn main() {
{{#include ../../tests/system_schedule.rs:plan}}
# }
```

Printed, the plan of the game above reads (the real names are full
function and type paths):

```text
AfterPhysics
  fall + regenerate (batch)
  movement (batch of one; after `fall`: conflict on component `Vel`)
```

A new stage starts:

| Where | Why |
|---|---|
| at every phase | phases are barriers |
| before and after a `&mut Ctx` system | it may touch anything |
| before and after a view system that holds sound, a camera, a physics world or networking | one system at a time may hold those services |
| before and after a view system that sends *and* reads the same event type | it must see its own events at once |
| where a system conflicts with one already in the batch | the later one must see the earlier one's work |
| at `Game::barrier(phase)` | you said so |
| between the two systems of `Game::order(first, second)` | you said so |

A view system can share a batch while reading input, the clock, its random
stream and the spatial grid.

Because batches only form from *neighbours*, the registration order
matters for speed: a `&mut Ctx` system between two view systems splits
them into two stages. Register the view systems of a phase next to each
other.

A running game gives the same answer as data: `ctx.ecs_diagnostics()` (and
the `ecs_diagnostics` tool for agents) lists every system's declaration,
its stage, why it runs alone, why each stage starts where it does, and how
often each batch went to other threads. See
[ECS diagnostics](agents.md#ecs-diagnostics) for the fields and a worked
example.

### Explicit barriers and order

Declarations cover what the engine can see. For an order it can't see (two
systems that talk through a file, a channel or a global), say it:

```rust
# use ponko::access::SystemAccess;
# use ponko::prelude::*;
# use ponko::system_view::SystemView;
# #[derive(Component)]
# struct A;
# #[derive(Component)]
# struct B;
# fn export(_: &mut SystemView) {} fn upload(_: &mut SystemView) {}
# fn write_a(_: &mut SystemView) {} fn write_b(_: &mut SystemView) {}
let mut game = Game::new("ordered", 1, 1).parallel_systems(true);
let export = game.add_system(Phase::AfterPhysics, export.access(SystemAccess::new().read::<A>()));
let upload = game.add_system(Phase::AfterPhysics, upload.access(SystemAccess::new().read::<B>()));
// `upload` must run after `export` has finished.
let game = game.order(export, upload);
assert_eq!(game.schedule_plan().stages_in(Phase::AfterPhysics).len(), 2);

// Or cut a phase in two: systems added after the barrier never share a
// batch with the ones before it.
let game = Game::new("cut", 1, 1)
    .add(Phase::AfterPhysics, write_a.access(SystemAccess::new().write::<A>()))
    .barrier(Phase::AfterPhysics)
    .add(Phase::AfterPhysics, write_b.access(SystemAccess::new().write::<B>()));
assert_eq!(game.schedule_plan().stages_in(Phase::AfterPhysics).len(), 2);
```

`order` doesn't reorder anything: `first` must already run before
`second`, and the call only keeps them out of one batch.

## Barriers: where commands apply and events arrive

The end of every stage is a **barrier**. There, and only there:

| What | Happens at the barrier |
|---|---|
| Commands recorded in the stage | are applied, system by system in registration order |
| Events sent in the stage | become visible to other systems, in registration order |
| Changes to tracked components | start to show in later systems' `Added`, `Changed` and `removed` |

A system that runs alone is its own stage, so for it this is the rule
systems always had: its commands apply and its events are visible as soon
as it returns ([When changes become visible](systems.md#when-changes-become-visible)).

Inside a batch, every system has its own command buffer and its own event
outbox, and reads the events that existed when the batch started. The
conflict rules make sure no system in the batch could have told the
difference from running one at a time:

- A system that sends `E` never shares a batch with a *later* reader of
  `E`. (An *earlier* reader can: run one at a time it would have read
  those events on its next run too.)
- A system that records commands is always the last of its batch: every
  later system starts a new stage and sees the commands applied.
- Two systems that write the same component or resource, or where one
  reads what the other writes, are never in one batch.

Converting a `&mut Ctx` system that leaned on the flush after every system?
[Moving a system to a view system](systems.md#moving-a-system-to-a-view-system)
lists what to check.

So the scheduler is not a new set of visibility rules to learn. With it on
or off, a system sees the commands and events of every system before it
that it could conflict with.

## Determinism

With the scheduler on, a simulation computes exactly what it computes with
it off, and with it off nothing changes at all: the scheduler doesn't even
start the thread pool. On:

- **The plan depends only on the systems and their declarations**, never
  on timing or on the machine.
- **Thread timing is not observable.** Systems of a batch touch disjoint
  data (the views enforce it; a system that reaches for something it
  didn't declare panics with its name, it does not race). Commands and
  events are delivered at the barrier in registration order, whichever
  thread finished first.
- **Whether a batch uses threads at all only changes speed.** A batch that
  stays on the game's thread goes through the same code: same views, same
  buffers, same barrier.
- **Random numbers** come from the system's own stream (seed, tick and
  system), as with `ctx.random()`. **Clocks** are read once per batch.
- **Change ticks** are the same numbers. Run one at a time, every system
  has a change tick of its own; the systems of a batch write in exactly
  those ticks. So `change_tick()` returns the same value in a system,
  `Added` and `Changed` compare the same against a tick a system kept, and
  the world's tick after a stage doesn't depend on the scheduler.
- **Lockstep**: peers may run with different settings. `tests/system_schedule.rs`
  keeps one peer with the scheduler off and one with it on in sync, and
  runs randomly generated sets of systems one at a time, in batches on one
  thread and in batches on several, comparing every result.
- **Profiling**: per-system times, calls and allocations are reported for
  the same systems in the same order. While the profiler is on, batches
  stay on the game's thread, because allocations are counted for the whole
  process and can only be charged to systems that run one at a time; frame
  times measured while profiling are therefore the sequential ones.

What stays your job, as with any system: don't let systems talk through
something the engine can't see (a global, a file, a channel) without
`order` or a barrier, and keep per-entity work inside `par_for_each` free
of side effects.

### When a system panics

A panic in a system stops the game, as always. What is left behind matters
to code that catches it (a test, a tool that reloads the game):

- The other systems of the batch still run to their end, on threads and
  in order alike: on threads they may be running already. Then the panic
  is passed on; if several systems panicked, the one registered first.
- The batch gets no barrier. The commands its systems recorded and the
  events they sent are **dropped**, and the events they read stay unread:
  nothing is left over for a later batch to deliver. What the systems
  wrote to components and resources stays, as when a system that runs
  alone panics halfway.
- Change detection is left as after runs alone: a system that returned
  isn't shown its own writes of that batch again, the batch's change ticks
  have ended, and the system that panicked sees its own writes once more
  (as it does when it panics running alone).
- With the scheduler off, systems after the panicking one never start.
  That one difference can't be avoided once systems run at the same time.

To check a game, run its tests in both modes without changing code: the
environment variable `PONKO_PARALLEL_SYSTEMS` overrides the builder
(`0` off, `1` on, `always` on with every batch sent to the thread pool).

## When it doesn't pay off

Handing a batch to the thread pool costs about 20 to 45 µs (waking the
workers and waiting for them), so the engine only does it when the batch
has more to gain: when the usual run time of its systems, *not counting
the slowest*, is at least 50 µs (`Game::parallel_threshold` changes it).
The slowest system doesn't count because the batch takes at least that
long on any number of threads. "Usual" is a moving average of measured run
times, so a batch moves to threads and back as its systems grow or shrink.

Measured on an Apple M4 Max (10 performance cores), one simulation tick,
`cargo run --release -p ponko_bench --bin ponko_bench -- schedule/`:

| Schedule | Off | On | On, threads forced |
|---|---|---|---|
| 64 tiny systems (100 entities each) | 8 µs | 8 µs | 49 µs |
| 8 light systems, 16,000 entities each | 21 µs | 21 µs | 39 µs |
| 8 light systems, 64,000 entities each | 81 µs | 59 µs | 58 µs |
| 8 light systems, 250,000 entities each | 322 µs | 113 µs | 114 µs |
| 8 heavy systems, 4,000 entities each | 1.66 ms | 0.31 ms | 0.31 ms |
| 8 heavy systems, 16,000 entities each | 7.15 ms | 1.04 ms | 1.04 ms |
| 2 heavy systems, 64,000 entities each | 7.16 ms | 3.65 ms | 3.66 ms |
| a chain of 8 (each reads the one before) | 29 µs | 28 µs | 28 µs |
| a mixed simulation with a `Ctx` system | 8.45 ms | 5.71 ms | 5.75 ms |
| 1 big system and 7 tiny ones | 3.63 ms | 3.62 ms | 3.62 ms |

(Light is one multiply-add per entity, heavy is tens of nanoseconds per
entity. With the scheduler off, the same games take what they took before
it existed: 8 µs for 64 tiny systems, before and after. The machine was in
use; details and the caveats are in
[`crates/ponko_bench/SCHEDULE.md`](https://github.com/ponkoengine/ponko/blob/main/crates/ponko_bench/SCHEDULE.md).)

So:

- **Many tiny systems gain nothing.** The scheduler notices and keeps them
  on the game's thread, where a batch costs the same as running its
  systems one at a time. A typical small game should leave it off.
- **Chains gain nothing**: if every system reads what the previous one
  wrote, every stage has one system.
- **One dominant system gains nothing** from running next to small ones.
  Split its own work with [`par_for_each`](ecs.md) instead.
- **`&mut Ctx` systems in between** cut batches short, and their own time
  is not shared: the mixed simulation gains 1.5×, not 4×.
- **Memory-bound systems gain less** than compute-bound ones (2.8× against
  7.5× with eight systems).
- **On the web** there are no threads to use: batches run in order.

It pays off for simulations with several independent, heavy systems per
tick (thousands of entities each, or expensive per-entity work), written
as view systems.
