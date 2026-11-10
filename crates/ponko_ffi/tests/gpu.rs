//! The GPU report and tiers from C: per-tier reports and features without
//! a GPU, the tier presets, and headless contexts answering "no GPU".
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

/// Reads a text call's output: sized first, then filled.
fn text(call: impl Fn(*mut std::ffi::c_char, usize, &mut usize) -> CrabbyStatus) -> String {
    let mut len = 0usize;
    assert_eq!(call(null_mut(), 0, &mut len), CRABBY_OK);
    let mut buffer = vec![0u8; len + 1];
    let mut again = 0usize;
    assert_eq!(call(buffer.as_mut_ptr().cast(), buffer.len(), &mut again), CRABBY_OK);
    assert_eq!(again, len);
    assert_eq!(buffer[len], 0, "NUL-terminated");
    String::from_utf8(buffer[..len].to_vec()).unwrap()
}

fn tier_report(tier: u32) -> serde_json::Value {
    let json = text(|b, c, l| unsafe { crabby_gpu_tier_report(tier, b, c, l) });
    serde_json::from_str(&json).unwrap()
}

#[test]
fn tier_reports_are_json_with_their_tier_and_fallbacks() {
    let names = ["webgl2", "baseline", "desktop"];
    for (i, name) in names.iter().enumerate() {
        let r = tier_report(i as u32);
        assert_eq!(r["tier"], *name);
        assert_eq!(r["forced_tier"], serde_json::Value::Null);
        let features = r["features"].as_array().unwrap();
        assert_eq!(features.len(), 13);
        for f in features {
            let available = f["available"].as_bool().unwrap();
            assert_eq!(f["fallback"].is_null(), available, "{f}");
            assert_eq!(f["reason"].is_null(), available, "{f}");
        }
    }
    let webgl2 = tier_report(0);
    let taa = &webgl2["features"][CrabbyGpuFeature::CRABBY_GPU_FEATURE_TEMPORAL_ANTIALIASING as usize];
    assert_eq!(taa["feature"], "temporal_antialiasing");
    assert_eq!(taa["fallback"], "FXAA instead of TAA");
    assert_eq!(webgl2["limits"]["max_compute_workgroups_per_dimension"], 0);
}

#[test]
fn tier_features_match_the_documented_table() {
    use CrabbyGpuFeature::*;
    let has = |tier: u32, f: CrabbyGpuFeature| {
        let mut on = false;
        assert_eq!(unsafe { crabby_gpu_tier_feature(tier, f as u32, &mut on) }, CRABBY_OK);
        on
    };
    // WebGL2: nothing that needs compute; multisampling stays.
    assert!(!has(0, CRABBY_GPU_FEATURE_COMPUTE) && !has(0, CRABBY_GPU_FEATURE_SCREEN_SPACE_REFLECTIONS));
    assert!(has(0, CRABBY_GPU_FEATURE_MULTISAMPLING));
    // Baseline: screen-space effects and culling, not skinning, decals or GI.
    for f in [
        CRABBY_GPU_FEATURE_COMPUTE_CULLING,
        CRABBY_GPU_FEATURE_TEMPORAL_ANTIALIASING,
        CRABBY_GPU_FEATURE_AMBIENT_OCCLUSION,
        CRABBY_GPU_FEATURE_OCCLUSION_CULLING,
    ] {
        assert!(has(1, f), "{f:?}");
    }
    for f in [
        CRABBY_GPU_FEATURE_GPU_SKINNING,
        CRABBY_GPU_FEATURE_DECALS,
        CRABBY_GPU_FEATURE_GLOBAL_ILLUMINATION,
    ] {
        assert!(!has(1, f) && has(2, f), "{f:?}");
    }
    // Timestamps depend on the device at every tier.
    assert!(!has(2, CRABBY_GPU_FEATURE_TIMESTAMPS));
}

