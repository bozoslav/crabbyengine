//! The flat `Ctx` methods from before the services (`ctx.time`, `ctx.input`,
//! ...) stay for one deprecation cycle: each forwards to its service.
#![allow(deprecated)]

mod common;

#[allow(unused_imports)]
use ponko::actions::InputEvent;
#[allow(unused_imports)]
use ponko::audio::AudioEffects;
#[allow(unused_imports)]
use ponko::d2::Transform as Transform2D;
#[allow(unused_imports)]
#[cfg(feature = "3d")]
use ponko::d3::{
    AudioListener as AudioListener3D, AudioSettings as AudioSettings3D, ReflectionProbe as ReflectionProbe3D,
    Transform as Transform3D,
};
use ponko::prelude::*;
#[allow(unused_imports)]
#[cfg(feature = "3d")]
use ponko::render::{ComputePass, GraphicsQuality, MaterialDesc, MeshData, QualityLevel};
#[allow(unused_imports)]
use ponko::residency::ResidencyKind;

#[test]
fn the_flat_time_and_input_spellings_still_forward() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.set_sim_speed(2.0);
        assert_eq!(ctx.sim_speed(), ctx.time.speed());
        assert_eq!(ctx.time(), ctx.time.elapsed());
        assert_eq!(ctx.frame(), ctx.time.frame());
        assert_eq!(ctx.fps(), ctx.time.fps());
        assert_eq!(ctx.sim_tick(), ctx.time.sim_tick());
        assert_eq!(ctx.fixed_dt(), ctx.time.fixed_dt());
        assert_eq!(ctx.fixed_steps(), ctx.time.fixed_steps());
        assert_eq!(ctx.fixed_alpha(), ctx.time.fixed_alpha());
        ctx.pause();
        assert!(ctx.is_paused() && ctx.time.is_paused());
        ctx.resume();
        ctx.step();
        assert_eq!(ctx.turns(4), ctx.time.turns(4));

        ctx.feed_input(InputEvent::Key(Key::Space, true));
        assert!(ctx.input.key_pressed(Key::Space));
        ctx.set_cursor_locked(true);
        assert!(ctx.cursor_locked() && ctx.input.cursor_locked());
        ctx.ui_insert_text("x");
        assert_eq!(ctx.typed_text(), "x");
        assert_eq!(ctx.mouse_delta(), ctx.input.mouse_delta());
        assert_eq!(ctx.mouse_wheel(), ctx.input.mouse_wheel());
        assert_eq!(ctx.gamepads().len(), ctx.input.gamepads().len());
        ctx.set_actions(ActionMap::default());
        assert!(!ctx.action_down("jump"));
    });
}

#[test]
fn the_flat_perf_spellings_still_forward() {
    Game::new("t", 1, 1).run_headless(1, |ctx| {
        ctx.set_perf_hud(true);
        assert!(ctx.perf_hud() && ctx.perf.hud());
        ctx.set_profiling(true);
        assert!(ctx.profiling() && ctx.perf.profiling());
        assert_eq!(ctx.profile("compat/closure", |ctx| ctx.world.len()), 0);
        assert_eq!(ctx.system_timings().len(), ctx.perf.system_timings().len());
        assert_eq!(ctx.render_stats(), ctx.perf.render_stats());
        assert_eq!(ctx.gpu_passes().len(), ctx.perf.gpu_passes().len());
        assert_eq!(ctx.scope_timings().len(), ctx.perf.scope_timings().len());
        assert_eq!(ctx.frame_stats().fps, ctx.perf.frame_stats().fps);
        assert_eq!(ctx.perf_report().entities, ctx.perf.report().entities);
        ctx.record_trace("unused.json", 1); // headless: does nothing
    });
}

#[test]
fn the_flat_perf_report_counts_entities_spawned_by_the_running_system() {
    fn spawn_then_report(ctx: &mut Ctx) {
        let before = ctx.perf_report().entities;
        ctx.world.spawn((Transform2D::default(),));
        assert_eq!(ctx.perf_report().entities, before + 1);
    }
    Game::new("t", 1, 1).system(spawn_then_report).run_headless(3, |_| {});
}

