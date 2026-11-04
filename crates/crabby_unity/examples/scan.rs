//! Parses every Unity text asset of a project and reports what failed:
//!
//!   cargo run --release -p crabby_unity --example scan -- <project folder>

use std::path::{Path, PathBuf};
use std::time::Instant;

use crabby_unity::{Project, UnityFile, UnityMesh};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e, "unity" | "prefab" | "mat" | "asset" | "controller" | "anim"))
        {
            out.push(path);
        }
    }
}

fn main() {
    let root = std::env::args().nth(1).expect("usage: scan <project folder>");
    let start = Instant::now();
    let project = Project::open(&root).unwrap_or_else(|e| panic!("{e}"));
    println!("{} GUIDs in {:.2} s", project.len(), start.elapsed().as_secs_f64());
    let mut paths = Vec::new();
    files(&Path::new(&root).join("Assets"), &mut paths);
    let (mut ok, mut binary, mut bytes, mut objects, mut meshes, mut vertices) = (0, 0, 0u64, 0, 0, 0);
    let mut failed = Vec::new();
    let start = Instant::now();
    for path in &paths {
        let Ok(data) = std::fs::read(path) else { continue };
        let Ok(text) = std::str::from_utf8(&data) else {
            binary += 1;
            continue;
        };
        if !text.starts_with("%YAML") {
            binary += 1;
            continue;
        }
        bytes += data.len() as u64;
        match UnityFile::parse(text) {
            Ok(file) => {
                ok += 1;
                objects += file.objects.len();
                for o in file.of_class(43) {
                    match UnityMesh::from_object(o) {
                        Ok(m) => {
                            meshes += 1;
                            vertices += m.positions.len();
                        }
                        Err(e) => failed.push(format!("{}: {e}", path.display())),
                    }
                }
            }
            Err(e) => failed.push(format!("{}: {e}", path.display())),
        }
    }
    let secs = start.elapsed().as_secs_f64();
    println!(
        "{ok} text files ({:.0} MB, {objects} objects) parsed in {secs:.2} s ({:.0} MB/s); {binary} binary skipped",
        bytes as f64 / 1e6,
        bytes as f64 / 1e6 / secs
    );
    println!("{meshes} meshes decoded, {vertices} vertices");
    println!("{} failed", failed.len());
    for f in failed.iter().take(20) {
        println!("  {f}");
    }
}
