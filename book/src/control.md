# Tools and agents: the control endpoint

A running game, windowed or headless, can open a **control endpoint** that
tools use to look inside it and drive it: AI agents (through
[`crabby-mcp`](agents.md)), the inspector, test scripts. Several instances can run
at once; each gets its own endpoint.

## Turning it on

It's off by default, so shipped games never open a port. Any of these turns it
on (C and C++ games too: `game.control_endpoint(true)` or the same
environment variables):

```rust
Game::new("Colony", 1280, 720).control_endpoint(true)
```

```sh
CRABBY_CONTROL=1 cargo run --release --example boids
CRABBY_CONTROL=1 CRABBY_START_PAUSED=1 HEADLESS=1 cargo run --release --example boids
```

The environment variable works for every game without code changes.
`CRABBY_START_PAUSED=1` starts the simulation paused, so a tool can set
things up and step it tick by tick.

## Finding instances

Each instance writes a small JSON file to `~/.crabby/instances/`
(`%LOCALAPPDATA%\crabby\instances` on Windows, or `$CRABBY_INSTANCES_DIR`)
and removes it on exit:

```json
{
  "name": "Crabby Engine: boids",
  "pid": 41213,
  "port": 50113,
  "token": "9f1c…",
  "headless": false,
  "cwd": "/home/me/my_game",
  "exe": "/home/me/my_game/target/release/my_game",
  "started": 1793642400,
  "engine_version": "0.1.0"
}
```

A file whose `pid` is no longer running is stale (the game crashed); tools
skip it.

## Security

- The endpoint listens on `127.0.0.1` only.
- Every request must carry the instance's `token`. The registry file is
  readable only by your user (mode `0600` on Unix).

## Protocol

TCP, one JSON object per line in each direction.

```json
{"id": 1, "token": "9f1c…", "method": "entities.list", "params": {"with": ["Transform2D"], "limit": 2}}
{"id": 1, "result": {"total": 20000, "offset": 0, "entities": [{"entity": 4294967296, "components": {"Transform2D": {"position": [10.5, 3.0], "rotation": 0.0, "scale": [0.5, 0.5]}}, "other_components": ["Sprite", "Vel"]}, …]}}
```

Errors come back as `{"id": 1, "error": "entity 999 does not exist"}`.

Requests are applied by the game loop between frames (between ticks when
headless), so they never race the simulation or each other. A paused
headless game waits for requests and runs only the ticks you step.

| Method | Params | Returns |
|---|---|---|
| `methods` | | every method with a description |
| `info` | | pid, headless, tick, sim time, paused, speed, fps, entity count, render stats (sprites, draw calls, CPU and GPU ms) |
| `world.summary` | | entity count, reflected components and resources, archetypes and sizes |
| `entities.list` | `with?`, `where?`, `limit?` (100), `offset?` | entities with reflected components as JSON, names of the others |
| `entities.stats` | `fields`, `with?`, `where?` | `{entities, fields: {path: {count, min, max, mean} or {count, values}}}` (see below) |
| `entity.get` | `entity` | `{component: value}` |
| `entity.spawn` | `components` | `{entity}` |
| `entity.despawn` | `entity` | `true` if it existed |
| `component.set` | `entity`, `component`, `value` | the new value (object fields merge) |
| `component.remove` | `entity`, `component` | `true` if it had it |
| `resources.list` | | `{resource: value}` for every reflected resource (`null` if it's not in the world) |
| `resource.get` | `resource` | its value |
| `resource.set` | `resource`, `value` | the new value (object fields merge; inserted if missing) |
| `world.save` | `path` | writes a [save](determinism.md#saving-and-loading-the-world); entity and resource counts, bytes, tick |
| `world.load` | `path` | replaces every entity from a save (same ids; checked first) |
| `sim.pause` / `sim.resume` | | |
| `sim.step` | `ticks?` (1) | ticks queued |
| `sim.speed` | `speed` | |
| `systems.timings` | | each system's last and average ms |
| `events.list` | | reflected [event](events.md) types and how many are kept now |
| `events.subscribe` | `types?` (all), `capacity?` (1000) | starts collecting new events between frames; the newest `capacity` are kept |
| `events.take` | | `{events: [{tick, type, data}], dropped}` since the last take |
| `events.unsubscribe` | | `true` if there was a subscription |
| `logs.get` | `lines?` (100), `since?` | `{lines, next, missed}`: the game's recent stdout and stderr (see below) |
| `camera.get` | | position, zoom, center, visible rect |
| `camera.set` | `position`?, `zoom`?, `look_at`? | move the camera; returns `camera.get` |
| `screenshot` | `path`?, `max_size`? | save the next frame as a PNG (windowed games only); returns `path`, `width`, `height` |
| `game.quit` | | |

Component values use [reflection](reflection.md): call
`world.reflect::<T>()` for a component, `world.reflect_resource::<R>()` for
a resource, to make it readable and writable.
Entity ids are the numbers from `Entity::to_bits`.

## Questions about many entities

A simulation with a million units is too big to list. `entities.stats`
answers questions about all of them at once, from reflected fields:

```json
{"method": "entities.stats", "params": {
  "fields": ["Unit.hp", "Transform2D.position", "Unit.team"],
  "where": ["Unit.hp < 5"]}}
```

gives, for the matching entities, `count`, `min`, `max` and `mean` of
numbers (per axis for vectors like positions) and counts per value for
names and enums (`"values": {"Red": 120, "Blue": 98}`). A field path is a
component name, then fields, array elements by number:
`Transform2D.position.0` is x. A bare component name (`Team`) is its whole
value.

`where` takes conditions on field paths with `==`, `!=`, `<`, `<=`, `>`,
`>=` and a JSON value (a bare word is a string: `Unit.team == Red`). It
filters `entities.list` too. On the battle demo, stats over 1M units take
about 0.3 s (release); the game is paused for that long.

## Logs

While the endpoint is open, the engine keeps the game's last 5,000 lines of
stdout and stderr: `println!`, C and C++ `printf` and `std::cout`, panic
messages and the engine's own notes. The terminal still shows everything.
So a tool can read the output of a game you started yourself, from a
terminal or an IDE. `logs.get` is answered right away, not between frames,
so it works while a frame is stuck too.

Without `since` you get the newest `lines`. Pass the `next` of one answer as
`since` in the next request to get only what's new; `missed` counts lines
that were dropped before you read them. This works on macOS, Linux and
Windows. A Windows game built as a GUI program (no console) has no output to
keep; `logs.get` then says it isn't captured.

## Trying it by hand

```sh
CRABBY_CONTROL=1 cargo run --release --example boids &
f=$(ls ~/.crabby/instances/*.json | head -1)
port=$(jq .port "$f"); token=$(jq -r .token "$f")
echo "{\"id\":1,\"token\":\"$token\",\"method\":\"info\"}" | nc -q1 127.0.0.1 "$port"
```
