//! One handle lifecycle: `Handle`, `Ctx::is_live`, `Ctx::free` and
//! `Ctx::live_counts` for every resource kind, in headless runs.

use ponko::*;

/// Writes the built-in font to a file `load_font` can read.
fn font_file(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("ponko-handles-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, epaint_default_fonts::UBUNTU_LIGHT).unwrap();
    path.to_str().unwrap().to_string()
}

/// Frees `handle`, then checks it is dead, stays dead when freed again,
/// and that `make` gives a live handle with another id.
fn frees<H: Handle>(ctx: &mut Ctx, handle: H, make: impl Fn(&mut Ctx) -> H) {
    assert!(ctx.is_live(handle), "{handle:?} is live before free");
    assert!(ctx.is_live(H::from_id(handle.id())), "from_id keeps the id");
    ctx.free(handle);
    assert!(!ctx.is_live(handle), "{handle:?} is dead after free");
    ctx.free(handle); // again: nothing
    let next = make(ctx);
    assert!(ctx.is_live(next));
    assert_ne!(next.id(), handle.id(), "{handle:?}: ids are never reused");
    assert!(!ctx.is_live(handle), "{handle:?} stays dead");
    ctx.free(next);
}

/// [`frees`] for meshes and GPU buffers, whose freed ids the next one made
/// takes (streamed worlds make and free them all the time).
#[cfg(feature = "3d")]
fn frees_reusing<H: Handle>(ctx: &mut Ctx, handle: H, make: impl Fn(&mut Ctx) -> H) {
    assert!(ctx.is_live(handle), "{handle:?} is live before free");
    ctx.free(handle);
    assert!(!ctx.is_live(handle), "{handle:?} is dead after free");
    ctx.free(handle); // again: nothing
    let next = make(ctx);
    assert_eq!(next.id(), handle.id(), "{handle:?}: the freed id is taken again");
    assert!(ctx.is_live(next));
    ctx.free(next);
}

#[test]
fn none_is_never_live_and_freeing_it_does_nothing() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let before = ctx.live_counts();
        assert!(!ctx.is_live(Texture::NONE));
        assert!(!ctx.is_live(Sound::NONE));
        assert!(!ctx.is_live(Font::NONE));
        ctx.free(Texture::NONE);
        ctx.free(Sound::NONE);
        ctx.free(Font::NONE);
        assert_eq!(ctx.live_counts(), before);
        // Playing nothing is silent, not a panic.
        ctx.play_sound(&Sound::NONE);
        ctx.gfx.draw_text_ex(
            "hi",
            vec2(0.0, 0.0),
            TextParams {
                font: Font::NONE,
                ..Default::default()
            },
        );
    });
}

#[test]
fn textures_sounds_and_fonts_free_and_stay_dead() {
    let file = font_file("free.ttf");
    Game::new("t", 1, 1).run_headless(0, move |ctx| {
        let before = ctx.live_counts();
        let pixel = |ctx: &mut Ctx| ctx.gfx.create_texture(1, 1, &[255; 4]);
        let t = pixel(ctx);
        frees(ctx, t, pixel);
        let beep = |ctx: &mut Ctx| ctx.audio.create(8_000, &[0.0; 80]);
        let s = beep(ctx);
        frees(ctx, s, beep);
        let font = ctx.gfx.load_font(&file);
        frees(ctx, font, |ctx| ctx.gfx.load_font(&file));
        assert_eq!(ctx.live_counts(), before, "everything made was freed");
    });
}

#[test]
fn the_built_in_font_is_never_freed() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.free(Font::DEFAULT);
        assert!(ctx.is_live(Font::DEFAULT));
        assert!(ctx.gfx.has_font(Font::DEFAULT));
    });
}

#[test]
fn text_in_a_freed_font_uses_the_built_in_one() {
    let file = font_file("text.ttf");
    Game::new("t", 1, 1).run_headless(0, move |ctx| {
        let font = ctx.gfx.load_font(&file);
        let size = ctx.gfx.measure_text_ex("Freed", font, 20.0);
        ctx.free(font);
        assert!(!ctx.gfx.has_font(font));
        assert_eq!(
            ctx.gfx.measure_text_ex("Freed", font, 20.0),
            size,
            "same metrics as the built-in font"
        );
        // Loading the file again loads it afresh, with a new id.
        let again = ctx.gfx.load_font(&file);
        assert_ne!(again, font);
        assert!(ctx.is_live(again));
    });
}

#[test]
fn live_counts_follow_creates_and_frees() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let before = ctx.live_counts();
        let a = ctx.gfx.create_texture(1, 1, &[255; 4]);
        let _b = ctx.gfx.create_texture(1, 1, &[255; 4]);
        let s = ctx.audio.create(8_000, &[0.0; 8]);
        let now = ctx.live_counts();
        assert_eq!(now.textures, before.textures + 2);
        assert_eq!(now.sounds, before.sounds + 1);
        ctx.free(a);
        ctx.free(s);
        let after = ctx.live_counts();
        assert_eq!(after.textures, before.textures + 1);
        assert_eq!(after.sounds, before.sounds);
    });
}

