//! Asset sources: named assets read from somewhere other than `assets/`.

use std::cell::Cell;
use std::rc::Rc;

use ponko::*;

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::RgbaImage::from_pixel(w, h, image::Rgba([255, 0, 0, 255]))
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap();
    bytes
}

fn game() -> Game {
    Game::new("asset sources", 1, 1).hot_reload(false)
}

#[test]
fn a_texture_loads_by_name_from_a_memory_source() {
    game().run_headless(0, |ctx| {
        ctx.add_asset_source(MemorySource::new("mem").with("tiles/red.png", png(3, 2)));
        let red = ctx.load_texture("tiles/red.png");
        assert_eq!((red.width(), red.height()), (3, 2));
        assert_eq!(
            ctx.load_texture("tiles/red.png"),
            red,
            "loading again returns the same handle"
        );
    });
}

#[test]
fn a_scene_and_the_texture_it_names_load_from_a_memory_source() {
    // A scene saved by the engine, kept only in memory.
    let mut saved = String::new();
    game().run_headless(0, |ctx| {
        ctx.add_asset_source(MemorySource::new("mem").with("red.png", png(2, 2)));
        let red = ctx.load_texture("red.png");
        let mut sprite = Sprite::new(red);
        sprite.z = 2.0;
        ctx.world.spawn((Transform2D::at(vec2(4.0, 5.0)), sprite));
        saved = serde_json::to_string(&ctx.world.save()).unwrap();
    });
    let source = MemorySource::new("mem")
        .with("red.png", png(2, 2))
        .with("levels/one.json", saved);
    let world = game().run_headless(0, |ctx| {
        ctx.add_asset_source(source);
        ctx.load_scene("levels/one.json").unwrap();
        let sprite = *ctx.world.query::<&Sprite>().iter().next().unwrap();
        assert_eq!(sprite.z, 2.0);
        assert_eq!(
            sprite.texture,
            ctx.load_texture("red.png"),
            "the scene loaded the source's texture"
        );
        assert_eq!((sprite.texture.width(), sprite.texture.height()), (2, 2));
    });
    assert_eq!(world.len(), 1);
}

#[test]
fn prefabs_and_resources_load_from_a_memory_source() {
    #[derive(serde::Deserialize)]
    struct Rules {
        speed: f32,
    }
    let prefab = r#"{ "format": "crabby-prefab", "version": 1, "components": {
        "Transform2D": { "position": [0.0, 0.0], "rotation": 0.0, "scale": [1.0, 1.0] },
        "Sprite": { "texture": "red.png", "z": 0.0 }
    } }"#;
    game().run_headless(0, |ctx| {
        ctx.add_asset_source(
            MemorySource::new("mem")
                .with("red.png", png(1, 1))
                .with("prefabs/crab.json", prefab)
                .with("rules.json", r#"{"speed": 7}"#),
        );
        let e = ctx.spawn_prefab("prefabs/crab.json", vec2(1.0, 2.0)).unwrap();
        assert_eq!(ctx.world.get::<Transform2D>(e).unwrap().position, vec2(1.0, 2.0));
        ctx.load_resource::<Rules>("rules.json");
        assert_eq!(ctx.world.resource::<Rules>().speed, 7.0);
    });
}

#[test]
fn a_missing_name_reports_the_sources_and_folders_searched() {
    game().run_headless(0, |ctx| {
        ctx.add_asset_source(MemorySource::new("mem"));
        let err = ctx.load_scene("levels/nowhere.json").unwrap_err();
        assert!(err.contains("\"levels/nowhere.json\" not found in assets/"), "{err}");
        assert!(err.contains("Asset sources searched first: \"mem\""), "{err}");
    });
}

#[test]
#[should_panic(expected = "\"broken.png\" (from asset source \"mem\") is not a valid image")]
fn corrupt_bytes_name_the_asset_and_its_source() {
    game().run_headless(0, |ctx| {
        ctx.add_asset_source(MemorySource::new("mem").with("broken.png", b"not a png".to_vec()));
        ctx.load_texture("broken.png");
    });
}

#[test]
fn a_bad_scene_from_a_source_is_an_error_not_a_panic() {
    game().run_headless(0, |ctx| {
        ctx.add_asset_source(
            MemorySource::new("mem")
                .with("bad.json", "{ not json")
                .with("binary.json", vec![0xff, 0xfe]),
        );
        let err = ctx.load_scene("bad.json").unwrap_err();
        assert!(err.starts_with("bad.json:"), "{err}");
        let err = ctx.load_scene("binary.json").unwrap_err();
        assert!(err.contains("(from asset source \"mem\") is not UTF-8 text"), "{err}");
    });
}

/// A source that counts reads, to see what the cache does.
struct Counting {
    id: &'static str,
    reads: Rc<Cell<u32>>,
    bytes: Vec<u8>,
}

