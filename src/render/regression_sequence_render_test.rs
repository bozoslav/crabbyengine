//! The behavioural regression sequences: what the renderer must do when
//! something changes between frames. Camera cuts by rotation and teleports
//! (the temporal history is dropped), window resizes (every size draws
//! what a fresh renderer draws), forced capability tiers (plain scenes
//! look the same, missing effects fall back to the frame without them) and
//! shader hot reload (the picture follows the code, a broken reload keeps
//! the last good one). The scenes are `regression_stage.rs`'s; the single
//! frame checks and reference images are in `regression_render_test.rs`.
use super::regression_stage::*;
use super::render_test_support::*;
use crate::prelude::*;
use crate::render::{AntiAliasing, GpuFeature, GpuTier};

/// The edge scene drawn with TAA until it settled, then `step`, then the
/// next frame and why the renderer dropped its history for it. `None`
/// without a GPU or TAA.
fn edge_frame_after(step: impl FnOnce(&mut Stage)) -> Option<(Image, super::gpu3d::HistoryReset)> {
    let mut stage = Stage::new()?;
    if !needs(&stage.device, GpuFeature::TemporalAntialiasing) {
        return None;
    }
    edge(&mut stage);
    stage.ctx.post.anti_aliasing = AntiAliasing::Taa;
    stage.settled(40);
    step(&mut stage);
    let image = stage.image();
    let reset = stage.renderer.history_reset();
    stage.finish();
    Some((image, reset))
}

#[test]
fn a_camera_cut_by_rotation_drops_the_history_like_an_explicit_reset() {
    // The camera turns a quarter turn in one frame to face the slab's
    // twin: the same picture on screen, and nobody calls reset. The
    // renderer must see the turn as a cut and draw what a reset draws (the
    // same jitter phase), not blend with the history of the other view.
    let (Some((turned, why)), Some((reset, _)), Some((kept, kept_why))) = (
        edge_frame_after(face_twin),
        edge_frame_after(|stage| stage.ctx.gfx.reset_temporal_history()),
        edge_frame_after(|_| {}),
    ) else {
        return;
    };
    assert!(why.camera_cut && !why.requested, "the turn is a camera cut: {why:?}");
    assert!(!kept_why.any(), "a still camera keeps its history: {kept_why:?}");
    let tight = Tolerance {
        channel: 4,
        outliers: 0.002,
        edge: 0,
    };
    assert_similar(&turned, &reset, tight, "the frame after the turn against a reset's");
    // Negative control: the settled history draws a visibly smoother edge.
    assert_differs(
        &turned,
        &kept,
        tight,
        "the frame after the turn against the kept history's",
    );
}

#[test]
fn a_small_turn_keeps_the_history_and_a_turn_past_the_threshold_drops_it() {
    // The cut is at an eighth of a turn in one frame.
    let turn = |angle: f32| {
        move |stage: &mut Stage| {
            let (sin, cos) = angle.sin_cos();
            stage.ctx.camera3d.look_at(vec3(-5.0 * sin, 0.0, 5.0 - 5.0 * cos));
        }
    };
    let (Some((_, small)), Some((_, large))) = (edge_frame_after(turn(0.5)), edge_frame_after(turn(1.0))) else {
        return;
    };
    assert!(!small.any(), "half a radian is a pan, not a cut: {small:?}");
    assert!(large.camera_cut, "a radian in one frame is a cut: {large:?}");
}

/// The mirror scene with reflections, temporal ambient occlusion and TAA
/// on, drawn until it settled, then `step`, then the next frame.
fn mirror_frame_after(step: impl FnOnce(&mut Stage)) -> Option<Image> {
    let mut stage = Stage::new()?;
    for feature in [
        GpuFeature::TemporalAntialiasing,
        GpuFeature::ScreenSpaceReflections,
        GpuFeature::AmbientOcclusion,
    ] {
        if !needs(&stage.device, feature) {
            return None;
        }
    }
    mirror(&mut stage);
    stage.ctx.post.anti_aliasing = AntiAliasing::Taa;
    stage.ctx.post.ssr = 1.0;
    stage.ctx.post.ambient_occlusion.enabled = true;
    stage.ctx.post.ao_temporal = true;
    stage.settled(24);
    step(&mut stage);
    let image = stage.image();
    stage.finish();
    Some(image)
}

