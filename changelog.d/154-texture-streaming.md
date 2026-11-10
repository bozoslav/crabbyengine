### Added
- Asset streaming without gameplay-thread stalls: `Ctx::stream_texture` and
  `Ctx::stream_mesh` decode on job threads and return `None` until the asset
  is uploaded, at the start of a frame, within `Graphics::set_upload_budget`
  (16 MiB a frame by default), in the order assets were asked for. Textures
  arrive as a stand-in of at most 32 pixels a side and double in resolution
  each upload under the same handle.
- `AssetState` for every streamed or resident asset (`Graphics::asset_state`,
  `asset_error`, `streamed_assets`); `Graphics::set_deterministic_uploads` makes
  every state change in the same frame each run; `Ctx::finish_uploads` and
  `Graphics::upload_stats` (`UploadStats`).
- Texture residency budgets: `ResidencyKind::Texture`, `Ctx::resident_texture`,
  `try_resident_texture`, `pin_resident_texture`, `unpin_resident_texture`.
  Over budget, the least recently used textures drop to their stand-in and
  stream back under the same handle; streaming stops at the resolution that
  fits.
- `StreamedLod`: the levels of a `LodGroup` stream in from files as the
  camera needs them, showing the closest coarser level meanwhile.
- `perf.get` reports `streaming`: upload counters and each asset's state.
- C and C++: `ponko_ctx_stream_texture`, `stream_mesh`, `asset_state`,
  `asset_error`, `set_residency_budget`, `residency`, `is_resident`,
  `resident_texture`, `resident_mesh`, `unpin_resident`, `set_upload_budget`,
  `set_deterministic_uploads`, `upload_stats`, `finish_uploads`,
  `set_streamed_lod`, with `ponko::ctx` methods.
- Guide chapter "Streaming: memory budgets and loading without waiting", and
  the `streaming/` benchmark.

### Changed
- Residency budgets are also checked at the start of every frame, so what
  entities stop showing is evicted without waiting for the next load.
  Assets asked for in this or the previous frame or tick are kept (before,
  only the current frame and tick), so what a game asks for every frame or
  every fixed-step tick is never evicted between two asks.
- Freeing an evicted resident mesh's kept handle releases its id.
- A resident mesh evicted by its budget keeps its id for its file: a handle
  the game kept reads as not live and comes back under the same id, instead
  of possibly naming another mesh.
