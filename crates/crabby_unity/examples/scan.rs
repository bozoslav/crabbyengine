//! Parses every Unity text asset of a project and reports what failed:
//!
//!   cargo run --release -p crabby_unity --example scan -- <project folder>

use std::path::{Path, PathBuf};
use std::time::Instant;

use crabby_unity::{Project, UnityFile, UnityMesh, Value};

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

/// Counts references to other files: (resolved, unresolved).
fn refs(v: &Value, project: &Project, counts: &mut (usize, usize)) {
    match v {
        Value::Map(m) => {
            if let Some(r) = v.obj_ref()
                && let Some(g) = &r.guid
                && !r.is_none()
                && g.chars().any(|c| c != '0')
            {
                // Built-in resources (0000000000000000e000..., f000...) have no .meta.
                if project.path_of(g).is_some() {
                    counts.0 += 1;
                } else if !g.starts_with("0000000000000000") {
                    counts.1 += 1;
                }
            }
            for (_, c) in m {
                refs(c, project, counts);
            }
        }
        Value::Seq(s) => s.iter().for_each(|c| refs(c, project, counts)),
        _ => {}
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
    let mut links = (0usize, 0usize);
    let start = Instant::now();
    for path in &paths {
        let Ok(data) = std::fs::read(path) else { continue };
        bytes += data.len() as u64;
        let is_text = data.starts_with(b"%YAML");
        if !is_text {
            binary += 1;
        }
        match UnityFile::read(path) {
            Ok(file) => {
                ok += 1;
                objects += file.objects.len();
                for o in &file.objects {
                    refs(&o.body, &project, &mut links);
                }
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
        "{ok} files ({:.0} MB, {objects} objects; {binary} of them binary) parsed in {secs:.2} s ({:.0} MB/s)",
        bytes as f64 / 1e6,
        bytes as f64 / 1e6 / secs
    );
    println!("{meshes} meshes decoded, {vertices} vertices");
    println!("references to other files: {} resolved, {} not", links.0, links.1);
    println!("{} failed", failed.len());
    for f in failed.iter().take(20) {
        println!("  {f}");
    }
}
