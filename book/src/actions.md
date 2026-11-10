# Named input actions

Bind physical inputs to names once, then query those names in game systems.
An action is a button, a scalar axis, or a vector axis. Several sources can
control the same action; this makes keyboard and controller support share
the same game logic. Configuration and rebinding may allocate. Event handling,
frame resets and action queries allocate nothing.

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/action_map.rs:action_map}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:action_map}}
```

</div>
</div>

`feed_input` supplies events manually for headless tests and replay. Live
keyboard, mouse and gamepad input updates the same action state automatically.
Frame edges clear after each frame (after each tick in headless runs).

## Input scenarios

To replay input by simulation tick, build an `InputScenario` with
`.at(tick, event)` and install it with `Game::input_scenario` (Rust only for
now). Before each tick's simulation systems run, the scenario feeds that tick's
events through `feed_input`, the same path as live input, so actions see
ordinary pressed, held and released states. The same seed and scenario
therefore give the same headless run. Nothing is injected unless a game calls
`input_scenario` (or a tool is explicitly allowed to, see below).

```rust
# struct Jumps { count: u32 }
let scenario = ponko::scenario::InputScenario::new()
    .named("jump-once")
    .at(10, ponko::actions::InputEvent::Key(Key::Space, true)) // pressed on tick 10, held after
    .expect(10, ponko::scenario::ScenarioCheck::action("jump").pressed(true).down(true))
    .at(12, ponko::actions::InputEvent::Key(Key::Space, false))
    .expect(12, ponko::scenario::ScenarioCheck::action("jump").released(true))
    .expect(12, ponko::scenario::ScenarioCheck::resource("Jumps", "/count", 1)); // a reflected resource, by JSON pointer
let (world, report) = Game::new("test", 1, 1)
    .sim_system(|ctx: &mut Ctx| {})
    .run_headless_scenario(scenario, |ctx| {});