#[test]
fn the_flat_audio_spellings_still_forward() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let beep = ctx.create_sound(8000, &[0.0; 80]);
        let playing = ctx.play_sound_ex(&beep, SoundParams::default());
        ctx.stop_sound(playing);
        let music = ctx.audio_bus("music");
        assert_eq!(music, ctx.audio.bus("music"));
        ctx.set_audio_bus_volume(music, 0.5);
        ctx.mute_audio_bus(music, false);
        let on_bus = ctx.play_sound_on(&beep, SoundParams::default(), music, AudioEffects::default());
        ctx.set_sound_effects(on_bus, AudioEffects::default());
        assert!(ctx.try_load_sound("missing.ogg").is_err());
    });
}

#[cfg(feature = "3d")]
#[test]
fn the_flat_listener_reads_the_camera_live() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.camera3d.position = vec3(9.0, 0.0, 0.0);
        // The flat spelling had all of `Ctx`, so it saw the camera at once.
        assert_eq!(ctx.audio_listener().position, vec3(9.0, 0.0, 0.0));
        let beep = ctx.create_sound(8000, &[0.0; 80]);
        let playing = ctx.play_sound_at(&beep, vec3(1.0, 0.0, 0.0), AudioSettings3D::default());
        ctx.stop_sound(playing);
        ctx.set_audio_listener(Some(AudioListener3D::default()));
        assert_eq!(ctx.audio_listener().position, Vec3::ZERO);
    });
}

#[test]
fn the_flat_graphics_spellings_still_forward() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.clear(Color::BLACK);
        let pixel = ctx.create_texture(1, 1, &[255; 4]);
        ctx.update_texture(&pixel, &[0; 4]);
        ctx.set_texture_filter(&pixel, TextureFilter::Linear);
        ctx.draw_ex(&pixel, Vec2::ZERO, DrawParams::default());
        ctx.draw_rect(Rect::new(0.0, 0.0, 1.0, 1.0), Color::WHITE);
        ctx.draw_line(Vec2::ZERO, Vec2::ONE, 1.0, Color::WHITE);
        ctx.draw_circle(Vec2::ZERO, 1.0, Color::WHITE);
        assert_eq!(ctx.sprite_count(), 4);
        assert_eq!(ctx.sprite_count(), ctx.gfx.sprite_count());
        ctx.draw_text_ex("flat", Vec2::ZERO, TextParams::default());
        assert_eq!(ctx.measure_text("flat", 16.0), ctx.gfx.measure_text("flat", 16.0));
        assert_eq!(
            ctx.measure_text_ex("flat", Font::DEFAULT, 16.0),
            ctx.gfx.measure_text("flat", 16.0)
        );
        assert!(ctx.has_font(Font::DEFAULT));
        assert!(ctx.try_load_texture("missing.png").is_err());
        assert!(ctx.try_load_font("missing.ttf").is_err());
        assert!(ctx.gpu_capabilities().is_none());
        ctx.save_screenshot("unused.png"); // headless: does nothing
        assert!(!ctx.is_resident(ResidencyKind::Texture, "missing.png"));
        assert_eq!(
            ctx.residency(ResidencyKind::Texture),
            ctx.gfx.residency(ResidencyKind::Texture)
        );
    });
}

