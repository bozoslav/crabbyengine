# Retained game UI

Build a tree once in `ctx.ui_retained`, then update its text, values and styles.
The engine handles layout/input before frame systems and draws the tree above
scene sprites. The immediate-mode `ui_panel`, `ui_label` and `ui_button` APIs
remain available.

In C++ the tree is `ctx.ui_retained()`, a `crabby::ui::tree`; layouts and paints are
built by chaining (`ui::layout().column().size(...)`, `ui::paint().wrap()`). In C it
is the `crabby_ctx_ui_*` functions with a flat `CrabbyUiLayout` and `CrabbyUiPaint`
(see [C and C++](c-cpp.md) and below).

The tree is owned by `Ui`, independently of ECS entities. Stable `UiId` handles
identify widgets and events. Removing a subtree invalidates its IDs permanently.
Taffy supplies flexbox, grid, percentage sizes, min/max constraints, padding,
margin, gap and absolute positioning through the `ui_layout` re-export. It owns
neither the renderer nor the event loop. Crabby owns interaction and appearance.

## A shop and inventory grid

The runnable Rust example is `cargo run --example ui_shop`. Build it without
opening a window with `cargo build --example ui_shop`.

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/ui_shop.rs:shop_tree}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:shop_tree}}
```

</div>
</div>

A row/column is a `Panel` with Taffy's `flex_direction`. An inventory/list is a
panel with grid/column layout whose children are buttons, labels or images.
Keep scroll content from shrinking (`flex_shrink: 0.0`); give the scroll view a
bounded size. `UiWidget::Scroll` clips descendants and supports wheel scrolling,
content dragging and a vertical scrollbar. `set_scroll` uses unscaled layout
units and clamps offsets during layout. Nested scroll views intersect clips.
Navigation scrolls the focused widget into view.

`UiPaint` supplies background colors for normal/hover/pressed/disabled states,
borders, font, line height, text color, wrapping and horizontal alignment.
Set `texture` for an image or textured panel, and `slice` to the source border
size in texture pixels for nine-slice backgrounds. `UiWidget::Toggle` uses
zero/one values; `Slider` uses normalized 0..1 values. Read them with `value`
after a `Changed` event. Changes made with setters do not emit user events.

## Named grid areas and repeat tracks

Template areas name rectangular regions. A row is a space-separated list of names;
`.` (or multiple dots) leaves a cell unnamed. All rows must have equal widths and
each name must fill a rectangle. C/C++ rejects invalid templates with a status
message. Names are UTF-8 identifiers using letters, digits, `_` and `-`, without
CSS escapes; reserved CSS keywords and names starting with digits are rejected.

<div class="lang-pair">
<div>

```rust
use ponko::{Ui, UiWidget, ui_layout::*};
use ponko::ui_layout::style_helpers::*;
let mut tree = Ui::default();
let dashboard = tree.add(UiWidget::Panel, Style {
    display: Display::Grid,
    size: Size { width: length(600.0), height: length(400.0) },
    grid_template_columns: vec![repeat(2, vec![fr(1.0)])],
    grid_template_rows: vec![length(40.0), fr(1.0)],
    grid_template_areas: Some(GridTemplateAreas {
        row_count: 2, column_count: 2,
        areas: vec![
            GridTemplateArea { name: "header".into(), row_start: 1, row_end: 2, column_start: 1, column_end: 3 },
            GridTemplateArea { name: "side".into(), row_start: 2, row_end: 3, column_start: 1, column_end: 2 },
            GridTemplateArea { name: "main".into(), row_start: 2, row_end: 3, column_start: 2, column_end: 3 },
        ],
    }),
    ..Style::default()
});
let placement = GridPlacement::NamedLine("main".into(), 0);
let main = tree.add_child(dashboard, UiWidget::Label, Style {
    grid_column: Line { start: placement.clone(), end: placement.clone() },
    grid_row: Line { start: placement.clone(), end: placement },
    ..Style::default()
});
tree.set_text(main, "Content");
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:ui_grid_areas}}
```

</div>
</div>

C keeps `CrabbyUiTrack` and the original flat track arrays unchanged. For repeats,
use `CrabbyUiTemplateTrack` arrays in `grid_template_columns`/`grid_template_rows`;
a nonzero component count overrides the corresponding original track array.
`repetition` is 0 for a single `track`, 1 for `count` repetitions of `tracks`,
2 for auto-fill, or 3 for auto-fit. Repeat fragments cannot contain repeats.
C++ accepts `ui::repeat(3, ui::fr(1))`, multi-track fragments such as
`ui::repeat(2, {ui::track_points(40), ui::fr(1)})`, and
`ui::auto_fill(ui::track_points(100))` / `ui::auto_fit(ui::track_points(100))`.
Automatic repeats require fixed point or percentage tracks throughout the axis
and at most one automatic repeat per axis, following taffy's supported subset.

`column_names({"left", "middle split", "right"})` and `row_names(...)` name
line boundaries. With repeats these are component boundaries, rather than every
expanded track; use `ui::repeat(3, ui::fr(1)).names({"start", "end"})` for names inside
a repeat fragment. Supply exactly one more name set than repeated tracks.
`tree.set_grid_named_line(node, 0, false, "middle", 1)` places a column start at
the first matching line (axis 1 selects rows, `true` selects the end). Negative
occurrences count backwards; zero also resolves implicit area start/end names.
Unknown names follow taffy's implicit-line placement rules. Named spans are
currently Rust-only. Nested repeats are not supported by taffy.

C can use the layout descriptor fields or change existing nodes with
`crabby_ctx_ui_set_grid_template_areas`, `crabby_ctx_ui_set_grid_line_names`,
`crabby_ctx_ui_set_grid_area`, and `crabby_ctx_ui_set_grid_named_line`.
Each takes the same screen or world-panel `target` as other UI calls. Inputs
are copied during the call. Passing zero area/name rows clears those definitions;
an empty area name restores automatic placement. `set_layout` replaces the whole
style, including areas and names. The C layout struct grew, so rebuild native
callers against the regenerated header.

## Screen stack and modal dialogs

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/ui_shop.rs:shop_dialog}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:shop_dialog}}
```

