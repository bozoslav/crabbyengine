//! Animation graphs and animators from C: motions, the graph builder,
//! animators on skeleton entities, layers and root motion. The clips come from
//! a small glTF file written to the temp folder, and everything runs headless
//! with the animator updated by hand (`crabby_ctx_animator_update`).
use ponko_ffi::*;
use std::ffi::{CString, c_void};
use std::ptr::{null, null_mut};

use CrabbyAnimationCondition::*;
use CrabbyStatus::*;

const OK: CrabbyStatus = CRABBY_OK;
const BAD: CrabbyStatus = CRABBY_INVALID_ARGUMENT;

/// A skeleton of two joints (`root`, then `arm` under it, with a triangle on
/// it) and two clips of one second: `slide` moves `root` from x 0 to x 2,
/// `idle` holds it at x 5.
fn gltf() -> String {
    let mut bytes = Vec::new();
    let mut floats = |v: &[f32]| v.iter().for_each(|f| bytes.extend(f.to_le_bytes()));
    floats(&[0., 0., 0., 1., 0., 0., 0., 1., 0.]); // 0: the triangle
    floats(&[0., 1.]); // 36: key times
    floats(&[0., 0., 0., 2., 0., 0.]); // 44: slide
    floats(&[5., 0., 0., 5., 0., 0.]); // 68: idle
    format!(
        r#"{{"asset": {{"version": "2.0"}}, "scene": 0, "scenes": [{{"nodes": [0]}}],
        "nodes": [{{"name": "root", "children": [1]}}, {{"name": "arm", "mesh": 0, "translation": [0, 1, 0]}}],
        "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0}}}}]}}],
        "animations": [
            {{"name": "slide", "samplers": [{{"input": 1, "output": 2, "interpolation": "LINEAR"}}],
              "channels": [{{"sampler": 0, "target": {{"node": 0, "path": "translation"}}}}]}},
            {{"name": "idle", "samplers": [{{"input": 1, "output": 3, "interpolation": "LINEAR"}}],
              "channels": [{{"sampler": 0, "target": {{"node": 0, "path": "translation"}}}}]}}],
        "buffers": [{{"byteLength": 92, "uri": "data:application/octet-stream;base64,{}"}}],
        "bufferViews": [{{"buffer": 0, "byteLength": 36}}, {{"buffer": 0, "byteOffset": 36, "byteLength": 8}},
            {{"buffer": 0, "byteOffset": 44, "byteLength": 24}}, {{"buffer": 0, "byteOffset": 68, "byteLength": 24}}],
        "accessors": [
            {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0, 0, 0], "max": [1, 1, 0]}},
            {{"bufferView": 1, "componentType": 5126, "count": 2, "type": "SCALAR", "min": [0], "max": [1]}},
            {{"bufferView": 2, "componentType": 5126, "count": 2, "type": "VEC3"}},
            {{"bufferView": 3, "componentType": 5126, "count": 2, "type": "VEC3"}}]}}"#,
        base64(&bytes)
    )
}

