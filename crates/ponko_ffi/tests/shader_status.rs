//! Shader compile results from C: whether a shader compiled, and where in
//! the game's own WGSL an error is (Rust `Graphics::shader_status`).
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use CrabbyStatus::*;

#[derive(Default)]
struct Seen {
    count: usize,
    statuses: Vec<CrabbyShaderStatus>,
    names: Vec<String>,
    texts: Vec<String>,
    json: String,
    past_the_end: Option<CrabbyStatus>,
    short_buffer: Option<(CrabbyStatus, usize, u8)>,
}

unsafe fn read(f: impl Fn(*mut std::ffi::c_char, usize, *mut usize) -> CrabbyStatus) -> String {
    let mut len = 0usize;
    assert_eq!(f(null_mut(), 0, &mut len), CRABBY_OK);
    let mut buffer = vec![0u8; len + 1];
    assert_eq!(f(buffer.as_mut_ptr().cast(), buffer.len(), &mut len), CRABBY_OK);
    buffer.truncate(len);
    String::from_utf8(buffer).unwrap()
}

unsafe extern "C" fn shaders(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let seen = &mut *user.cast::<Seen>();
        let mut shader = CrabbyShader::default();
        let good = c"fn surface(s: Surface) -> Surface { var o = s; o.albedo *= 0.5; return o; }";
        assert_eq!(
            crabby_ctx_create_shader(ctx, c"tint".as_ptr(), good.as_ptr(), &mut shader),
            CRABBY_OK
        );
        // Line 3 returns a number, not a `Surface`.
        let broken = c"// red\nfn surface(s: Surface) -> Surface {\n    return 1.0;\n}";
        assert_eq!(
            crabby_ctx_create_shader(ctx, c"red.wgsl".as_ptr(), broken.as_ptr(), &mut shader),
            CRABBY_INVALID_ARGUMENT
        );
        let mut effect = CrabbyEffect::default();
        let effect_src = c"fn effect(p: Pixel) -> vec3<f32> {\n  return nope;\n}";
        assert_eq!(
            crabby_ctx_create_effect(ctx, c"fx.wgsl".as_ptr(), effect_src.as_ptr(), &mut effect),
            CRABBY_INVALID_ARGUMENT
        );

        assert_eq!(crabby_ctx_shader_status_count(ctx, &mut seen.count), CRABBY_OK);
        for i in 0..seen.count {
            let mut s = CrabbyShaderStatus::default();
            assert_eq!(crabby_ctx_shader_status(ctx, i, &mut s), CRABBY_OK);
            seen.statuses.push(s);
            seen.names
                .push(read(|b, c, n| crabby_ctx_shader_status_name(ctx, i, b, c, n)));
            seen.texts
                .push(read(|b, c, n| crabby_ctx_shader_status_text(ctx, i, b, c, n)));
        }
        seen.json = read(|b, c, n| crabby_ctx_shader_status_json(ctx, b, c, n));
        let mut s = CrabbyShaderStatus::default();
        seen.past_the_end = Some(crabby_ctx_shader_status(ctx, seen.count, &mut s));
        // Too small: the length, and an empty string.
        let (mut small, mut len) = ([7u8; 4], 0usize);
        let status = crabby_ctx_shader_status_name(ctx, 1, small.as_mut_ptr().cast(), small.len(), &mut len);
        seen.short_buffer = Some((status, len, small[0]));
        assert_eq!(
            crabby_ctx_shader_status_name(ctx, 0, null_mut(), 4, &mut len),
            CRABBY_INVALID_ARGUMENT,
            "a null buffer with room"
        );
        assert_eq!(crabby_ctx_shader_status_count(ctx, null_mut()), CRABBY_INVALID_ARGUMENT);
    }
}

#[test]
fn shader_status_says_what_compiled_and_where_errors_are_from_c() {
    let mut seen = Seen::default();
    unsafe {
        let g = crabby_game_new(c"shaders".as_ptr(), 64, 64);
        let mut world = null_mut();
        assert_eq!(
            crabby_game_run_headless(g, 1, Some(shaders), (&raw mut seen).cast(), &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
    assert_eq!(seen.count, 3);
    assert_eq!(seen.names, ["tint", "red.wgsl", "fx.wgsl"]);

    let tint = seen.statuses[0];
    assert_eq!(
        (tint.kind, tint.outcome, tint.ok),
        (CRABBY_SHADER_MATERIAL, CRABBY_SHADER_COMPILED, true)
    );
    assert_eq!((tint.diagnostics, tint.line, tint.column), (0, 0, 0));
    assert!(seen.texts[0].is_empty());

    let red = seen.statuses[1];
    assert_eq!((red.outcome, red.ok, red.diagnostics), (CRABBY_SHADER_FAILED, false, 1));
    assert_eq!((red.line, red.column, red.in_engine_code), (3, 12, false));
    assert!(seen.texts[1].starts_with("red.wgsl:3:12: error: "), "{}", seen.texts[1]);
    assert!(seen.texts[1].contains("\n        return 1.0;"), "{}", seen.texts[1]);

    let fx = seen.statuses[2];
    assert_eq!(
        (fx.kind, fx.outcome, fx.line),
        (CRABBY_SHADER_EFFECT, CRABBY_SHADER_FAILED, 2)
    );

    let json: serde_json::Value = serde_json::from_str(&seen.json).unwrap();
    let list = json["shaders"].as_array().unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[1]["outcome"], "failed");
    assert_eq!(list[1]["diagnostics"][0]["line"], 3);
    assert_eq!(list[2]["kind"], "effect");

    assert_eq!(seen.past_the_end, Some(CRABBY_INVALID_ARGUMENT));
    assert_eq!(seen.short_buffer, Some((CRABBY_OK, "red.wgsl".len(), 0)));
}
