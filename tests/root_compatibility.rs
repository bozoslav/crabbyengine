//! Intentional coverage of retained root spellings; application code uses modules.
#![allow(deprecated)]
use ponko::*;
#[test]
fn old_glob_and_public_constructors_keep_their_identity() {
    let _: d2::Transform = Transform2D::at(vec2(1.0, 2.0));
    let mut world = World::new();
    let entity = world.spawn((Transform2D::default(),));
    assert!(world.get::<d2::Transform>(entity).is_some());
    let _: UiId = Ui::default().root().id();
}
#[cfg(feature = "3d")]
#[test]
fn old_3d_glob_preserves_component_layout_and_tuple_construction() {
    let _: d3::Transform = Transform3D::default();
    Game::new("legacy materials", 1, 1).run_headless(0, |ctx| {
        let material = ctx.gfx.default_material();
        let ModelMaterial(actual) = ModelMaterial(material);
        assert_eq!(actual, material);
    });
    let _: render::LodHysteresis = LodHysteresis(0.5);
    assert_eq!(std::mem::size_of::<Transform3D>(), std::mem::size_of::<d3::Transform>());
    assert_eq!(std::mem::align_of::<Transform3D>(), 16);
    let _: d3::ReceivesDecals = ReceivesDecals(1);
    let LodHysteresis(value) = LodHysteresis(0.5);
    let ReceivesDecals(mask) = ReceivesDecals(1);
    assert_eq!((value, mask), (0.5, 1));
}
#[cfg(feature = "physics3d")]
#[test]
fn old_enum_imports_remain_available() {
    use ponko::Shape3D::Sphere;
    assert!(matches!(Sphere { radius: 1.0 }, d3::Shape::Sphere { radius: 1.0 }));
}

#[test]
fn old_root_service_types_do_not_require_an_optional_subsystem() {
    // This is the old default import contract, independently of the new prelude.
    // In particular, turning 3D off must not hide 2D, UI, networking or profiling.
    macro_rules! available { ($($ty:ty),* $(,)?) => { $(let _: Option<$ty> = None;)* }; }
    available!(
        SystemAccess,
        Service,
        AccessConflict,
        Audio,
        AudioEffects,
        Camera2D,
        Transform2D,
        Sprite,
        Assets,
        MemorySource,
        AssetInventory,
        AssetClass,
        GpuResidency,
        Input,
        Sound,
        SoundParams,
        Playing,
        Font,
        TextParams,
        Texture,
        DrawParams,
        Rect,
        Graphics,
        Time,
        Turns,
        ActionMap,
        InputEvent,
        Random,
        Rng,
        Net,
        NetState,
        Pose,
        Authority,
        Interest,
        Perf,
        PerfReport,
        Budget,
        AllocReport,
        ScopeTiming,
        FrameStats,
        SystemTiming,
        Ui,
        UiId,
        UiImmediate,
        UiInput,
        UiLayout,
        UiPaint,
        UiRole,
        UiStyle,
        UiTextAlign,
        UiTheme,
        UiWidget,
        UiAccessNode,
        UiAccessTree,
        UiAccessValue,
    );
    let _: fn() -> perf::AllocCounts = alloc_counts;
    let _: fn(Vec2, f32) -> u64 = spatial_key;
    const DIFFERENCE: i16 = seq_diff(1, 0);
    assert_eq!(DIFFERENCE, 1);
}

