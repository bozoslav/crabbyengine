//! Asset sources for 3D files: meshes, shaders, screen effects and compute
//! passes load by name from memory, with the same caching and messages as
//! files from `assets/`.
#![cfg(feature = "3d")]

use std::cell::Cell;
use std::rc::Rc;

use ponko::*;

const HOUSE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/house.glb"));
const SURFACE: &str = "fn surface(s: Surface) -> Surface { return s; }";
const EFFECT: &str = "fn effect(p: Pixel) -> vec3<f32> { return p.color; }";
const COMPUTE: &str = "fn compute(c: Cell) -> vec4<f32> { return c.value; }";

fn game() -> Game {
    Game::new("asset sources 3d", 1, 1).hot_reload(false)
}

/// Counts reads of every name it has, to see what the caches do.
struct Counting {
    id: &'static str,
    reads: Rc<Cell<u32>>,
    name: &'static str,
    bytes: Vec<u8>,
}

impl AssetSource for Counting {
    fn id(&self) -> &str {
        self.id
    }
    fn read(&self, name: &str) -> std::io::Result<Vec<u8>> {
        if name != self.name {
            return Err(std::io::ErrorKind::NotFound.into());
        }
        self.reads.set(self.reads.get() + 1);
        Ok(self.bytes.clone())
    }
}

#[test]
fn a_mesh_loads_by_name_from_a_memory_source() {
    game().run_headless(0, |ctx| {
        ctx.assets
            .add_source(MemorySource::new("pack").with("models/home.glb", HOUSE));
        let home = ctx.gfx.load_mesh("models/home.glb");
        assert_eq!(
            ctx.gfx.load_mesh("models/home.glb"),
            home,
            "loading again returns the same mesh"
        );
        let data = ctx.gfx.load_mesh_data("models/home.glb").unwrap();
        assert!(!data.indices.is_empty());
        ctx.world.spawn((Transform3D::default(), Model::new(home)));
    });
}

#[test]
fn a_mesh_in_a_source_is_found_by_the_names_a_scene_gives() {
    // A saved scene names its mesh; it is found in the source too.
    let mut saved = String::new();
    game().run_headless(0, |ctx| {
        ctx.assets
            .add_source(MemorySource::new("pack").with("models/home.glb", HOUSE));
        let home = ctx.gfx.load_mesh("models/home.glb");
        ctx.world.spawn((Transform3D::default(), Model::new(home)));
        saved = serde_json::to_string(&ctx.world.save()).unwrap();
    });
    let pack = MemorySource::new("pack")
        .with("models/home.glb", HOUSE)
        .with("level.json", saved);
    let world = game().run_headless(0, |ctx| {
        ctx.assets.add_source(pack);
        ctx.load_scene("level.json").unwrap();
        assert_eq!(ctx.world.query::<&Model>().iter().count(), 1);
    });
    assert_eq!(world.len(), 1);
}

#[test]
fn repeated_mesh_loads_read_a_source_once() {
    let reads = Rc::new(Cell::new(0));
    let counted = reads.clone();
    game().run_headless(0, move |ctx| {
        ctx.assets.add_source(Counting {
            id: "counting",
            reads: counted.clone(),
            name: "home.glb",
            bytes: HOUSE.to_vec(),
        });
        let first = ctx.gfx.load_mesh("home.glb");
        for _ in 0..3 {
            assert_eq!(ctx.gfx.load_mesh("home.glb"), first);
        }
        assert!(ctx.gfx.try_load_clips("home.glb").is_err(), "no skins in this file");
    });
    assert_eq!(reads.get(), 1, "meshes, skeletons and clips share one read");
}

#[test]
fn the_same_mesh_name_in_two_sources_is_two_meshes_and_the_first_wins() {
    game().run_headless(0, |ctx| {
        ctx.assets.add_source(MemorySource::new("first").with("m.glb", HOUSE));
        ctx.assets.add_source(MemorySource::new("second").with("m.glb", HOUSE));
        let from_first = ctx.gfx.load_mesh("m.glb");
        assert_eq!(ctx.gfx.load_mesh("m.glb"), from_first);
    });
    // The folder's copy and a source's copy are separate assets.
    game().run_headless(0, |ctx| {
        let from_folder = ctx.gfx.load_mesh("models/house.glb");
        ctx.assets
            .add_source(MemorySource::new("pack").with("models/house.glb", HOUSE));
        let from_source = ctx.gfx.load_mesh("models/house.glb");
        assert_ne!(from_source, from_folder, "same name, other source: another mesh");
        assert_eq!(ctx.gfx.load_mesh("models/house.glb"), from_source);
    });
}

