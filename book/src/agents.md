# Working with AI agents (MCP)

`crabby-mcp` is a [Model Context Protocol](https://modelcontextprotocol.io)
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
cargo install crabby_mcp
claude mcp add crabby -- crabby-mcp            # Claude Code
claude mcp add crabby-view -- crabby-mcp --read-only
```

Other clients take the same command (`crabby-mcp`, stdio transport) in their
MCP config.

Then make the components you want agents to see reflected:

```rust
ctx.world.reflect::<Health>();
```

## What an agent can do

| Tool | Changes the game | What it's for |
|---|---|---|
| `list_instances` | | find running games |
| `launch` | ✓ | build and start a game or example; headless and paused by default |
| `instance_info` | | tick, sim time, paused, speed, fps, entity count, render CPU/GPU ms |
| `world_summary` | | archetypes, entity counts, reflected components |
| `list_entities` | | entities with component values; filter by component names and `where` conditions (`Health.current < 5`), paged |
| `get_entity` | | one entity's components |
| `entity_stats` | | min, max, mean and value counts over every matching entity: "how many units per team, how hurt are they?" ([details](control.md#questions-about-many-entities)) |
| `list_resources` / `get_resource` | | game-wide state: scores, rules, the current round |
| `system_timings` | | which systems are slow |
| `list_events` | | event types the game shows tools, and how many are in flight |
| `subscribe_events` / `take_events` | | collect [events](events.md) while ticks run, then see what happened (each with its tick) |
| `read_logs` | | recent stdout/stderr of any running game, also one you started yourself ([logs](control.md#logs)); pass `next` back as `since` for only new lines |
| `screenshot` | | the next frame of a windowed game, as an image the agent can look at |
| `set_camera` | | move or zoom the camera (`look_at`, `zoom`, `position`), e.g. before a screenshot |
| `run_ticks` | ✓ | run exactly N ticks of a paused sim and wait |
| `watch` | ✓ | run N ticks and sample every K: tick, resources, entity counts, time per tick |
| `pause` / `resume` / `set_speed` | ✓ | sim clock |
| `spawn_entity` / `despawn_entity` | ✓ | |
| `set_component` / `remove_component` | ✓ | change values; object fields merge |
| `set_resource` | ✓ | change game-wide state; object fields merge |
| `save_world` | | checkpoint the world to a file |
| `load_world` | ✓ | roll back to a checkpoint (same entity ids) |
| `quit_instance` | ✓ | |

`--read-only` hides every tool marked ✓, for agents that should only watch.

## Several agents, several games

Every game with the endpoint on registers itself in `~/.crabby/instances/`,
and every `crabby-mcp` reads that registry. So:

- each agent runs its own `crabby-mcp`, and all of them see all games;
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
- Read sizes and seeds from environment variables (like the examples'
  `BOIDS` and `HEADLESS`), so `launch` can set them through `env`.
- Support headless: put game logic in sim systems; it then runs without a
  window or GPU, which is faster for agents and works on CI.
