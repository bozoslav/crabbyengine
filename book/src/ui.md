# Retained game UI

Build a tree once in `ctx.ui_retained`, then update its text, values and styles.
The engine handles layout/input before frame systems and draws the tree above
scene sprites. The immediate-mode `ui_panel`, `ui_label` and `ui_button` APIs
remain available.

The tree is owned by `Ui`, independently of ECS entities. Stable `UiId` handles
identify widgets and events. Removing a subtree invalidates its IDs permanently.
Taffy supplies flexbox, grid, percentage sizes, min/max constraints, padding,
margin, gap and absolute positioning through the `ui_layout` re-export. It owns
neither the renderer nor the event loop. Crabby owns interaction and appearance.

## A shop and inventory grid

The runnable Rust example is `cargo run --example ui_shop`. Build it without
opening a window with `cargo build --example ui_shop`.

```rust
{{#include ../../examples/ui_shop.rs:shop_tree}}
```
<!-- TODO C++ pair -->

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

```rust
{{#include ../../examples/ui_shop.rs:shop_dialog}}
```
<!-- TODO C++ pair -->

Push a detached root with `push_screen(root, modal)`. All screens render in stack
order; only the top receives input, even when it is not modal. A modal screen
dims earlier screens and owns pointer input outside its rectangle too. Pop with
`pop_screen()`; nodes remain available for reuse. Escape/gamepad East pops the
top screen and emits `ScreenPopped`, consuming the rest of that input snapshot.
The previous screen's focus is restored when possible. Even the last screen can
be popped; game code decides whether to reopen it.

## Events, navigation and ownership

```rust
{{#include ../../examples/ui_shop.rs:shop_events}}
```
<!-- TODO C++ pair -->

`events()` drains `Clicked`, `Changed`, `Submitted` and `ScreenPopped` events.
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

This chapter currently has Rust examples only; C/C++ bindings are deferred.