#[cfg(feature = "3d")]
mod three {
    use super::*;

    const EFFECT: &str = "fn effect(p: Pixel) -> vec3<f32> { return p.color; }";
    const COMPUTE: &str = "fn compute(c: Cell) -> vec4<f32> { return c.value; }";
    const SHADER: &str = "fn surface(s: Surface) -> Surface { return s; }";

    #[test]
    fn every_3d_handle_kind_frees_and_stays_dead() {
        Game::new("t", 1, 1).run_headless(0, |ctx| {
            let white = ctx.gfx.default_material();
            let before = ctx.live_counts();
            let cube = |ctx: &mut Ctx| {
                let m = ctx.gfx.default_material();
                ctx.gfx.create_mesh(&MeshData::cube(1.0), m)
            };
            let mesh = cube(ctx);
            frees_reusing(ctx, mesh, cube);
            let plain = |ctx: &mut Ctx| ctx.gfx.create_material(MaterialDesc::default());
            let material = plain(ctx);
            frees(ctx, material, plain);
            let shader = |ctx: &mut Ctx| ctx.gfx.create_shader("s", SHADER);
            let s = shader(ctx);
            frees(ctx, s, shader);
            let compute = |ctx: &mut Ctx| ctx.gfx.create_compute("c", COMPUTE);
            let c = compute(ctx);
            frees(ctx, c, compute);
            let effect = |ctx: &mut Ctx| ctx.gfx.create_effect("e", EFFECT);
            let e = effect(ctx);
            frees(ctx, e, effect);
            let buffer = |ctx: &mut Ctx| ctx.gfx.create_gpu_buffer(4, &[]);
            let b = buffer(ctx);
            frees_reusing(ctx, b, buffer);
            let cube_mesh = cube(ctx);
            let batch = move |ctx: &mut Ctx| {
                ctx.gfx
                    .create_instance_batch(cube_mesh, &[Mat4::IDENTITY], InstanceBatchOptions::default())
            };
            let i = batch(ctx);
            frees(ctx, i, batch);
            ctx.free(cube_mesh);
            let array = |ctx: &mut Ctx| ctx.gfx.create_texture_array(1, 1, &[&[255; 4]], TextureKind::Color);
            let a = array(ctx);
            frees(ctx, a, array);
            let target = |ctx: &mut Ctx| ctx.gfx.create_render_target(8, 8);
            let t = target(ctx);
            assert!(ctx.gfx.is_render_target(t));
            frees(ctx, t, target);
            assert!(!ctx.gfx.is_render_target(t));
            assert_eq!(ctx.live_counts(), before, "everything made was freed");
            assert!(ctx.is_live(white));
            for none in [Mesh::NONE.id(), Material::NONE.id(), GpuBuffer::NONE.id()] {
                assert_eq!(none, u32::MAX);
            }
            assert!(!ctx.is_live(Mesh::NONE) && !ctx.is_live(Material::NONE) && !ctx.is_live(Shader::NONE));
            assert!(!ctx.is_live(Compute::NONE) && !ctx.is_live(Effect::NONE) && !ctx.is_live(GpuBuffer::NONE));
            assert!(!ctx.is_live(InstanceBatch::NONE) && !ctx.is_live(TextureArray::NONE));
        });
    }

    #[test]
    fn the_old_names_agree_with_is_live() {
        Game::new("t", 1, 1).run_headless(0, |ctx| {
            let material = ctx.gfx.create_material(MaterialDesc::default());
            let shader = ctx.gfx.create_shader("s", SHADER);
            let compute = ctx.gfx.create_compute("c", COMPUTE);
            let effect = ctx.gfx.create_effect("e", EFFECT);
            let array = ctx.gfx.create_texture_array(1, 1, &[&[255; 4]], TextureKind::Color);
            let mesh = ctx.gfx.create_mesh(&MeshData::cube(1.0), material);
            assert!(ctx.gfx.has_material(material) && ctx.gfx.has_shader(shader) && ctx.gfx.has_compute(compute));
            assert!(ctx.gfx.has_effect(effect) && ctx.gfx.has_texture_array(array) && ctx.gfx.is_mesh_live(mesh));
            ctx.free(material);
            ctx.free(shader);
            ctx.free(compute);
            ctx.free(effect);
            ctx.free(array);
            ctx.free(mesh);
            assert!(!ctx.gfx.has_material(material) && !ctx.gfx.has_shader(shader) && !ctx.gfx.has_compute(compute));
            assert!(!ctx.gfx.has_effect(effect) && !ctx.gfx.has_texture_array(array) && !ctx.gfx.is_mesh_live(mesh));
        });
    }

