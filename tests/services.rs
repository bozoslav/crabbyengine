//! The service fields of `Ctx` (`ctx.time`, `ctx.input`, ...): each is its
//! own field, so it borrows apart from the world and from the others, and
//! the flat `Ctx` methods from before them still forward.

use ponko::*;

struct Pos(f32);

#[test]
fn time_reads_while_a_world_query_is_alive() {
    fn fall(ctx: &mut Ctx) {
        // The query borrows `ctx.world`; the clock is another field.
        for p in ctx.world.query::<&mut Pos>().iter() {
            p.0 += ctx.time.dt();
        }
    }
    let mut world = Game::new("t", 1, 1)
        .fixed_hz(10)
        .sim_system(fall)
        .run_headless(10, |ctx| {
            ctx.world.spawn((Pos(0.0),));
        });
    let p = world.query::<&Pos>().iter().next().unwrap().0;
    assert!((p - 1.0).abs() < 1e-5, "ten ticks of 0.1 s, got {p}");
}

#[test]
fn input_reads_while_a_world_query_is_alive() {
    fn jump(ctx: &mut Ctx) {
        for p in ctx.world.query::<&mut Pos>().iter() {
            if ctx.input.key_down(Key::Space) {
                p.0 += 1.0;
            }
        }
    }
    let mut world = Game::new("t", 1, 1).sim_system(jump).run_headless(3, |ctx| {
        ctx.world.spawn((Pos(0.0),));
        ctx.input.feed(InputEvent::Key(Key::Space, true));
    });
    assert_eq!(world.query::<&Pos>().iter().next().unwrap().0, 3.0);
}

#[test]
fn dt_in_a_sim_system_is_the_sim_step_and_frame_dt_stays_the_frames() {
    fn check(ctx: &mut Ctx) {
        assert_eq!(ctx.time.dt(), ctx.time.sim_dt());
        assert_eq!(ctx.time.sim_dt(), ctx.time.fixed_dt());
        assert_eq!(ctx.dt(), ctx.time.dt(), "the shortcut is the same number");
        // A headless run has one frame per tick.
        assert_eq!(ctx.time.frame_dt(), ctx.time.fixed_dt());
    }
    Game::new("t", 1, 1)
        .fixed_hz(50)
        .sim_system(check)
        .run_headless(4, |ctx| {
            // Setup is outside the simulation: `dt` is the frame's, and no frame ran yet.
            assert_eq!(ctx.time.dt(), ctx.time.frame_dt());
            assert_eq!(ctx.time.frame_dt(), 0.0);
            assert_eq!(ctx.time.sim_dt(), 1.0 / 50.0);
        });
}

#[test]
fn sim_dt_covers_every_step_of_a_slower_system() {
    fn slow(ctx: &mut Ctx) {
        assert_eq!(ctx.time.sim_dt(), 4.0 * ctx.time.fixed_dt());
        assert_eq!(ctx.time.dt(), ctx.time.sim_dt());
        ctx.world.resource_mut::<Runs>().0 += 1;
    }
    struct Runs(u32);
    let world = Game::new("t", 1, 1)
        .sim_system_every(4, slow)
        .run_headless(8, |ctx| ctx.world.insert_resource(Runs(0)));
    assert_eq!(world.resource::<Runs>().0, 2);
}

#[test]
fn the_clock_counts_ticks_and_controls_the_simulation() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        assert_eq!(ctx.time.sim_tick(), 0);
        assert_eq!(ctx.time.speed(), 1.0);
        ctx.time.set_speed(4.0);
        assert_eq!(ctx.time.speed(), 4.0);
        assert!(!ctx.time.is_paused());
        ctx.time.pause();
        assert!(ctx.time.is_paused());
        ctx.time.resume();
        assert!(!ctx.time.is_paused());
    });
    fn count(ctx: &mut Ctx) {
        let tick = ctx.time.sim_tick();
        assert_eq!(ctx.time.sim_time(), tick as f64 * f64::from(ctx.time.fixed_dt()));
    }
    Game::new("t", 1, 1)
        .fixed_hz(64)
        .sim_system(count)
        .run_headless(5, |_| {});
}

#[test]
#[should_panic(expected = "Time::set_speed: speed must be a finite number >= 0")]
fn a_negative_sim_speed_panics_with_the_service_name() {
    Game::new("t", 1, 1).run_headless(0, |ctx| ctx.time.set_speed(-1.0));
}

