//! `load_component_json::<T>` / `load_resource_json::<T>`: the name comes from the type.
use ponko::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
struct Stats {
    hp: f32,
    armor: f32,
}
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
struct Tuning {
    speed: f32,
    gravity: f32,
}

/// A file under `assets/` that is deleted when the test ends.
struct TempFile(&'static str);
impl TempFile {
    fn new(name: &'static str, json: &str) -> Self {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(name);
        std::fs::write(path, json).unwrap();
        TempFile(name)
    }
}
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(self.0),
        );
    }
}

#[test]
fn typed_component_and_resource_loaders_match_the_string_forms() {
    let stats = TempFile::new("test-typed-stats.json", r#"{"hp": 7.0}"#);
    let tuning = TempFile::new("test-typed-tuning.json", r#"{"speed": 3.0}"#);
    Game::new("typed", 1, 1).run_headless(0, |ctx| {
        ctx.world.reflect::<Stats>();
        ctx.world.reflect_resource::<Tuning>();
        ctx.world.insert_resource(Tuning {
            speed: 1.0,
            gravity: 9.0,
        });
        let e = ctx.world.spawn((Stats { hp: 1.0, armor: 5.0 },));
        let e2 = ctx.world.spawn((Stats { hp: 1.0, armor: 5.0 },));

        ctx.load_component_json::<Stats>(stats.0, e).unwrap();
        ctx.load_component_json_named(stats.0, e2, "Stats").unwrap();
        // Object fields merge: hp from the file, armor kept.
        assert_eq!(ctx.world.get::<Stats>(e), Some(&Stats { hp: 7.0, armor: 5.0 }));
        assert_eq!(ctx.world.get::<Stats>(e2), ctx.world.get::<Stats>(e));

        ctx.load_resource_json::<Tuning>(tuning.0).unwrap();
        assert_eq!(
            *ctx.world.resource::<Tuning>(),
            Tuning {
                speed: 3.0,
                gravity: 9.0
            }
        );
    });
}

#[test]
fn a_type_that_was_not_reflected_is_an_error_not_a_panic() {
    let stats = TempFile::new("test-typed-unreflected.json", r#"{"hp": 7.0}"#);
    Game::new("typed", 1, 1).run_headless(0, |ctx| {
        let e = ctx.world.spawn((Stats::default(),));
        let error = ctx.load_component_json::<Stats>(stats.0, e).unwrap_err();
        assert!(error.contains("Stats"), "{error}");
        let error = ctx.load_resource_json::<Tuning>(stats.0).unwrap_err();
        assert!(error.contains("Tuning"), "{error}");
    });
}