#[test]
fn a_teleport_with_every_temporal_effect_on_draws_what_an_explicit_reset_draws() {
    // The camera and everything it sees jump 200 units: the same picture,
    // and nobody calls reset. Reflections read the last frame's image,
    // ambient occlusion and TAA their histories: all three must start
    // over, exactly as when the game asks.
    let teleport = |stage: &mut Stage| {
        let jump = vec3(200.0, 0.0, 0.0);
        stage.ctx.camera3d.position += jump;
        stage
            .ctx
            .world
            .query::<&mut crate::d3::Transform>()
            .for_each(|t| t.position += jump);
    };
    let (Some(teleported), Some(reset), Some(kept)) = (
        mirror_frame_after(teleport),
        mirror_frame_after(|stage| stage.ctx.gfx.reset_temporal_history()),
        mirror_frame_after(|_| {}),
    ) else {
        return;
    };
    let tight = Tolerance {
        channel: 6,
        outliers: 0.005,
        edge: 0,
    };
    assert_similar(
        &teleported,
        &reset,
        tight,
        "the frame after the teleport against a reset's",
    );
    // Negative control: with its history the frame shows the reflection
    // the reset frame has not built yet.
    assert_differs(
        &teleported,
        &kept,
        tight,
        "the frame after the teleport against the kept history's",
    );
}

/// The sizes a window goes through: bigger, wide, odd and tall, back, tiny,
/// and large and odd.
const SIZES: [(u32, u32); 6] = [(192, 192), (200, 120), (63, 97), (128, 128), (16, 16), (257, 131)];

/// Draws `scene` through `SIZES` on one stage, three frames each, and
/// checks every frame against a fresh stage's at that size: the first
/// within `SAME_PICTURE` (nothing of the old size is left), the next ones
/// within `later`.
fn resized_frames_match_fresh_ones(what: &str, later: Tolerance, scene: impl Fn(&mut Stage)) {
    let Some(mut stage) = Stage::new() else { return };
    scene(&mut stage);
    stage.settled(3);
    for size in SIZES {
        stage.resize(size);
        let Some(mut fresh) = Stage::with(Setup {
            size,
            ..Default::default()
        }) else {
            return;
        };
        scene(&mut fresh);
        for frame in 0..3 {
            let (resized, new) = (stage.image(), fresh.image());
            assert_eq!((resized.width, resized.height), size, "the frame has the new size");
            assert_similar(
                &resized,
                &new,
                if frame == 0 { SAME_PICTURE } else { later },
                &format!("{what} at {size:?}, frame {frame} after the resize, against a fresh renderer's"),
            );
        }
        fresh.finish();
    }
    stage.finish();
}

#[test]
fn every_size_in_a_resize_sequence_draws_what_a_fresh_renderer_draws() {
    // Shadows, post antialiasing and bloom: targets of every pass are made
    // again at each size, and nothing of the old size shows.
    resized_frames_match_fresh_ones("the shadowed box", SAME_PICTURE, |stage| {
        shadowed(stage);
        stage.ctx.post.anti_aliasing = AntiAliasing::Fxaa;
        stage.ctx.post.bloom = 0.2;
    });
}

#[test]
fn resized_frames_with_screen_space_effects_match_a_fresh_renderers() {
    // Reflections read the last frame's image and ambient occlusion the
    // depth: after a resize both are another size, and must be dropped
    // like a fresh renderer's first frame has none.
    let Some(probe) = Stage::new() else { return };
    let effects = [GpuFeature::ScreenSpaceReflections, GpuFeature::AmbientOcclusion];
    if !effects.into_iter().all(|feature| needs(&probe.device, feature)) {
        return;
    }
    drop(probe);
    // From the second frame on the reflection's ray steps are offset by a
    // noise that moves with the renderer's frame count, which the two
    // renderers don't share: the reflection's edge pixels may differ.
    let noisy_edges = Tolerance {
        channel: 6,
        outliers: 0.04,
        edge: 0,
    };
    resized_frames_match_fresh_ones("the mirror floor", noisy_edges, |stage| {
        mirror(stage);
        stage.ctx.post.ssr = 1.0;
        stage.ctx.post.ambient_occlusion.enabled = true;
        stage.ctx.post.ao_temporal = false;
    });
}

/// A stage on `tier` (`None`: the desktop device), or `None` (skipped,
/// logged) when it can't be opened.
fn on_tier(tier: Option<GpuTier>) -> Option<Stage> {
    Stage::with(Setup {
        tier,
        ..Default::default()
    })
}

const TIERS: [Option<GpuTier>; 3] = [None, Some(GpuTier::Baseline), Some(GpuTier::WebGl2)];

fn tier_name(tier: Option<GpuTier>) -> String {
    tier.map_or_else(|| "desktop".to_owned(), |t| t.to_string())
}

