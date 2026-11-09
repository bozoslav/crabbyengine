//! Shader errors as data: where in the game's own WGSL a compile failed,
//! why, and whether a hot reload took effect ([`Graphics::shader_status`](ponko::Graphics::shader_status)).
#![cfg(all(feature = "3d", feature = "window"))]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ponko::*;

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
