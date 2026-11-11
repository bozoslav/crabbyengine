### Added

- Retained UI accessibility: `Ui::set_accessible_description` and
  `Ui::set_live_region` (builders `.accessible_description(..)`,
  `.live_region(..)`), text inputs' caret and selection in the snapshot, and
  `UiAccessTracker`, which reports only the nodes that changed since the last
  look and allocates nothing while the UI is unchanged.
- Requests from assistive technology: `Ui::push_access_action` with
  `UiAccessAction` (`Focus`, `Activate`, `Increment`, `Decrement`,
  `SetValue`, `SetText`). The next `Ui::frame` applies them as the matching
  key would, with the same events.
- Cargo feature `accessibility` (off by default): on macOS the engine exposes
  `ctx.ui` to `NSAccessibility` through AccessKit, so VoiceOver reads and
  operates it. `UiAccessAdapter` does the same for a host with its own view.
  Windows, Linux and the web have no adapter yet; the feature builds there
  and does nothing.
- C and C++: `ponko_ctx_ui_set_accessible_name`, `_set_accessible_description`,
  `_set_live_region` and their getters, `ponko_ctx_ui_access_action`,
  `ponko_ctx_ui_access_text` and the `ponko_ui_access_*` tracker
  (`ponko::ui::access` in C++); `ponko_ffi` has the same `accessibility`
  feature.
- `examples/ui_menu.rs`, a small settings menu for trying a screen reader,
  and the guide's "Accessibility semantics" section covers all of the above,
  the supported adapters and their limits.
- Checked on macOS in a real window three ways, with the records in the
  guide's "Screen reader records" page: read and operated from another
  process through the system accessibility API, a VoiceOver pass with its
  speech recorded, and a test that runs the engine's own window
  (`PONKO_ACCESS_WINDOW=1`, in `tests/ui_access_macos.rs`).

### Fixed

- A game read by a screen reader answered one accessibility question per
  frame, so VoiceOver reported it as not responding. While it is being
  read, the engine now leaves the event loop idle for 4 ms between
  vsync-paced frames (a read of the whole window: 1.7 to 1.9 s before, 15 ms
  after).
