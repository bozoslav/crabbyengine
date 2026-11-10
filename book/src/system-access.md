# Declared system access

Every system you have seen so far takes `&mut Ctx`: it may touch anything,
so the engine runs systems one at a time. A system can also **declare** what
it reads and writes. The declaration is checked, shows up in the access
report, and is what a future parallel scheduler
([#138](https://github.com/ponkoengine/ponko/issues/138)) will use to run
systems side by side.

Today nothing runs in parallel: declared systems run in phase and
registration order like any other. Plain `sim_system`, `system` and `add`
systems keep their API and stay *exclusive* (they conflict with
everything), so existing games behave exactly as before.

## View systems

A view system declares its access with a `SystemAccess` and gets a
`SystemView` instead of `&mut Ctx`: exactly what it declared, and nothing
else.

<div class="lang-pair">
<div>

```rust
{{#include ../../tests/system_views.rs:view_system}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:declared_access}}
```

</div>
</div>

(C and C++ systems keep their `ctx`; their declarations are checked in
debug builds, see [below](#c-and-c).)

What a declaration can name:

| Builder | Allows |
|---|---|
| `read::<T>()` / `write::<T>()` | component `T` (`&T` / `&mut T` in queries, `get` / `get_mut`) |
| `read_resource::<R>()` / `write_resource::<R>()` | resource `R` (writing includes inserting and removing it) |
| `read_events::<E>()` / `send_events::<E>()` | reading / sending `E` events (separate rights) |
| `commands()` | `sys.commands()`: spawning, despawning, inserting and removing components |
| `read_service(s)` / `write_service(s)` | an engine service (below) |

`sys.world` is a `WorldView`: queries, `get`, `get_mut`, resources, and
entity checks (`is_alive`, `has`, `len`), all limited to the declaration. It
has no `spawn`, `despawn`, `insert` or `remove`: structural changes go
through commands, which apply after the system as usual. Its writes count
for [change detection](ecs.md#change-detection) like any others, and its
queries take the same filters as world queries: `with`, `without` and
`filter::<F>()` with `With`, `Without`, `Or`, `Added<T>` and `Changed<T>`.
A change filter on `T` reads the stamps `T`'s writers leave, so it needs `T`
declared (`read::<T>()` is enough); `sys.world.removed::<T>()` needs the
same. Like a `&mut Ctx` system, a view system is shown what changed since it
last ran. A view can't start tracking a component (that reallocates its
storage), so call `ctx.world.track_changes::<T>()` in setup; a change
filter on an untracked component panics and says so.

```rust
# use ponko::*;
struct Health(i32);
struct Hurt(Entity);

fn report_hurt(sys: &mut SystemView) {
    let mut hurt = Vec::new();
    sys.world
        .query::<Entity>()
        .filter::<Changed<Health>>()
        .for_each(|e| hurt.push(e));
    for e in hurt {
        sys.events().send(Hurt(e));
    }
}

# fn main() {
let _game = Game::new("t", 1, 1).add_view(
    Phase::AfterPhysics,
    SystemAccess::new().read::<Health>().send_events::<Hurt>(),
    report_hurt,
);
# }
```

Every accessor
checks the declaration **in every build** and panics with the system's
name and what is missing:

```text
system `my_game::fall` doesn't declare that it may write component `my_game::Pos`;
add it to its declared access
```

The check runs once per call (a query, a `get`, a resource), never per
entity: a view system's loops cost the same as a `Ctx` system's.

### Services

Services follow the service fields of `Ctx`. A view offers the ones whose
state can be handed out on its own:

| `Service` | In a `SystemView` |
|---|---|
| `Input` | read: `key_down`, `mouse_pos`, `action_down`, ... |
| `Time` | read: `dt`, `fixed_dt`, `sim_tick`, `time`, `frame` |
| `Random` | read: `random()`, the system's stream for this tick (the same one `ctx.random()` gives) |
| `Audio` | write: `play_sound`, `play_sound_ex` |
| `Camera` | read `camera()` / `camera3d()`, write `camera_mut()` / `camera3d_mut()` |
| `Spatial` | read: `spatial()` |
| `d2::Physics`, `d3::Physics` | read `physics()` (ray casts, overlaps), write `physics_mut()` |
| `Net` | read `net()`, write `net_mut()` |
| `Gfx`, `Ui`, `Assets`, `Perf` | not offered |

Drawing, UI, assets and profiling hold window, renderer and asset-table
state; systems that need them keep `&mut Ctx`. `Game::add_view` panics at
registration if a declaration names a service the view can't offer, or
writing `Input`, `Time` or `Spatial`.

## Declaring `Ctx` systems

`Game::add_with_access` attaches a declaration to an ordinary `&mut Ctx`
system. It runs with the whole `Ctx`; in builds with debug assertions the
engine records what it touched (components, resources, events, commands
and structural changes) and panics after the run if it went beyond the
declaration. Services are not recorded for `Ctx` systems. Use it to
document and check existing systems; only view systems can ever run in
parallel, because only they can't reach anything undeclared.

## The access report

`Game::access_report()` lists every system with its declaration and, for
each pair of declared systems in the same phase, whether they could share
a parallel batch:

```rust
# use ponko::prelude::*;
# struct Vel(f32);
let report = Game::new("report", 1, 1)
    .add_view(Phase::AfterPhysics, ponko::access::SystemAccess::new().write::<Vel>(), |_| {})
    .add_view(Phase::AfterPhysics, ponko::access::SystemAccess::new().read::<Vel>(), |_| {})
    .access_report();
// "... + ...: conflict on component `...Vel`"
print!("{report}");
assert_eq!(report.parallel_pairs().count(), 0);
```

`compatible_pairs()` lists pairs whose declarations don't conflict;
`parallel_pairs()` only those where both are view systems. Each
`ReportedSystem` says whether it is `enforced` (a view system).

## When two systems conflict

Within a phase, two declared systems conflict, and so must keep their
order, when:

- either is exclusive (undeclared systems are);
- both touch the same component, resource or service and at least one
  writes it;
- events or commands make the order visible (below).

Systems that only read the same data never conflict. Phases are barriers:
systems in different phases are never compared.

### Commands and events: the batch barrier

`SystemAccess::conflicts_with` is the conservative, symmetric rule: any
system that sends an event type conflicts with any other that reads or
sends it, and a command recorder conflicts with every other system.

`SystemAccess::conflicts_before(later)` is the finer rule for a known
order (the report uses it). It holds under the **batch contract** a
parallel executor must keep:

1. Commands recorded in a parallel batch apply when the batch ends, in
   registration order.
2. Events sent in a batch become visible when it ends, in registration
   order, stamped as if sent sequentially.
3. A system outside every batch (exclusive, or any conflicting system) is
   a barrier of its own, exactly as today.

Under that contract, for an earlier system `a` and a later system `b`:

| `a` | `b` | Conflict? | Why |
|---|---|---|---|
| sends `E` | reads `E` | yes | sequentially `b` sees `a`'s events in the same tick |
| reads `E` | sends `E` | no | sequentially `a` only sees them on its next run anyway |
| sends `E` | sends `E` | no | events are published in registration order |
| records commands | anything | yes | sequentially `b` sees the applied commands |
| anything | records commands | no | `a` never saw `b`'s commands |

"Anything" includes a system that declares no data at all: every view can
see the world's structure (`len`, `is_alive`, `has`, `with` and `without`
filters) without declaring it, and commands change that structure.

Two systems can only share a batch if every system registered between
them in the phase could move out of the way: the report adds a
`Between` conflict naming the first one that can't (an undeclared system,
or one that conflicts with either), and `parallel_pairs()` also treats a
`Ctx` system between them as a barrier, since its declaration is only
checked in debug builds.

Running systems one after another (what the engine does now) keeps this
contract trivially, so declaring access never changes results.
`tests/system_views.rs` pins the sequential behaviour each relaxed rule
relies on.

### If you rely on the per-system flush

Today every system is a barrier: a system sees the commands and events of
every system before it in the same tick. Under a parallel executor, view
systems batched together see each other's commands and events only after
the batch. To stay correct either way:

- If `b` needs `a`'s commands or events in the same tick, declare both so
  the rules above make them conflict (`a` records commands or sends `E`;
  `b` reads `E`), or put `b` in a later phase.
- Don't count on registration order between systems that don't conflict.
- Keep "make a change and look at it" inside one system, through
  `world` (immediate) or a single `commands.add` closure.

## C and C++

C and C++ systems declare access by registered name (component, resource
and event names; C++ uses the type's name, as the world does):

<div class="lang-pair">
<div>

```rust
# use ponko::prelude::*;
# struct Health(i32);
# struct Died(Entity);
# fn hurt(_: &mut Ctx) {}
// The same declaration, by name, as C and C++ build it.
let game = Game::new("named", 1, 1).add_with_access(
    Phase::AfterPhysics,
    ponko::access::SystemAccess::new()
        .write_named(std::any::type_name::<Health>())
        .send_events_named(std::any::type_name::<Died>())
        .commands(),
    hurt,
);
# let _ = game;
```

</div>
<div>

```cpp
// ponko_access_new / ponko_access_write_component("Health") /
// ponko_access_send_events("Died") / ponko_access_commands, then
// ponko_game_add_system_with_access. In C++:
game.add(ponko::phase::after_physics,
         ponko::access().write<Health>().send_events<Died>().commands(),
         hurt, "hurt");
```

</div>
</div>

They run with their `ctx` and are checked in debug builds of the library
like `add_with_access` systems; a violation stops the run with an error
naming the system. Raw C access can't tell reads from writes, so a
component or resource declared by name covers any use of it: declare a
write when the system changes it, so the report shows the conflict.
`ponko_access_conflicts` and `ponko_game_access_report` expose the rules
and the report. Build declarations at registration, not per frame: each
step copies the declaration and keeps its names for the rest of the
program. C and C++ systems always stay sequential.

## Design: the soundness strategy (RFC)

*Status: accepted for #137; the executor is #138.*

**Goal.** Let a scheduler run independent systems at the same time without
ever giving two threads `&mut Ctx` or `&mut World`, and without trusting
declarations it can't check.

**Declarations are metadata; views are enforcement.** A `SystemAccess` is
data: components, resources and events by type, commands, services. On its
own it proves nothing; a `&mut Ctx` system can use anything. So:

1. *View systems* get a `SystemView` built from separate borrows of `Ctx`
   fields. Its world part is the ECS's `WorldView`: a shared `&World` plus
   the declaration. There is no path from the view to anything else, and
   each accessor checks the declaration in every build. A wrong
   declaration is a panic naming the system, never a data race.
2. *Declared `Ctx` systems* (Rust `add_with_access`, C and C++) are checked
   after each run in debug builds by the world's access recorder. They are
   never candidates for parallel execution.
3. *Undeclared systems* are exclusive, as before.

**How two views share one world.** `ViewState::prepare` takes `&mut World`
before the views exist: it registers the declared components and moves the
declared *written* resources out of the world into the state. The views
then share `&World`:

- Components are written through the archetype columns' own raw pointers,
  which carry write permission from their allocation. Two views whose
  declarations don't overlap (`ViewAccess::overlaps`: no component one
  writes and the other reads or writes) touch disjoint columns.
- A written resource lives in its view's state, so no other view can reach
  it; an overlapping declaration finds it missing (a panic, not a race).
- No view can change the world's structure, so archetypes and columns
  don't move while views exist; structural changes are commands, applied
  with `&mut World` at the barrier.
- `ViewState::finish` puts the resources back; `ViewState::run` does it
  even when the system panics.

The unsafe entry point is `ViewState::view(&World)`; its contract is
exactly "no overlapping view or `&mut World` use while it lives", which
the scheduler establishes from `conflicts_before`. A state prepared for
one world refuses to view another. The two-thread test in
`crates/ponko_ecs/tests/views.rs` runs under Miri.

**What the executor (#138) adds.** Per-view command buffers and event
outboxes merged at the batch barrier in registration order (the batch
contract above), per-system event cursors that don't share a mutable
channel, and the batching itself. Services in the view are borrowed from
separate `Ctx` fields; giving two views the same writable service is a
declared conflict, so they are never batched together.

**Hot patching** swaps a system's code, not its declaration. A patched
view system still can't reach anything undeclared; a patched `Ctx` system
is checked on its next debug run.

**Costs.** Release builds skip the `Ctx` recorder entirely. A view adds one
declaration lookup per accessor call, no per-entity cost, and no heap
allocation per run once its buffers have grown
(`crates/ponko_ecs/tests/views_alloc.rs`).

**Limits.** Custom `QueryData` types must implement the shared lookup to
be queried through a view. A view's filters never register a component or
start tracking one: `With<T>` of a component no entity ever had matches
nothing, `Without<T>` of one matches everything, and `Added<T>` /
`Changed<T>` of an untracked `T` panic.
