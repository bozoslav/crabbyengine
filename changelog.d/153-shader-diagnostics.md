### Added
- C and C++ bindings for shader status: `crabby_ctx_shader_status_count`,
  `crabby_ctx_shader_status` (`CrabbyShaderStatus`, `CRABBY_SHADER_*`
  kinds and outcomes), `crabby_ctx_shader_status_name`,
  `crabby_ctx_shader_status_text` and `crabby_ctx_shader_status_json`
  (and `ponko_*` spellings); C++ `ctx::shader_status()` and
  `ctx::shader_status_json()`.
- The guide's "Shader errors and reloads" section: what the compiler and
  the GPU check on each platform, and what isn't caught.

### Changed
- With hot reload on, a shader, screen effect or compute pass file that
  doesn't compile at load no longer panics: it gets its handle and a
  stand-in (the built-in look, the frame as it is, or a skipped pass), is
  reported as `ShaderOutcome::Failed`, stays watched, and loads when a fix
  is saved. Without hot reload it is an error as before.
- Screen effect and compute pass errors now have the shader's line and
  column, its kind and variant (they used the compiler's text before),
  and their pipelines are built in a GPU error scope: a pipeline the GPU
  rejects is an error, and a failed reload keeps the last good one.
- In a browser with WebGPU, which answers GPU error scopes a frame or so
  later, shader pipelines the GPU rejects are replaced by the previous
  ones (or a stand-in) on the next frame and reported in
  `Ctx::shader_status`. Before, the web only caught compiler errors.

### Fixed
- A failed reload of a shader file that never compiled stays `Failed`
  instead of `ReloadFailed` (there is no good version to keep).