#[test]
fn input_edges_and_the_cursor_lock_live_on_the_input_service() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.input.feed(InputEvent::Mouse(MouseButton::Left, true));
        assert!(ctx.input.mouse_pressed(MouseButton::Left));
        assert!(ctx.input.mouse_down(MouseButton::Left));
        assert!(!ctx.input.cursor_locked());
        ctx.input.set_cursor_locked(true);
        assert!(ctx.input.cursor_locked());
        ctx.input.insert_text("hi");
        assert_eq!(ctx.input.typed_text(), "hi");
        // The shortcuts read the same state.
        ctx.input.feed(InputEvent::Key(Key::A, true));
        assert!(ctx.key_down(Key::A) && ctx.key_pressed(Key::A));
        assert_eq!(ctx.mouse_pos(), ctx.input.mouse_pos());
    });
}

#[test]
fn perf_reads_while_a_world_query_is_alive_and_scopes_borrow_nothing() {
    fn work(ctx: &mut Ctx) {
        // The guard holds no borrow of `ctx`, so the timed code may use all of it.
        let _timing = ctx.perf.scope("services/work");
        for p in ctx.world.query::<&mut Pos>().iter() {
            p.0 += ctx.perf.system_timings().len() as f32;
        }
        ctx.commands.spawn((Pos(0.0),));
    }
    let mut world = Game::new("t", 1, 1).sim_system(work).run_headless(2, |ctx| {
        ctx.perf.set_profiling(true);
        ctx.world.spawn((Pos(0.0),));
    });
    assert_eq!(world.query::<&Pos>().count(), 3);
}

#[test]
fn the_report_counts_the_entities_left_by_the_last_system() {
    fn grow(ctx: &mut Ctx) {
        ctx.commands.spawn((Pos(0.0),));
    }
    struct Seen(Vec<usize>);
    fn look(ctx: &mut Ctx) {
        let entities = ctx.perf.report().entities;
        ctx.world.resource_mut::<Seen>().0.push(entities);
    }
    let world = Game::new("t", 1, 1)
        .sim_system(grow)
        .sim_system(look)
        .run_headless(3, |ctx| ctx.world.insert_resource(Seen(Vec::new())));
    assert_eq!(world.resource::<Seen>().0, [1, 2, 3]);
}

#[test]
fn the_hud_switch_turns_profiling_on_and_scopes_get_timed() {
    fn timed(ctx: &mut Ctx) {
        let _timing = ctx.perf.scope("services/timed");
        std::hint::black_box((0..1000).sum::<u64>());
    }
    struct Names(Vec<&'static str>);
    fn read(ctx: &mut Ctx) {
        let names: Vec<_> = ctx.perf.scope_timings().iter().map(|s| s.name).collect();
        ctx.world.resource_mut::<Names>().0 = names;
    }
    let world = Game::new("t", 1, 1)
        .sim_system(timed)
        .sim_system(read)
        .run_headless(3, |ctx| {
            ctx.world.insert_resource(Names(Vec::new()));
            assert!(!ctx.perf.profiling());
            ctx.perf.set_hud(true);
            assert!(ctx.perf.hud());
            assert!(ctx.perf.profiling(), "the overlay needs the deeper numbers");
        });
    assert!(world.resource::<Names>().0.contains(&"services/timed"));
}

struct Beep(Sound);

#[test]
fn sounds_play_while_a_world_query_is_alive() {
    fn beep_each(ctx: &mut Ctx) {
        let beep = ctx.world.resource::<Beep>().0;
        for p in ctx.world.query::<&mut Pos>().iter() {
            // `ctx.audio` is its own field: no need to collect first.
            let playing = ctx.audio.play(&beep);
            ctx.audio.stop(playing);
            p.0 += 1.0;
        }
    }
    let mut world = Game::new("t", 1, 1).sim_system(beep_each).run_headless(2, |ctx| {
        let beep = ctx.audio.create(8000, &[0.0; 80]);
        ctx.world.insert_resource(Beep(beep));
        ctx.world.spawn_batch([(Pos(0.0),), (Pos(0.0),)]);
    });
    assert!(world.query::<&Pos>().iter().all(|p| p.0 == 2.0));
}

#[test]
fn the_audio_service_owns_sounds_buses_and_the_shortcuts_reach_it() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let beep = ctx.audio.create(8000, &[0.0; 8000]);
        assert_eq!(beep.seconds(), 1.0);
        assert!(ctx.is_live(beep));
        // The shortcut plays through the same service.
        let playing = ctx.play_sound(&beep);
        ctx.audio.stop(playing);
        let music = ctx.audio.bus("music");
        assert_eq!(ctx.audio.bus("music"), music, "a name gives the same bus");
        ctx.audio.set_bus_volume(music, 0.25);
        ctx.audio.mute_bus(music, true);
        let on_bus = ctx
            .audio
            .play_on(&beep, SoundParams::default(), music, AudioEffects::default());
        ctx.audio.set_effects(on_bus, AudioEffects::default());
        ctx.audio.stop(on_bus);
        assert!(ctx.audio.try_load("missing.ogg").is_err());
        ctx.free(beep);
        assert!(!ctx.is_live(beep));
    });
}

