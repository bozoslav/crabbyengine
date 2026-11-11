### Fixed

- CI: the packaged-game check accepts the engine's own crates as
  dev-dependencies (third-party ones must still be in the notices), and the
  macOS accessibility test no longer stops `cargo test --test '*'` on builds
  without the `accessibility` feature.