fn base64(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

struct Run {
    path: CString,
}

/// A float parameter 0, a bool parameter 1 and a trigger parameter 2.
unsafe fn builder_with_parameters() -> *mut CrabbyAnimationGraphBuilder {
    unsafe {
        let b = crabby_animation_graph_builder_new();
        let mut i = 99;
        assert_eq!(crabby_animation_graph_add_float(b, 0.0, &mut i), OK);
        assert_eq!(i, 0);
        assert_eq!(crabby_animation_graph_add_bool(b, false, &mut i), OK);
        assert_eq!(i, 1);
        assert_eq!(crabby_animation_graph_add_trigger(b, &mut i), OK);
        assert_eq!(i, 2);
        b
    }
}

unsafe fn clip_motion(clip: *const CrabbyClip) -> *mut CrabbyMotion {
    let mut m = null_mut();
    assert_eq!(unsafe { crabby_motion_clip(clip, &mut m) }, OK);
    m
}

/// State 0 plays `idle`, state 1 `slide`; one transition 0 -> 1 with the one
/// condition, no fade. Starts in state 0.
unsafe fn two_states(
    idle: *const CrabbyClip,
    slide: *const CrabbyClip,
    kind: CrabbyAnimationCondition,
    parameter: usize,
    value: f32,
) -> *mut CrabbyAnimationGraph {
    unsafe {
        let b = builder_with_parameters();
        let (mut s, mut t) = (99, 99);
        assert_eq!(
            crabby_animation_graph_add_state(b, clip_motion(idle), 1.0, -1, true, &mut s),
            OK
        );
        assert_eq!(s, 0);
        assert_eq!(
            crabby_animation_graph_add_state(b, clip_motion(slide), 1.0, -1, true, &mut s),
            OK
        );
        assert_eq!(s, 1);
        assert_eq!(
            crabby_animation_graph_add_transition(b, 0, 1, -1.0, 0.0, 0.0, &mut t),
            OK
        );
        assert_eq!(t, 0);
        assert_eq!(crabby_animation_graph_add_condition(b, t, kind, parameter, value), OK);
        let mut g = null_mut();
        assert_eq!(crabby_animation_graph_build(b, 0, &mut g), OK);
        g
    }
}

unsafe fn state_of(ctx: *mut CrabbyCtx, e: u64) -> (usize, f32) {
    let (mut s, mut t) = (99, f32::NAN);
    assert_eq!(unsafe { crabby_ctx_animator_state(ctx, e, &mut s, &mut t) }, OK);
    (s, t)
}

unsafe fn root_x(ctx: *mut CrabbyCtx, e: u64) -> f32 {
    let mut p = std::mem::MaybeUninit::<CrabbyTransform3D>::uninit();
    assert_eq!(unsafe { crabby_ctx_joint_pose(ctx, e, 0, p.as_mut_ptr()) }, OK);
    unsafe { p.assume_init() }.position.x
}

/// A fresh entity with the file's skeleton, and its animator on `graph`.
unsafe fn rigged(ctx: *mut CrabbyCtx, run: &Run, graph: *const CrabbyAnimationGraph) -> u64 {
    unsafe {
        let w = crabby_ctx_world(ctx);
        let mut transform = 0u32;
        assert_eq!(crabby_transform3d_component(w, &mut transform), OK);
        let t = crabby_transform3d_at(CrabbyVec3 { x: 0.0, y: 0.0, z: 0.0 });
        let part = CrabbyValue {
            component: transform,
            data: (&raw const t).cast(),
            size: size_of::<CrabbyTransform3D>(),
        };
        let mut e = 0u64;
        assert_eq!(crabby_spawn(w, &part, 1, &mut e), OK);
        assert_eq!(crabby_ctx_load_skeleton(ctx, e, run.path.as_ptr()), OK);
        assert_eq!(crabby_ctx_set_animator(ctx, e, graph), OK);
        assert_eq!(crabby_ctx_animator_set_manual(ctx, e, true), OK);
        e
    }
}

fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-4, "{a} != {b}");
}

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let run = &*user.cast::<Run>();
        let (mut idle, mut slide) = (null_mut(), null_mut());
        assert_eq!(
            crabby_ctx_load_clip(ctx, run.path.as_ptr(), c"idle".as_ptr(), &mut idle),
            OK
        );
        assert_eq!(
            crabby_ctx_load_clip(ctx, run.path.as_ptr(), c"slide".as_ptr(), &mut slide),
            OK
        );
        near(crabby_clip_duration(slide), 1.0);

        conditions_drive_transitions(ctx, run, idle, slide);
        crossfades_and_events(ctx, run, idle, slide);
        speed_looping_and_blend_trees(ctx, run, idle, slide);
        layers(ctx, run, idle, slide);
        root_motion(ctx, run, idle, slide);
        bad_arguments(ctx, run, idle, slide);

        crabby_clip_free(idle);
        crabby_clip_free(slide);
    }
}

unsafe fn conditions_drive_transitions(
    ctx: *mut CrabbyCtx,
    run: &Run,
    idle: *const CrabbyClip,
    slide: *const CrabbyClip,
) {
    unsafe {
        // A float above a value.
        let g = two_states(idle, slide, CRABBY_CONDITION_GREATER, 0, 0.5);
        let e = rigged(ctx, run, g);
        assert_eq!(crabby_ctx_animator_set_float(ctx, e, 0, 0.4), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.25), OK);
        assert_eq!(state_of(ctx, e).0, 0, "0.4 is not above 0.5");
        near(root_x(ctx, e), 5.0);
        assert_eq!(crabby_ctx_animator_set_float(ctx, e, 0, 0.6), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.5), OK);
        let (s, t) = state_of(ctx, e);
        assert_eq!(s, 1);
        near(t, 0.5);
        near(root_x(ctx, e), 1.0);

        // A float below a value; the first matching transition wins.
        let g2 = two_states(idle, slide, CRABBY_CONDITION_LESS, 0, 1.0);
        let e2 = rigged(ctx, run, g2);
        assert_eq!(crabby_ctx_animator_set_float(ctx, e2, 0, 2.0), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e2, 0.1), OK);
        assert_eq!(state_of(ctx, e2).0, 0);
        assert_eq!(crabby_ctx_animator_set_float(ctx, e2, 0, 0.5), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e2, 0.1), OK);
        assert_eq!(state_of(ctx, e2).0, 1);

        // A bool.
        let g3 = two_states(idle, slide, CRABBY_CONDITION_BOOL, 1, 1.0);
        let e3 = rigged(ctx, run, g3);
        assert_eq!(crabby_ctx_animator_update(ctx, e3, 0.1), OK);
        assert_eq!(state_of(ctx, e3).0, 0);
        assert_eq!(crabby_ctx_animator_set_bool(ctx, e3, 1, true), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e3, 0.1), OK);
        assert_eq!(state_of(ctx, e3).0, 1);

        // A trigger fires once and is consumed by the transition it fires.
        let g4 = two_states(idle, slide, CRABBY_CONDITION_TRIGGER, 2, 0.0);
        let e4 = rigged(ctx, run, g4);
        assert_eq!(crabby_ctx_animator_update(ctx, e4, 0.1), OK);
        assert_eq!(state_of(ctx, e4).0, 0);
        assert_eq!(crabby_ctx_animator_set_trigger(ctx, e4, 2, true), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e4, 0.1), OK);
        assert_eq!(state_of(ctx, e4).0, 1);
        // Back to state 0: it stays there, the trigger was used up.
        assert_eq!(crabby_ctx_animator_crossfade(ctx, e4, 0, 0.0, 0.0), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e4, 0.1), OK);
        assert_eq!(state_of(ctx, e4).0, 0);
        // A trigger can be cleared before it fires.
        assert_eq!(crabby_ctx_animator_set_trigger(ctx, e4, 2, true), OK);
        assert_eq!(crabby_ctx_animator_set_trigger(ctx, e4, 2, false), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e4, 0.1), OK);
        assert_eq!(state_of(ctx, e4).0, 0);

        // The graph is shared: freeing our share keeps the animators working.
        crabby_animation_graph_free(g4);
        assert_eq!(crabby_ctx_animator_set_trigger(ctx, e4, 2, true), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e4, 0.1), OK);
        assert_eq!(state_of(ctx, e4).0, 1);
        crabby_animation_graph_free(g);
        crabby_animation_graph_free(g2);
        crabby_animation_graph_free(g3);
    }
}