#[test]
fn old_root_constants_and_moved_types_still_resolve_to_their_module_items() {
    assert_eq!(MAX_GAMEPADS, ponko::input::MAX_GAMEPADS);
    assert_eq!(MAX_PLAYERS, ponko::net::MAX_PLAYERS);
    assert_eq!(MAX_INPUT_BYTES, ponko::net::MAX_INPUT_BYTES);
    assert_eq!(MAX_MESSAGE_BYTES, ponko::net::MAX_MESSAGE_BYTES);
    assert_eq!(MAX_INTEREST_TIERS, ponko::net::MAX_INTEREST_TIERS);
    assert_eq!(DEFAULT_SEED, ponko::random::DEFAULT_SEED);
    assert_eq!(SCENARIO_VERSION, ponko::scenario::SCENARIO_VERSION);
    assert_eq!(MAX_SCENARIO_STEPS, ponko::scenario::MAX_SCENARIO_STEPS);
    assert_eq!(MAX_SCENARIO_TICKS, ponko::scenario::MAX_SCENARIO_TICKS);
    assert_eq!(DEFAULT_UPLOAD_BUDGET, ponko::streaming::DEFAULT_UPLOAD_BUDGET);
    assert_eq!(ECS_DIAGNOSTICS_VERSION, ponko::diagnostics::ECS_DIAGNOSTICS_VERSION);
    assert_eq!(SPATIAL_SORT_EVERY, ponko::spatial::SpatialGrid::<()>::SORT_EVERY);
    #[cfg(feature = "physics")]
    assert_eq!(MAX_POLYGON_POINTS, ponko::physics::MAX_POLYGON_POINTS);
    #[cfg(feature = "physics3d")]
    assert_eq!(PHYSICS3D_HZ, ponko::physics3d::PHYSICS3D_HZ);
    #[cfg(feature = "3d")]
    {
        assert_eq!(MAX_DECALS, d3::Decal::MAX);
        assert_eq!(TERRAIN_LODS, terrain::TerrainTile::LODS);
        assert_eq!(MIN_RENDER_SCALE, render::GraphicsQuality::MIN_RENDER_SCALE);
    }
    // Moved types keep their identity behind the old spellings.
    let attached: d2::Attached = Attached::to(Entity::from_bits(1));
    assert_eq!(attached.to, Entity::from_bits(1));
    let _: Option<&d2::TileMap> = None::<&TileMap>;
    let _: Option<&ponko::gfx::Graphics> = None::<&Graphics>;
    let _: Option<ponko::reflect::Field> = None::<Field>;
    let _: Option<ponko::ecs_core::ComponentId> = None::<ComponentId>;
    let _: Option<ponko::ecs_core::WorldView<'static>> = None::<WorldView<'static>>;
    let start: ponko::perf::Instant = Instant::now();
    assert!(start.elapsed().as_secs() < 60);
}

#[cfg(feature = "physics3d")]
#[test]
fn old_3d_joint_enum_variants_import_from_the_root() {
    use ponko::JointAxis3D::*;
    use ponko::JointMotion3D::{Free, Locked};
    assert!(matches!(LinearX, d3::JointAxis::LinearX));
    assert!(matches!(LinearZ, d3::JointAxis::LinearZ));
    assert!(matches!(Free, d3::JointMotion::Free));
    assert!(matches!(Locked, d3::JointMotion::Locked));
}

#[cfg(feature = "3d")]
#[test]
fn old_3d_particle_and_text_enum_variants_import_from_the_root() {
    use ponko::ParticleBlend3D::Additive;
    use ponko::ParticleShape3D::Point;
    use ponko::ParticleSpace3D::Local;
    use ponko::TextAlign3D::Center;
    assert!(matches!(Additive, d3::ParticleBlend::Additive));
    assert!(matches!(Point, d3::ParticleShape::Point));
    assert!(matches!(Local, d3::ParticleSpace::Local));
    assert!(matches!(Center, d3::TextAlign::Center));
}

// The documented global-allocator spelling: a unit struct used as a value.
use ponko::CountingAllocator as RootCountingAllocator;
static COUNTING: RootCountingAllocator = RootCountingAllocator;

#[test]
fn the_root_counting_allocator_is_the_unit_struct_value() {
    let _: &ponko::alloc_count::CountingAllocator = &COUNTING;
    assert_eq!(std::mem::size_of_val(&COUNTING), 0);
}
