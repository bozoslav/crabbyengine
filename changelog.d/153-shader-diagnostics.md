### Added
- C and C++ bindings for shader status: `ponko_ctx_shader_status_count`,
  `ponko_ctx_shader_status` (`PonkoShaderStatus`, `PONKO_SHADER_*`
  kinds and outcomes), `ponko_ctx_shader_status_name`,
  `ponko_ctx_shader_status_text` and `ponko_ctx_shader_status_json`
  (`ponko.h` spellings over the version-2 ABI); C++ `ctx::shader_status()` and
  `ctx::shader_status_json()`.
- The guide's "Shader errors and reloads" section: what the compiler and
  the GPU check on each platform, and what isn't caught.

### Changed
- With hot reload on (the default in debug builds, `Game::hot_reload`,
  `PONKO_HOT_RELOAD`), a shader, screen effect or compute pass file that
  doesn't compile at load no longer fails: `load_shader`, `load_effect` and
  `load_compute` don't panic, and `try_load_shader`, `try_load_effect`,
  `try_load_compute` and the C `ponko_ctx_load_shader`,
  `ponko_ctx_load_effect` and `ponko_ctx_load_compute` return OK with a
  handle. The handle uses a stand-in (the built-in look, the frame as it
  is, or a skipped pass), is reported as `ShaderOutcome::Failed` with the
  error, stays watched, and loads when a fix is saved. Code that relied on
  that error to detect a broken file should check `Graphics::shader_status`.
  With hot reload off (release builds by default) it is an error as before.
- Screen effect and compute pass errors now have the shader's line and
  column, its kind and variant (they used the compiler's text before),
  and their pipelines are built in a GPU error scope: a pipeline the GPU
  rejects is an error, and a failed reload keeps the last good one.
- In a browser with WebGPU, which answers GPU error scopes a frame or so
  later, shader pipelines the GPU rejects are replaced by the previous
  ones (or a stand-in) on the next frame and reported in
  `Graphics::shader_status`. Before, the web only caught compiler errors.

### Fixed
- A failed reload of a shader file that never compiled stays `Failed`
  instead of `ReloadFailed` (there is no good version to keep).
