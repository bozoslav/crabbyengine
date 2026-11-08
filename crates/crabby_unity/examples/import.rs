//! Imports Unity scenes or prefabs into a folder of a Crabby game's assets:
//!
//!   cargo run --release -p ponko_unity --example import -- \
//!       <Unity project> <out folder> <its asset name> <scene or prefab>...
//!
//! For example `... MyUnityGame my_game/assets/town town/ Assets/Scenes/Town.unity`
//! writes `my_game/assets/town/Town.json` (load it with
//! `ctx.load_scene("town/Town.json")`), the meshes in `town/meshes/` and
//! their textures in `town/meshes/textures/`, then says what it left out.
//! Scenes with the same file name are told apart by their folders.
//! `ONE=name.json` puts every scene's entities into that one scene instead.

use std::path::Path;
use std::time::Instant;

use ponko_unity::{Hierarchy, Importer, Project, UnityFile};

/// The scene's output name: its file name, or with the folders that tell it
/// apart from the others when they share one.
fn out_name(scene: &str, all: &[String]) -> String {
    let stem = |s: &str| Path::new(s).file_stem().unwrap().to_string_lossy().into_owned();
    if all.iter().filter(|s| stem(s) == stem(scene)).count() == 1 {
        return format!("{}.json", stem(scene));
    }
    // Drop the folders every scene shares.
    let parts = |s: &str| s.split(['/', '\\']).map(str::to_owned).collect::<Vec<_>>();
    let shared = all
        .iter()
        .map(|s| parts(s))
        .reduce(|a, b| {
            a.into_iter()
                .zip(b)
                .take_while(|(x, y)| x == y)
                .map(|(x, _)| x)
                .collect()
        })
        .map_or(0, |p| p.len());
    let mut p = parts(scene);
    let last = p.len() - 1;
    p[last] = stem(scene);
    format!("{}.json", p[shared.min(last)..].join("_"))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [project, out, prefix, scenes @ ..] = &args[..] else {
        eprintln!("usage: import <Unity project> <out folder> <its asset name> <scene or prefab>...");
        std::process::exit(2);
    };
    let one = std::env::var("ONE").ok();
    let start = Instant::now();
    let project = Project::open(project).unwrap_or_else(|e| panic!("{e}"));
    let mut import = Importer::new(&project, out, prefix);
    std::fs::create_dir_all(out).unwrap();
    let mut merged: Vec<serde_json::Value> = Vec::new();
    for scene in scenes {
        let file = UnityFile::read(project.root().join(scene)).unwrap_or_else(|e| panic!("{scene}: {e}"));
        let hierarchy = Hierarchy::build(&file, &project).unwrap_or_else(|e| panic!("{scene}: {e}"));
        let mut world = import.scene(&hierarchy).unwrap_or_else(|e| panic!("{scene}: {e}"));
        let entities = world["entities"].as_array().map_or(0, Vec::len);
        if one.is_some() {
            for mut e in world["entities"].as_array_mut().map(std::mem::take).unwrap_or_default() {
                e["entity"] = merged.len().into();
                merged.push(e);
            }
            continue;
        }
        let at = Path::new(out).join(out_name(scene, scenes));
        std::fs::write(&at, serde_json::to_string_pretty(&world).unwrap()).unwrap();
        println!(
            "{scene}: {} objects, {entities} entities -> {}",
            hierarchy.objects.len(),
            at.display()
        );
    }
    if let Some(name) = &one {
        let at = Path::new(out).join(name);
        let world = serde_json::json!({
            "format": "crabby-world",
            "version": 1,
            "entities": merged,
            "resources": {},
        });
        std::fs::write(&at, serde_json::to_string(&world).unwrap()).unwrap();
        println!("{} scenes -> {}", scenes.len(), at.display());
    }
    let report = import.finish().unwrap_or_else(|e| panic!("{e}"));
    println!(
        "{} entities, {} meshes, {} textures in {:.1} s",
        report.entities,
        report.meshes,
        report.textures,
        start.elapsed().as_secs_f32()
    );
    println!("left out:");
    for (why, n) in &report.skipped {
        println!("  {n:6}  {why}");
    }
    println!("shaders (renderers using each):");
    let mut shaders: Vec<_> = report.shaders.iter().collect();
    shaders.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    for (shader, n) in shaders {
        println!("  {n:6}  {shader}");
    }
}