#[cfg(feature = "3d")]
#[test]
fn the_default_listener_is_the_camera_as_the_running_system_found_it() {
    fn listen(ctx: &mut Ctx) {
        // The camera the system started with is where the ears are...
        assert_eq!(ctx.audio.listener().position, ctx.camera3d.position);
        // ...and a listener of the game's own replaces it at once.
        let ears = AudioListener3D {
            position: vec3(1.0, 2.0, 3.0),
            rotation: Quat::IDENTITY,
        };
        ctx.audio.set_listener(Some(ears));
        assert_eq!(ctx.audio.listener().position, ears.position);
        ctx.audio.set_listener(None);
        ctx.camera3d.position.x += 1.0;
    }
    Game::new("t", 1, 1).sim_system(listen).run_headless(3, |ctx| {
        ctx.camera3d.position = vec3(5.0, 0.0, 0.0);
    });
}

struct Dot(Texture);

#[test]
fn drawing_queues_while_a_world_query_is_alive() {
    struct Queued(usize);
    fn draw_each(ctx: &mut Ctx) {
        // `ctx.gfx` is its own field: draw from inside the query.
        for (dot, at) in ctx.world.query::<(&Dot, &Transform2D)>().iter() {
            ctx.gfx.draw(&dot.0, at.position);
            ctx.gfx
                .draw_rect(Rect::new(at.position.x, at.position.y, 1.0, 1.0), Color::WHITE);
        }
        let queued = ctx.gfx.sprite_count();
        ctx.world.resource_mut::<Queued>().0 = queued;
    }
    let world = Game::new("t", 8, 8).sim_system(draw_each).run_headless(1, |ctx| {
        let pixel = ctx.gfx.create_texture(1, 1, &[255; 4]);
        ctx.world.insert_resource(Queued(0));
        ctx.world
            .spawn_batch((0..3).map(|i| (Dot(pixel), Transform2D::at(vec2(i as f32, 0.0)))));
    });
    assert_eq!(world.resource::<Queued>().0, 6, "a sprite and a rectangle per entity");
}

#[test]
fn the_graphics_service_owns_textures_and_the_shortcuts_reach_it() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let before = ctx.live_counts().textures;
        let pixel = ctx.gfx.create_texture(2, 2, &[255; 16]);
        assert_eq!((pixel.width(), pixel.height()), (2, 2));
        assert!(ctx.is_live(pixel));
        assert_eq!(ctx.live_counts().textures, before + 1);
        ctx.gfx.update_texture(&pixel, &[0; 16]);
        ctx.gfx.set_texture_filter(&pixel, TextureFilter::Nearest);
        // The shortcuts queue through the same service.
        ctx.draw(&pixel, Vec2::ZERO);
        ctx.gfx.draw_ex(&pixel, Vec2::ZERO, DrawParams::default());
        assert_eq!(ctx.gfx.sprite_count(), 2);
        ctx.draw_text("headless text is measured, not drawn", Vec2::ZERO, 16.0, Color::WHITE);
        assert!(ctx.gfx.measure_text("wide", 16.0).x > 0.0);
        assert!(ctx.gfx.has_font(Font::DEFAULT));
        assert!(ctx.gfx.try_load_texture("missing.png").is_err());
        assert!(ctx.gfx.try_load_font("missing.ttf").is_err());
        assert!(ctx.gfx.gpu_capabilities().is_none(), "no GPU in a headless run");
        ctx.free(pixel);
        assert!(!ctx.is_live(pixel));
    });
}

#[test]
#[should_panic(expected = "a 2 x 2 texture needs 16 bytes of RGBA, got 4")]
fn a_texture_with_the_wrong_byte_count_panics_from_the_service() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.gfx.create_texture(2, 2, &[255; 4]);
    });
}

