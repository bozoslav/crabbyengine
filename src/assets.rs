//! Finding and decoding asset files.

use std::path::{Path, PathBuf};

/// Folders we look in for `assets/`, in order:
/// 1. next to `Cargo.toml` when started with `cargo run` (`CARGO_MANIFEST_DIR`)
/// 2. next to the executable (shipped games)
/// 3. the current working directory
fn asset_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        roots.push(PathBuf::from(dir).join("assets"));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        roots.push(dir.join("assets"));
    }
    if let Ok(dir) = std::env::current_dir() {
        roots.push(dir.join("assets"));
    }
    roots.dedup();
    roots
}

/// Resolves `name` (relative to `assets/`) to an existing file.
pub(crate) fn resolve(name: &str) -> Result<PathBuf, String> {
    let roots = asset_roots();
    for root in &roots {
        let candidate = root.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    let searched: Vec<String> = roots.iter().map(|r| format!("  {}", r.display())).collect();
    Err(format!(
        "\"{name}\" not found in assets/. Looked in:\n{}",
        searched.join("\n")
    ))
}

/// Decoded RGBA8 image.
pub(crate) struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub(crate) fn load_image(name: &str) -> Result<Image, String> {
    let path = resolve(name)?;
    let bytes =
        std::fs::read(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    decode_image(&bytes).map_err(|e| format!("\"{name}\" is not a valid image: {e}"))
}

pub(crate) fn decode_image(bytes: &[u8]) -> Result<Image, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let rgba = img.into_rgba8();
    Ok(Image {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_asset_error_names_file_and_folders() {
        let err = resolve("definitely-not-here.png").unwrap_err();
        assert!(err.contains("\"definitely-not-here.png\" not found in assets/"));
        assert!(err.contains("assets"));
    }

    #[test]
    fn garbage_bytes_are_rejected() {
        assert!(decode_image(b"not a png").is_err());
    }

    #[test]
    fn decodes_png() {
        // 1x1 red pixel PNG, encoded in the test so no asset file is needed.
        let mut png = Vec::new();
        image::RgbaImage::from_raw(1, 1, vec![255, 0, 0, 255])
            .unwrap()
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let img = decode_image(&png).unwrap();
        assert_eq!((img.width, img.height), (1, 1));
        assert_eq!(img.rgba, [255, 0, 0, 255]);
    }
}