</div>
</div>

Push a detached root with `push_screen(root, modal)`. All screens render in stack
order; only the top receives input, even when it is not modal. A modal screen
dims earlier screens and owns pointer input outside its rectangle too. Pop with
`pop_screen()`; nodes remain available for reuse. Escape/gamepad East pops the
top screen and emits `ScreenPopped`, consuming the rest of that input snapshot.
The previous screen's focus is restored when possible. Even the last screen can
be popped; game code decides whether to reopen it.

## Events, navigation and ownership

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/ui_shop.rs:shop_events}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:shop_events}}
```

</div>
</div>

`events()` drains `Clicked`, `Changed`, `Submitted` and `ScreenPopped` events
(C++: `tree.events()` or `tree.next_event()`; C: `crabby_ctx_ui_next_event`, one
tagged `CrabbyUiEvent` at a time, `CRABBY_NOT_FOUND` when none are left).
Unread events expire at the next UI frame. Event IDs avoid callbacks borrowing
the context and game state simultaneously. Drain events before mutating the tree.

Tab/Shift-Tab traverses enabled controls. Arrows/d-pad use spatial navigation;
Enter, Space or gamepad South activates the focused control. Text inputs reserve
left/right, Home/End, Backspace/Delete, Shift-selection and Ctrl/Cmd-A for editing;
Enter submits. Clicking/dragging selects text. Text is UTF-8 with scalar-boundary
editing; grapheme-cluster editing, clipboard and IME composition are not provided.
Native committed Unicode is available to both UI and games as `ctx.typed_text()`.
Custom hosts/tests can supply it through `ctx.ui_insert_text()`.

`ctx.mouse_over_ui()` includes immediate and retained UI, including captured
drags. `ctx.ui_wants_keyboard()` reports retained keyboard/gamepad ownership
while any screen is open. These queries do not erase raw game input: game code
uses them to suppress world actions. UI screen changes cancel pointer capture,
so releasing a dialog button cannot activate an underlying screen.

## Scale and headless tests

Layout and pointer coordinates use logical window units. DPI already belongs
to the window/text renderer; `set_scale(1.25)` adds a user preference on top,
scaling geometry, text and hit testing together. Percentage dimensions resolve
against the available logical viewport, so roots can adapt to resolution.

`Ui::frame(viewport, &UiInput)` runs without a GPU and allows deterministic tests
of layout, events and editing. `Ui::layout` resolves geometry without processing
input. Standalone `Ui` measurements use the built-in font; context-owned UI also
measures loaded custom fonts. The engine keeps existing sprite batching and adds
per-batch scissor rectangles; world sprites and immediate UI remain unclipped.

## UI on surfaces in the world

A computer screen, a shop terminal or a sign with buttons is a `UiPanel3D`: a
component with its own `Ui` that the engine draws every frame into a render
target, which a glowing, alpha-blended quad shows. Build `panel.ui` just like the
screen's tree; its widgets are laid out in the target's pixels, so a 600 x 400
panel has a 600 x 400 viewport. `ctx.create_ui_panel(width, height, size)` makes
the render target, the material and the quad (`size` is in meters, in the
entity's XY plane, facing +Z) and returns the panel and the `Model` to spawn
with it, next to a `Transform3D`. The quad is an ordinary model: move and
rotate the entity as usual.

In C++ the entity comes first: `ctx.create_ui_panel(entity, width, height, size)`
gives it the panel and the model, and `ctx.panel_ui(entity)` is the panel's
`ui::tree`, with every call of the screen's tree.

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/ui_panel.rs:panel_create}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:panel_create}}
```

