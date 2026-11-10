//! Finding and decoding asset files.
//!
//! Loading a named asset has three steps, kept apart here and in
//! [`Ctx`](crate::Ctx):
//! 1. **Acquire** its bytes by name: from the [`AssetSource`]s a game added,
//!    in order, then from the default source (the `assets/` folders on
//!    desktop, the files fetched from `assets/index.json` in a browser).
//! 2. **Decode** them on the CPU ([`decode_image`]): no GPU, any thread.
//! 3. **Create** the GPU resource on the game thread (`Ctx::create_texture`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A place named assets can be read from, other than the `assets/` folder:
/// bytes embedded in the game, a pack file, a test fixture in memory.
///
/// Add one with [`Ctx::add_asset_source`](crate::Ctx::add_asset_source).
/// Sources are asked in the order they were added, before the default
/// `assets/` search; a source that doesn't have a name says so with an
/// error of kind [`std::io::ErrorKind::NotFound`], and the next one is
/// asked. Any other error stops the search and is reported.
///
/// [`MemorySource`] is a ready-made one.
///
/// # Names, caching and hot reload
///
/// Names are the same logical names the default source uses
/// (`"sprites/crab.png"`, `/`-separated), and scenes and prefabs refer to
/// assets by them. A loaded asset is cached by its source's
/// [`id`](Self::id) and its name, so the same name in two sources is two
/// assets, and loading a name again returns the first handle without
/// reading the source again. This holds for every kind of asset: textures,
/// fonts, data, scenes, prefabs, sounds, streamed music and (with the `3d`
/// feature) meshes, skeletons, animation clips, shaders, screen effects and
/// compute passes. Images a glTF file refers to are shared between models
/// by the source they were found in, so two sources with the same image
/// name give two images. Texture arrays, reflection probe images and
/// [`load_asset_with`](crate::Ctx::load_asset_with) read their source on
/// every call (they aren't cached).
///
/// # In a browser
///
/// Added sources work the same in web builds: they are asked first, and
/// what they have is never fetched. The default source there is the files
/// listed in `assets/index.json`, fetched before setup.
///
/// # Hot reload
///
/// Only files found in the default source on desktop are watched and
/// reloaded when they change (textures, fonts, sounds, data, shaders,
/// effects, compute). Assets that came from an added source are loaded once
/// and never reloaded, whatever the source does later, so a source can keep
/// its bytes in memory without being polled. The same name found in `assets/`
/// before a source was added keeps being watched.
///
/// `read` runs on the game thread, inside the load call that needs it.
pub trait AssetSource: 'static {
    /// A short name for this source, unique among the sources added to a
    /// game: part of every cache key and error message.
    fn id(&self) -> &str;

    /// The bytes of asset `name`, or an error of kind
    /// [`NotFound`](std::io::ErrorKind::NotFound) if this source doesn't
    /// have it.
    ///
    /// # Errors
    ///
    /// `NotFound` lets the next source try; any other error is reported to
    /// the caller of the load.
    fn read(&self, name: &str) -> std::io::Result<Vec<u8>>;

    /// Whether this source has asset `name`, without handing over its
    /// bytes. Used to tell which source a file a model refers to comes
    /// from, so it is shared with what was loaded from that source before
    /// instead of read again.
    ///
    /// The default reads it and checks the error is not
    /// [`NotFound`](std::io::ErrorKind::NotFound); override it when that is
    /// cheaper to answer (an index lookup).
    fn contains(&self, name: &str) -> bool {
        !matches!(self.read(name), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
    }
}

/// An [`AssetSource`] holding its files in memory: assets embedded with
/// `include_bytes!`, generated at startup, or made up in a test.
///
/// ```
/// # use ponko::*;
/// Game::new("t", 1, 1).run_headless(0, |ctx| {
///     let mut pixel = Vec::new();
///     image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
///         .write_to(&mut std::io::Cursor::new(&mut pixel), image::ImageFormat::Png)
///         .unwrap();
///     ctx.add_asset_source(MemorySource::new("embedded").with("red.png", pixel));
///     let red = ctx.load_texture("red.png");
///     assert_eq!((red.width(), red.height()), (1, 1));
/// });
/// ```
#[derive(Clone, Default)]
pub struct MemorySource {
    id: String,
    files: HashMap<String, Arc<[u8]>>,
}

impl MemorySource {
    /// An empty source called `id`.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            files: HashMap::new(),
        }
    }

    /// Adds (or replaces) file `name`, builder style.
    #[must_use]
    pub fn with(mut self, name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.insert(name, bytes);
        self
    }

    /// Adds (or replaces) file `name`.
    pub fn insert(&mut self, name: impl Into<String>, bytes: impl Into<Vec<u8>>) {
        self.files.insert(name.into(), bytes.into().into());
    }
}