report.assert_passed(); // or look at report.failure
```

- **Ticks.** The first tick of a run is 0. A step at tick `t` is fed before
  tick `t`'s systems run, and several steps on one tick keep the order they were
  added in, so a press and a release on the same tick keep both edges. A scenario
  started mid-run (`ctx.input.start_scenario`, or a tool) counts from the next
  tick. Pausing doesn't move a step, because a paused game doesn't advance the tick.
- **Several ticks per frame.** In a window, one frame can run several ticks. A
  step is still fed before its own tick, but edges last until the end of the
  frame, as with live input: a press on the first tick of a three-tick frame is
  also `pressed` on the other two. Headless runs have one tick per frame.
- **Checks** run after the tick's systems: `ScenarioCheck::action` compares an
  action's `pressed`, `down` and `released` (an unbound name fails rather than
  passing), `resource` compares a reflected resource's JSON field, and `count`
  counts the entities with a reflected component. The first check that doesn't
  hold ends the run, and `ScenarioReport::failure` says which: its `step`, its
  `tick`, what was `expected` and what was `actual`, as data.
- **The report** is a `ScenarioReport` resource in the world (status `pending`,
  `running`, `passed`, `failed` or `cancelled`, ticks run, checks passed).
  `run_headless_scenario` runs exactly `scenario.ticks()` ticks, which is the
  bound on a run, and hands it back with the world.
- **Releasing input.** `ctx.input.cancel_scenario(&mut ctx.world, &mut ctx.time)`, a scenario that replaces
  it, a failed check and a tool's disconnect release every key, button and axis
  the scenario still holds. A run in game code that simply finishes leaves its
  input as it was, unless the scenario says `.release_when_done()`; a run a tool
  loads releases it by default (see below). Only what the scenario itself left
  held is released, but the input state is one shared set of held keys with no
  record of which device holds one: a key a player also holds physically reads as
  up until its next key event.
- **Started from a system.** `ctx.input.start_scenario` called from a simulation
  system begins on the next tick; nothing of it (input or checks) happens on the
  tick that started it.

### The scenario file

A scenario also has a small versioned JSON form, which tools send and tests can
keep next to their fixtures (`InputScenario::from_json`, `to_json`):

```json
{
  "version": 1,
  "id": "jump-once",
  "duration": 120,
  "release_when_done": true,
  "steps": [
    { "tick": 10, "type": "key", "key": "Space", "down": true },
    { "tick": 10, "type": "expect_action", "action": "jump", "pressed": true },
    { "tick": 12, "type": "key", "key": "Space", "down": false },
    { "tick": 12, "type": "expect_resource", "resource": "Jumps", "pointer": "/count", "equals": 1 },
    { "tick": 12, "type": "expect_count", "component": "Coin", "equals": 2 }
  ]
}
```

Input steps are `key`, `mouse`, `gamepad_button`, `gamepad_axis`, `focus_lost`
and `gamepad_disconnected`; check steps are `expect_action`, `expect_resource`
and `expect_count`. Steps may come in any order, and a step's number in a
failure report is its position in the file. Unknown fields, a `version` this
engine doesn't read, more than 100,000 steps and steps past `duration` are
errors, so a typo can't quietly weaken a test. The format is for reproducing
gameplay on one build; it isn't a promise of identical replays across engine
versions.

### From tools

With the [control endpoint](control.md) on, `input.scenario.load`,
`input.scenario.status` and `input.scenario.cancel` run a scenario in a live
game, and the MCP server's `run_input_scenario` steps a paused game through one
and returns the report (see [Working with AI agents](agents.md)). A game that
only opens the endpoint for inspection refuses to load scenarios: allow it with
`Game::input_injection(true)` or `PONKO_INPUT_INJECTION=1` (the MCP `launch`
tool sets it). A scenario loaded with `bind_connection` is cancelled, and its
input released, when the client that loaded it disconnects.

A tool's scenario releases the keys it holds when it ends, however it ends, so a
tool that dies mid-run leaves nothing pressed. With `keep_input_when_done` they
stay held (to look at the result, say), and stay owned by the scenario's owner:
`input.scenario.cancel`, or the owner's disconnect when it was bound, still
releases them, and the report's `still_held` says how many are left. With
`max_ticks` a too-long scenario is refused before the running one is touched, and
with `queue_ticks` a paused game gets the scenario's own ticks queued as steps; a
failed check or a cancel takes back only those that haven't run, never ticks
another client asked for with `sim.step`.

| Action | Rust sources | C++ sources |
| --- | --- | --- |
| `Action::Button` / `action::button` | `ButtonBinding::Key`, `Mouse`, `Gamepad` | `action_binding::key`, `mouse`, `button` |
| `Action::Axis` / `action::axis` | `AxisBinding::Keys`, `Gamepad` | `action_binding::keys`, `axis` |
| `Action::Axis2` / `action::axis2` | `Axis2Binding::Keys`, `Gamepad` | `action_binding::keys2`, `stick` |

For example, add `ButtonBinding::Mouse(MouseButton::Left)` or
`action_binding::mouse(PONKO_MOUSE_LEFT)` to a button action. A scalar key pair
takes the negative key followed by the positive key. Four-key vectors take
left, right, up, down, so WASD is A, D, W, S. Opposing keys cancel and keyboard
diagonals are normalized. Vectors use screen coordinates: positive Y is down.

Buttons combine with OR. `action_pressed` fires when the first source becomes
held; `action_released` fires when the last held source releases. Pressing a
second source or releasing one of two held sources produces neither edge.
A press and release within one frame preserves both edges.

Scalar and vector axes choose the source with the largest absolute value or
vector length; the first binding wins ties. Sources do not add together.
Sticks use a radial deadzone; scalar gamepad axes use a component deadzone.
Inside it the value is zero, and outside it the remaining range is rescaled
to reach one. A deadzone must be finite and in `0..1`; invalid values disable
that source. Non-finite analog input reads zero. Disconnection releases the
pad's buttons and centers its axes. An unconnected slot starts at zero.

Use `action_down`, `action_pressed`, `action_released`, `action_axis` and
`action_axis2` on `Ctx` / `ctx`. Missing names and queries of the wrong kind
return false or zero. Names are case-sensitive. Queries borrow their names
and allocate nothing.

In Rust, resolve each name once and keep the `Copy` `ActionId`: queries by id
are an array index instead of a name lookup, and an unknown name is an error
logged when you resolve it, not a silent `false` every frame. `ctx.input.action_id("jump")`
resolves one, `ctx.input.action_ids(&["jump", "fire"])` several, and the `actions!`
macro (as in the example above) declares the names in one place. Ids stay
valid when you rebind with `set_actions`. The queries accept an id or a name,
so both spellings work. C and C++ keep the name forms for now.

To rebind, copy `ctx.input.actions()`, call `set` or `remove`, then install it with
`ctx.set_actions`. Rust uses `.clone()` to copy the map; C++ returns an owned
map. Installation resets action edges and uses currently held inputs as the
baseline. Rebinding a held key therefore does not generate a new press.

A map reads back: `get("jump")` gives one action's bindings, `iter()` every
action in name order (the order the JSON file stores them in), and `len()`
counts them. An `Action`'s `buttons()`, `axes()` and `axes2()` are its
sources as slices (empty for the other kinds). A settings screen can show
what `ctx.input.actions()` binds instead of keeping its own copy. To change
one action without rebuilding the map, `ctx.input.set_action("jump", action)`
replaces it in place. The name keeps its `ActionId`, the other actions keep
their state and edges, and held sources become the baseline as above:

```rust
# use ponko::*;
# fn example(ctx: &mut Ctx) {
let mut jump = ctx.input.actions().get("jump").cloned().unwrap_or(Action::Button(vec![]));
if let Action::Button(sources) = &mut jump {
    sources.retain(|s| !matches!(s, ButtonBinding::Key(_)));
    sources.insert(0, ButtonBinding::Key(Key::K)); // the player's new key
}
ctx.input.set_action("jump", jump);
# }
```

The read-back and `set_action` are Rust-only for now; C and C++ rebuild the map
and install it as described above.

`ActionMap::to_json` / `action_map::to_json` save bindings only, with the same
serde JSON format in both languages. Write that string to your settings file
using normal file I/O. Read it back with `ActionMap::from_json` /
`action_map::from_json`, then install the result. Invalid JSON fails before
changing active bindings. Rust returns `Result`; C++ throws the usual engine
exception. Button state is never saved.

The C API uses an owned `PonkoActionMap`: create/free, set/remove,
load/save JSON, then `ponko_ctx_set_actions`. `ponko_action_map_set` takes
a kind (0 button, 1 scalar, 2 vector) and an array of `PonkoActionBinding`.
`PonkoActionSource` documents each source's fields; unused fields are zero.
Save follows the standard two-call buffer convention: query the length,
allocate length + 1 bytes, then save again. Context queries are named
`ponko_ctx_action_down`, `pressed`, `released`, `axis` and `axis2`.

This map has no chords, action contexts, smoothing, interactive binding UI,
or device-specific profiles. Each gamepad binding names a stable controller
slot, the same slot used by the raw gamepad API.