#[test]
fn a_texture_loaded_by_the_service_is_saved_by_file_name() {
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255; 4]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let world = Game::new("t", 1, 1).hot_reload(false).run_headless(0, |ctx| {
        ctx.add_asset_source(MemorySource::new("mem").with("dot.png", png));
        // The service needs no world to load; the world still learns the name.
        let dot = ctx.gfx.load_texture("dot.png");
        assert_eq!(ctx.gfx.load_texture("dot.png"), dot, "a file is loaded once");
        ctx.world.spawn((Transform2D::default(), Sprite::new(dot)));
        // A freed texture's name goes with it.
        let gone = ctx.gfx.create_texture(1, 1, &[0; 4]);
        ctx.world.spawn((Transform2D::default(), Sprite::new(gone)));
        ctx.free(gone);
    });
    let save = world.save().to_string();
    assert_eq!(save.matches("\"texture\":\"dot.png\"").count(), 1, "{save}");
}

#[cfg(feature = "3d")]
mod three {
    use ponko::*;

    struct Box3(Mesh);

    #[test]
    fn meshes_are_made_while_a_world_query_is_alive() {
        fn grow(ctx: &mut Ctx) {
            let material = ctx.gfx.default_material();
            for b in ctx.world.query::<&mut Box3>().iter() {
                // The world is borrowed by the query; `ctx.gfx` is free.
                let bigger = ctx.gfx.create_mesh(&MeshData::cube(2.0), material);
                ctx.gfx.free_mesh(b.0);
                b.0 = bigger;
            }
        }
        let mut world = Game::new("t", 1, 1).sim_system(grow).run_headless(2, |ctx| {
            let material = ctx.gfx.default_material();
            let mesh = ctx.gfx.create_mesh(&MeshData::cube(1.0), material);
            ctx.world.spawn((Box3(mesh),));
        });
        assert_eq!(world.query::<&Box3>().count(), 1);
    }

    #[test]
    fn the_calls_that_touch_entities_take_the_world() {
        Game::new("t", 1, 1).run_headless(0, |ctx| {
            let probe = ctx.world.spawn((Transform3D::default(), ReflectionProbe3D::default()));
            ctx.gfx.bake_reflection_probe(&mut ctx.world, probe).unwrap();
            assert_ne!(ctx.world.get::<ReflectionProbe3D>(probe).unwrap().content, 0);
            let bare = ctx.world.spawn((Transform3D::default(),));
            assert!(ctx.gfx.bake_reflection_probe(&mut ctx.world, bare).is_err());
            ctx.gfx.write_animated_morphs(&mut ctx.world);
        });
    }

    #[test]
    fn freeing_a_gpu_buffer_drops_its_compute_passes() {
        Game::new("t", 1, 1).run_headless(0, |ctx| {
            let buffer = ctx.gfx.create_gpu_buffer(4, &[]);
            let update = ctx.gfx.create_compute("up", "fn update(i: u32) { items[i].x += 1.0; }");
            ctx.compute_passes.push(ComputePass::for_buffer(update, buffer));
            assert!(ctx.is_live(buffer));
            ctx.free(buffer);
            assert!(ctx.compute_passes.is_empty());
            assert_eq!(ctx.gfx.gpu_buffer_len(buffer), None);
            // The id is reused, and starts clean.
            assert_eq!(ctx.gfx.create_gpu_buffer(2, &[]), buffer);
        });
    }

    #[test]
    fn quality_presets_write_the_post_and_lighting_settings_they_are_given() {
        Game::new("t", 1, 1).run_headless(0, |ctx| {
            ctx.gfx
                .set_quality(&mut ctx.post, &mut ctx.lighting, GraphicsQuality::low());
            assert_eq!(ctx.gfx.quality(&ctx.post, &ctx.lighting).level(), QualityLevel::Low);
            assert_eq!(ctx.post.render_scale, GraphicsQuality::low().render_scale);
            // A tweak on top stays, and the level says so.
            ctx.lighting.shadow_distance += 1.0;
            assert_eq!(ctx.gfx.quality(&ctx.post, &ctx.lighting).level(), QualityLevel::Custom);
        });
    }

    #[test]
    fn handles_from_the_service_are_live_until_freed() {
        Game::new("t", 1, 1).run_headless(0, |ctx| {
            let material = ctx.gfx.create_material(MaterialDesc::default());
            let mesh = ctx.gfx.create_mesh(&MeshData::cube(1.0), material);
            assert!(ctx.is_live(material) && ctx.is_live(mesh));
            assert!(ctx.gfx.has_material(material) && ctx.gfx.is_mesh_live(mesh));
            ctx.free(mesh);
            ctx.free(material);
            assert!(!ctx.is_live(material) && !ctx.is_live(mesh));
            assert!(!ctx.gfx.has_material(material));
        });
    }
}