unsafe fn crossfades_and_events(ctx: *mut CrabbyCtx, run: &Run, idle: *const CrabbyClip, slide: *const CrabbyClip) {
    unsafe {
        // Two states; `slide` has events at 0.5 ("mid") and 0.25 ("early"),
        // reported in time order. Starts in state 1.
        let b = crabby_animation_graph_builder_new();
        let mut s = 0;
        assert_eq!(
            crabby_animation_graph_add_state(b, clip_motion(idle), 1.0, -1, true, &mut s),
            OK
        );
        assert_eq!(
            crabby_animation_graph_add_state(b, clip_motion(slide), 1.0, -1, true, &mut s),
            OK
        );
        assert_eq!(s, 1);
        assert_eq!(crabby_animation_graph_add_event(b, 1, 0.5, c"mid".as_ptr()), OK);
        assert_eq!(crabby_animation_graph_add_event(b, 1, 0.25, c"early".as_ptr()), OK);
        let mut g = null_mut();
        assert_eq!(crabby_animation_graph_build(b, 1, &mut g), OK);
        let e = rigged(ctx, run, g);
        assert_eq!(state_of(ctx, e), (1, 0.0));

        let mut name = [0 as std::ffi::c_char; 16];
        let mut found = true;
        assert_eq!(
            crabby_ctx_animator_next_event(ctx, e, name.as_mut_ptr(), 16, &mut found),
            OK
        );
        assert!(!found, "no events yet");
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.6), OK);
        for expected in [c"early", c"mid"] {
            assert_eq!(
                crabby_ctx_animator_next_event(ctx, e, name.as_mut_ptr(), 16, &mut found),
                OK
            );
            assert!(found);
            assert_eq!(std::ffi::CStr::from_ptr(name.as_ptr()), expected);
        }
        assert_eq!(
            crabby_ctx_animator_next_event(ctx, e, name.as_mut_ptr(), 16, &mut found),
            OK
        );
        assert!(!found, "both were taken");
        // The clock wraps: the next pass reports them again. A short buffer
        // gets the name cut to fit, still NUL-terminated.
        assert_eq!(crabby_ctx_animator_update(ctx, e, 1.0), OK);
        let mut short = [0x7f as std::ffi::c_char; 4];
        assert_eq!(
            crabby_ctx_animator_next_event(ctx, e, short.as_mut_ptr(), 3, &mut found),
            OK
        );
        assert!(found);
        assert_eq!(std::ffi::CStr::from_ptr(short.as_ptr()), c"ea");
        assert_eq!(short[3], 0x7f, "nothing written past the capacity");
        // No buffer at all still takes the event.
        assert_eq!(crabby_ctx_animator_next_event(ctx, e, null_mut(), 0, &mut found), OK);
        assert!(found);
        assert_eq!(crabby_ctx_animator_next_event(ctx, e, null_mut(), 0, &mut found), OK);
        assert!(!found);

        // A jump with an offset: normalized time starts at the offset.
        assert_eq!(crabby_ctx_animator_crossfade(ctx, e, 0, 0.0, 0.0), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
        assert_eq!(state_of(ctx, e), (0, 0.0));
        near(root_x(ctx, e), 5.0);
        assert_eq!(crabby_ctx_animator_crossfade(ctx, e, 1, 0.0, 0.25), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
        let (state, t) = state_of(ctx, e);
        assert_eq!(state, 1);
        near(t, 0.25);
        near(root_x(ctx, e), 0.5);

        // A timed fade mixes the old pose (x 5, frozen... it keeps advancing)
        // with the new one, so halfway through it is between the two.
        assert_eq!(crabby_ctx_animator_crossfade(ctx, e, 0, 1.0, 0.0), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.5), OK);
        let x = root_x(ctx, e);
        assert!(x > 1.0 && x < 5.0, "halfway through a fade: {x}");
        assert_eq!(state_of(ctx, e).0, 0, "state reports the one being faded to");
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.6), OK);
        near(root_x(ctx, e), 5.0);

        crabby_animation_graph_free(g);
    }
}

