//! Sprite blend modes and sprite materials from C: shaders and materials
//! as handles, their errors, the components, and styled immediate draws.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use PonkoStatus::*;

#[derive(Default)]
struct Seen {
    material: Option<PonkoSpriteMaterial>,
    desc: Option<PonkoSpriteMaterialDesc>,
    after_free: Option<bool>,
    statuses: Vec<PonkoShaderStatus>,
    blend_round_trip: Option<u32>,
    material_round_trip: Option<PonkoSpriteMaterial>,
    bad_blend: Option<PonkoStatus>,
    done: bool,
}

const TINT: &std::ffi::CStr = c"fn fragment(p: SpritePixel) -> vec4<f32> { return p.color * material.params[0]; }";
/// Line 3 returns a number, not a color.
const BROKEN: &std::ffi::CStr = c"// flash\nfn fragment(p: SpritePixel) -> vec4<f32> {\n    return 1.0;\n}";

unsafe extern "C" fn setup(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let seen = &mut *user.cast::<Seen>();
        let mut shader = PonkoSpriteShader { id: PONKO_HANDLE_NONE };
        assert_eq!(
            ponko_ctx_create_sprite_shader(ctx, c"tint".as_ptr(), TINT.as_ptr(), &mut shader),
            PONKO_OK
        );
        let mut unused = shader;
        assert_eq!(
            ponko_ctx_create_sprite_shader(ctx, c"flash.wgsl".as_ptr(), BROKEN.as_ptr(), &mut unused),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(unused, shader, "a failed call leaves the output alone");

        let white = [255u8; 4];
        let mut pixel = PonkoTexture {
            id: 0,
            width: 0,
            height: 0,
        };
        assert_eq!(
            ponko_ctx_create_texture(ctx, 1, 1, white.as_ptr(), 4, &mut pixel),
            PONKO_OK
        );

        let mut desc = ponko_sprite_material_desc_default();
        desc.has_shader = true;
        desc.shader = shader;
        desc.params[..4].copy_from_slice(&[1.0, 0.5, 0.25, 1.0]);
        desc.has_textures[1] = true;
        desc.textures[1] = pixel;
        let mut material = PonkoSpriteMaterial { id: PONKO_HANDLE_NONE };
        assert_eq!(ponko_ctx_create_sprite_material(ctx, &desc, &mut material), PONKO_OK);

        // A shader this game didn't make, or a param that isn't a number, is refused.
        let mut stranger = desc;
        stranger.shader = PonkoSpriteShader { id: 77 };
        let mut no = material;
        assert_eq!(
            ponko_ctx_create_sprite_material(ctx, &stranger, &mut no),
            PONKO_INVALID_ARGUMENT
        );
        let mut nan = desc;
        nan.params[5] = f32::NAN;
        assert_eq!(
            ponko_ctx_create_sprite_material(ctx, &nan, &mut no),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(no, material);

        // Animating a parameter.
        desc.params[0] = 2.0;
        assert_eq!(ponko_ctx_update_sprite_material(ctx, material, &desc), PONKO_OK);
        let mut read = ponko_sprite_material_desc_default();
        let mut found = false;
        assert_eq!(
            ponko_ctx_sprite_material_desc(ctx, material, &mut read, &mut found),
            PONKO_OK
        );
        assert!(found);
        seen.desc = Some(read);
        seen.material = Some(material);

        // Styled immediate draws: every blend mode, a material, or none.
        for blend in [
            PONKO_BLEND_ALPHA,
            PONKO_BLEND_ADDITIVE,
            PONKO_BLEND_MULTIPLY,
            PONKO_BLEND_PREMULTIPLIED,
        ] {
            let none = PonkoSpriteMaterial { id: PONKO_HANDLE_NONE };
            assert_eq!(
                ponko_ctx_draw_styled(ctx, pixel, PonkoVec2::default(), null(), blend, none),
                PONKO_OK
            );
            let params = ponko_draw_params_default();
            assert_eq!(
                ponko_ctx_draw_styled(ctx, pixel, PonkoVec2::default(), &params, blend, material),
                PONKO_OK
            );
        }
        seen.bad_blend = Some(ponko_ctx_draw_styled(
            ctx,
            pixel,
            PonkoVec2::default(),
            null(),
            4,
            material,
        ));

        // The components: a checked blend mode and a plain material.
        let world = ponko_ctx_world(ctx);
        let (mut blend_id, mut material_id) = (0, 0);
        assert_eq!(ponko_blend_mode_component(world, &mut blend_id), PONKO_OK);
        assert_eq!(ponko_sprite_material_component(world, &mut material_id), PONKO_OK);
        let additive = PONKO_BLEND_ADDITIVE;
        let values = [
            PonkoValue {
                component: blend_id,
                data: (&raw const additive).cast(),
                size: 4,
            },
            PonkoValue {
                component: material_id,
                data: (&raw const material).cast(),
                size: 4,
            },
        ];
        let mut e = 0u64;
        assert_eq!(ponko_spawn(world, values.as_ptr(), values.len(), &mut e), PONKO_OK);
        let mut blend_back = 0u32;
        assert_eq!(ponko_get(world, e, blend_id, (&raw mut blend_back).cast(), 4), PONKO_OK);
        let mut material_back = PonkoSpriteMaterial { id: 0 };
        assert_eq!(
            ponko_get(world, e, material_id, (&raw mut material_back).cast(), 4),
            PONKO_OK
        );
        seen.blend_round_trip = Some(blend_back);
        seen.material_round_trip = Some(material_back);
        let unknown = 9u32;
        assert_eq!(
            ponko_set(world, e, blend_id, (&raw const unknown).cast(), 4),
            PONKO_INVALID_ARGUMENT,
            "a blend mode that doesn't exist"
        );

        let mut count = 0usize;
        assert_eq!(ponko_ctx_shader_status_count(ctx, &mut count), PONKO_OK);
        for i in 0..count {
            let mut s = PonkoShaderStatus::default();
            assert_eq!(ponko_ctx_shader_status(ctx, i, &mut s), PONKO_OK);
            seen.statuses.push(s);
        }

        // Freeing: the material, then the shader, through the one handle call.
        let mut live = false;
        assert_eq!(
            ponko_ctx_handle_is_live(
                ctx,
                PonkoHandleKind::PONKO_HANDLE_SPRITE_MATERIAL as u32,
                material.id,
                &mut live
            ),
            PONKO_OK
        );
        assert!(live);
        assert_eq!(
            ponko_ctx_handle_free(ctx, PonkoHandleKind::PONKO_HANDLE_SPRITE_MATERIAL as u32, material.id),
            PONKO_OK
        );
        assert_eq!(
            ponko_ctx_handle_free(ctx, PonkoHandleKind::PONKO_HANDLE_SPRITE_SHADER as u32, shader.id),
            PONKO_OK
        );
        for (kind, id) in [
            (PonkoHandleKind::PONKO_HANDLE_SPRITE_MATERIAL, material.id),
            (PonkoHandleKind::PONKO_HANDLE_SPRITE_SHADER, shader.id),
        ] {
            assert_eq!(ponko_ctx_handle_is_live(ctx, kind as u32, id, &mut live), PONKO_OK);
            assert!(!live);
        }
        let mut found = true;
        assert_eq!(
            ponko_ctx_sprite_material_desc(ctx, material, &mut read, &mut found),
            PONKO_OK
        );
        seen.after_free = Some(found);
        // Updating a freed material does nothing; its freed shader can't make a new one.
        let built_in = ponko_sprite_material_desc_default();
        assert_eq!(ponko_ctx_update_sprite_material(ctx, material, &built_in), PONKO_OK);
        assert_eq!(
            ponko_ctx_create_sprite_material(ctx, &desc, &mut no),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_ctx_sprite_material_desc(ctx, material, null_mut(), &mut found),
            PONKO_INVALID_ARGUMENT
        );
        seen.done = true;
    }
}

#[test]
fn sprite_materials_and_blend_modes_work_from_c() {
    let mut seen = Seen::default();
    unsafe {
        let g = ponko_game_new(c"sprites".as_ptr(), 64, 64);
        let mut world = null_mut();
        assert_eq!(
            ponko_game_run_headless(g, 1, Some(setup), (&raw mut seen).cast(), &mut world),
            PONKO_OK
        );
        ponko_world_free(world);
    }
    assert!(seen.done);
    let desc = seen.desc.unwrap();
    assert!(desc.has_shader);
    assert_eq!(&desc.params[..4], &[2.0, 0.5, 0.25, 1.0]);
    assert_eq!(desc.has_textures, [false, true]);
    assert_eq!(seen.after_free, Some(false));
    assert_eq!(seen.bad_blend, Some(PONKO_INVALID_ARGUMENT));
    assert_eq!(seen.blend_round_trip, Some(PONKO_BLEND_ADDITIVE));
    assert_eq!(seen.material_round_trip, seen.material);

    let kinds: Vec<_> = seen.statuses.iter().map(|s| (s.kind, s.outcome)).collect();
    assert_eq!(
        kinds,
        [
            (PONKO_SHADER_SPRITE, PONKO_SHADER_COMPILED),
            (PONKO_SHADER_SPRITE, PONKO_SHADER_FAILED)
        ]
    );
    assert_eq!((seen.statuses[1].line, seen.statuses[1].in_engine_code), (3, false));
}

#[test]
fn the_default_sprite_material_desc_is_the_built_in_look() {
    let d = ponko_sprite_material_desc_default();
    assert!(!d.has_shader && d.has_textures == [false, false]);
    assert_eq!(d.shader.id, PONKO_HANDLE_NONE);
    assert_eq!(d.params, [0.0; 16]);
}
