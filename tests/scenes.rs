//! Scenes and prefabs: worlds and entities saved to files and loaded back.

mod common;

use common::TempAsset;
#[allow(unused_imports)]
use ponko::d2::{Sprite, Transform as Transform2D};
use ponko::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
struct Palm {
    height: f32,
}

fn game() -> Game {
    Game::new("scenes", 1, 1)
}

#[test]
fn a_saved_scene_loads_back_with_the_same_entities() {
    let file = TempAsset::new("test-scene-roundtrip.json");
    let mut first = None;
    game().run_headless(0, |ctx| {
        ctx.world.reflect::<Palm>();
        first = Some(ctx.world.spawn((Transform2D::at(vec2(1.0, 2.0)), Palm { height: 3.0 })));
        ctx.world.spawn((Transform2D::at(vec2(5.0, 6.0)),));
        ctx.assets.save_scene(&ctx.world, file.name()).unwrap();
    });
    let world = game().run_headless(0, |ctx| {
        ctx.world.reflect::<Palm>();
        ctx.world.spawn((Transform2D::default(),)); // replaced by the scene
        ctx.load_scene(file.name()).unwrap();
    });
    assert_eq!(world.len(), 2);
    let e = first.unwrap();
    assert_eq!(world.get::<Palm>(e), Some(&Palm { height: 3.0 }), "same id, same palm");
    assert_eq!(world.get::<Transform2D>(e).unwrap().position, vec2(1.0, 2.0));
}

#[test]
fn prefabs_spawn_where_asked_with_their_saved_components() {
    let file = TempAsset::new("test-prefab-palm.json");
    let mut world = game().run_headless(0, |ctx| {
        ctx.world.reflect::<Palm>();
        let model = ctx
            .world
            .spawn((Transform2D::at(vec2(70.0, 80.0)).rotated(0.5), Palm { height: 9.0 }));
        ctx.assets.save_prefab(&ctx.world, model, file.name()).unwrap();
        ctx.world.despawn(model);
        for x in 0..3 {
            ctx.spawn_prefab(file.name(), vec2(x as f32 * 10.0, 100.0)).unwrap();
        }
    });
    let mut palms: Vec<(Vec2, f32, f32)> = world
        .query::<(&Transform2D, &Palm)>()
        .iter()
        .map(|(t, p)| (t.position, t.rotation, p.height))
        .collect();
    palms.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
    assert_eq!(
        palms,
        vec![
            (vec2(0.0, 100.0), 0.5, 9.0),
            (vec2(10.0, 100.0), 0.5, 9.0),
            (vec2(20.0, 100.0), 0.5, 9.0),
        ]
    );
}

#[test]
fn a_prefab_without_a_transform_gets_one() {
    let file = TempAsset::new("test-prefab-bare.json");
    std::fs::write(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(file.name()),
        r#"{ "format": "crabby-prefab", "version": 1, "components": { "Palm": { "height": 2 } } }"#,
    )
    .unwrap();
    let mut spawned = None;
    let world = game().run_headless(0, |ctx| {
        ctx.world.reflect::<Palm>();
        spawned = Some(ctx.spawn_prefab(file.name(), vec2(4.0, 5.0)).unwrap());
    });
    assert_eq!(
        world.get::<Transform2D>(spawned.unwrap()).unwrap().position,
        vec2(4.0, 5.0)
    );
}

