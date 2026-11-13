### Breaking changes
- GPU asset loading, streaming and residency live on one service,
  `Graphics` (`ctx.gfx`), next to `load_texture`/`load_mesh`:
  `ctx.resident_mesh(path)` → `ctx.gfx.resident_mesh(path)`, and the same
  for `try_resident_mesh`, `pin_resident_mesh`, `unpin_resident_mesh`,
  `resident_texture`, `try_resident_texture`, `pin_resident_texture`,
  `unpin_resident_texture`, `set_residency_budget`, `stream_texture` and
  `stream_mesh`. `ctx.finish_uploads()` stays on `Ctx` (it may evict, which
  looks at the world).
- Residency calls no longer evict. Eviction happens only in the engine's
  streaming step at the start of each frame (and in `ctx.finish_uploads()`),
  which looks at the game's own world for what entities show. A
  `resident_*` load, a lower `set_residency_budget` or an `unpin_resident_*`
  can therefore leave usage over the budget for at most one frame, until
  the next frame's check evicts what may go (assets asked for in this or the
  previous frame are protected either way). This also covers
  `stream_mesh` on a mesh the game already loaded with `load_mesh`: it now
  joins the budget at the next frame's check instead of during the call.
- The `ponko::residency` module is merged into `ponko::streaming`:
  `ponko::residency::{ResidencyKind, ResidencyStats}` →
  `ponko::streaming::{ResidencyKind, ResidencyStats}`. `ponko::stream`
  (`Job`, `CellStreamer`) stays: it holds no assets.
- `UiLayout` is the retained UI's one layout type: `Ui::add(widget, style)`,
  `Ui::add_child(parent, widget, style)`, `Ui::set_layout(id, style)` and
  `UiBuilder::layout(..)` take a `UiLayout` instead of a Taffy `Style`
  (`ui.add(w, Style::default())` → `ui.add(w, UiLayout::auto())`; a raw
  style → `UiLayout::custom(style)`). `Ui::layout_style(id) -> Option<&Style>`
  → `Ui::node_layout(id) -> Option<UiLayout>`. `From<UiLayout> for Style` is
  removed: use `UiLayout::into_style()`.
- `ponko::ui::layout` (the Taffy crate) → `ponko::ui::unstable_taffy`, an
  escape hatch outside the stability promise for what `UiLayout`'s methods
  don't cover (`UiLayout::custom`, `style`, `style_mut`, `into_style`).
- `Ui::take_click()` is removed (it dropped every other queued event):
  `ui.events().filter_map(UiEvent::clicked).last()`, with the new
  `UiEvent::clicked()` accessor. `ui.events()` now takes
  events one by one, so events left untaken stay queued for the frame;
  `ui.next_event()` stays for loops that change the tree. The C and C++
  `take_click` are unchanged.
- `UiWidget` and `UiEvent` are `#[non_exhaustive]`: matches need a `_` arm.
- 2D physics queries: `cast_ray` takes a `QueryFilter` like 3D, and
  `cast_ray_filtered` is removed (`physics.cast_ray(o, d, max)` →
  `physics.cast_ray(o, d, max, d2::QueryFilter::default().with_sensors())`
  for the old sensor-including result; `cast_ray_filtered(..)` →
  `cast_ray(..)`). `overlap_ball` → `overlap_circle`;
  `entities_at(p, out)` → `overlap_point(p, QueryFilter::default().with_sensors(), out)`;
  `entities_in_circle(c, r, out)` → `overlap_circle(c, r, QueryFilter::default().with_sensors(), out)`.
- 3D physics queries: `cast_ray_filtered` → `cast_ray` (same arguments),
  `overlap_ball` → `overlap_sphere`. The C and C++ query functions are
  unchanged.
- `DrawParams` and `TextParams` are `#[non_exhaustive]`: struct literals
  (`DrawParams { rotation: 0.5, ..Default::default() }`) →
  `DrawParams::default().with_rotation(0.5)`. Both gain `with_*` builders
  (`DrawParams::with_scale`, `with_rotation`, `with_color`, `with_z`,
  `with_pivot`, `with_screen`, `with_region`; `TextParams::with_font`,
  `with_size`, `with_color`, `with_z`); their fields stay public to read and
  set. `TileMap` keeps `u16` frame indices (half the memory per tile);
  `SpriteSheet::frame` takes `u32::from(tile)`.
