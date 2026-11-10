### Added
- `Ctx::try_load_stream`: `load_stream` with errors as values. Streamed music
  now comes from added asset sources too, kept in memory still encoded and
  decoded as it plays.
- `Ctx::scene_asset_field::<T>(field, load)`: a reflected component of your
  own can name assets (a name or a list of names in a field); scenes,
  prefabs, the editor and the control endpoint load them first, through the
  asset sources, and fail before any entity changes if one can't load.
- `AssetSource::contains` (a default method): whether a source has a name
  without handing over its bytes; `MemorySource` answers from its index.

### Changed
- Texture arrays and reflection probe images load through the added asset
  sources too.

### Fixed
- Images a glTF file refers to are shared by the source they were found in,
  not by name: a model from one source no longer shows another source's (or
  the `assets/` folder's) image of the same name.
- `load_stream` in web builds reads the fetched copy of the file instead of
  failing.
