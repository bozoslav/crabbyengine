//! Asset handle accounting: live handles by class with estimated bytes, and
//! leak checks that create and free resources in a loop and expect the
//! inventory back at its baseline.

use ponko::*;

fn headless(setup: impl FnOnce(&mut Ctx)) {
    Game::new("assets", 1, 1).run_headless(0, setup);
}

#[test]
fn textures_and_sounds_are_counted_with_their_bytes() {
    headless(|ctx| {
        let before = ctx.asset_inventory();
        assert!(before.gpu.is_none(), "headless runs have no GPU to count");
        let _a = ctx.gfx.create_texture(4, 2, &[0; 4 * 2 * 4]);
        let _b = ctx.gfx.create_texture(1, 1, &[0; 4]);
        let _beep = ctx.audio.create(1000, &[0.0; 100]);
        let after = ctx.asset_inventory();
        let textures = after.get(AssetClass::Texture);
        assert_eq!(textures.live, before.get(AssetClass::Texture).live + 2);
        assert_eq!(textures.bytes, before.get(AssetClass::Texture).bytes + (8 + 1) * 4);
        assert_eq!(
            after.get(AssetClass::Sound).live,
            before.get(AssetClass::Sound).live + 1
        );
        let grown: Vec<AssetClass> = after.growth_since(&before).iter().map(|g| g.class).collect();
        assert!(
            grown.contains(&AssetClass::Texture) && grown.contains(&AssetClass::Sound),
            "{grown:?}"
        );
        assert!(after.total_bytes() >= textures.bytes);
    });
}

#[test]
fn loading_the_same_texture_again_adds_nothing() {
    headless(|ctx| {
        let _ = ctx.load_texture("crab.png");
        let base = ctx.asset_inventory();
        for _ in 0..10 {
            let _ = ctx.load_texture("crab.png");
        }
        assert!(ctx.asset_inventory().growth_since(&base).is_empty());
    });
}

#[cfg(feature = "3d")]
mod three {
    use super::*;

    fn cube_bytes() -> u64 {
        let cube = MeshData::cube(1.0);
        cube.positions.len() as u64 * 44 + cube.indices.len() as u64 * 4
    }

    #[test]
    fn mesh_create_free_cycles_return_to_baseline() {
        headless(|ctx| {
            let material = ctx.gfx.default_material();
            let base = ctx.asset_inventory();
            for round in 0..200 {
                let meshes: Vec<Mesh> = (0..5)
                    .map(|i| ctx.gfx.create_mesh(&MeshData::cube(1.0 + i as f32), material))
                    .collect();
                let during = ctx.asset_inventory();
                assert_eq!(during.get(AssetClass::Mesh).live, base.get(AssetClass::Mesh).live + 5);
                assert_eq!(
                    during.get(AssetClass::Mesh).bytes,
                    base.get(AssetClass::Mesh).bytes + 5 * cube_bytes()
                );
                for mesh in meshes {
                    ctx.gfx.free_mesh(mesh);
                    ctx.gfx.free_mesh(mesh); // twice: still freed once
                }
                let growth = ctx.asset_inventory().growth_since(&base);
                assert!(growth.is_empty(), "round {round}: {growth:?}");
            }
            assert_eq!(ctx.asset_inventory(), base);
        });
    }

    #[test]
    fn a_leak_names_the_class_that_grows() {
        headless(|ctx| {
            let material = ctx.gfx.default_material();
            let base = ctx.asset_inventory();
            for _ in 0..3 {
                let kept = ctx.gfx.create_mesh(&MeshData::cube(1.0), material);
                let freed = ctx.gfx.create_mesh(&MeshData::cube(1.0), material);
                ctx.gfx.free_mesh(freed);
                let _ = kept; // never freed: the leak
            }
            let growth = ctx.asset_inventory().growth_since(&base);
            assert_eq!(growth.len(), 1, "{growth:?}");
            assert_eq!(growth[0].class, AssetClass::Mesh);
            assert_eq!(growth[0].live, 3);
            assert_eq!(growth[0].bytes, 3 * cube_bytes() as i64);
        });
    }

    #[test]
    fn gltf_free_and_reload_cycles_do_not_grow() {
        headless(|ctx| {
            // The first load makes the file's textures and materials; they
            // are kept for the next load of the same file.
            let house = ctx.gfx.load_mesh("models/house.glb");
            ctx.gfx.free_mesh(house);
            let base = ctx.asset_inventory();
            for round in 0..20 {
                let house = ctx.gfx.load_mesh("models/house.glb");
                assert!(ctx.gfx.is_mesh_live(house));
                let during = ctx.asset_inventory();
                assert_eq!(during.get(AssetClass::Mesh).live, base.get(AssetClass::Mesh).live + 1);
                assert_eq!(during.get(AssetClass::Material), base.get(AssetClass::Material));
                assert_eq!(during.get(AssetClass::Texture), base.get(AssetClass::Texture));
                ctx.gfx.free_mesh(house);
                let growth = ctx.asset_inventory().growth_since(&base);
                assert!(growth.is_empty(), "round {round}: {growth:?}");
            }
        });
    }

    #[test]
    fn gpu_buffer_cycles_return_to_baseline() {
        headless(|ctx| {
            let base = ctx.asset_inventory();
            for len in 1..100u32 {
                let buffer = ctx.gfx.create_gpu_buffer(len, &[]);
                let during = ctx.asset_inventory().get(AssetClass::GpuBuffer);
                assert_eq!((during.live, during.bytes), (1, u64::from(len) * 16));
                ctx.free(buffer);
            }
            assert!(ctx.asset_inventory().growth_since(&base).is_empty());
        });
    }

    #[test]
    fn never_freed_classes_count_what_was_made() {
        headless(|ctx| {
            let base = ctx.asset_inventory();
            let _ = ctx.gfx.create_material(MaterialDesc::default());
            let _ = ctx.gfx.create_render_target(16, 8);
            let _ = ctx
                .gfx
                .create_texture_array(2, 2, &[&[0; 16], &[0; 16]], TextureKind::Color);
            let now = ctx.asset_inventory();
            assert_eq!(
                now.get(AssetClass::Material).live,
                base.get(AssetClass::Material).live + 1
            );
            let target = now.get(AssetClass::RenderTarget);
            assert_eq!((target.live, target.bytes), (1, 16 * 8 * 8));
            // Two layers of 2x2 and 1x1 mipmaps.
            let arrays = now.get(AssetClass::TextureArray);
            assert_eq!((arrays.live, arrays.bytes), (1, 2 * (4 + 1) * 4));
        });
    }
}

#[test]
fn freed_handles_leave_the_inventory() {
    headless(|ctx| {
        let base = ctx.asset_inventory();
        for _ in 0..3 {
            let texture = ctx.gfx.create_texture(4, 4, &[0; 4 * 4 * 4]);
            let beep = ctx.audio.create(1000, &[0.0; 100]);
            assert_eq!(
                ctx.asset_inventory().get(AssetClass::Texture).live,
                base.get(AssetClass::Texture).live + 1
            );
            ctx.free(texture);
            ctx.free(beep);
        }
        let after = ctx.asset_inventory();
        for class in [AssetClass::Texture, AssetClass::Sound] {
            assert_eq!(
                (after.get(class).live, after.get(class).bytes),
                (base.get(class).live, base.get(class).bytes),
                "{class:?}"
            );
        }
        assert!(after.growth_since(&base).is_empty());
    });
}