impl AssetSource for MemorySource {
    fn id(&self) -> &str {
        &self.id
    }

    fn contains(&self, name: &str) -> bool {
        self.files.contains_key(name)
    }

    fn read(&self, name: &str) -> std::io::Result<Vec<u8>> {
        self.files.get(name).map(|b| b.to_vec()).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("\"{name}\" is not in memory source \"{}\"", self.id),
            )
        })
    }
}

/// Where a loaded asset came from; with its kind, the cache key.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Origin {
    /// The default source: the file [`resolve`] found (on the web, the
    /// fetched copy's name). The only kind hot reload watches.
    File(PathBuf),
    /// An added [`AssetSource`], by its id, and the logical name.
    Source { source: String, name: String },
}

impl Origin {
    /// `name` for error messages, with where it came from.
    pub(crate) fn describe(&self, name: &str) -> String {
        match self {
            Origin::File(_) => format!("\"{name}\""),
            Origin::Source { source, .. } => format!("\"{name}\" (from asset source \"{source}\")"),
        }
    }

    /// Where the asset is, for listings: the file's path, or `source:name`.
    #[cfg_attr(not(feature = "3d"), allow(dead_code))]
    pub(crate) fn display(&self) -> String {
        match self {
            Origin::File(path) => path.display().to_string(),
            Origin::Source { source, name } => format!("{source}:{name}"),
        }
    }
}

/// What [`Sources::lookup`] found for a name.
pub(crate) enum Lookup<T> {
    /// The asset was loaded before: where it came from (3D files need it to
    /// find their skeleton and clips), and what it became.
    #[cfg_attr(not(feature = "3d"), allow(dead_code))]
    Cached(Origin, T),
    /// Not loaded yet.
    Found(Found),
}

/// A named asset that was found: where, and its bytes if reading it was
/// how it was found (added sources).
pub(crate) struct Found {
    pub origin: Origin,
    bytes: Option<Vec<u8>>,
}

impl Found {
    /// The asset's bytes, reading the file now if it came from the default source.
    /// A failed read is [`ErrorKind::Io`](crate::ErrorKind::Io).
    pub(crate) fn into_bytes(self) -> Result<Vec<u8>, crate::Error> {
        match (self.bytes, &self.origin) {
            (Some(bytes), _) => Ok(bytes),
            (None, Origin::File(path)) => read(path).map_err(|e| {
                crate::Error::new(crate::ErrorKind::Io, format!("could not read {}: {e}", path.display()))
            }),
            (None, Origin::Source { .. }) => unreachable!("sources hand over their bytes when found"),
        }
    }

    /// The asset as UTF-8 text; `name` is for messages.
    pub(crate) fn into_text(self, name: &str) -> Result<String, String> {
        let origin = self.origin.clone();
        let bytes = self.into_bytes()?;
        String::from_utf8(bytes).map_err(|e| match &origin {
            Origin::File(path) => format!("{} is not UTF-8 text: {e}", path.display()),
            Origin::Source { .. } => format!("{} is not UTF-8 text: {e}", origin.describe(name)),
        })
    }
}

/// The asset sources a game added, asked before the default one.
#[derive(Default)]
pub(crate) struct Sources {
    list: Vec<Box<dyn AssetSource>>,
}

impl Sources {
    /// Adds `source` after the others.
    ///
    /// # Panics
    /// If its id is empty or another source already has it.
    pub(crate) fn add(&mut self, source: Box<dyn AssetSource>) {
        let id = source.id();
        assert!(!id.is_empty(), "ponko: an asset source needs a non-empty id");
        assert!(
            self.list.iter().all(|s| s.id() != id),
            "ponko: an asset source called \"{id}\" was already added"
        );
        self.list.push(source);
    }

    /// The cache keys `name` would have in each added source, in order.
    pub(crate) fn origins<'a>(&'a self, name: &'a str) -> impl Iterator<Item = Origin> + 'a {
        self.list.iter().map(move |s| Origin::Source {
            source: s.id().to_string(),
            name: name.to_string(),
        })
    }

    /// Finds asset `name` unless `cached` already has it from where it
    /// would be found. Sources are only read for what isn't cached, so
    /// loading a name again never reads a source again.
    ///
    /// # Errors
    /// As [`find`](Self::find).
    pub(crate) fn lookup<T>(
        &self,
        name: &str,
        cached: impl Fn(&Origin) -> Option<T>,
    ) -> Result<Lookup<T>, crate::Error> {
        for origin in self.origins(name) {
            if let Some(value) = cached(&origin) {
                return Ok(Lookup::Cached(origin, value));
            }
        }
        let found = self.find(name)?;
        Ok(match cached(&found.origin) {
            Some(value) => Lookup::Cached(found.origin, value),
            None => Lookup::Found(found),
        })
    }