    /// Spawns models that use one of each kind, frees them all, and keeps
    /// stepping: nothing panics. (The offscreen render test checks what
    /// such models draw.)
    #[test]
    fn models_whose_resources_were_freed_still_step() {
        Game::new("t", 64, 64)
            .system(|ctx: &mut Ctx| {
                let models = ctx.world.query::<&Model>().iter().count();
                assert_eq!(models, 2);
                ctx.draw_text("still here", vec2(0.0, 0.0), 16.0, Color::WHITE);
            })
            .run_headless(3, |ctx| {
                let image = ctx.gfx.create_texture(2, 2, &[255; 16]);
                let normal = ctx.gfx.create_texture(2, 2, &[128; 16]);
                let shader = ctx.gfx.create_shader("s", SHADER);
                let array = ctx.gfx.create_texture_array(1, 1, &[&[255; 4]], TextureKind::Color);
                let material = ctx.gfx.create_material(MaterialDesc {
                    texture: Some(image),
                    normal_map: Some(normal),
                    shader: Some(shader),
                    custom_arrays: [Some(array), None, None],
                    ..Default::default()
                });
                let mesh = ctx.gfx.create_mesh(&MeshData::cube(1.0), material);
                let other = ctx.gfx.create_mesh(&MeshData::cube(1.0), material);
                let target = ctx.gfx.create_render_target(16, 16);
                let effect = ctx.gfx.create_effect("e", EFFECT);
                let compute = ctx.gfx.create_compute("c", COMPUTE);
                ctx.effects.push(ScreenEffect::new(effect).with_material(material));
                ctx.compute_passes.push(ComputePass::new(compute, target));
                ctx.target_cameras.push(TargetCamera::new(ctx.camera3d, target));
                ctx.world.spawn((Transform3D::default(), Model::new(mesh)));
                // Drawn with the material as an override too (ModelMaterial).
                ctx.world.spawn((
                    Transform3D::at(vec3(2.0, 0.0, 0.0)),
                    Model::new(other),
                    ModelMaterial(material),
                ));
                for h in [image, normal, target] {
                    ctx.free(h);
                }
                ctx.free(shader);
                ctx.free(array);
                ctx.free(material);
                ctx.free(mesh);
                ctx.free(effect);
                ctx.free(compute);
                // A freed material can't be brought back by updating it.
                ctx.gfx.update_material(material, MaterialDesc::default());
                assert!(!ctx.is_live(material));
                // A freed texture takes no pixels.
                ctx.gfx.update_texture(&image, &[0; 16]);
                // New materials may still name freed handles: they draw as defaults.
                let late = ctx.gfx.create_material(MaterialDesc {
                    texture: Some(image),
                    shader: Some(shader),
                    ..Default::default()
                });
                assert!(ctx.is_live(late));
                assert!(ctx.is_live(other));
            });
    }

    #[test]
    fn freeing_the_default_material_makes_a_new_one() {
        Game::new("t", 1, 1).run_headless(0, |ctx| {
            let white = ctx.gfx.default_material();
            ctx.free(white);
            let again = ctx.gfx.default_material();
            assert_ne!(again, white);
            assert!(ctx.is_live(again));
        });
    }

    #[test]
    fn freeing_decal_textures_clears_them() {
        Game::new("t", 1, 1).run_headless(0, |ctx| {
            let images = ctx.gfx.create_texture_array(1, 1, &[&[255; 4]], TextureKind::Data);
            ctx.gfx.set_decal_textures(Some(images));
            ctx.free(images);
            assert_eq!(ctx.gfx.decal_textures(), None);
        });
    }

    #[test]
    fn a_freed_loaded_mesh_loads_again() {
        Game::new("t", 1, 1).run_headless(0, |ctx| {
            let house = ctx.gfx.load_mesh("models/house.glb");
            let counts = ctx.live_counts();
            ctx.free(house);
            assert!(!ctx.is_live(house));
            assert_eq!(ctx.live_counts().meshes, counts.meshes - 1);
            // Loaded afresh (into the freed id, which the next mesh takes).
            let again = ctx.gfx.load_mesh("models/house.glb");
            assert!(ctx.is_live(again));
            assert_eq!(ctx.live_counts().meshes, counts.meshes);
        });
    }

    #[test]
    fn a_streamed_out_cell_frees_its_resources() {
        Game::new("t", 1, 1).run_headless(0, |ctx| {
            // ANCHOR: handles
            let before = ctx.live_counts();
            let bricks = ctx.gfx.create_texture(1, 1, &[200, 80, 60, 255]);
            let wall = ctx.gfx.create_material(MaterialDesc {
                texture: Some(bricks),
                ..Default::default()
            });
            let block = ctx.gfx.create_mesh(&MeshData::cube(1.0), wall);
            ctx.world.spawn((Transform3D::default(), Model::new(block)));
            // Going out of range: free what the cell made. The model still
            // draws (untextured), until its entity is despawned too.
            ctx.free(bricks);
            ctx.free(wall);
            ctx.free(block);
            assert!(!ctx.is_live(bricks) && !ctx.is_live(wall) && !ctx.is_live(block));
            assert!(!ctx.is_live(Texture::NONE));
            assert_eq!(ctx.live_counts(), before);
            // ANCHOR_END: handles
        });
    }
}
