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
crabby_ctx_ui_add_panel(ctx, CRABBY_UI_NONE, &column, &root);
crabby_ctx_ui_add_button(ctx, root, "Buy", NULL, &buy); /* NULL layout: default */
crabby_ctx_ui_push_screen(ctx, root, false);

CrabbyUiEvent event;
while (crabby_ctx_ui_next_event(ctx, &event) == CRABBY_OK)
    if (event.kind == CRABBY_UI_EVENT_CLICKED && event.id == buy) { /* ... */ }
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
