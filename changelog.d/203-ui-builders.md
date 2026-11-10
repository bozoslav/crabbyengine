### Added
- Retained UI builders: `Ui::root`, `panel`, `label`, `button`, `image`,
  `scroll_area`, `text_input`, `toggle` and `slider` return a `UiBuilder`
  that sets layout, text, paint, border width, enabled state and value in
  one expression and ends with `.id()`.
- `UiLayout` presets for the common layouts (`auto`, `row`, `column`,
  `fill`, `fixed`, `anchored`, `centered`, `grid`), with `gap`, `padding`,
  `grow`, direction and size modifiers; they convert into
  `ui_layout::Style`, and `UiLayout::custom` keeps Taffy's full style.
- `UiTheme` on the retained tree (`Ui::theme`, `Ui::set_theme`): new nodes
  take their paint from it, and changing it restyles every paint field
  still equal to the old theme's value, so fields set on a node stay
  overrides. The default theme draws what the old `UiPaint` defaults drew.
  `Ui::paint` reads a node's resolved paint.
- Retained UI titles: `Ui::title(parent, text)` and `UiTheme::title` /
  `title_size`; titles follow the theme like other nodes.
- Nodes found by name: `UiBuilder::name`, `Ui::set_name`, `Ui::find`,
  `Ui::name`, so update systems need no resource of ids. `Ui::take_click`
  returns the last clicked node for screens that only react to clicks.
- `UiLayout::top` and `UiLayout::left` place a preset in points (a centered
  dialog near the top).
- C and C++: layout presets (`ponko_ui_layout_row`, `_column`, `_fill`,
  `_fixed`, `_anchored`, `_centered`; C++ `ui::row`, `ui::fill`, ... with
  `.top` / `.left`), the theme (`PonkoUiTheme`, `ponko_ui_theme_default`,
  `ponko_ctx_ui_theme`, `ponko_ctx_ui_set_theme`), titles
  (`ponko_ctx_ui_add_title`), names (`ponko_ctx_ui_set_name`, `_find`,
  `_name`), `ponko_ctx_ui_take_click` and `ponko_ctx_ui_paint`, and the matching C++
  `ui::tree` methods.

### Changed
- One theme for the screen's UI: `Ui::set_theme` also restyles the
  immediate widgets (`ctx.ui.immediate.style`), keeping fields set there by
  hand; `UiTheme::style()` is the style it gives them.
- The guide's UI chapters use the builders, presets and theme, and say
  which UI to use: the retained tree for game UI, the immediate `ui_*`
  calls for debug overlays and prototypes.
- The `ui_shop` and `ui_panel` examples are built with the builders, presets
  and names, at half their old length; an offscreen GPU test checks they draw
  the same pixels as the hand-built trees they replace.

### Fixed
- A screen root's `auto` margins now take the window's free space (both
  sides center it), as they do for a child; before, they were ignored.
