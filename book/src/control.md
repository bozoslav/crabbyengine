# Tools and agents: the control endpoint

A running game, windowed or headless, can open a **control endpoint** that
tools use to look inside it and drive it: AI agents (through
[`ponko-mcp`](agents.md)), the inspector, test scripts. Several instances can run
at once; each gets its own endpoint.

## Turning it on

It's off by default, so shipped games never open a port. Any of these turns it
on (C and C++ games too: `game.control_endpoint(true)` or the same
environment variables):

```rust
Game::new("Colony", 1280, 720).control_endpoint(true)
```

```sh
PONKO_CONTROL=1 cargo run --release --example boids
PONKO_CONTROL=1 PONKO_START_PAUSED=1 HEADLESS=1 cargo run --release --example boids
```

The environment variable works for every game without code changes.
`PONKO_START_PAUSED=1` starts the simulation paused, so a tool can set
things up and step it tick by tick.

## Finding instances

Each instance writes a small JSON file to `~/.crabby/instances/`
(`%LOCALAPPDATA%\crabby\instances` on Windows, or `$PONKO_INSTANCES_DIR`)
and removes it on exit:

```json
{
  "name": "Ponko: boids",
  "pid": 41213,
  "port": 50113,
  "token": "9f1c…",
  "headless": false,
  "cwd": "/home/me/my_game",
  "exe": "/home/me/my_game/target/release/my_game",
  "started": 1793642400,
  "engine_version": "1.0.0"
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
| `capabilities` | | what this instance can do ([details](#capabilities)) |
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
| `scene.save` / `scene.load` | `path` | a [scene](scenes.md) in `assets/` (loading also loads sprite images) |
| `prefab.spawn` | `path`, `at` | spawns a [prefab](scenes.md) at `[x, y]`; returns `entity` |
| `prefab.save` | `entity`, `path` | saves an entity as a prefab in `assets/` |
| `sim.pause` / `sim.resume` | | |
| `sim.step` | `ticks?` (1) | ticks queued |
| `sim.speed` | `speed` | |
| `input.scenario.load` | `scenario`, `bind_connection?`, `keep_input_when_done?`, `max_ticks?`, `queue_ticks?` | starts a tick-indexed [input scenario](actions.md#input-scenarios) on the next tick, replacing a running one; refused unless the game allows input injection (`Game::input_injection(true)` or `PONKO_INPUT_INJECTION=1`). The keys it holds are released when it ends unless `keep_input_when_done`. With `bind_connection`, it is cancelled when this connection closes. `max_ticks` and an empty scenario are refused before anything running is touched; `queue_ticks` (paused games) queues the scenario's own ticks as steps. Returns `{report, bound_to_connection}` |
| `input.scenario.status` | | the latest scenario's report (`status`, `ticks_run`, `checks_passed`, `released`, `still_held`, and for a failed check its `step`, `tick`, `expected` and `actual`), or `null` |
| `input.scenario.cancel` | `reason?` | ends the running scenario, or releases what a finished one kept held, and releases the input; returns `{cancelled, report}`. A failed check or a cancel takes back only the steps the scenario queued itself (`queue_ticks`), not ticks queued with `sim.step` |
| `systems.timings` | | each system's last and average ms |
| `ecs.diagnostics` | `with?`, `limit?` (100) | archetypes, component storage and the schedule (see [ECS diagnostics](agents.md#ecs-diagnostics)); with `with`, also `matching` |
| `events.list` | | reflected [event](events.md) types and how many are kept now |
| `events.subscribe` | `types?` (all), `capacity?` (1000) | starts collecting new events between frames; the newest `capacity` are kept |
| `events.take` | | `{events: [{tick, type, data}], dropped}` since the last take |
| `events.unsubscribe` | | `true` if there was a subscription |
| `logs.get` | `lines?` (100), `since?` | `{lines, next, missed}`: the game's recent stdout and stderr (see below) |
| `perf.get` | `meshes?` (0) | where frame time goes, over the last few seconds: fps; avg, p50, p95, p99 and max ms; CPU in systems, render and display wait; GPU ms in total and per pass; draw calls and 3D models and triangles (camera and shadows); the slowest systems; with `meshes`, the meshes drawing the most triangles; `render_inspector`, the last frame's rendering cost in a versioned schema (see [Profiling a frame](performance.md#the-render-pass-inspector)) |
| `perf.trace` | `path?` (a temp file), `frames?` (120) | records the next frames as a Chrome/Perfetto trace; returns the path, written once the frames are done |
| `perf.hud` | `on` | shows or hides the performance overlay |
| `shaders.status` | | each custom shader's last compile or hot reload result (`compiled`, `reloaded`, `failed`, `reload_failed`), reload count, frame, and errors with line and column in the game's own WGSL, cause and GPU variant |
| `camera.get` | | position, zoom, center, visible rect |
| `camera.set` | `position`?, `zoom`?, `look_at`? | move the camera; returns `camera.get` |
| `screenshot` | `path`?, `max_size`? | save the next frame as a PNG (windowed games only); returns `path`, `width`, `height` |
| `game.quit` | | |

Component values use [reflection](reflection.md): call
`world.reflect::<T>()` for a component, `world.reflect_resource::<R>()` for
a resource, to make it readable and writable.
Entity ids are the numbers from `Entity::to_bits`.

## Capabilities

`capabilities` answers, without changing anything, what this running build
supports, so a tool can check before sending a request that would fail:

- `mode`: `headless` or `windowed`; `engine_version`; `features`: every
  Cargo feature and whether this build has it.
- `subsystems`: `renderer`, `inspector`, `gamepad`, `physics`, `physics3d`,
  `audio`, `networking`, `profiling`, `control`. Each says whether it's
  compiled in and active, with a `reason` when it isn't. What isn't known
  is `null` with a reason (the GPU adapter before a GPU opens; physics
  before the game loop starts), never a guess.
- `methods`: every method with `available` and, when it would be refused
  here (`screenshot` in a headless game), the same `reason` the refusal
  gives; `limited` notes calls that are accepted but do less.
- `reflection`: reflected components and resources, each with a `schema`.
  Serde can't say what a type serializes to, so a type registered with
  `reflect::<T>()` is `"status": "opaque"` until the game describes it;
  types described with `reflect_fields` get theirs from the fields:

```rust
# #[derive(serde::Serialize, serde::Deserialize)] struct Health(f32);
ctx.world.reflect::<Health>();
ctx.world.describe_component("Health", ValueSchema::object([
    ("current", ValueSchema::Number),
    ("max", ValueSchema::Number),
])).unwrap();
// capabilities → { "name": "Health", "schema": { "status": "described",
//   "json_schema": { "type": "object", "properties": { "current": { "type": "number" }, ... },
//                    "required": ["current", "max"] } } }
```

`ValueSchema` has a constructor for each kind of shape: `object` (fields
not marked `optional` are `required`), `strict_object` (an object that
refuses other fields: `"additionalProperties": false`), `enum_of` (one of
some strings), `any_of` (one of some shapes, like an enum's variants that
carry data), `array`, `optional`, and scalars such as `Number`, `Vec2` and
`Entity`. The game itself can read the same answer with
`ctx.capabilities_json()`.

The MCP tool `instance_capabilities` returns this plus `mcp.tools`: each
tool's availability for that game and server (a read-only server marks the
tools that change games unavailable). Against an engine from before this
method it reports `"supported": false` instead of guessing.

Components reflected from C and C++ with fields get schemas from those
fields. `ponko_world_describe_component` / `world.describe_component`
describe any other shape, and `ponko_ctx_capabilities_json` /
`ctx.capabilities_json()` return the answer (see [Agents](agents.md)).

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
PONKO_CONTROL=1 cargo run --release --example boids &
f=$(ls ~/.crabby/instances/*.json | head -1)
port=$(jq .port "$f"); token=$(jq -r .token "$f")
echo "{\"id\":1,\"token\":\"$token\",\"method\":\"info\"}" | nc -q1 127.0.0.1 "$port"
```