#[test]
fn a_missing_mesh_reports_the_sources_and_folders_searched() {
    game().run_headless(0, |ctx| {
        ctx.assets.add_source(MemorySource::new("pack"));
        let err = ctx.gfx.load_mesh_data("models/nowhere.glb").unwrap_err();
        assert!(err.contains("\"models/nowhere.glb\" not found in assets/"), "{err}");
        assert!(err.contains("Asset sources searched first: \"pack\""), "{err}");
    });
}

#[test]
#[should_panic(expected = "models/broken.glb")]
fn corrupt_mesh_bytes_are_an_error_naming_the_file() {
    game().run_headless(0, |ctx| {
        ctx.assets
            .add_source(MemorySource::new("pack").with("models/broken.glb", b"nope".to_vec()));
        ctx.gfx.load_mesh("models/broken.glb");
    });
}

#[test]
fn shaders_effects_and_compute_load_by_name_from_a_memory_source() {
    game().run_headless(0, |ctx| {
        ctx.assets.add_source(
            MemorySource::new("pack")
                .with("shaders/plain.wgsl", SURFACE)
                .with("effects/plain.wgsl", EFFECT)
                .with("compute/plain.wgsl", COMPUTE),
        );
        let shader = ctx.gfx.load_shader("shaders/plain.wgsl");
        let effect = ctx.gfx.load_effect("effects/plain.wgsl");
        let compute = ctx.gfx.load_compute("compute/plain.wgsl");
        assert_eq!(ctx.gfx.load_shader("shaders/plain.wgsl"), shader);
        assert_eq!(ctx.gfx.load_effect("effects/plain.wgsl"), effect);
        assert_eq!(ctx.gfx.load_compute("compute/plain.wgsl"), compute);
    });
}

#[test]
fn repeated_shader_loads_read_a_source_once() {
    let reads = Rc::new(Cell::new(0));
    let counted = reads.clone();
    game().run_headless(0, move |ctx| {
        ctx.assets.add_source(Counting {
            id: "counting",
            reads: counted.clone(),
            name: "plain.wgsl",
            bytes: SURFACE.as_bytes().to_vec(),
        });
        let first = ctx.gfx.load_shader("plain.wgsl");
        for _ in 0..3 {
            assert_eq!(ctx.gfx.load_shader("plain.wgsl"), first);
        }
    });
    assert_eq!(reads.get(), 1);
}

#[test]
fn the_same_shader_name_in_two_sources_is_two_shaders() {
    game().run_headless(0, |ctx| {
        // The assets folder has shaders/water.wgsl; a source's copy is another shader.
        let from_folder = ctx.gfx.load_shader("shaders/water.wgsl");
        ctx.assets
            .add_source(MemorySource::new("pack").with("shaders/water.wgsl", SURFACE));
        let from_source = ctx.gfx.load_shader("shaders/water.wgsl");
        assert_ne!(from_source, from_folder);
        assert_eq!(ctx.gfx.load_shader("shaders/water.wgsl"), from_source);
    });
}

#[test]
fn shader_errors_name_the_source() {
    game().run_headless(0, |ctx| {
        ctx.assets
            .add_source(MemorySource::new("pack").with("bad.wgsl", vec![0xff, 0xfe, 0xfd]));
        let err = ctx.gfx.try_load_shader("bad.wgsl").unwrap_err();
        assert!(
            err.contains("\"bad.wgsl\" (from asset source \"pack\") is not UTF-8 text"),
            "{err}"
        );
        let err = ctx.gfx.try_load_effect("missing.wgsl").unwrap_err();
        assert!(err.contains("\"missing.wgsl\" not found in assets/"), "{err}");
        assert!(err.contains("Asset sources searched first: \"pack\""), "{err}");
        let err = ctx.gfx.try_load_compute("missing.wgsl").unwrap_err();
        assert!(err.contains("Asset sources searched first: \"pack\""), "{err}");
    });
}

#[test]
fn assets_without_sources_still_say_where_they_looked() {
    game().run_headless(0, |ctx| {
        let err = ctx.gfx.try_load_shader("nowhere.wgsl").unwrap_err();
        assert!(
            err.contains("\"nowhere.wgsl\" not found in assets/. Looked in:"),
            "{err}"
        );
        assert!(!err.contains("Asset sources searched first"), "{err}");
    });
}