#[test]
fn bad_tiers_features_and_outputs_are_refused() {
    let mut on = false;
    let mut len = 0usize;
    unsafe {
        assert_eq!(crabby_gpu_tier_feature(3, 0, &mut on), CRABBY_INVALID_ARGUMENT);
        assert_eq!(crabby_gpu_tier_feature(0, 13, &mut on), CRABBY_INVALID_ARGUMENT);
        assert_eq!(crabby_gpu_tier_feature(0, 0, null_mut()), CRABBY_INVALID_ARGUMENT);
        assert_eq!(
            crabby_gpu_tier_report(7, null_mut(), 0, &mut len),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_gpu_tier_report(0, null_mut(), 0, null_mut()),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_gpu_tier_report(0, null_mut(), 8, &mut len),
            CRABBY_INVALID_ARGUMENT
        );
    }
    // Too small a buffer: the length is reported, the text left empty.
    let mut small = [1u8; 8];
    assert_eq!(
        unsafe { crabby_gpu_tier_report(0, small.as_mut_ptr().cast(), small.len(), &mut len) },
        CRABBY_OK
    );
    assert!(len > small.len());
    assert_eq!(small[0], 0);
}

#[test]
fn tier_presets_are_low_medium_and_high_without_what_the_tier_lacks() {
    let preset = |level: u32| {
        let mut q = unsafe { std::mem::zeroed::<CrabbyGraphicsQuality>() };
        assert_eq!(unsafe { crabby_graphics_quality_preset(level, &mut q) }, CRABBY_OK);
        q
    };
    let for_tier = |tier: u32| {
        let mut q = unsafe { std::mem::zeroed::<CrabbyGraphicsQuality>() };
        assert_eq!(unsafe { crabby_graphics_quality_for_tier(tier, &mut q) }, CRABBY_OK);
        q
    };
    let web = for_tier(0);
    assert_eq!(
        web,
        CrabbyGraphicsQuality {
            anti_aliasing: 1,
            ..preset(0)
        }
    );
    assert!(!web.ambient_occlusion && web.ssr == 0.0 && !web.gi);
    assert_eq!(for_tier(1), preset(1));
    assert_eq!(for_tier(2), preset(2));
    let mut q = for_tier(2);
    assert_eq!(
        unsafe { crabby_graphics_quality_for_tier(3, &mut q) },
        CRABBY_INVALID_ARGUMENT
    );
    assert_eq!(
        unsafe { crabby_graphics_quality_for_tier(0, null_mut()) },
        CRABBY_INVALID_ARGUMENT
    );
}

struct Seen {
    statuses: Vec<CrabbyStatus>,
}

unsafe extern "C" fn ask(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let seen = &mut *user.cast::<Seen>();
        let mut len = 0usize;
        let mut tier = 9u32;
        let mut on = true;
        let mut q = std::mem::zeroed::<CrabbyGraphicsQuality>();
        seen.statuses = vec![
            crabby_ctx_gpu_capabilities(ctx, null_mut(), 0, &mut len),
            crabby_ctx_gpu_tier(ctx, &mut tier),
            crabby_ctx_gpu_feature(ctx, 0, &mut on),
            crabby_ctx_recommended_graphics_quality(ctx, &mut q),
            // A bad feature is refused before asking for the GPU.
            crabby_ctx_gpu_feature(ctx, 99, &mut on),
        ];
        assert_eq!((tier, on), (9, true), "nothing written");
    }
}

#[test]
fn headless_contexts_have_no_gpu_report() {
    let mut seen = Seen { statuses: Vec::new() };
    unsafe {
        let g = crabby_game_new(c"gpu".as_ptr(), 1, 1);
        let mut world = null_mut();
        assert_eq!(
            crabby_game_run_headless(g, 1, Some(ask), (&raw mut seen).cast(), &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
    assert_eq!(
        seen.statuses,
        [
            CRABBY_NOT_FOUND,
            CRABBY_NOT_FOUND,
            CRABBY_NOT_FOUND,
            CRABBY_NOT_FOUND,
            CRABBY_INVALID_ARGUMENT
        ]
    );
    let mut tier = 0u32;
    assert_eq!(
        unsafe { crabby_ctx_gpu_tier(null(), &mut tier) },
        CRABBY_INVALID_ARGUMENT
    );
}
