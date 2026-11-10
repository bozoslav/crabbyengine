### Added
- `ActionMap` reads back: `get`, `iter` (in name order), `len` and `is_empty`,
  with `Action::buttons`, `axes` and `axes2` slices and `PartialEq` on `Action`
  and the binding types. A rebinding screen can show `ctx.input.actions()`
  instead of keeping its own copy. Rust-only for now.
- `Input::set_action` replaces one installed action in place: its `ActionId`
  stays valid, the other actions keep their state, and held sources are the
  baseline, so the change fires no edge.

### Changed
- The complete-game example's settings screen shows the bindings it reads back
  from the installed map and rebinds one action in place. The saved controls
  are read back from the map instead of being kept beside it.
