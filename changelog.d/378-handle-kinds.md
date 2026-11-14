### Breaking changes
- `ponko_ffi::PonkoHandleKind` has two new values,
  `PONKO_HANDLE_SPRITE_SHADER` and `PONKO_HANDLE_SPRITE_MATERIAL`, so a Rust
  `match` over every value no longer compiles. C and C++ code is not affected.
  Instead: add arms for the two new values, or a `_` arm.
  Before:
  ```rust
  match kind {
      PonkoHandleKind::PONKO_HANDLE_TEXTURE => free_texture(id),
      // ... one arm per value up to 0.1.0
  }
  ```
  After:
  ```rust
  match kind {
      PonkoHandleKind::PONKO_HANDLE_TEXTURE => free_texture(id),
      // ... the 0.1.0 arms
      PonkoHandleKind::PONKO_HANDLE_SPRITE_SHADER
      | PonkoHandleKind::PONKO_HANDLE_SPRITE_MATERIAL => {}
  }
  ```