unsafe fn speed_looping_and_blend_trees(
    ctx: *mut CrabbyCtx,
    run: &Run,
    idle: *const CrabbyClip,
    slide: *const CrabbyClip,
) {
    unsafe {
        // Speed 2: a looping state wraps (normalized time counts full plays),
        // a non-looping one holds its last frame.
        for (looping, expected_time, expected_x) in [(true, 1.5, 1.0), (false, 1.0, 2.0)] {
            let b = crabby_animation_graph_builder_new();
            assert_eq!(
                crabby_animation_graph_add_state(b, clip_motion(slide), 2.0, -1, looping, null_mut()),
                OK
            );
            let mut g = null_mut();
            assert_eq!(crabby_animation_graph_build(b, 0, &mut g), OK);
            let e = rigged(ctx, run, g);
            assert_eq!(crabby_ctx_animator_update(ctx, e, 0.75), OK);
            near(state_of(ctx, e).1, expected_time);
            near(root_x(ctx, e), expected_x);
            crabby_animation_graph_free(g);
        }

        // A float parameter scales a state's speed.
        let b = crabby_animation_graph_builder_new();
        let mut p = 0;
        assert_eq!(crabby_animation_graph_add_float(b, 0.5, &mut p), OK);
        assert_eq!(
            crabby_animation_graph_add_state(b, clip_motion(slide), 1.0, p as i64, true, null_mut()),
            OK
        );
        let mut g = null_mut();
        assert_eq!(crabby_animation_graph_build(b, 0, &mut g), OK);
        let e = rigged(ctx, run, g);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.5), OK);
        near(state_of(ctx, e).1, 0.25);
        assert_eq!(crabby_ctx_animator_set_float(ctx, e, 0, 2.0), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.25), OK);
        near(state_of(ctx, e).1, 0.75);
        crabby_animation_graph_free(g);

        // A 1D blend tree: idle (x 5) at 0, slide (x 0 at its start) at 1.
        let b = crabby_animation_graph_builder_new();
        let mut p = 0;
        assert_eq!(crabby_animation_graph_add_float(b, 0.0, &mut p), OK);
        let mut tree = null_mut();
        assert_eq!(crabby_motion_blend1d(p, &mut tree), OK);
        assert_eq!(crabby_motion_blend1d_add(tree, 0.0, clip_motion(idle), 1.0), OK);
        assert_eq!(crabby_motion_blend1d_add(tree, 1.0, clip_motion(slide), 1.0), OK);
        assert_eq!(crabby_animation_graph_add_state(b, tree, 1.0, -1, true, null_mut()), OK);
        let mut g = null_mut();
        assert_eq!(crabby_animation_graph_build(b, 0, &mut g), OK);
        let e = rigged(ctx, run, g);
        for (value, x) in [(0.0, 5.0), (0.5, 2.5), (1.0, 0.0)] {
            assert_eq!(crabby_ctx_animator_set_float(ctx, e, 0, value), OK);
            assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
            near(root_x(ctx, e), x);
        }
        crabby_animation_graph_free(g);

        // A 2D blend tree: exact at its points, between them in the middle.
        let b = crabby_animation_graph_builder_new();
        let (mut px, mut py) = (0, 0);
        assert_eq!(crabby_animation_graph_add_float(b, 0.0, &mut px), OK);
        assert_eq!(crabby_animation_graph_add_float(b, 0.0, &mut py), OK);
        let mut tree = null_mut();
        assert_eq!(crabby_motion_blend2d(px, py, &mut tree), OK);
        assert_eq!(crabby_motion_blend2d_add(tree, 0.0, 0.0, clip_motion(idle), 1.0), OK);
        assert_eq!(crabby_motion_blend2d_add(tree, 1.0, 0.0, clip_motion(slide), 1.0), OK);
        assert_eq!(crabby_animation_graph_add_state(b, tree, 1.0, -1, true, null_mut()), OK);
        let mut g = null_mut();
        assert_eq!(crabby_animation_graph_build(b, 0, &mut g), OK);
        let e = rigged(ctx, run, g);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
        near(root_x(ctx, e), 5.0);
        assert_eq!(crabby_ctx_animator_set_float(ctx, e, 0, 1.0), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
        near(root_x(ctx, e), 0.0);
        assert_eq!(crabby_ctx_animator_set_float(ctx, e, 0, 0.5), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
        let x = root_x(ctx, e);
        assert!(x > 0.1 && x < 4.9, "between the two clips: {x}");
        crabby_animation_graph_free(g);
    }
}

