### Changed
- The immediate widgets (`ctx.ui.immediate`) follow the screen UI's scale:
  `Ui::set_scale` (C: `ponko_ctx_ui_set_scale`) now scales their positions,
  sizes, text, the button outline and hit testing like the retained tree's,
  so a debug overlay is as large as the menu next to it. The widgets' units
  are the tree's layout units; at the default scale of 1 nothing moves. Only
  a game that called `set_scale` and drew immediate widgets sees them change
  (they were drawn unscaled before).

### Added
- The screen's theme styles the immediate widgets' font and borders too:
  `UiImmediate::font` follows `UiTheme::font`, and `UiImmediate::border_width`
  (a border on buttons, in the accent color) follows `UiTheme::border_width`.
  Both follow `Ui::set_theme` by the same rule as the colors (a value set by
  hand stays), and both default to what the widgets drew before.
- `UiImmediate::scale()` reads the scale the widgets are drawn at.
