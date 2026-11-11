### Breaking changes
- The calls that still returned `Result<_, String>` now return `Result<_, ponko::Error>`, so their
  `kind()` can be matched and `?` passes them into a `Box<dyn Error>`. Code that reads the error as
  a `String` (`.contains(..)`, `let s: String = ..`) changes to `e.message()` or `e.to_string()`;
  `?` into a function returning `Result<_, String>` keeps working, since `String: From<ponko::Error>`.
  This ships in the same breaking release as the root-name removal.
  - `Ctx::load_scene`, `Ctx::load_scene_lockstep` (`NotFound`, `Json`, an asset's own error, and
    `Unsupported` for a load refused during a lockstep session) and `Ctx::spawn_prefab`.
  - `Ctx::start_ragdoll` (`NotFound`, `Invalid`).
  - `PerfReport::from_json`, `PerfReport::load` and `Budget::from_json` (`Json`, `NotFound`, `Io`).
  - `MeshData::check` (`Invalid`) and `EcsDiagnostics::matching` (`NotFound`).

### Added
- C: `ponko_last_error_kind()` returns the kind of the last failure on the thread as a
  `PonkoErrorKind` (`PONKO_ERROR_NOT_FOUND`, `_IO`, `_DECODE`, `_SHADER`, `_UNSUPPORTED`,
  `_BAD_HANDLE`, `_JSON`, `_LIMIT`, `_INVALID`, `_OTHER`; `_NONE` before any), next to
  `ponko_last_error()`'s message. Statuses are unchanged. C++: the wrapper throws `ponko::error`, a
  `std::runtime_error` with `kind()`.