#[test]
fn forced_tiers_draw_the_plain_scenes_like_the_desktop_device() {
    // None of these scenes needs a feature a tier lacks: opaque and
    // see-through models, cutouts, sun shadows and a posed skeleton (posed
    // on the CPU where the tier has no compute shaders) look the same on
    // every tier, and no tier reports a GPU error drawing them.
    type Scene = (&'static str, fn(&mut Stage));
    let scenes: [Scene; 5] = [
        ("opaque", |stage| {
            opaque(stage);
        }),
        ("transparency", |stage| panes(stage, [0, 1, 2])),
        ("cutout", |stage| {
            cutout(stage, HOLES_SHADER);
        }),
        ("shadows", |stage| {
            shadowed(stage);
        }),
        ("skinned", |stage| {
            let bar = skinned(stage);
            bend(stage, bar, std::f32::consts::FRAC_PI_2);
        }),
    ];
    // `PONKO_GPU_TIER` lowers every device this opens to the forced tier:
    // a tier above it would be drawn on the forced one and compared with
    // itself. Only the tiers that stay apart are compared, the highest of
    // them standing in for the desktop device.
    let rank = |tier: Option<GpuTier>| match tier {
        Some(GpuTier::WebGl2) => 0,
        Some(GpuTier::Baseline) => 1,
        _ => 2,
    };
    let forced = forced_tier();
    let apart: Vec<Option<GpuTier>> = TIERS
        .into_iter()
        .filter(|&tier| forced.is_none_or(|f| rank(tier) <= rank(Some(f))))
        .collect();
    let Some((&highest, lower)) = apart.split_first().filter(|(_, lower)| !lower.is_empty()) else {
        skip(format_args!(
            "PONKO_GPU_TIER forces the {} tier on every device, so the tiers would be compared with themselves: \
             run without it (or with a higher tier) to compare them",
            tier_name(forced)
        ));
        return;
    };
    if apart.len() < TIERS.len() {
        eprintln!(
            "PONKO_GPU_TIER forces the {} tier: comparing only the tiers from it down",
            tier_name(forced)
        );
    }
    for (name, scene) in scenes {
        let Some(mut stage) = on_tier(highest) else { return };
        scene(&mut stage);
        let reference = stage.settled(3);
        stage.finish();
        for &tier in lower {
            let Some(mut stage) = on_tier(tier) else { return };
            scene(&mut stage);
            assert_similar(
                &stage.settled(3),
                &reference,
                SAME_PICTURE,
                &format!(
                    "the {name} scene on the {} tier against the {} device",
                    tier_name(tier),
                    tier_name(highest)
                ),
            );
            stage.finish();
        }
    }
}

/// One effect a tier may lack: the scene that shows it, how to turn it on,
/// and what the frame must equal where the device lacks it (`fallback`
/// sets that up on a second stage).
struct Effect {
    feature: GpuFeature,
    scene: fn(&mut Stage),
    on: fn(&mut Stage),
    fallback: fn(&mut Stage),
    frames: usize,
}

const EFFECTS: [Effect; 4] = [
    Effect {
        feature: GpuFeature::ScreenSpaceReflections,
        scene: |stage| {
            mirror(stage);
        },
        on: |stage| stage.ctx.post.ssr = 1.0,
        fallback: |_| {},
        frames: 4,
    },
    Effect {
        feature: GpuFeature::AmbientOcclusion,
        scene: |stage| {
            crease(stage);
        },
        on: |stage| {
            stage.ctx.post.ambient_occlusion.enabled = true;
            stage.ctx.post.ao_temporal = false;
        },
        fallback: |_| {},
        frames: 3,
    },
    Effect {
        feature: GpuFeature::GlobalIllumination,
        scene: |stage| {
            red_room(stage, false);
        },
        on: |stage| stage.ctx.lighting.gi.enabled = true,
        fallback: |_| {},
        frames: 40,
    },
    Effect {
        // Without compute shaders TAA is drawn as FXAA.
        feature: GpuFeature::TemporalAntialiasing,
        scene: edge,
        on: |stage| stage.ctx.post.anti_aliasing = AntiAliasing::Taa,
        fallback: |stage| stage.ctx.post.anti_aliasing = AntiAliasing::Fxaa,
        frames: 12,
    },
];

#[test]
fn an_effect_a_tier_lacks_draws_its_fallback_frame_and_one_it_has_shows() {
    // Asked for on every tier: where the device has the effect the frame
    // changes, where it lacks it the frame is the fallback's (the frame
    // without it, or FXAA for TAA), and no GPU error either way.
    let mut fallbacks = 0;
    for tier in TIERS {
        for effect in &EFFECTS {
            let (Some(mut asked), Some(mut plain)) = (on_tier(tier), on_tier(tier)) else {
                return;
            };
            (effect.scene)(&mut asked);
            (effect.scene)(&mut plain);
            (effect.on)(&mut asked);
            (effect.fallback)(&mut plain);
            let has = available(&asked.device, effect.feature);
            let (with, without) = (asked.settled(effect.frames), plain.settled(effect.frames));
            let what = format!(
                "{:?} asked for on the {} tier (which {} it) against its fallback",
                effect.feature,
                tier_name(tier),
                if has { "has" } else { "lacks" }
            );
            if has {
                assert_differs(&with, &without, SAME_PICTURE, &what);
            } else {
                fallbacks += 1;
                assert_similar(&with, &without, SAME_PICTURE, &what);
            }
            asked.finish();
            plain.finish();
        }
    }
    // The WebGL2 tier has none of the four, whatever the adapter.
    assert!(fallbacks >= EFFECTS.len(), "the fallbacks were drawn: {fallbacks}");
}

#[test]
fn a_reloaded_shader_changes_the_sheet_and_its_shadow_and_the_old_code_brings_them_back() {
    let Some(mut stage) = Stage::new() else { return };
    let (shader, _) = cutout(&mut stage, HOLES_SHADER);
    // A point of the sheet that is solid between two holes and inside a
    // cut-away stripe, and one on the floor, clear of the sheet, in the
    // shadow of such a place.
    let (between_holes, floor) = (STRIPE_NOT_HOLE, vec3(0.3, 0.0, 0.5));
    let before = stage.settled(3);
    let (sheet, shade) = (stage.color_at(&before, between_holes), stage.color_at(&before, floor));
    assert!(sheet[0] > sheet[2] * 2.5, "the sheet is solid yellow there: {sheet:?}");

    stage.reload(shader, "holes.wgsl", STRIPES_SHADER).unwrap();
    let after = stage.settled(3);
    let (cut, lit) = (stage.color_at(&after, between_holes), stage.color_at(&after, floor));
    eprintln!("the sheet {sheet:?} then {cut:?}; the floor under it {shade:?} then {lit:?}");
    assert!((cut[0] - cut[2]).abs() < 12.0, "the stripe is cut out there: {cut:?}");
    // The shadow is drawn by the same shader, and kept between frames: a
    // reload must redraw it.
    assert!(
        luma(lit) > luma(shade) * 1.5,
        "the shadow has the stripe too: {lit:?} vs {shade:?}"
    );
    assert_differs(&after, &before, SAME_PICTURE, "the stripes against the holes");

    stage.reload(shader, "holes.wgsl", HOLES_SHADER).unwrap();
    assert_similar(
        &stage.settled(3),
        &before,
        SAME_PICTURE,
        "the holes reloaded against the first holes",
    );
    stage.finish();
}

#[test]
fn a_failed_reload_keeps_drawing_the_last_good_picture() {
    let Some(mut stage) = Stage::new() else { return };
    let (shader, _) = cutout(&mut stage, HOLES_SHADER);
    let before = stage.settled(3);
    // A syntax error, then code that compiles but returns the wrong type.
    for broken in [
        "fn surface(s: Surface) -> Surface { var o = s; o.alpha = ; return o; }",
        "fn surface(s: Surface) -> f32 { return 1.0; }",
    ] {
        let error = stage.reload(shader, "holes.wgsl", broken).unwrap_err();
        eprintln!("rejected: {error}");
        assert!(error.contains("holes.wgsl"), "the error names the file: {error}");
        assert_similar(
            &stage.settled(2),
            &before,
            Tolerance {
                channel: 0,
                outliers: 0.0,
                edge: 0,
            },
            "the frame after a failed reload against the one before it",
        );
    }
    // The device is still usable: a good reload takes effect.
    stage.reload(shader, "holes.wgsl", STRIPES_SHADER).unwrap();
    assert_differs(
        &stage.settled(3),
        &before,
        SAME_PICTURE,
        "the fixed shader against the old one",
    );
    stage.finish();
}

#[test]
fn a_reload_while_taa_is_settled_shows_the_new_shader_without_a_trail_of_the_old() {
    let Some(mut stage) = Stage::new() else { return };
    if !needs(&stage.device, GpuFeature::TemporalAntialiasing) {
        return;
    }
    let (shader, _) = cutout(&mut stage, HOLES_SHADER);
    let between_holes = STRIPE_NOT_HOLE;
    stage.ctx.post.anti_aliasing = AntiAliasing::Taa;
    stage.settled(24);
    stage.reload(shader, "holes.wgsl", STRIPES_SHADER).unwrap();
    // A few frames on, the history has let go of the old sheet...
    let image = stage.settled(12);
    let cut = stage.color_at(&image, between_holes);
    assert!(
        (cut[0] - cut[2]).abs() < 14.0,
        "no yellow left in the new stripe: {cut:?}"
    );
    // ...and the picture is the one a stage that always had the stripes
    // settles on.
    let Some(mut always) = Stage::new() else { return };
    cutout(&mut always, STRIPES_SHADER);
    always.ctx.post.anti_aliasing = AntiAliasing::Taa;
    always.settled(24);
    assert_similar(
        &stage.settled(24),
        &always.settled(12),
        Tolerance {
            channel: 10,
            outliers: 0.02,
            edge: 0,
        },
        "the reloaded stripes under TAA against stripes from the start",
    );
    always.finish();
    stage.finish();
}