unsafe fn layers(ctx: *mut CrabbyCtx, run: &Run, idle: *const CrabbyClip, slide: *const CrabbyClip) {
    unsafe {
        // The base animator plays `slide`; the layer's graph starts on `idle`
        // (x 5) and moves to `slide` on its own parameters.
        let base = crabby_animation_graph_builder_new();
        assert_eq!(
            crabby_animation_graph_add_state(base, clip_motion(slide), 1.0, -1, true, null_mut()),
            OK
        );
        let mut base_graph = null_mut();
        assert_eq!(crabby_animation_graph_build(base, 0, &mut base_graph), OK);
        let top = two_states(idle, slide, CRABBY_CONDITION_GREATER, 0, 0.5);
        let e = rigged(ctx, run, base_graph);

        let mut layer = 99;
        assert_eq!(
            crabby_ctx_animator_add_layer(
                ctx,
                e,
                top,
                CrabbyLayerBlend::CRABBY_LAYER_OVERRIDE as u32,
                null(),
                &mut layer
            ),
            OK
        );
        assert_eq!(layer, 0);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.5), OK);
        near(root_x(ctx, e), 5.0); // full weight: the layer's pose

        assert_eq!(crabby_ctx_animator_set_layer_weight(ctx, e, 0, 0.0), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
        near(root_x(ctx, e), 1.0); // the base's pose, at 0.5 s
        assert_eq!(crabby_ctx_animator_set_layer_weight(ctx, e, 0, 0.5), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
        near(root_x(ctx, e), 3.0); // halfway
        // Weights are clamped to 0..1.
        assert_eq!(crabby_ctx_animator_set_layer_weight(ctx, e, 0, 7.0), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
        near(root_x(ctx, e), 5.0);

        // The layer's own parameters and crossfades: a float above 0.5 moves
        // it from `idle` to `slide`, whose clock then runs on.
        assert_eq!(crabby_ctx_animator_layer_set_float(ctx, e, 0, 0, 0.9), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
        near(root_x(ctx, e), 0.0);
        assert_eq!(crabby_ctx_animator_layer_crossfade(ctx, e, 0, 0, 0.0, 0.0), OK);
        assert_eq!(crabby_ctx_animator_layer_set_float(ctx, e, 0, 0, 0.0), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
        near(root_x(ctx, e), 5.0);

        // Bool and trigger parameters of a layer's graph.
        for (kind, parameter) in [(CRABBY_CONDITION_BOOL, 1), (CRABBY_CONDITION_TRIGGER, 2)] {
            let g = two_states(idle, slide, kind, parameter, 1.0);
            let e = rigged(ctx, run, base_graph);
            assert_eq!(crabby_ctx_animator_add_layer(ctx, e, g, 0, null(), null_mut()), OK);
            assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
            near(root_x(ctx, e), 5.0);
            if parameter == 1 {
                assert_eq!(crabby_ctx_animator_layer_set_bool(ctx, e, 0, 1, true), OK);
            } else {
                assert_eq!(crabby_ctx_animator_layer_set_trigger(ctx, e, 0, 2, true), OK);
            }
            assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK);
            near(root_x(ctx, e), 0.0);
            crabby_animation_graph_free(g);
        }

        // A layer limited to `arm` and its children leaves `root` alone.
        let e = rigged(ctx, run, base_graph);
        assert_eq!(
            crabby_ctx_animator_add_layer(ctx, e, top, 0, c"arm".as_ptr(), &mut layer),
            OK
        );
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.5), OK);
        near(root_x(ctx, e), 1.0);

        // Errors.
        assert_eq!(
            crabby_ctx_animator_add_layer(ctx, e, top, 0, c"tail".as_ptr(), null_mut()),
            BAD
        );
        assert_eq!(crabby_ctx_animator_add_layer(ctx, e, top, 9, null(), null_mut()), BAD);
        assert_eq!(
            crabby_ctx_animator_add_layer(ctx, e, null(), 0, null(), null_mut()),
            BAD
        );
        assert_eq!(crabby_ctx_animator_set_layer_weight(ctx, e, 5, 0.5), BAD);
        assert_eq!(crabby_ctx_animator_set_layer_weight(ctx, e, 0, f32::NAN), BAD);
        assert_eq!(crabby_ctx_animator_layer_set_float(ctx, e, 5, 0, 1.0), BAD);
        assert_eq!(crabby_ctx_animator_layer_set_bool(ctx, e, 5, 1, true), BAD);
        assert_eq!(crabby_ctx_animator_layer_set_trigger(ctx, e, 5, 2, true), BAD);
        assert_eq!(crabby_ctx_animator_layer_crossfade(ctx, e, 5, 0, 0.0, 0.0), BAD);
        // A parameter of the wrong kind or index, and bad crossfade values,
        // are invalid arguments (not engine panics) for a layer too.
        assert_eq!(crabby_ctx_animator_layer_set_float(ctx, e, 0, 1, 1.0), BAD, "a bool");
        assert_eq!(
            crabby_ctx_animator_layer_set_float(ctx, e, 0, 9, 1.0),
            BAD,
            "no such parameter"
        );
        assert_eq!(crabby_ctx_animator_layer_set_float(ctx, e, 0, 0, f32::NAN), BAD);
        assert_eq!(crabby_ctx_animator_layer_set_bool(ctx, e, 0, 0, true), BAD, "a float");
        assert_eq!(crabby_ctx_animator_layer_set_bool(ctx, e, 0, 2, true), BAD, "a trigger");
        assert_eq!(crabby_ctx_animator_layer_set_trigger(ctx, e, 0, 1, true), BAD, "a bool");
        assert_eq!(crabby_ctx_animator_layer_set_trigger(ctx, e, 0, 9, true), BAD);
        assert_eq!(
            crabby_ctx_animator_layer_crossfade(ctx, e, 0, 7, 0.0, 0.0),
            BAD,
            "no state 7"
        );
        assert_eq!(crabby_ctx_animator_layer_crossfade(ctx, e, 0, 0, -1.0, 0.0), BAD);
        assert_eq!(crabby_ctx_animator_layer_crossfade(ctx, e, 0, 0, f32::NAN, 0.0), BAD);
        assert_eq!(crabby_ctx_animator_layer_crossfade(ctx, e, 0, 0, 0.1, -0.5), BAD);
        assert_eq!(
            crabby_ctx_animator_layer_crossfade(ctx, e, 0, 0, 0.1, f32::INFINITY),
            BAD
        );

        crabby_animation_graph_free(top);
        crabby_animation_graph_free(base_graph);
    }
}

