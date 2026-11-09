//! Helpers shared by the integration tests.
//!
//! Not every test binary uses every helper.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// A file under the crate's `assets/` folder that is deleted when the value
/// is dropped, pass or fail.
///
/// The engine resolves scene, prefab and JSON names against `assets/`, so
/// tests that save or load files have to write there. A fixed name would
/// collide across processes: two `cargo test` runs in one checkout (an IDE
/// runner next to a terminal, two agents sharing a tree) would delete each
/// other's files. [`TempAsset::new`] therefore makes the name unique to the
/// process and the call: `test-<pid>-<n>-<name>`.
///
/// Tests keep asserting on the content of what they saved, not on the name
/// of the temporary file.
pub struct TempAsset {
    name: String,
}

impl TempAsset {
    /// A unique name for `name` (for example `scene.json`). Nothing is
    /// written until [`TempAsset::write`] or the code under test saves to it.
    pub fn new(name: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = name.strip_prefix("test-").unwrap_or(name);
        TempAsset {
            name: format!("test-{}-{n}-{stem}", std::process::id()),
        }
    }

    /// Like [`TempAsset::new`], with `contents` written to the file.
    pub fn with_contents(name: &str, contents: &str) -> Self {
        let file = Self::new(name);
        file.write(contents);
        file
    }

    /// The asset name: what `load_scene`, `save_prefab` and friends take.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The file's path on disk.
    pub fn path(&self) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join(&self.name)
    }

    /// Writes `contents` to the file, replacing it.
    pub fn write(&self, contents: &str) {
        std::fs::write(self.path(), contents).unwrap();
    }

    /// Reads the file as text.
    pub fn read(&self) -> String {
        std::fs::read_to_string(self.path()).unwrap()
    }
}

impl Drop for TempAsset {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.path());
    }
}
