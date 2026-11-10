//! Shader errors as data: where in the game's own WGSL a compile failed,
//! why, and whether a hot reload took effect ([`Graphics::shader_status`](ponko::Graphics::shader_status)).
#![cfg(all(feature = "3d", feature = "window"))]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ponko::prelude::*;
#[allow(unused_imports)]
#[cfg(feature = "3d")]
use ponko::render::{
    Compute, ComputePass, Effect, Material, MaterialDesc, ScreenEffect, Shader, ShaderKind, ShaderOutcome, ShaderStatus,
};

/// A minimal invalid material shader: line 3 returns a number, not a `Surface`.
const BROKEN: &str = "// tints everything red\nfn surface(s: Surface) -> Surface {\n    return 1.0;\n}\n";
const GOOD: &str = "fn surface(s: Surface) -> Surface {\n    var out = s;\n    out.albedo = vec3<f32>(1.0, 0.0, 0.0);\n    return out;\n}\n";

#[test]
fn a_broken_shader_says_where_and_why_in_its_own_lines() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let err = ctx.gfx.try_create_shader("red.wgsl", BROKEN).unwrap_err();
        assert!(err.starts_with("red.wgsl:3:12: error: "), "{err}");

        let status = ctx.gfx.shader_status();
        assert_eq!(status.len(), 1);
        let s = &status[0];
        assert_eq!((s.shader.as_str(), s.kind), ("red.wgsl", ShaderKind::Material));
        assert_eq!(s.outcome, ShaderOutcome::Failed);
        assert!(!s.is_ok());
        let d = &s.diagnostics[0];
        assert_eq!((d.line, d.column, d.in_engine_code), (Some(3), Some(12), false));
        assert_eq!(d.source_line.as_deref(), Some("    return 1.0;"));
        assert!(d.message.contains("Surface"), "{}", d.message);
        assert!(!d.variant.is_empty());
        assert_eq!(err, d.to_string(), "the error string is the diagnostic");
    });
}

#[test]
fn good_shaders_are_listed_as_compiled() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.gfx.create_shader("red.wgsl", GOOD);
        ctx.gfx
            .create_effect("fx", "fn effect(p: Pixel) -> vec3<f32> { return p.color; }");
        let status = ctx.gfx.shader_status();
        assert_eq!(status.len(), 2);
        assert!(status.iter().all(|s| s.is_ok() && s.diagnostics.is_empty()));
        assert_eq!(status[1].kind, ShaderKind::Effect);
    });
}

#[test]
fn broken_effects_and_compute_passes_have_diagnostics_too() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        assert!(
            ctx.gfx
                .try_create_effect("fx.wgsl", "fn effect(p: Pixel) -> vec4<f32> {\n  return nope;\n}")
                .is_err()
        );
        assert!(
            ctx.gfx
                .try_create_compute("sim.wgsl", "\n\nfn compute(c: Cell) -> f32 { return 1.0; }")
                .is_err()
        );
        let status = ctx.gfx.shader_status();
        assert_eq!(status[0].kind, ShaderKind::Effect);
        assert_eq!(status[0].diagnostics[0].line, Some(2));
        assert_eq!(status[1].kind, ShaderKind::Compute);
        assert_eq!(status[1].outcome, ShaderOutcome::Failed);
    });
}

#[test]
fn a_hook_with_the_wrong_signature_is_reported_in_the_engines_code() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        // Effects return the new color as `vec3<f32>`; the engine's entry
        // point that calls this one is what breaks.
        let err = ctx
            .gfx
            .try_create_effect(
                "fx.wgsl",
                "fn effect(p: Pixel) -> vec4<f32> {\n    return vec4<f32>(p.color, 1.0);\n}",
            )
            .unwrap_err();
        let d = &ctx.gfx.shader_status()[0].diagnostics[0];
        assert!(d.in_engine_code, "{err}");
        assert!(err.starts_with("fx.wgsl (in the engine's code around it"), "{err}");
    });
}