unsafe fn root_motion(ctx: *mut CrabbyCtx, run: &Run, idle: *const CrabbyClip, slide: *const CrabbyClip) {
    unsafe {
        let _ = idle;
        let b = crabby_animation_graph_builder_new();
        assert_eq!(
            crabby_animation_graph_add_state(b, clip_motion(slide), 1.0, -1, true, null_mut()),
            OK
        );
        let mut g = null_mut();
        assert_eq!(crabby_animation_graph_build(b, 0, &mut g), OK);
        let e = rigged(ctx, run, g);
        let (mut p, mut q) = (
            CrabbyVec3 { x: 9.0, y: 9.0, z: 9.0 },
            CrabbyQuat {
                x: 9.0,
                y: 9.0,
                z: 9.0,
                w: 9.0,
            },
        );

        // Off (the default): the pose keeps the movement and there is no delta.
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.5), OK);
        near(root_x(ctx, e), 1.0);
        assert_eq!(crabby_ctx_animator_root_motion(ctx, e, &mut p, &mut q), OK);
        near(p.x, 0.0);
        near(q.w, 1.0);

        // Extract: the horizontal movement leaves the pose and is reported.
        assert_eq!(
            crabby_ctx_animator_set_root_motion(ctx, e, CrabbyRootMotion::CRABBY_ROOT_MOTION_EXTRACT as u32, 0),
            OK
        );
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.25), OK);
        near(root_x(ctx, e), 0.0);
        assert_eq!(crabby_ctx_animator_root_motion(ctx, e, &mut p, &mut q), OK);
        near(p.x, 0.5); // from x 1.0 at 0.5 s to x 1.5 at 0.75 s
        near(p.y, 0.0);
        near(p.z, 0.0);
        near(q.w, 1.0);
        // Either output may be null.
        assert_eq!(crabby_ctx_animator_root_motion(ctx, e, null_mut(), null_mut()), OK);
        assert_eq!(crabby_ctx_animator_root_motion(ctx, e, &mut p, null_mut()), OK);
        assert_eq!(crabby_ctx_animator_root_motion(ctx, e, null_mut(), &mut q), OK);

        // Apply is accepted too; unknown modes are not.
        assert_eq!(
            crabby_ctx_animator_set_root_motion(ctx, e, CrabbyRootMotion::CRABBY_ROOT_MOTION_APPLY as u32, 0),
            OK
        );
        assert_eq!(crabby_ctx_animator_set_root_motion(ctx, e, 3, 0), BAD);
        crabby_animation_graph_free(g);
    }
}

