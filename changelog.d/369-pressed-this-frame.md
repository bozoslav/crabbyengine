### Added
- `Input::pressed_this_frame()` lists the keys, mouse buttons, gamepad
  buttons and gamepad axes that went down this frame as `PressedInput`, which
  wraps the `ButtonBinding` or `AxisBinding` an `ActionMap` accepts, so a
  rebinding screen can bind the answer directly. Axes count when they cross
  `PRESS_THRESHOLD` (0.5) during the frame; `pressed_this_frame_with_threshold`
  takes another. C: `ponko_ctx_pressed_this_frame`; C++:
  `ctx::pressed_this_frame()` and `ponko::pressed_input`.

### Changed
- The reference game's rebinding prompt reads `pressed_this_frame()` instead of
  polling a list of keys and buttons.
