### Added
- `Audio::try_load_stream` (`ctx.audio.try_load_stream`): `load_stream` with
  errors as values. Streamed music
  now comes from added asset sources too, kept in memory still encoded and
  decoded as it plays.
- `Assets::scene_asset_field::<T>(field, load)` (on `ctx.assets`): a reflected component of your
  own can name assets (a name or a list of names in a field); scenes,
  prefabs, the editor and the control endpoint load them first, through the
  asset sources, and fail before any entity changes if one can't load.
- `AssetSource::contains` (a default method): whether a source has a name
  without handing over its bytes; `MemorySource` answers from its index.

### Changed
- Texture arrays and reflection probe images load through the added asset
  sources too.
- The panic text of `load_stream` for a missing or broken file changed: it now
  prints the error's own text.

### Fixed
- Images a glTF file refers to are shared by the source they were found in,
  not by name: a model from one source no longer shows another source's (or
  the `assets/` folder's) image of the same name.
- `load_stream` in web builds reads the fetched copy of the file instead of
  failing.
