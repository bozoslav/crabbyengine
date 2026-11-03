//! Finding and decoding asset files.

use std::path::{Path, PathBuf};

/// Folders we look in for `assets/`, in order:
/// 1. next to `Cargo.toml` when started with `cargo run` (`CARGO_MANIFEST_DIR`)
/// 2. next to the executable (shipped games)
/// 3. the current working directory
#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
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

/// Reads an asset file found by [`resolve`].
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read(path: &Path) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

/// Writes `name` (relative to `assets/`): over the file where [`resolve`]
/// finds it, else into the first `assets/` folder that exists (or the first
/// one, created). Returns where it went.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn write(name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let path = match resolve(name) {
        Ok(existing) => existing,
        Err(_) => {
            let roots = asset_roots();
            let root = roots
                .iter()
                .find(|r| r.is_dir())
                .or(roots.first())
                .ok_or("no assets/ folder")?;
            root.join(name)
        }
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    }
    std::fs::write(&path, bytes).map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(path)
}

/// Files in folder `dir` of `assets/` ending in `.{extension}`, as asset
/// names (`"prefabs/palm.json"`), sorted, from every assets folder found.
#[cfg(all(not(target_arch = "wasm32"), feature = "inspector"))]
pub(crate) fn list(dir: &str, extension: &str) -> Vec<String> {
    let mut names = Vec::new();
    for root in asset_roots() {
        let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|e| e == extension)
                && let Some(file) = path.file_name().and_then(|f| f.to_str())
            {
                names.push(format!("{dir}/{file}"));
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// Web builds have no folders to list.
#[cfg(all(target_arch = "wasm32", feature = "inspector"))]
pub(crate) fn list(_dir: &str, _extension: &str) -> Vec<String> {
    Vec::new()
}

/// Web builds can't write files.
#[cfg(target_arch = "wasm32")]
pub(crate) fn write(name: &str, _: &[u8]) -> Result<PathBuf, String> {
    Err(format!("can't save \"{name}\": web builds have no files"))
}

/// Reads an asset file found by [`resolve`] as UTF-8 text.
pub(crate) fn read_to_string(path: &Path) -> Result<String, String> {
    let bytes = read(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    String::from_utf8(bytes).map_err(|e| format!("{} is not UTF-8 text: {e}", path.display()))
}

// In a browser there are no files: the web build fetches everything listed
// in `assets/index.json` before setup runs (see `crate::web`), and asset
// loading reads those copies.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static WEB_FILES: std::cell::RefCell<std::collections::HashMap<String, std::rc::Rc<[u8]>>> =
        Default::default();
}

/// Stores a fetched asset for later loads (web builds).
#[cfg(target_arch = "wasm32")]
pub(crate) fn insert_web_file(name: String, bytes: Vec<u8>) {
    WEB_FILES.with(|files| files.borrow_mut().insert(name, bytes.into()));
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn resolve(name: &str) -> Result<PathBuf, String> {
    if WEB_FILES.with(|files| files.borrow().contains_key(name)) {
        Ok(PathBuf::from(name))
    } else {
        Err(format!(
            "\"{name}\" not found in assets/. Web builds load the files listed in assets/index.json; \
             is it listed there?"
        ))
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn read(path: &Path) -> std::io::Result<Vec<u8>> {
    let name = path.to_string_lossy();
    WEB_FILES
        .with(|files| files.borrow().get(name.as_ref()).map(|b| b.to_vec()))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, format!("{name} was not fetched")))
}

/// Decoded RGBA8 image.
pub(crate) struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Reads and decodes the image at `path`; `name` is for messages.
pub(crate) fn load_image_at(path: &Path, name: &str) -> Result<Image, String> {
    let bytes = read(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
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
