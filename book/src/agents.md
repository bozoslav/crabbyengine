# Working with AI agents (MCP)

`ponko-mcp` is a [Model Context Protocol](https://modelcontextprotocol.io)
server. It lets an AI coding agent (Claude Code, Cursor, Codex, ...) start
your game, look inside it while it runs, change it, and step the simulation,
instead of guessing from source code and print statements.

It builds on the [control endpoint](control.md) and [reflection](reflection.md).

## Setup

In this repository it's already configured: `.mcp.json` registers the server
for any MCP client that reads project config (Claude Code asks once to
approve it).

For your own game, install the binary and add it to your agent:

```sh
cargo install ponko_mcp
claude mcp add ponko -- ponko-mcp            # Claude Code
claude mcp add ponko-view -- ponko-mcp --read-only
```

Other clients take the same command (`ponko-mcp`, stdio transport) in their
MCP config.

Then make the components you want agents to see reflected:

```rust
# #[derive(serde::Serialize, serde::Deserialize)] struct Health(f32);
ctx.world.reflect::<Health>();
```

## What an agent can do

| Tool | Changes the game | What it's for |
|---|---|---|
| `list_instances` | | find running games |
| `launch` | ✓ | build and start a game or example; headless and paused by default |
| `instance_info` | | tick, sim time, paused, speed, fps, entity count, render CPU/GPU ms |
| `instance_capabilities` | | what this game can do before you try: headless or windowed, compiled features, subsystems and why one is off, which tools work on it (and why not), reflected types with their field schema ([details](control.md#capabilities)) |
| `world_summary` | | archetypes, entity counts, reflected components |
| `list_entities` | | entities with component values; filter by component names and `where` conditions (`Health.current < 5`), paged |
| `get_entity` | | one entity's components |
| `entity_stats` | | min, max, mean and value counts over every matching entity: "how many units per team, how hurt are they?" ([details](control.md#questions-about-many-entities)) |
| `list_resources` / `get_resource` | | game-wide state: scores, rules, the current round |
| `system_timings` | | which systems are slow |
| `ecs_diagnostics` | | archetypes, component storage, and every system's phase, order and timings ([details](#ecs-diagnostics)) |
| `perf_report` | | why frames are slow: fps and frame-time percentiles, CPU vs GPU, GPU time per pass, triangles drawn for the camera and the shadows, the heaviest meshes ([details](performance.md#profiling-a-frame)) |
| `shader_status` | | did a shader edit take effect? Each custom shader's last compile or hot reload result, and its errors with line and column in your `.wgsl` file ([details](3d.md)) |
| `record_trace` | | the next N frames as a Perfetto trace, to find hitches |
| `perf_hud` | | show the performance overlay, e.g. before a screenshot |
| `list_events` | | event types the game shows tools, and how many are in flight |
| `subscribe_events` / `take_events` | | collect [events](events.md) while ticks run, then see what happened (each with its tick) |
| `read_logs` | | recent stdout/stderr of any running game, also one you started yourself ([logs](control.md#logs)); pass `next` back as `since` for only new lines |
| `screenshot` | | the next frame of a windowed game, as an image the agent can look at |
| `set_camera` | | move or zoom the camera (`look_at`, `zoom`, `position`), e.g. before a screenshot |
| `run_ticks` | ✓ | run exactly N ticks of a paused sim and wait |
| `watch` | ✓ | run N ticks and sample every K: tick, resources, entity counts, time per tick |
| `run_input_scenario` | ✓ | run a tick-indexed [input scenario](actions.md#input-scenarios) (key, mouse and gamepad steps plus checks) on a paused sim; returns the report, with the failing step, tick, expected and actual for a failed check. The keys it holds are released when it ends (`keep_input_when_done` keeps them until `cancel_input_scenario`). Needs a game that allows input injection (`launch` sets it) |
| `cancel_input_scenario` | ✓ | end a scenario and release the input it holds |
| `pause` / `resume` / `set_speed` | ✓ | sim clock |
| `spawn_entity` / `despawn_entity` | ✓ | |
| `set_component` / `remove_component` | ✓ | change values; object fields merge |
| `set_resource` | ✓ | change game-wide state; object fields merge |
| `save_world` | | checkpoint the world to a file |
| `load_world` | ✓ | roll back to a checkpoint (same entity ids) |
| `save_scene` / `load_scene` | ✓ | write or read a [scene](scenes.md) in the game's `assets/` |
| `spawn_prefab` / `save_prefab` | ✓ | place a [prefab](scenes.md) at a point, or save an entity as one |
| `quit_instance` | ✓ | |

`--read-only` hides every tool marked ✓, for agents that should only watch.

## ECS diagnostics

`ecs_diagnostics` (endpoint method `ecs.diagnostics`, Rust
`ctx.ecs_diagnostics()`) answers "why does this tick cost what it does?".
It is built only when asked, so a game that never asks pays nothing. The
answer has a `version` (1); field names stay stable and new ones may be added.

- `archetypes`: `index`, `components` (full type names), `entities`,
  `bytes_per_entity`, `allocated_bytes`. Empty ones are left out;
  `archetypes_total` counts all of them.
- `components`: `name`, `short_name`, `size`, `align`, and how many
  `archetypes`, `entities` and `bytes` hold it.
- `systems`, in run order: `name`, `phase` (`Input`, `BeforePhysics`,
  `Physics`, `AfterPhysics`, `SimLast`, `Frame`, `Animate`, `Late`, `Draw`),
  `order`, `sim`, `every`/`offset` (runs on ticks where
  `tick % every == offset`), `access` and `last_ms`/`avg_ms`. Every system is
  `exclusive` for now: it runs alone with `&mut Ctx`, and its commands are
  applied right after it.
- With `with: ["Pos", "Vel"]`, `matching` counts the archetypes and entities
  a query for those components visits.

Troubleshooting a slow tick, for example:

1. `system_timings` shows `game::steer` at 6 ms average.
2. `ecs_diagnostics` with `with: ["Transform2D", "Vel"]` gives
   `"matching": {"archetypes": 41, "entities": 20000}`, and `archetypes`
   lists 41 small ones that differ only by a `Stunned` marker component.
3. So the query walks many archetypes. The fix is a `bool` field instead of
   the marker; then `ecs_diagnostics` again: 1 archetype, and `steer`'s
   `avg_ms` drops.

## Several agents, several games

Every game with the endpoint on registers itself in `~/.crabby/instances/`,
and every `ponko-mcp` reads that registry. So:

- each agent runs its own `ponko-mcp`, and all of them see all games;
- every tool takes `instance` (the id, pid, port or name from
  `list_instances`), optional when only one game runs;
- the game applies requests between ticks, one at a time, so agents never
  race the simulation or each other.

A typical split: one agent edits gameplay code and relaunches the game, while
a read-only agent watches system timings in a long-running instance.

## A session

An agent asked to "find out why boids clump at the edges" might do:

1. `launch` with `example: "boids"`. The game builds, starts headless and
   paused, and returns `{"id": "6257-57560", "name": "boids", ...}`.
2. `world_summary`: one archetype `[Transform2D, Sprite, Vel]` with 20,000 entities.
3. `run_ticks` with `ticks: 600` (10 simulated seconds), then
   `entity_stats` with `fields: ["Transform2D.position"]` and
   `where: ["Transform2D.position.0 < 20"]` to count how many are near the
   left border (and `list_entities` with the same `where` to look at some).
4. Edit the steering code, `quit_instance`, `launch` again, and compare.
5. `system_timings` to check the fix didn't make the tick slower.

Or, without relaunching: `save_world` at tick 0, `set_component` to try a
different value on some boids, `run_ticks 600`, look, then `load_world` to
roll back and try the next value.

Because the sim only advances on `run_ticks`, every run is repeatable and the
agent can check results at exact ticks.

To see how a run develops, `watch` does it in one call. In the battle
example, with its `Battle` stats reflected:

```json
{"ticks": 3600, "every": 600, "resources": ["Score"], "count": ["Unit"]}
```

returns seven samples (before, then every 10 simulated seconds), each with
the tick, the `Score` resource, how many entities have a `Unit`, and the
wall-clock time per tick, so "who's winning, and does it get slower as
the armies meet?" is one answer.

To check how things look, `launch` with `headless: false` and call
`screenshot`. It works while the window is hidden behind others, and is
scaled down to 1024 pixels by default (`max_size`) to keep it cheap to look
at. A screenshot of the ants example after 40 seconds showed food carriers
looping instead of going home; giving them a sense of where home is raised
deliveries in that time from 328 to 1,747.

## Making your game agent-friendly

- Reflect the components that matter for gameplay, the resources that
  hold game-wide state (`world.reflect_resource::<Score>()`), so an agent
  can ask "who's winning?" without counting entities, and the events
  (`ctx.events.reflect::<Died>()`), so it can see what happened, not only
  the state afterwards.
- Describe the shape of reflected types an agent should write
  (`world.describe_component("Health", ValueSchema::object([...]))`), so
  `instance_capabilities` lists their fields; otherwise they show as opaque.
- Read sizes and seeds from environment variables (like the examples'
  `BOIDS` and `HEADLESS`), so `launch` can set them through `env`.
- Support headless: put game logic in sim systems; it then runs without a
  window or GPU, which is faster for agents and works on CI.


## A reproducible development loop

Start with a small reproduction and record the engine revision, example or
game command, features, seed, input sequence and expected state. Use `launch`
with a controlled environment, inspect the reflected state, then `run_ticks`
to a fixed tick and record `get_entity`/`get_resource` results. Save a
checkpoint when useful; only reflected/serialized game data can be inspected
or restored through these tools.

After editing code, compile, run the relevant tests and relaunch the same
reproduction. Compare the same state and ticks. For a visual change, use a
windowed instance and record camera, resolution, quality and screenshots;
headless success does not verify rendering. Collect a `perf_report` or
`record_trace` when the change affects performance, and preserve comparable
before/after workloads. A screenshot alone does not establish simulation
correctness or a performance improvement.

### Validation bundles (first stage)

`cargo run --example validation_bundle -- --out DIR --variant buggy|fixed`
runs a small headless interaction driven by a tick-indexed
[`InputScenario`](actions.md) and writes one self-contained directory: a
versioned `manifest.json` (engine version, commit/features/rustc when you
supply them, scenario id and seed, commands, assertion outcomes with the
failing tick, artifact sizes and SHA-256 hashes), `logs/` and a per-tick
`trace.csv`. `--step "cmd args"` runs a bounded command first (a build, a
launch check); a failed or timed-out step ends the run with its output kept
(start and end, bounded). Control tokens, passwords and authorization values
are replaced by `<redacted>`, and `--redact-paths` swaps the working, home and
bundle directories for `<CWD>`, `<HOME>` and `<BUNDLE>` without touching the
rest of a path. Metrics a run can't produce (screenshots, performance) are
marked `unavailable` with the reason. The bundle shows what one run observed;
it is not proof of pixel identity or visual correctness. Screenshots,
performance reports and driving a launched game through the MCP tools are
later stages.

These tools exist today. A general gameplay-input recording/replay service,
structured scheduler inspection, richer shader diagnostics and broader
GPU/asset memory reports are not implied by their presence. Follow
[#145](https://github.com/ponkoengine/ponko/issues/145),
[#153](https://github.com/ponkoengine/ponko/issues/153) and
[#157](https://github.com/ponkoengine/ponko/issues/157) for the relevant
planned extensions. Reliable autonomous creation of complete games remains
a long-term objective, with human review and reproducible evidence still
required. See [the project vision](https://github.com/ponkoengine/ponko/blob/main/PROJECT_VISION.md).

### Instance schemas and GPU details

The read-only `capabilities` control request (and MCP `instance_capabilities` tool)
reports this instance's mode, compiled features, operation availability, and
reflected component/resource schemas. In a windowed renderer,
`subsystems.renderer.adapter` identifies the adapter/backend and
`subsystems.renderer.gpu` contains the same limits and feature fallback reasons
as `Gfx::gpu_capabilities()`. Before a GPU opens, those values are null and
`adapter_reason` explains why. Headless instances report an inactive renderer.
A tool being present in MCP does not imply that the selected instance supports it.

Built-in transforms, attachments, sprites, models, LOD, lights, decals and
registered physics components carry explicit schemas. Asset references use their
serialized file names; they are not GPU handles. Arbitrary Serde registrations
still work without extra bounds and remain explicitly opaque until described.
Schemas describe JSON shape, not asset existence or all value constraints.

C uses `ponko_world_describe_component(world, name, schema)` and
`ponko_world_describe_resource(world, name, schema)`; C++ uses
`world.describe_component(name, schema)` / `describe_resource(name, schema)`.
Names must already be reflected. The schema string encodes Rust's `ValueSchema`,
for example `{"Object":[["hp","Number"],["label",{"Optional":"String"}]]}`.
Scalar names include `Bool`, `Integer`, `Unsigned`, `Number`, `String`, `Entity`,
`Vec2`, `Vec3`, `Color`, `Null`, and `Opaque`. Containers are `Object` (ordered
name/schema pairs), `Array`, `Optional`, `Tuple` (an array of schemas), `OneOf`
(alternative schemas), and `Enum` (string values). Invalid JSON or an unknown
reflected name returns an error without replacing existing metadata. Describing
again replaces metadata only: serialization and field registration are unchanged.
The native `capability_schema.c` and `.cpp` tests compile and exercise both APIs.
