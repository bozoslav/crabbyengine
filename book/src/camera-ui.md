# Camera, text and UI

Big worlds need a camera, and games need words on screen. Everything here is
drawn with sprites, so it batches with the rest of the frame.

## World space and screen space

There are two kinds of coordinates:

- **World units**: sprites, `ctx.draw`, and the shapes below. They go
  through the camera.
- **Window pixels**: UI and text, drawn on top of the world. `(0, 0)` is the
  window's top-left. Set `DrawParams { screen: true, .. }` to draw a sprite
  in window pixels (an icon in a HUD).

The default camera shows world point `(0, 0)` at the top-left at zoom 1, so
until you move it, world units are window pixels.

## The camera

`ctx.camera` is a [`Camera2D`](api/ponko/struct.Camera2D.html) (C++:
`ctx.camera()` / `ctx.set_camera(c)` with the same two fields):
`position` (the world point at the window's top-left) and `zoom` (window
pixels per world unit). Its methods do the usual jobs:

| Want to | Call |
|---|---|
| follow something | `ctx.camera.look_at(hero)` every frame |
| zoom toward the mouse | `ctx.camera.zoom_at(ctx.mouse_pos(), 1.1)` |
| know what the mouse points at | `ctx.mouse_world()` |
| put a label on a unit | `ctx.camera.world_to_screen(unit)` |
| skip work for things off screen | `ctx.camera.visible_rect().contains(p)` |

For looking around a big world, add the ready-made system:

<div class="lang-pair">
<div>

```rust
Game::new("Colony", 1280, 720)
    .system(camera_controls) // right/middle drag pans, wheel zooms
```

</div>
<div>

```cpp
ponko::game("Colony", 1280, 720)
    .camera_controls()       // right/middle drag pans, wheel zooms
```

</div>
</div>

The ants example starts zoomed out to fit every colony:

<div class="lang-pair">
<div>

```rust
# let bounds = vec2(3200.0, 1800.0);
let fit = ctx.window_size() / bounds;
ctx.camera.zoom = fit.x.min(fit.y);
ctx.camera.look_at(bounds / 2.0);
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:camera_fit}}
```

</div>
</div>

## Shapes

In world units, like sprites:

<div class="lang-pair">
<div>

```rust
# let (from, to, tower) = (vec2(0.0, 0.0), vec2(100.0, 50.0), vec2(300.0, 300.0));
ctx.gfx.draw_rect(Rect::new(0.0, 0.0, 200.0, 20.0), Color::hex(0x3a5a40));
ctx.gfx.draw_line(from, to, 2.0, Color::RED);
ctx.gfx.draw_circle(tower, 120.0, Color::BLUE.with_alpha(0.2));
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:shapes}}
```

</div>
</div>

C++ rectangles are `{min, max}`; `ponko::rect_at(min, size)` makes one
from a corner and a size, like Rust's `Rect::new`.

## Text

<div class="lang-pair">
<div>

```rust
ctx.draw_text("Hello", vec2(20.0, 20.0), 24.0, Color::WHITE);
let size = ctx.gfx.measure_text("Paused", 32.0); // for centering
let title = ctx.gfx.load_font("fonts/Title.ttf"); // TrueType or OpenType
ctx.gfx.draw_text_ex("Colony", vec2(20.0, 60.0), TextParams { font: title, size: 40.0, ..Default::default() });
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:text}}
```

</div>
</div>

In C++, `draw_text(text, pos, params)` is `draw_text_ex`, with
`ponko::text_defaults()` to start from.

- `size` is the line height in window pixels; `\n` starts a new line.
- The built-in font is Ubuntu Light (Ubuntu Font Licence).
- Glyphs are rasterized at the screen's real pixel size the first time
  they're drawn and kept in an atlas texture, so text is sharp on Retina
  screens and costs only a lookup per glyph after that.
- `measure_text` works headless too, so layout code can be tested.

## UI

A few immediate-mode widgets, drawn in window pixels, for debug overlays and
quick prototypes; a game's menus and HUD belong in the [retained UI](ui.md).
Call them every frame:

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/ui.rs:hud}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:hud}}
```

</div>
</div>

The widgets are on `ctx.ui.immediate` and say what they use: all of them
draw through `ctx.gfx`, and a button also reads the mouse from `ctx.input`
(C++: `ctx.ui_panel(rect)`, `ctx.ui_label(pos, text)`, `ctx.ui_button(rect, label)`).

- `panel(&mut ctx.gfx, rect)`: a background.
- `label(&mut ctx.gfx, pos, text)`: text in the style's size and color;
  returns the space it took, so you can stack lines.
- `button(&mut ctx.gfx, &ctx.input, rect, label)`: `true` on the frame it's
  clicked.
- `ctx.ui.immediate.style` sets colors, text size and padding (C++: `ui_style()`,
  `set_ui_style(s)`).
- `ctx.ui.wants_pointer()` says whether the mouse is over UI, so a click on a
  button doesn't also act on the world behind it. `camera_controls` checks
  it before zooming.

The whole scene from `cargo run --release --example ui`:

```rust
{{#include ../../examples/ui.rs:scene}}
```

## For agents

The control endpoint has `camera.get` and `camera.set`, and `ponko-mcp`
has a `set_camera` tool. Combined with `screenshot`, an agent can look at
any part of a running world.
