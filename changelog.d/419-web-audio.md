### Added
- `Audio::unlocked()`: whether the game can be heard yet. Always `true` on the
  desktop; in a browser `false` until the player first clicks, taps or presses
  a key, so a game can show "click to start".

### Changed
- In a browser, the audio output opens at the player's first click, tap or key
  press. Looping sounds (music, ambience) played before it wait and start then,
  with their volume, bus, effects and 3D placement, instead of being lost;
  one-shots played before it are dropped. Desktop behaviour is unchanged.
- In a browser, `load_stream` keeps the bytes encoded and decodes them a packet
  at a time as they play, instead of decoding the whole file when loaded: a
  three-minute stereo track holds its encoded size plus at most 1.6 MB instead
  of about 64 MB.