#[cfg(feature = "3d")]
#[test]
fn the_flat_3d_spellings_still_forward() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let material = ctx.create_material(MaterialDesc::default());
        assert!(ctx.has_material(material) && ctx.gfx.has_material(material));
        assert_eq!(ctx.default_material(), ctx.gfx.default_material());
        let mesh = ctx.create_mesh(&MeshData::cube(1.0), material);
        assert!(ctx.is_mesh_live(mesh));
        ctx.set_mesh_shadows(mesh, false);
        assert!(!ctx.mesh_shadows(mesh));
        ctx.free_mesh(mesh);
        assert!(!ctx.gfx.is_mesh_live(mesh));
        assert_eq!(ctx.models_drawn(), ctx.gfx.models_drawn());
        assert_eq!(ctx.shader_status().len(), ctx.gfx.shader_status().len());

        // The flat spellings of the calls that touch entities pass the world themselves.
        let probe = ctx.world.spawn((Transform3D::default(), ReflectionProbe3D::default()));
        ctx.bake_reflection_probe(probe).unwrap();
        assert!(ctx.load_reflection_probe(probe, "missing-probe.exr").is_err());
        ctx.write_animated_morphs();

        ctx.set_graphics_quality(GraphicsQuality::medium());
        assert_eq!(ctx.graphics_quality(), ctx.gfx.quality(&ctx.post, &ctx.lighting));
        assert_eq!(ctx.graphics_quality().level(), QualityLevel::Medium);

        // Freeing a buffer the flat way still drops its passes and reuses the id.
        let buffer = ctx.create_gpu_buffer(4, &[]);
        let update = ctx.create_compute("up", "fn update(i: u32) { items[i].x += 1.0; }");
        ctx.compute_passes.push(ComputePass::for_buffer(update, buffer));
        ctx.free_gpu_buffer(buffer);
        assert!(ctx.compute_passes.is_empty());
        assert_eq!(ctx.gpu_buffer_len(buffer), None);
        assert_eq!(ctx.create_gpu_buffer(4, &[]), buffer);
    });
}

#[test]
fn the_flat_ui_spellings_still_forward() {
    Game::new("t", 800, 600).run_headless(0, |ctx| {
        assert!(!ctx.ui_wants_keyboard());
        assert!(!ctx.mouse_over_ui());
        // The mouse starts at (0, 0), inside this button.
        let button = Rect::new(0.0, 0.0, 100.0, 30.0);
        assert!(!ctx.ui_button(button, "Go"), "hovering isn't clicking");
        assert!(ctx.mouse_over_ui());
        ctx.ui_panel(Rect::new(500.0, 500.0, 10.0, 10.0));
        assert!(ctx.ui_label(vec2(500.0, 500.0), "label").y > 0.0);
        assert_eq!(ctx.sprite_count(), ctx.gfx.sprite_count());
    });
}

#[cfg(feature = "3d")]
#[test]
fn the_flat_pointer_ray_still_forwards() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.set_ui_pointer_ray(Some((Vec3::ZERO, Vec3::X)));
        ctx.set_ui_pointer_ray(None);
    });
}

#[derive(Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
struct Stats {
    hp: f32,
}

#[test]
fn the_flat_asset_spellings_still_forward() {
    let prefab = common::TempAsset::new("flat-prefab.json");
    let scene = common::TempAsset::new("flat-scene.json");
    let (prefab, scene) = (prefab.name(), scene.name());
    Game::new("t", 1, 1).hot_reload(false).run_headless(0, |ctx| {
        ctx.add_asset_source(MemorySource::new("mem").with("stats.json", r#"{"hp": 7}"#));
        let hp = ctx
            .load_asset_with("stats.json", |bytes| {
                serde_json::from_slice::<Stats>(bytes).map_err(|e| e.to_string())
            })
            .unwrap()
            .hp;
        assert_eq!(hp, 7.0);
        ctx.load_resource::<Stats>("stats.json");
        assert_eq!(ctx.world.resource::<Stats>().hp, 7.0);

        ctx.world.reflect::<Stats>();
        ctx.world.reflect_resource::<Stats>();
        let e = ctx.world.spawn((Transform2D::default(), Stats::default()));
        ctx.load_component_json::<Stats>("stats.json", e).unwrap();
        ctx.load_component_json_named("stats.json", e, "Stats").unwrap();
        assert_eq!(ctx.world.get::<Stats>(e).unwrap().hp, 7.0);
        ctx.load_resource_json::<Stats>("stats.json").unwrap();
        ctx.load_resource_json_named("stats.json", "Stats").unwrap();

        ctx.save_prefab(e, prefab).unwrap();
        ctx.save_scene(scene).unwrap();
        assert!(ctx.reloaded_files().is_empty() && ctx.reloaded_textures().is_empty());
    });
}
