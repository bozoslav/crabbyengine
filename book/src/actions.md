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

To replay input by simulation tick, build an `InputScenario` with
`.at(tick, event)` and install it with `Game::input_scenario` (Rust only for
now). Before each tick's simulation systems run, the scenario feeds that tick's
events through `feed_input`. The same seed and scenario therefore give the same
headless run.

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