#[test]
fn bad_files_are_errors_that_change_nothing() {
    let file = TempAsset::new("test-not-a-prefab.json");
    file.write(r#"{ "format": "crabby-world", "version": 1, "entities": [] }"#);
    game().run_headless(0, |ctx| {
        let missing = ctx.load_scene("test-no-such-scene.json").unwrap_err();
        assert!(missing.contains("not found in assets/"), "{missing}");
        let wrong = ctx.spawn_prefab(file.name(), Vec2::ZERO).unwrap_err();
        assert!(wrong.contains("not a prefab"), "{wrong}");
        assert_eq!(ctx.world.len(), 0);
        let dead = ctx.world.spawn((Transform2D::default(),));
        ctx.world.despawn(dead);
        assert!(
            ctx.assets
                .save_prefab(&ctx.world, dead, "test-never-written.json")
                .is_err()
        );
    });
}

#[test]
fn scenes_with_huge_entity_ids_are_errors_that_change_nothing() {
    let file = TempAsset::new("test-huge-entity-id.json");
    game().run_headless(0, |ctx| {
        let kept = ctx.world.spawn((Transform2D::at(vec2(1.0, 2.0)),));
        let (slots, bytes) = (ctx.world.entity_slots(), ctx.world.allocated_bytes());
        for id in [50_000_000u64, 1 << 31] {
            // The issue's 181-byte file, with this id.
            let mut scene = World::new().save();
            scene["entities"] = serde_json::json!([ { "entity": id,
                "components": { "Transform2D": { "position": [0.0, 0.0], "rotation": 0.0, "scale": [1.0, 1.0] } } } ]);
            file.write(&scene.to_string());
            let err = ctx.load_scene(file.name()).unwrap_err();
            assert!(err.contains("may only use indices below"), "{err}");
            assert_eq!(ctx.world.len(), 1);
            assert_eq!((ctx.world.entity_slots(), ctx.world.allocated_bytes()), (slots, bytes));
            assert!(ctx.world.is_alive(kept));
        }
    });
}

// ANCHOR: scenes
fn beach(ctx: &mut Ctx) -> Result<(), String> {
    ctx.world.reflect::<Palm>(); // components in files must be reflected
    ctx.load_scene("levels/beach.json")?; // the level: its entities, ids kept
    for i in 0..3 {
        // three more palms, all alike, in a row
        ctx.spawn_prefab("prefabs/palm.json", vec2(400.0 + i as f32 * 64.0, 400.0))?;
    }
    Ok(())
}
// ANCHOR_END: scenes

#[test]
fn the_guide_beach_has_five_palms() {
    let mut world = game().run_headless(0, |ctx| beach(ctx).unwrap());
    let mut heights: Vec<f32> = world.query::<&Palm>().iter().map(|p| p.height).collect();
    heights.sort_by(f32::total_cmp);
    assert_eq!(heights, [3.0, 4.0, 4.0, 4.0, 5.0]);
}

#[test]
fn sprites_are_saved_by_their_image_name_and_load_it() {
    let file = TempAsset::new("test-prefab-crab.json");
    game().run_headless(0, |ctx| {
        let crab = ctx.load_texture("crab.png");
        let mut sprite = Sprite::new(crab);
        sprite.z = 3.0;
        let model = ctx.world.spawn((Transform2D::default(), sprite));
        ctx.assets.save_prefab(&ctx.world, model, file.name()).unwrap();
    });
    let text = file.read();
    assert!(text.contains(r#""texture": "crab.png""#), "{text}");
    // A new game hasn't loaded crab.png: spawning the prefab loads it.
    game().run_headless(0, |ctx| {
        let e = ctx.spawn_prefab(file.name(), vec2(5.0, 5.0)).unwrap();
        let sprite = *ctx.world.get::<Sprite>(e).unwrap();
        assert_eq!(sprite.z, 3.0);
        assert_eq!(sprite.texture, ctx.load_texture("crab.png"), "the same handle");
    });
}

#[test]
fn sprites_of_textures_made_in_code_are_left_out_of_saves() {
    game().run_headless(0, |ctx| {
        let made = ctx.gfx.create_texture(1, 1, &[255; 4]);
        let e = ctx.world.spawn((Transform2D::default(), Sprite::new(made)));
        let json = ctx.world.entity_json(e).unwrap();
        assert!(
            json.get("Transform2D").is_some() && json.get("Sprite").is_none(),
            "{json}"
        );
    });
}

#[test]
fn a_sprite_needs_an_image_that_exists() {
    game().run_headless(0, |ctx| {
        let e = ctx.world.spawn((Transform2D::default(),));
        let missing = ctx
            .world
            .set_json(e, "Sprite", serde_json::json!({ "z": 1 }))
            .unwrap_err();
        assert!(missing.to_string().contains("needs \"texture\""), "{missing}");
        let unloaded = ctx
            .world
            .set_json(e, "Sprite", serde_json::json!({ "texture": "crab.png" }))
            .unwrap_err();
        assert!(unloaded.to_string().contains("isn't loaded"), "{unloaded}");
    });
}