</div>
</div>

The mouse uses the nearest panel the camera ray through it hits, at the pixel
it hits, unless the screen's own UI wants the pointer. Clicking a panel also
gives it the keyboard (typing, Tab, Enter) until another is clicked; the others
only see the pointer. `panel.pointer()` is where the aim is on it this frame, in
its pixels (`None` when it is off the panel), and `panel.interactive = false`
turns its input off. Events, focus and the screen stack belong to each panel's
own `Ui`: read them from it.

When the cursor is locked (a first-person game) or the aim isn't the mouse (a
controller in VR), give panels a ray yourself with `ctx.set_ui_pointer_ray`,
for example from the camera along its forward direction. `None` goes back to the
mouse.

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/ui_panel.rs:panel_events}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:panel_events}}
```

</div>
</div>

Panels are drawn in a pass of their own before the 3D frame that shows them, so
the quad is never a frame behind its contents. Headless runs build and lay out
panels (and `Ui::frame` drives them by hand, like the screen's tree) but draw
nothing and give them no mouse.

## From C

The C API mirrors the Rust one function by function. Nodes are `uint64_t` ids
(`CRABBY_UI_NONE` for "no node"); a removed node's id fails with
`CRABBY_NOT_FOUND` instead of panicking, and a call that fails changes nothing.

```c
CrabbyUiLayout column = crabby_ui_layout_default();
column.flex_direction = CRABBY_UI_FLEX_COLUMN;
column.width = (CrabbyUiLength){CRABBY_UI_LENGTH_PERCENT, 1.0f};
column.padding.top = column.padding.right = column.padding.bottom = column.padding.left =
    (CrabbyUiLength){CRABBY_UI_LENGTH_POINTS, 24.0f};

uint64_t root, buy;
crabby_ctx_ui_add_panel(ctx, CRABBY_UI_SCREEN, CRABBY_UI_NONE, &column, &root);
crabby_ctx_ui_add_button(ctx, CRABBY_UI_SCREEN, root, "Buy", NULL, &buy); /* NULL layout: default */
crabby_ctx_ui_push_screen(ctx, CRABBY_UI_SCREEN, root, false);

CrabbyUiEvent event;
while (crabby_ctx_ui_next_event(ctx, CRABBY_UI_SCREEN, &event) == CRABBY_OK)
    if (event.kind == CRABBY_UI_EVENT_CLICKED && event.id == buy) { /* ... */ }
```

Retained tree operations take a `target` after the context: `CRABBY_UI_SCREEN`
for the screen's own UI, or the entity of a world-space panel to build and read
that panel's UI with the same calls. Node ids are local to the UI that made them;
the same numeric id can exist in several trees. `crabby_ctx_ui_insert_text` and
`crabby_ctx_ui_wants_keyboard` operate on context-wide input and have no target.
Panels are made with `crabby_ctx_create_ui_panel(ctx, entity, width, height, size, &texture)`,
which gives the entity (it needs a `CrabbyTransform3D`) the panel and its model;
`crabby_ctx_ui_panel_pointer`, `crabby_ctx_set_ui_panel_interactive`,
`crabby_ctx_remove_ui_panel`, `crabby_ctx_set_ui_pointer_ray` and
`crabby_ctx_clear_ui_pointer_ray` do the rest.

```c
CrabbyTexture texture;
crabby_ctx_create_ui_panel(ctx, terminal, 600, 400, (CrabbyVec2){1.2f, 0.8f}, &texture);
uint64_t buy;
crabby_ctx_ui_add_button(ctx, terminal, CRABBY_UI_NONE, "Buy", NULL, &buy);
crabby_ctx_ui_push_screen(ctx, terminal, buy, false);
```

`CrabbyUiLayout` is the Taffy `Style` flattened: display, position, flex
direction, wrap, grow, shrink and basis, the alignment properties, width, height,
min/max sizes, margin, padding, inset, gap, aspect ratio, grid tracks (arrays with
a count, read only during the call) and grid item placement. Lengths are a kind
(automatic, points or a fraction of the parent) and a value. Enum-valued fields
are `uint32_t` holding the constants of the matching `CrabbyUi*` enum, and an
unknown value is an error. `CrabbyUiPaint` is `UiPaint`; fonts come from
`crabby_ctx_load_font`. Text is read back into a buffer you size with
`crabby_ctx_ui_text` (call once with capacity 0 for the length).

`crabby_ctx_ui_frame` runs a frame by hand with a `CrabbyUiInput`, like
`Ui::frame`, which is how the C and C++ tests click buttons without a window.

This chapter's Rust and C++ examples build the same shop: the C++ one is
`examples/cpp/guide.cpp`, compiled and run headless in CI.
