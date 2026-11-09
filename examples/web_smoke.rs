//! The browser smoke-test fixture: a tiny game that `web/smoke/` builds with
//! `web/build.sh web_smoke` and drives in a headless browser.
//!
//! It loads one asset file (`crab.png`, fetched over HTTP on the web) and
//! two assets from an in-memory [asset source](Ctx::add_asset_source) (an
//! image and streamed music, never fetched), runs
//! a fixed-step simulation, counts left clicks and draws something on every
//! frame. Each frame it writes its state as JSON to the page's
//! `<html data-smoke="...">` attribute, which the runner reads:
//!
//! `{"ticks":120,"frames":118,"asset":[32,32],"source":[4,2],"stream":true,"clicks":1,"size":[800,600]}`
//!
//! Failure cases for the runner's self-tests, chosen with the page's query
//! string: `?smoke_case=panic` panics during setup, `?smoke_case=stall`
//! pauses the simulation after a few ticks.
//!
//! On the desktop it opens a window like any other example
//! (`cargo run --example web_smoke`) and prints its state once a second.

use ponko::*;
use std::cell::Cell;
use std::rc::Rc;

/// Ticks a `stall` run simulates before pausing. Keep it well below the
/// smoke harness's `MIN_PROGRESS_TICKS` (`web/smoke/smoke.mjs`), so the stall
/// is caught however early the harness first sees the state.
const STALL_AFTER: u64 = 20;

fn main() {
    let ticks = Rc::new(Cell::new(0u64));
    let sim_ticks = ticks.clone();
    Game::new("Ponko: web smoke", 800, 600)
        .fixed_hz(60)
        .sim_system(move |_ctx| sim_ticks.set(sim_ticks.get() + 1))
        .run(move |ctx| {
            let case = smoke_case();
            if case == "panic" {
                panic!("web_smoke: panicking during setup, as ?smoke_case=panic asked");
            }
            // The one fetched asset; a missing file panics with its name.
            let crab = ctx.load_texture("crab.png");
            // Assets from a source the game adds: found by name, never fetched.
            ctx.add_asset_source(
                MemorySource::new("embedded")
                    .with("embedded/dot.png", png(4, 2))
                    .with("embedded/beep.wav", silent_wav(800)),
            );
            let dot = ctx.load_texture("embedded/dot.png");
            let stream = ctx.try_load_stream("embedded/beep.wav").is_ok();
            let mut clicks = 0u32;
            let mut frames = 0u64;
            move |ctx: &mut Ctx| {
                frames += 1;
                if case == "stall" && ticks.get() >= STALL_AFTER && !ctx.is_paused() {
                    ctx.pause();
                }
                if ctx.mouse_pressed(MouseButton::Left) {
                    clicks += 1;
                }
                // The background changes with each click, so a click shows.
                ctx.clear(if clicks.is_multiple_of(2) {
                    Color::hex(0x1E2A38)
                } else {
                    Color::hex(0x2E4A28)
                });
                let size = ctx.window_size();
                let t = ticks.get() as f32 / 60.0;
                for i in 0..8 {
                    let angle = t + i as f32 * std::f32::consts::TAU / 8.0;
                    ctx.draw_ex(
                        &crab,
                        size / 2.0 + Vec2::from_angle(angle) * size.min_element() * 0.3,
                        DrawParams {
                            scale: Vec2::splat(3.0),
                            ..Default::default()
                        },
                    );
                }
                let state = format!(
                    r#"{{"ticks":{},"frames":{},"asset":[{},{}],"source":[{},{}],"stream":{},"clicks":{},"size":[{},{}]}}"#,
                    ticks.get(),
                    frames,
                    crab.width(),
                    crab.height(),
                    dot.width(),
                    dot.height(),
                    stream,
                    clicks,
                    size.x,
                    size.y
                );
                report(&state, frames);
            }
        });
}

/// A `width` x `height` PNG, made in memory.
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::RgbaImage::from_pixel(width, height, image::Rgba([255, 200, 0, 255]))
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("encoding a PNG in memory");
    bytes
}

/// A mono 16-bit WAV file of `samples` samples of silence at 8 kHz.
fn silent_wav(samples: u32) -> Vec<u8> {
    let data = samples * 2;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    for field in [
        16u32.to_le_bytes(),
        [1, 0, 1, 0],
        8000u32.to_le_bytes(),
        16_000u32.to_le_bytes(),
        [2, 0, 16, 0],
    ] {
        out.extend_from_slice(&field);
    }
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    out.resize(out.len() + data as usize, 0);
    out
}

/// The page's `smoke_case` query parameter (web), or `""`.
#[cfg(target_arch = "wasm32")]
fn smoke_case() -> String {
    let search = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    search
        .trim_start_matches('?')
        .split('&')
        .find_map(|pair| pair.strip_prefix("smoke_case="))
        .unwrap_or("")
        .to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn smoke_case() -> String {
    std::env::var("SMOKE_CASE").unwrap_or_default()
}

/// Publishes the state on `<html data-smoke>` for the browser runner.
#[cfg(target_arch = "wasm32")]
fn report(state: &str, _frames: u64) {
    if let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        let _ = root.set_attribute("data-smoke", state);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn report(state: &str, frames: u64) {
    if frames.is_multiple_of(60) {
        println!("{state}");
    }
}
