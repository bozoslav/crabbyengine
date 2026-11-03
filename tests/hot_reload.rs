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