unsafe fn bad_arguments(ctx: *mut CrabbyCtx, run: &Run, idle: *const CrabbyClip, slide: *const CrabbyClip) {
    unsafe {
        // Motions: nulls, wrong tree kinds and the output pointer.
        let mut m = null_mut();
        assert_eq!(crabby_motion_clip(null(), &mut m), BAD);
        assert_eq!(crabby_motion_clip(idle, null_mut()), BAD);
        assert_eq!(crabby_motion_blend1d(0, null_mut()), BAD);
        assert_eq!(crabby_motion_blend2d(0, 1, null_mut()), BAD);
        let mut tree1 = null_mut();
        let mut tree2 = null_mut();
        assert_eq!(crabby_motion_blend1d(0, &mut tree1), OK);
        assert_eq!(crabby_motion_blend2d(0, 1, &mut tree2), OK);
        // The child is taken over even when the call fails.
        assert_eq!(crabby_motion_blend1d_add(tree2, 0.0, clip_motion(idle), 1.0), BAD);
        assert_eq!(crabby_motion_blend2d_add(tree1, 0.0, 0.0, clip_motion(idle), 1.0), BAD);
        assert_eq!(crabby_motion_blend1d_add(null_mut(), 0.0, clip_motion(idle), 1.0), BAD);
        assert_eq!(
            crabby_motion_blend2d_add(null_mut(), 0.0, 0.0, clip_motion(idle), 1.0),
            BAD
        );
        assert_eq!(crabby_motion_blend1d_add(tree1, 0.0, null_mut(), 1.0), BAD);
        assert_eq!(crabby_motion_blend2d_add(tree2, 0.0, 0.0, null_mut(), 1.0), BAD);
        crabby_motion_free(tree1);
        crabby_motion_free(tree2);
        crabby_motion_free(null_mut());

        // Builders: nulls and out-of-range indices.
        let b = builder_with_parameters();
        assert_eq!(
            crabby_animation_graph_add_state(b, null_mut(), 1.0, -1, true, null_mut()),
            BAD
        );
        assert_eq!(
            crabby_animation_graph_add_state(null_mut(), clip_motion(idle), 1.0, -1, true, null_mut()),
            BAD
        );
        assert_eq!(
            crabby_animation_graph_add_event(b, 0, 0.5, c"x".as_ptr()),
            BAD,
            "no state 0 yet"
        );
        assert_eq!(
            crabby_animation_graph_add_state(b, clip_motion(idle), 1.0, -1, true, null_mut()),
            OK
        );
        assert_eq!(crabby_animation_graph_add_event(b, 0, 0.5, null()), BAD);
        assert_eq!(crabby_animation_graph_add_event(null_mut(), 0, 0.5, c"x".as_ptr()), BAD);
        assert_eq!(
            crabby_animation_graph_add_condition(b, 0, CRABBY_CONDITION_TRIGGER, 2, 0.0),
            BAD,
            "no transition 0 yet"
        );
        assert_eq!(
            crabby_animation_graph_add_condition(null_mut(), 0, CRABBY_CONDITION_TRIGGER, 2, 0.0),
            BAD
        );
        assert_eq!(crabby_animation_graph_add_float(null_mut(), 0.0, null_mut()), BAD);
        // A transition to a state that doesn't exist, or a condition on the
        // wrong kind of parameter, fails the build (and frees the builder).
        assert_eq!(
            crabby_animation_graph_add_transition(b, -1, 7, -1.0, 0.0, 0.0, null_mut()),
            OK
        );
        let mut g = null_mut();
        assert_ne!(crabby_animation_graph_build(b, 0, &mut g), OK);
        assert!(g.is_null());
        let b = builder_with_parameters();
        assert_eq!(
            crabby_animation_graph_add_state(b, clip_motion(idle), 1.0, -1, true, null_mut()),
            OK
        );
        assert_eq!(
            crabby_animation_graph_add_transition(b, 0, 0, -1.0, 0.0, 0.0, null_mut()),
            OK
        );
        assert_eq!(
            crabby_animation_graph_add_condition(b, 0, CRABBY_CONDITION_GREATER, 1, 0.5),
            OK
        );
        assert_ne!(crabby_animation_graph_build(b, 0, &mut g), OK, "parameter 1 is a bool");
        // No states, a bad initial state, and a null output.
        let b = crabby_animation_graph_builder_new();
        assert_ne!(crabby_animation_graph_build(b, 0, &mut g), OK);
        let b = crabby_animation_graph_builder_new();
        assert_eq!(
            crabby_animation_graph_add_state(b, clip_motion(idle), 1.0, -1, true, null_mut()),
            OK
        );
        assert_ne!(crabby_animation_graph_build(b, 3, &mut g), OK);
        assert_eq!(crabby_animation_graph_build(null_mut(), 0, &mut g), BAD);
        let b = crabby_animation_graph_builder_new();
        assert_eq!(crabby_animation_graph_build(b, 0, null_mut()), BAD);

        // A builder that is never built is freed on its own (null is fine).
        let unused = builder_with_parameters();
        crabby_animation_graph_builder_free(unused);
        crabby_animation_graph_builder_free(null_mut());

        // Animators: no graph, no skeleton, no animator, dead entity.
        let g = two_states(idle, slide, CRABBY_CONDITION_TRIGGER, 2, 0.0);
        let e = rigged(ctx, run, g);
        assert_eq!(crabby_ctx_set_animator(ctx, e, null()), BAD);
        assert_eq!(crabby_ctx_set_animator(ctx, 12345, g), BAD);
        let w = crabby_ctx_world(ctx);
        let mut transform = 0u32;
        assert_eq!(crabby_transform3d_component(w, &mut transform), OK);
        let t = crabby_transform3d_at(CrabbyVec3 { x: 0.0, y: 0.0, z: 0.0 });
        let part = CrabbyValue {
            component: transform,
            data: (&raw const t).cast(),
            size: size_of::<CrabbyTransform3D>(),
        };
        let mut bare = 0u64;
        assert_eq!(crabby_spawn(w, &part, 1, &mut bare), OK);
        assert_eq!(crabby_ctx_set_animator(ctx, bare, g), CRABBY_NOT_FOUND, "no skeleton");
        assert_eq!(
            crabby_ctx_animator_set_float(ctx, bare, 0, 1.0),
            CRABBY_NOT_FOUND,
            "no animator"
        );
        assert_eq!(crabby_ctx_animator_update(ctx, bare, 0.1), CRABBY_NOT_FOUND);
        assert_eq!(
            crabby_ctx_animator_state(ctx, bare, null_mut(), null_mut()),
            CRABBY_NOT_FOUND
        );
        assert_eq!(crabby_ctx_animator_set_float(ctx, 12345, 0, 1.0), BAD);
        assert_eq!(crabby_ctx_animator_update(ctx, 12345, 0.1), BAD);
        assert_eq!(
            crabby_ctx_animator_state(ctx, e, null_mut(), null_mut()),
            OK,
            "both outputs are optional"
        );
        assert_eq!(crabby_ctx_animator_next_event(ctx, e, null_mut(), 0, null_mut()), BAD);
        let mut found = false;
        assert_eq!(crabby_ctx_animator_next_event(ctx, e, null_mut(), 4, &mut found), BAD);
        // A parameter of the wrong kind or index, and bad crossfade values,
        // are invalid arguments: the animator is left as it was and the
        // engine never panics. The graph has a float 0, a bool 1, a trigger 2.
        assert_eq!(crabby_ctx_animator_set_float(ctx, e, 1, 1.0), BAD, "a bool");
        assert_eq!(crabby_ctx_animator_set_float(ctx, e, 9, 1.0), BAD, "no such parameter");
        assert_eq!(crabby_ctx_animator_set_float(ctx, e, 0, f32::NAN), BAD);
        assert_eq!(crabby_ctx_animator_set_float(ctx, e, 0, f32::INFINITY), BAD);
        assert_eq!(crabby_ctx_animator_set_bool(ctx, e, 0, true), BAD, "a float");
        assert_eq!(crabby_ctx_animator_set_bool(ctx, e, 2, true), BAD, "a trigger");
        assert_eq!(crabby_ctx_animator_set_bool(ctx, e, 9, true), BAD);
        assert_eq!(crabby_ctx_animator_set_trigger(ctx, e, 0, true), BAD, "a float");
        assert_eq!(crabby_ctx_animator_set_trigger(ctx, e, 1, true), BAD, "a bool");
        assert_eq!(crabby_ctx_animator_set_trigger(ctx, e, 9, true), BAD);
        assert_eq!(crabby_ctx_animator_crossfade(ctx, e, 7, 0.0, 0.0), BAD, "no state 7");
        assert_eq!(crabby_ctx_animator_crossfade(ctx, e, 0, -0.1, 0.0), BAD);
        assert_eq!(crabby_ctx_animator_crossfade(ctx, e, 0, f32::NAN, 0.0), BAD);
        assert_eq!(crabby_ctx_animator_crossfade(ctx, e, 0, 0.1, -1.0), BAD);
        assert_eq!(crabby_ctx_animator_crossfade(ctx, e, 0, 0.1, f32::NAN), BAD);
        // A time step that isn't a time is refused too, before the animator
        // or its skeleton is touched.
        let before = (state_of(ctx, e), root_x(ctx, e));
        assert_eq!(crabby_ctx_animator_update(ctx, e, f32::NAN), BAD);
        assert_eq!(crabby_ctx_animator_update(ctx, e, f32::INFINITY), BAD);
        assert_eq!(crabby_ctx_animator_update(ctx, e, -0.1), BAD);
        assert_eq!(
            (state_of(ctx, e), root_x(ctx, e)),
            before,
            "a refused update poses nothing"
        );
        assert_eq!(
            crabby_ctx_animator_update(ctx, bare, f32::NAN),
            CRABBY_NOT_FOUND,
            "no animator"
        );
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.0), OK, "zero is a valid step");
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.1), OK);
        assert_eq!(state_of(ctx, e).0, 0, "refused calls changed nothing");
        // Setting an animator again starts over.
        assert_eq!(crabby_ctx_animator_set_trigger(ctx, e, 2, true), OK);
        assert_eq!(crabby_ctx_animator_update(ctx, e, 0.1), OK);
        assert_eq!(state_of(ctx, e).0, 1);
        assert_eq!(crabby_ctx_set_animator(ctx, e, g), OK);
        assert_eq!(state_of(ctx, e), (0, 0.0));
        crabby_animation_graph_free(g);
    }
}

#[test]
fn animation_graphs_and_animators_work_from_c() {
    let file = std::env::temp_dir().join("ponko-ffi-animator-rig.gltf");
    std::fs::write(&file, gltf()).unwrap();
    let mut run = Run {
        path: CString::new(file.to_str().unwrap()).unwrap(),
    };
    unsafe {
        let g = crabby_game_new(c"animator".as_ptr(), 64, 64);
        let mut world = null_mut();
        let user = (&raw mut run).cast();
        assert_eq!(crabby_game_run_headless(g, 1, Some(setup), user, &mut world), OK);
        crabby_world_free(world);
    }
    let _ = std::fs::remove_file(&file);
}