    /// The bytes of `name` from the first place that has it, for files an
    /// asset refers to (a glTF's images, say).
    ///
    /// # Errors
    /// As [`find`](Self::find), or if reading fails.
    #[cfg_attr(not(feature = "3d"), allow(dead_code))]
    pub(crate) fn read(&self, name: &str) -> Result<Vec<u8>, String> {
        Ok(self.find(name)?.into_bytes()?)
    }

    /// Where asset `name` would be found, without reading it: the first
    /// added source that [has](AssetSource::contains) it, else the default
    /// source's file. `None` if nothing has it.
    #[cfg_attr(not(feature = "3d"), allow(dead_code))]
    pub(crate) fn locate(&self, name: &str) -> Option<Origin> {
        match self.list.iter().find(|s| s.contains(name)) {
            Some(source) => Some(Origin::Source {
                source: source.id().to_string(),
                name: name.to_string(),
            }),
            None => resolve(name).ok().map(Origin::File),
        }
    }

    /// Finds asset `name`: in the added sources in order, then the default one.
    ///
    /// # Errors
    /// If no source has it (naming every place searched), or a source
    /// failed to read it.
    /// Not finding it is [`ErrorKind::NotFound`](crate::ErrorKind::NotFound);
    /// a source that has it but fails to read it is
    /// [`Io`](crate::ErrorKind::Io). The error names `name` as its path.
    pub(crate) fn find(&self, name: &str) -> Result<Found, crate::Error> {
        for source in &self.list {
            match source.read(name) {
                Ok(bytes) => {
                    return Ok(Found {
                        origin: Origin::Source {
                            source: source.id().to_string(),
                            name: name.to_string(),
                        },
                        bytes: Some(bytes),
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    let message = format!("asset source \"{}\" could not read \"{name}\": {e}", source.id());
                    return Err(crate::Error::new(crate::ErrorKind::Io, message).with_path(name));
                }
            }
        }
        match resolve(name) {
            Ok(path) => Ok(Found {
                origin: Origin::File(path),
                bytes: None,
            }),
            Err(e) if self.list.is_empty() => Err(crate::Error::not_found(name, e)),
            Err(e) => {
                let ids: Vec<String> = self.list.iter().map(|s| format!("\"{}\"", s.id())).collect();
                let message = format!("{e}\nAsset sources searched first: {}", ids.join(", "));
                Err(crate::Error::not_found(name, message))
            }
        }
    }
}

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
    fn sources_are_asked_in_order_and_say_not_found() {
        let mut sources = Sources::default();
        sources.add(Box::new(MemorySource::new("a").with("x", *b"from a")));
        sources.add(Box::new(
            MemorySource::new("b").with("x", *b"from b").with("y", *b"only b"),
        ));
        let x = sources.find("x").unwrap();
        assert_eq!(
            x.origin,
            Origin::Source {
                source: "a".into(),
                name: "x".into()
            }
        );
        assert_eq!(x.into_bytes().unwrap(), b"from a");
        assert_eq!(sources.find("y").unwrap().into_text("y").unwrap(), "only b");
        let missing = MemorySource::new("a").read("z").unwrap_err();
        assert_eq!(missing.kind(), std::io::ErrorKind::NotFound);
        let err = sources.find("definitely-not-here.png").err().unwrap();
        assert_eq!(err.kind(), crate::ErrorKind::NotFound);
        assert!(
            err.message().ends_with("Asset sources searched first: \"a\", \"b\""),
            "{err}"
        );
    }

    /// A source that only implements `read`: `contains` falls back to it.
    struct ReadOnly;
    impl AssetSource for ReadOnly {
        fn id(&self) -> &str {
            "read-only"
        }
        fn read(&self, name: &str) -> std::io::Result<Vec<u8>> {
            match name {
                "here" => Ok(b"x".to_vec()),
                "broken" => Err(std::io::Error::other("disk on fire")),
                _ => Err(std::io::ErrorKind::NotFound.into()),
            }
        }
    }

    #[test]
    fn locate_names_the_first_source_that_has_a_name_without_reading_memory() {
        let mut sources = Sources::default();
        sources.add(Box::new(ReadOnly));
        sources.add(Box::new(
            MemorySource::new("mem").with("here", *b"y").with("only-mem", *b"z"),
        ));
        let in_source = |source: &str, name: &str| Origin::Source {
            source: source.into(),
            name: name.into(),
        };
        assert_eq!(sources.locate("here"), Some(in_source("read-only", "here")));
        assert_eq!(sources.locate("only-mem"), Some(in_source("mem", "only-mem")));
        // A source that has it but fails to read it still has it: the
        // load reports the error.
        assert_eq!(sources.locate("broken"), Some(in_source("read-only", "broken")));
        assert_eq!(sources.locate("definitely-not-here.png"), None);
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