/// Loads a shader file, breaks it on tick 1 and fixes it on tick 3, and
/// returns the shader's status seen on each tick.
#[test]
fn a_failed_reload_keeps_the_old_shader_and_a_fix_reloads() {
    let dir = std::env::temp_dir().join(format!("ponko-shader-reload-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("red.wgsl");
    std::fs::write(&path, GOOD).unwrap();
    let name = path.to_str().unwrap().to_string();
    let seen: Arc<Mutex<Vec<ShaderStatus>>> = Arc::default();
    let log = seen.clone();
    let file = path.clone();
    Game::new("shader", 1, 1)
        .hot_reload(true)
        .sim_system(move |ctx: &mut Ctx| {
            let status = ctx.gfx.shader_status().iter().find(|s| s.kind == ShaderKind::Material);
            log.lock().unwrap().push(status.unwrap().clone());
            let edit = match ctx.time.sim_tick() {
                1 => Some(BROKEN),
                3 => Some(GOOD),
                _ => None,
            };
            if let Some(text) = edit {
                std::thread::sleep(Duration::from_millis(50)); // a new modified time
                std::fs::write(&file, text).unwrap();
                std::thread::sleep(Duration::from_millis(400)); // past the next check
            }
        })
        .run_headless(6, move |ctx| {
            let shader = ctx.gfx.load_shader(&name);
            ctx.gfx.create_material(MaterialDesc {
                shader: Some(shader),
                ..Default::default()
            });
        });
    let seen = seen.lock().unwrap();
    let outcomes: Vec<ShaderOutcome> = seen.iter().map(|s| s.outcome).collect();
    use ShaderOutcome::*;
    assert_eq!(
        outcomes,
        [Compiled, Compiled, ReloadFailed, ReloadFailed, Reloaded, Reloaded]
    );
    let failed = &seen[2];
    assert_eq!(failed.diagnostics[0].line, Some(3), "{}", failed.diagnostics[0]);
    assert_eq!(failed.reloads, 0);
    assert!(seen[4].diagnostics.is_empty());
    assert_eq!(seen[4].reloads, 1, "the fix took effect");
    assert!(seen[4].frame > failed.frame);
}

const FX_BROKEN: &str = "fn effect(p: Pixel) -> vec3<f32> {\n    return p.color * nope;\n}\n";
const FX_GOOD: &str = "fn effect(p: Pixel) -> vec3<f32> {\n    return p.color * 0.5;\n}\n";
const CS_BROKEN: &str = "\nfn compute(c: Cell) -> vec4<f32> {\n    return 1.0;\n}\n";
const CS_GOOD: &str = "fn compute(c: Cell) -> vec4<f32> {\n    return c.value * 0.5;\n}\n";

/// With hot reload on, shader files that don't compile at first load get
/// a handle and stay watched: they stay `Failed` while still broken and
/// load when a fix is saved.
#[test]
fn shader_files_that_fail_at_first_load_are_watched_and_a_fix_loads_them() {
    let dir = std::env::temp_dir().join(format!("ponko-shader-first-load-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let files = [
        (dir.join("m.wgsl"), BROKEN, GOOD),
        (dir.join("fx.wgsl"), FX_BROKEN, FX_GOOD),
        (dir.join("cs.wgsl"), CS_BROKEN, CS_GOOD),
    ];
    for (path, broken, _) in &files {
        std::fs::write(path, broken).unwrap();
    }
    let names: Vec<String> = files.iter().map(|f| f.0.to_str().unwrap().to_string()).collect();
    let seen: Arc<Mutex<Vec<Vec<ShaderStatus>>>> = Arc::default();
    let log = seen.clone();
    let edits = files.clone();
    Game::new("shader", 1, 1)
        .hot_reload(true)
        .sim_system(move |ctx: &mut Ctx| {
            log.lock().unwrap().push(ctx.gfx.shader_status().to_vec());
            let fix = match ctx.time.sim_tick() {
                // Still broken, differently: no good version yet.
                1 => false,
                3 => true,
                _ => return,
            };
            std::thread::sleep(Duration::from_millis(50)); // a new modified time
            for (path, broken, good) in &edits {
                let text = if fix {
                    good.to_string()
                } else {
                    format!("// still\n{broken}")
                };
                std::fs::write(path, text).unwrap();
            }
            std::thread::sleep(Duration::from_millis(400)); // past the next check
        })
        .run_headless(6, move |ctx| {
            let shader = ctx.gfx.load_shader(&names[0]);
            assert_eq!(ctx.gfx.load_shader(&names[0]), shader, "loading again: the same handle");
            ctx.gfx.create_material(MaterialDesc {
                shader: Some(shader),
                ..Default::default()
            });
            let effect = ctx.gfx.load_effect(&names[1]);
            ctx.effects.push(ScreenEffect::new(effect));
            let compute = ctx.gfx.load_compute(&names[2]);
            let target = ctx.gfx.create_render_target(8, 8);
            ctx.compute_passes.push(ComputePass::new(compute, target));
        });
    let seen = seen.lock().unwrap();
    use ShaderOutcome::*;
    for kind in [ShaderKind::Material, ShaderKind::Effect, ShaderKind::Compute] {
        let of_kind: Vec<&ShaderStatus> = seen
            .iter()
            .map(|tick| tick.iter().find(|s| s.kind == kind).unwrap())
            .collect();
        let outcomes: Vec<ShaderOutcome> = of_kind.iter().map(|s| s.outcome).collect();
        assert_eq!(
            outcomes,
            [Failed, Failed, Failed, Failed, Reloaded, Reloaded],
            "{kind:?}"
        );
        let first = &of_kind[0].diagnostics[0];
        assert!(first.line.is_some() && !first.in_engine_code, "{first}");
        // The still-broken edit's error is one line further down.
        assert_eq!(of_kind[2].diagnostics[0].line, first.line.map(|l| l + 1), "{kind:?}");
        assert!(of_kind[4].diagnostics.is_empty());
        assert_eq!(of_kind[4].reloads, 1, "{kind:?}: the fix took effect");
    }
}

#[test]
fn without_hot_reload_a_shader_file_that_fails_is_an_error() {
    let dir = std::env::temp_dir().join(format!("ponko-shader-no-reload-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("m.wgsl");
    std::fs::write(&path, BROKEN).unwrap();
    let name = path.to_str().unwrap().to_string();
    Game::new("shader", 1, 1).hot_reload(false).run_headless(0, move |ctx| {
        let err = ctx.gfx.try_load_shader(&name).unwrap_err();
        assert!(err.contains(":3:12: error: "), "{err}");
        assert_eq!(ctx.gfx.shader_status()[0].outcome, ShaderOutcome::Failed);
        assert_eq!(ctx.live_counts().shaders, 0, "no handle was made");
    });
}

// ANCHOR: shader_errors
/// Prints every shader whose last compile or edit didn't take effect, with
/// where in its WGSL the error is; returns how many there are.
fn report_shader_errors(ctx: &Ctx) -> usize {
    let mut broken = 0;
    for status in ctx.gfx.shader_status().iter().filter(|s| !s.is_ok()) {
        broken += 1;
        for error in &status.diagnostics {
            eprintln!("{error}"); // water.wgsl:3:12: error: ...
        }
    }
    broken
}
// ANCHOR_END: shader_errors

#[test]
fn the_guide_reports_the_shader_that_failed() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.gfx.create_shader("tint.wgsl", GOOD);
        assert!(ctx.gfx.try_create_shader("water.wgsl", BROKEN).is_err());
        assert_eq!(report_shader_errors(ctx), 1);
    });
}
