### Added
- Image-format cargo features that no longer depend on `3d`: `image-jpeg`
  (on by default), and opt-in `image-webp`, `image-bmp`, `image-tga`,
  `image-gif`, `image-tiff`, `image-ico`, `image-qoi`, `image-pnm`,
  `image-dds`, `image-hdr`, `image-exr`, and `image-formats` for all of them.
  `3d` turns on `image-jpeg`, `image-hdr` and `image-exr`.

### Changed
- `load_texture` and the other texture loaders find an image's format from
  its first bytes, not its extension (TGA, which has none, from the
  extension), so a JPEG named `.png` loads.
- A file in a format the build leaves out now fails with an
  `ErrorKind::Unsupported` error that names the `image-*` feature to turn on,
  instead of a generic decode failure.
- Builds without default features no longer include a JPEG decoder;
  `scripts/feature-deps.sh` checks it.

### Fixed
- A 2D game could not load a JPEG unless it also enabled the `3d` feature.
