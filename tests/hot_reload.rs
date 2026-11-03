//! Asset hot reload: edited image files come back under the same handle.
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crabbyengine::*;

fn write_png(path: &Path, w: u32, h: u32, rgba: [u8; 4]) {
    let img = image::RgbaImage::from_pixel(w, h, image::Rgba(rgba));
    img.save(path).unwrap();
}

fn temp_png(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("crabby-hot-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    write_png(&path, 4, 4, [255, 0, 0, 255]);
    path
}

/// Runs a headless game that loads `path`, calls `edit` on tick 2 (then
/// waits past the next check), and returns the texture and what was
/// reloaded on each tick.
fn run(path: &Path, hot_reload: bool, edit: impl Fn() + 'static) -> (Texture, Vec<(u64, Vec<Texture>)>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    let mut tex = None;
    let name = path.to_str().unwrap().to_string();
    Game::new("hot", 1, 1)
        .hot_reload(hot_reload)
        .sim_system(move |ctx: &mut Ctx| {
            let reloaded = ctx.reloaded_textures().to_vec();
            if !reloaded.is_empty() {
                log.lock().unwrap().push((ctx.sim_tick(), reloaded));
            }
            if ctx.sim_tick() == 2 {
                edit();
                std::thread::sleep(Duration::from_millis(400));
            }
        })
        .run_headless(5, |ctx| tex = Some(ctx.load_texture(&name)));
    let seen = seen.lock().unwrap().clone();
    (tex.unwrap(), seen)
}

#[test]
fn edited_images_reload_under_the_same_handle() {
    let path = temp_png("crab.png");
    let p = path.clone();
    let (tex, seen) = run(&path, true, move || write_png(&p, 4, 4, [0, 0, 255, 255]));
    assert_eq!(seen, vec![(3, vec![tex])], "reloaded once, on the tick after the edit");
}

#[test]
fn images_that_change_size_are_not_reloaded() {
    let path = temp_png("bigger.png");
    let p = path.clone();
    let (tex, seen) = run(&path, true, move || write_png(&p, 8, 8, [0, 0, 255, 255]));
    assert_eq!((tex.width(), tex.height()), (4, 4));
    assert!(
        seen.is_empty(),
        "sprites keep the old size, so nothing changes: {seen:?}"
    );
}

#[test]
fn nothing_reloads_with_hot_reload_off() {
    let path = temp_png("off.png");
    let p = path.clone();
    let (_, seen) = run(&path, false, move || write_png(&p, 4, 4, [0, 0, 255, 255]));
    assert!(seen.is_empty());
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct Rules {
    speed: f32,
    damage: f32,
}

fn temp_json(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("crabby-data-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path
}

/// Loads `path` as `Rules`, rewrites it to `edit` on tick 2, and returns
/// the speed and reloaded file names seen on each tick.
fn run_rules(path: &Path, edit: &'static str) -> Vec<(f32, Vec<String>)> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    let name = path.to_str().unwrap().to_string();
    let file = path.to_path_buf();
    Game::new("data", 1, 1)
        .hot_reload(true)
        .sim_system(move |ctx: &mut Ctx| {
            let speed = ctx.world.resource::<Rules>().speed;
            log.lock().unwrap().push((speed, ctx.reloaded_files().to_vec()));
            if ctx.sim_tick() == 2 {
                std::fs::write(&file, edit).unwrap();
                std::thread::sleep(Duration::from_millis(400));
            }
        })
        .run_headless(5, |ctx| ctx.load_resource::<Rules>(&name));
    seen.lock().unwrap().clone()
}

#[test]
fn edited_data_files_replace_their_resource() {
    let path = temp_json("rules.json", r#"{"speed": 30, "damage": 2}"#);
    let seen = run_rules(&path, r#"{"speed": 45, "damage": 2}"#);
    let speeds: Vec<f32> = seen.iter().map(|s| s.0).collect();
    assert_eq!(speeds, [30.0, 30.0, 30.0, 45.0, 45.0]);
    let name = path.to_str().unwrap().to_string();
    assert_eq!(seen[3].1, [name], "listed as reloaded on that tick only");
    assert!(seen[4].1.is_empty());
}

#[test]
fn data_files_that_dont_parse_keep_the_old_values() {
    let path = temp_json("broken.json", r#"{"speed": 30, "damage": 2}"#);
    let seen = run_rules(&path, r#"{"speed": 45, "dama"#);
    assert!(seen.iter().all(|s| s.0 == 30.0 && s.1.is_empty()));
}

#[test]
#[should_panic(expected = "line 1 column")]
fn bad_data_files_say_where_at_load() {
    let path = temp_json("bad.json", r#"{"speed": "fast", "damage": 2}"#);
    let name = path.to_str().unwrap().to_string();
    Game::new("data", 1, 1).run_headless(0, |ctx| ctx.load_resource::<Rules>(&name));
}

#[test]
#[should_panic(expected = "not found in assets/")]
fn missing_data_files_say_where_they_were_looked_for() {
    Game::new("data", 1, 1).run_headless(0, |ctx| ctx.load_resource::<Rules>("no-such-rules.json"));
}

// ANCHOR: data_files
#[derive(serde::Deserialize)]
struct Tuning {
    speed: f32,
    damage: f32,
}

fn load_tuning(ctx: &mut Ctx, path: &str) {
    ctx.load_resource::<Tuning>(path); // e.g. "tuning.json": {"speed": 30, "damage": 2}
}

fn step_size(ctx: &Ctx) -> f32 {
    // The file's current values, every tick.
    ctx.world.resource::<Tuning>().speed * ctx.dt()
}
// ANCHOR_END: data_files

#[test]
fn the_guide_data_file_example_runs() {
    let path = temp_json("tuning.json", r#"{"speed": 30, "damage": 2}"#);
    let name = path.to_str().unwrap().to_string();
    let world = Game::new("data", 1, 1)
        .fixed_hz(10)
        .sim_system(|ctx| {
            let s = step_size(ctx);
            assert!((s - 3.0).abs() < 1e-5 && ctx.world.resource::<Tuning>().damage == 2.0);
        })
        .run_headless(1, |ctx| load_tuning(ctx, &name));
    assert_eq!(world.resource::<Tuning>().speed, 30.0);
}
