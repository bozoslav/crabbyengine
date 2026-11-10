//! Asset streaming without gameplay-thread stalls: a game that needs 16
//! new 1024 × 1024 images at once (a cell coming into view), loaded with
//! `load_texture` (read and decoded on the game thread) or `stream_texture`
//! (read and decoded on job threads, uploaded within the per-frame
//! budget). Measures the game thread's CPU time in its loading work each
//! frame: the load or stream calls plus the engine's upload step. CPU time,
//! not wall time, so the OS pausing the thread on a busy machine doesn't
//! count.
//! Headless: the GPU's share of uploads isn't included.
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use ponko::{Ctx, Game};

use crate::{Bench, report};

const TEXTURES: usize = 16;
const SIDE: u32 = 1024;
const FRAMES: u64 = 600;
/// The frame the game first needs the images in.
const NEED_AT: u64 = 10;

/// CPU time this thread has used.
fn thread_cpu() -> Duration {
    let mut t = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: `t` is a valid, writable timespec.
    let ok = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut t) };
    assert_eq!(ok, 0, "thread CPU clock");
    Duration::new(t.tv_sec as u64, t.tv_nsec as u32)
}

/// Writes 16 PNGs of noise (so decoding costs what a real image does) to a
/// temp folder and returns their paths.
fn noise_pngs() -> Vec<String> {
    let dir = std::env::temp_dir().join(format!("ponko-bench-streaming-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    (0..TEXTURES)
        .map(|i| {
            let mut s = (i as u32 + 1).wrapping_mul(0x9E37_79B9) | 1;
            let img = image::RgbaImage::from_fn(SIDE, SIDE, |_, _| {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                image::Rgba([s as u8, (s >> 8) as u8, (s >> 16) as u8, 255])
            });
            let path: PathBuf = dir.join(format!("t{i}.png"));
            img.save(&path).expect("write png");
            path.to_str().expect("utf-8 path").to_string()
        })
        .collect()
}

/// Each frame's loading work on the game thread, the frame (after the
/// game first needed them) all images were usable in, and the longest
/// time between frames.
fn frames(paths: &[String], streamed: bool) -> (Vec<Duration>, Option<u64>, Duration) {
    let work = Rc::new(RefCell::new(Vec::new()));
    let ready_at = Rc::new(RefCell::new(None));
    let wall = Rc::new(RefCell::new(Duration::ZERO));
    let (log, ready, worst) = (work.clone(), ready_at.clone(), wall.clone());
    let mut last: Option<Instant> = None;
    let mut since_system = Duration::ZERO;
    let paths = paths.to_vec();
    Game::new("bench", 1, 1)
        .sim_system(move |ctx: &mut Ctx| {
            let now = Instant::now();
            if let Some(last) = last {
                let mut w = worst.borrow_mut();
                *w = (*w).max(now - last);
            }
            last = Some(now);
            let tick = ctx.sim_tick();
            // The engine's upload step ran between the end of this system
            // last frame and now: count its CPU time (with a little engine
            // bookkeeping), then the loading calls'.
            let start = thread_cpu();
            let step = if streamed && tick > 0 {
                start.saturating_sub(since_system)
            } else {
                Duration::ZERO
            };
            if tick >= NEED_AT {
                let all = if streamed {
                    paths.iter().filter(|p| ctx.stream_texture(p).is_some()).count() == TEXTURES
                } else {
                    for p in &paths {
                        ctx.load_texture(p);
                    }
                    true
                };
                if all && ready.borrow().is_none() {
                    *ready.borrow_mut() = Some(tick - NEED_AT);
                }
            }
            log.borrow_mut().push(step + (thread_cpu() - start));
            // Gameplay work that must not wait: 1 ms of it each frame.
            while now.elapsed() < Duration::from_millis(1) {
                std::hint::black_box(0u64);
            }
            since_system = thread_cpu();
        })
        .run_headless(FRAMES, |_| {});
    let w = work.borrow().clone();
    let r = *ready_at.borrow();
    let worst = *wall.borrow();
    (w, r, worst)
}

pub fn run(b: &mut Bench) {
    if b.skips("streaming/") {
        return;
    }
    let paths = noise_pngs();
    for (name, streamed) in [("streaming/sync-load-16x1k", false), ("streaming/stream-16x1k", true)] {
        if b.skips(name) {
            continue;
        }
        let (mut times, ready, wall) = frames(&paths, streamed);
        let record = report::Record::new(name, report::Kind::Steady, 0, &mut times);
        let ready = ready.unwrap_or_else(|| panic!("{name}: the images never became usable"));
        println!(
            "{name:<32} worst frame's loading work {:>8.3} ms   usable after {ready} frames   longest frame {:>8.3} ms",
            crate::ms(record.max),
            crate::ms(wall)
        );
        b.report(record);
    }
    if let Some(dir) = paths.first().and_then(|p| std::path::Path::new(p).parent()) {
        let _ = std::fs::remove_dir_all(dir);
    }
}