impl AssetSource for Counting {
    fn id(&self) -> &str {
        self.id
    }
    fn read(&self, name: &str) -> std::io::Result<Vec<u8>> {
        if name != "shared.png" {
            return Err(std::io::ErrorKind::NotFound.into());
        }
        self.reads.set(self.reads.get() + 1);
        Ok(self.bytes.clone())
    }
}

#[test]
fn repeated_loads_read_a_source_once() {
    let reads = Rc::new(Cell::new(0));
    let counted = reads.clone();
    game().run_headless(0, move |ctx| {
        ctx.add_asset_source(Counting {
            id: "counting",
            reads: counted.clone(),
            bytes: png(1, 1),
        });
        let first = ctx.load_texture("shared.png");
        for _ in 0..3 {
            assert_eq!(ctx.load_texture("shared.png"), first);
        }
    });
    assert_eq!(reads.get(), 1);
}

#[test]
fn the_same_name_in_two_sources_is_two_assets_and_the_first_wins() {
    game().run_headless(0, |ctx| {
        ctx.add_asset_source(MemorySource::new("first").with("shared.png", png(1, 1)));
        ctx.add_asset_source(MemorySource::new("second").with("shared.png", png(2, 2)));
        assert_eq!(ctx.load_texture("shared.png").width(), 1, "the first source added wins");
    });
    game().run_headless(0, |ctx| {
        ctx.add_asset_source(MemorySource::new("second").with("shared.png", png(2, 2)));
        ctx.add_asset_source(MemorySource::new("first").with("shared.png", png(1, 1)));
        assert_eq!(
            ctx.load_texture("shared.png").width(),
            2,
            "swapped order, the other asset"
        );
    });
}

#[test]
fn a_name_only_in_a_later_source_falls_through_to_it() {
    game().run_headless(0, |ctx| {
        ctx.add_asset_source(MemorySource::new("a"));
        ctx.add_asset_source(MemorySource::new("b").with("only-b.png", png(5, 1)));
        assert_eq!(ctx.load_texture("only-b.png").width(), 5);
    });
}

#[test]
fn sources_are_asked_before_the_assets_folder() {
    // assets/crab.png exists in this repo; a source's own crab.png wins.
    game().run_headless(0, |ctx| {
        let from_folder = ctx.load_texture("crab.png");
        assert_ne!((from_folder.width(), from_folder.height()), (7, 7));
    });
    game().run_headless(0, |ctx| {
        ctx.add_asset_source(MemorySource::new("mem").with("crab.png", png(7, 7)));
        let crab = ctx.load_texture("crab.png");
        assert_eq!((crab.width(), crab.height()), (7, 7));
    });
}

#[test]
fn a_read_error_other_than_not_found_stops_the_search() {
    struct Broken;
    impl AssetSource for Broken {
        fn id(&self) -> &str {
            "broken"
        }
        fn read(&self, _: &str) -> std::io::Result<Vec<u8>> {
            Err(std::io::Error::other("disk on fire"))
        }
    }
    game().run_headless(0, |ctx| {
        ctx.add_asset_source(Broken);
        let err = ctx.load_scene("crab.png").unwrap_err();
        assert_eq!(err, "asset source \"broken\" could not read \"crab.png\": disk on fire");
    });
}

#[test]
#[should_panic(expected = "an asset source called \"mem\" was already added")]
fn source_ids_must_be_unique() {
    game().run_headless(0, |ctx| {
        ctx.add_asset_source(MemorySource::new("mem"));
        ctx.add_asset_source(MemorySource::new("mem"));
    });
}

#[test]
fn hot_reload_skips_assets_from_added_sources() {
    // With hot reload on, a source's texture loads and frames run without
    // anything being watched or reloaded (only `assets/` files are watched).
    let mut tex = None;
    Game::new("asset sources", 1, 1)
        .hot_reload(true)
        .sim_system(|ctx: &mut Ctx| assert!(ctx.reloaded_files().is_empty()))
        .run_headless(5, |ctx| {
            ctx.add_asset_source(MemorySource::new("mem").with("red.png", png(1, 1)));
            tex = Some(ctx.load_texture("red.png"));
        });
    assert_eq!(tex.unwrap().width(), 1);
}

// ANCHOR: embedded
#[derive(serde::Deserialize)]
struct Tuning {
    speed: f32,
}

#[test]
fn embedded_files_load_by_name_like_files_in_assets() {
    game().run_headless(0, |ctx| {
        // Compiled into the executable: nothing to ship next to it.
        ctx.add_asset_source(
            MemorySource::new("embedded")
                .with("sprites/crab.png", include_bytes!("../assets/crab.png"))
                .with("tuning.json", r#"{"speed": 30}"#),
        );

        // From here on, loads by name look there first.
        let crab = ctx.load_texture("sprites/crab.png");
        ctx.load_resource::<Tuning>("tuning.json");
        // ANCHOR_END: embedded
        assert!(crab.width() > 0);
        assert_eq!(ctx.world.resource::<Tuning>().speed, 30.0);
    });
}
