//! `use ponko::*;` next to a module glob: every name both bring in is the
//! same item, so neither glob makes it ambiguous. Each `use self::Name as _;`
//! fails to compile (E0659) if the two globs disagree on `Name`. The names that
//! differ by dimension on purpose are listed in book/src/naming.md.
#![allow(deprecated, unused_imports)]

mod root_and_d2 {
    use self::Animation as _;
    use self::Attached as _;
    #[cfg(any(feature = "physics", feature = "physics3d"))]
    use self::BodyKind as _;
    use self::Camera as _;
    #[cfg(feature = "physics")]
    use self::Collider as _;
    #[cfg(any(feature = "physics", feature = "physics3d"))]
    use self::CollisionStarted as _;
    #[cfg(any(feature = "physics", feature = "physics3d"))]
    use self::CollisionStopped as _;
    use self::FlowField as _;
    #[cfg(feature = "physics")]
    use self::Hit as _;
    use self::NavGrid as _;
    #[cfg(feature = "physics")]
    use self::Physics as _;
    #[cfg(feature = "physics")]
    use self::RigidBody as _;
    #[cfg(feature = "physics")]
    use self::Shape as _;
    use self::SpatialGrid as _;
    use self::Sprite as _;
    use self::SpriteSheet as _;
    use self::TileMap as _;
    use self::Transform as _;
    #[cfg(feature = "physics")]
    use self::Velocity as _;
    use ponko::d2::*;
    use ponko::*;
    // Different items on purpose: QueryFilter.
}

#[cfg(feature = "3d")]
mod root_and_d3 {
    use self::AmbientOcclusion as _;
    use self::AnimationClip as _;
    use self::AnimationCondition as _;
    use self::AnimationEvent as _;
    use self::AnimationGraph as _;
    use self::AnimationLayer as _;
    use self::AnimationParameter as _;
    use self::AnimationState as _;
    use self::AnimationTrack as _;
    use self::AnimationTransition as _;
    use self::Animator as _;
    use self::AntiAliasing as _;
    use self::AttachToJoint as _;
    use self::AudioListener as _;
    use self::AudioSettings as _;
    use self::AudioSource as _;
    use self::Blend as _;
    #[cfg(any(feature = "physics", feature = "physics3d"))]
    use self::BodyKind as _;
    use self::Camera as _;
    #[cfg(feature = "physics3d")]
    use self::Character as _;
    #[cfg(feature = "physics3d")]
    use self::ColliderFocus as _;
    #[cfg(feature = "physics3d")]
    use self::CollisionGroups as _;
    #[cfg(feature = "physics3d")]
    use self::CollisionMesh as _;
    #[cfg(feature = "physics3d")]
    use self::CollisionMeshData as _;
    #[cfg(any(feature = "physics", feature = "physics3d"))]
    use self::CollisionStarted as _;
    #[cfg(any(feature = "physics", feature = "physics3d"))]
    use self::CollisionStopped as _;
    use self::Compute as _;
    use self::ComputePass as _;
    #[cfg(feature = "physics3d")]
    use self::ContactModifier as _;
    #[cfg(feature = "physics3d")]
    use self::ContactPair as _;
    use self::Decal as _;
    use self::Effect as _;
    use self::FarTerrain as _;
    #[cfg(feature = "physics3d")]
    use self::FrictionCurve as _;
    use self::GlobalIllumination as _;
    use self::GpuBuffer as _;
    use self::GraphicsQuality as _;
    use self::Heightfield as _;
    #[cfg(feature = "physics3d")]
    use self::Hit as _;
    use self::InstanceBatch as _;
    use self::InstanceBatchOptions as _;
    use self::InstanceLevel as _;
    use self::Interpolation as _;
    use self::JointAnimation as _;
    #[cfg(feature = "physics3d")]
    use self::JointAxis as _;
    #[cfg(feature = "physics3d")]
    use self::JointDrive as _;
    use self::JointMask as _;
    #[cfg(feature = "physics3d")]
    use self::JointMotion as _;
    use self::LayerBlend as _;
    use self::Light as _;
    use self::Lighting as _;
    use self::Line as _;
    use self::LodGroup as _;
    use self::LodHysteresis as _;
    use self::LodLevel as _;
    use self::LodPick as _;
    use self::LodRenderer as _;
    use self::LodSettings as _;
    use self::LodShadow as _;
    use self::Material as _;
    use self::MaterialDesc as _;
    use self::Mesh as _;
    use self::MeshCost as _;
    use self::MeshData as _;
    use self::Model as _;
    use self::ModelMaterial as _;
    use self::ModelMaterials as _;
    use self::MorphAnimation as _;
    use self::MorphFrame as _;
    use self::MorphTarget as _;
    use self::MorphWeights as _;
    use self::Motion as _;
    use self::MotionChild as _;
    #[cfg(feature = "navmesh")]
    use self::NavAgent as _;
    #[cfg(feature = "navmesh")]
    use self::NavMesh as _;
    #[cfg(feature = "navmesh")]
    use self::NavObstacle as _;
    use self::ParticleBlend as _;
    use self::ParticleCurve as _;
    use self::ParticleEmitter as _;
    use self::ParticleGradient as _;
    use self::ParticleSettings as _;
    use self::ParticleShape as _;
    use self::ParticleSpace as _;
    use self::PointLight as _;
    use self::Post as _;
    use self::QualityLevel as _;
    #[cfg(feature = "physics3d")]
    use self::Ragdoll as _;
    #[cfg(feature = "physics3d")]
    use self::RagdollBone as _;
    use self::ReceivesDecals as _;
    use self::ReflectionProbe as _;
    use self::RootMotion as _;
    use self::ScreenEffect as _;
    use self::Shader as _;
    use self::ShaderDiagnostic as _;
    use self::ShaderKind as _;
    use self::ShaderOutcome as _;
    use self::ShaderStatus as _;
    use self::ShadowMesh as _;
    use self::Skeleton as _;
    use self::SkinOf as _;
    use self::Sky as _;
    use self::SpotLight as _;
    use self::StreamedLod as _;
    use self::TargetCamera as _;
    use self::TargetEffect as _;
    #[cfg(feature = "physics3d")]
    use self::TerrainCollider as _;
    use self::TerrainMaterialDesc as _;
    use self::TerrainTile as _;
    use self::TerrainTileData as _;
    use self::Text as _;
    use self::TextAlign as _;
    use self::TextureArray as _;
    use self::TextureKind as _;
    use self::Tonemap as _;
    use self::Trail as _;
    use self::Transform as _;
    use self::UiPanel as _;
    #[cfg(feature = "physics3d")]
    use self::Vehicle as _;
    #[cfg(feature = "physics3d")]
    use self::Wheel as _;
    use ponko::d3::*;
    use ponko::*;
    // Different items on purpose: Collider, RigidBody, Velocity, Shape, Physics, QueryFilter, Joint.
}

mod root_and_prelude {
    use self::Action as _;
    use self::ActionId as _;
    use self::ActionMap as _;
    use self::ActionRef as _;
    use self::Added as _;
    use self::AssetSource as _;
    use self::Assets as _;
    use self::AudioBus as _;
    use self::Axis2Binding as _;
    use self::AxisBinding as _;
    #[cfg(any(feature = "physics", feature = "physics3d"))]
    use self::BodyKind as _;
    use self::ButtonBinding as _;
    use self::Camera2D as _;
    #[cfg(feature = "3d")]
    use self::Camera3D as _;
    use self::Changed as _;
    #[cfg(any(feature = "physics", feature = "physics3d"))]
    use self::CollisionStarted as _;
    #[cfg(any(feature = "physics", feature = "physics3d"))]
    use self::CollisionStopped as _;
    use self::Color as _;
    use self::Commands as _;
    use self::Component as _;
    use self::ConfiguredGame as _;
    use self::Ctx as _;
    use self::DrawParams as _;
    use self::Entity as _;
    use self::Error as _;
    use self::ErrorKind as _;
    use self::Events as _;
    use self::Font as _;
    use self::Game as _;
    use self::Handle as _;
    use self::Key as _;
    #[cfg(feature = "3d")]
    use self::Mat4 as _;
    use self::MemorySource as _;
    #[cfg(feature = "3d")]
    use self::Model as _;
    use self::MouseButton as _;
    use self::Or as _;
    use self::Phase as _;
    use self::Placement as _;
    use self::Playing as _;
    use self::Plugin as _;
    use self::Quat as _;
    use self::Query as _;
    use self::Random as _;
    use self::Rect as _;
    use self::Rng as _;
    use self::SetupResult as _;
    use self::Sound as _;
    use self::SoundParams as _;
    use self::Sprite as _;
    use self::SpriteSheet as _;
    use self::SystemId as _;
    use self::Systems as _;
    use self::TextParams as _;
    use self::Texture as _;
    use self::TextureFilter as _;
    use self::Tick as _;
    use self::Time as _;
    use self::Transform2D as _;
    #[cfg(feature = "3d")]
    use self::Transform3D as _;
    use self::Turns as _;
    use self::UVec2 as _;
    use self::Ui as _;
    use self::UiId as _;
    use self::Vec2 as _;
    use self::Vec3 as _;
    #[cfg(feature = "3d")]
    use self::Vec4 as _;
    use self::With as _;
    use self::Without as _;
    use self::World as _;
    use self::WorldReflect as _;
    use self::actions as _;
    use self::d2 as _;
    #[cfg(feature = "3d")]
    use self::d3 as _;
    use self::json as _;
    use self::profile_scope as _;
    use self::systems as _;
    use self::uvec2 as _;
    use self::vec2 as _;
    use self::vec3 as _;
    #[cfg(feature = "3d")]
    use self::vec4 as _;
    use ponko::prelude::*;
    use ponko::*;
}

// The root glob next to a service module's glob: every name both bring in
// is the same item (generated from the modules' names that are also at the
// root).
mod root_and_anim {
    use self::Animation as _;
    #[cfg(feature = "3d")]
    use self::AnimationClip as _;
    #[cfg(feature = "3d")]
    use self::AnimationCondition as _;
    #[cfg(feature = "3d")]
    use self::AnimationEvent as _;
    #[cfg(feature = "3d")]
    use self::AnimationGraph as _;
    #[cfg(feature = "3d")]
    use self::AnimationLayer as _;
    #[cfg(feature = "3d")]
    use self::AnimationParameter as _;
    #[cfg(feature = "3d")]
    use self::AnimationState as _;
    #[cfg(feature = "3d")]
    use self::AnimationTrack as _;
    #[cfg(feature = "3d")]
    use self::AnimationTransition as _;
    #[cfg(feature = "3d")]
    use self::Animator as _;
    #[cfg(feature = "3d")]
    use self::AttachToJoint as _;
    #[cfg(feature = "3d")]
    use self::Blend as _;
    #[cfg(feature = "3d")]
    use self::Interpolation as _;
    #[cfg(feature = "3d")]
    use self::Joint as _;
    #[cfg(feature = "3d")]
    use self::JointAnimation as _;
    #[cfg(feature = "3d")]
    use self::JointMask as _;
    #[cfg(feature = "3d")]
    use self::LayerBlend as _;
    #[cfg(feature = "3d")]
    use self::MorphAnimation as _;
    #[cfg(feature = "3d")]
    use self::MorphFrame as _;
    #[cfg(feature = "3d")]
    use self::MorphTarget as _;
    #[cfg(feature = "3d")]
    use self::MorphWeights as _;
    #[cfg(feature = "3d")]
    use self::Motion as _;
    #[cfg(feature = "3d")]
    use self::MotionChild as _;
    #[cfg(feature = "3d")]
    use self::RootMotion as _;
    #[cfg(feature = "3d")]
    use self::Skeleton as _;
    #[cfg(feature = "3d")]
    use self::SkinOf as _;
    use self::SpriteSheet as _;
    use self::animate_sprites as _;
    #[cfg(feature = "3d")]
    use self::look_at_joint as _;
    #[cfg(feature = "3d")]
    use self::two_bone_ik as _;
    use ponko::anim::*;
    use ponko::*;
}

mod root_and_nav {
    use self::FlowField as _;
    #[cfg(feature = "navmesh")]
    use self::NavBakeError as _;
    #[cfg(feature = "navmesh")]
    use self::NavBakeSettings as _;
    use self::NavGrid as _;
    #[cfg(feature = "navmesh")]
    use self::NavSource as _;
    #[cfg(feature = "navmesh")]
    use self::step_navmesh3d as _;
    use ponko::nav::*;
    use ponko::*;
}

#[cfg(feature = "3d")]
mod root_and_terrain {
    use self::CellStreamer as _;
    use self::FarTerrain as _;
    use self::Heightfield as _;
    use self::TerrainMaterialDesc as _;
    use self::TerrainTile as _;
    use self::TerrainTileData as _;
    use self::TextureArray as _;
    use self::TextureKind as _;
    use ponko::terrain::*;
    use ponko::*;
}

mod root_and_access {
    use self::AccessConflict as _;
    use self::AccessReport as _;
    use self::ReportedPair as _;
    use self::ReportedSystem as _;
    use self::Service as _;
    use self::SystemAccess as _;
    use ponko::access::*;
    use ponko::*;
}

mod root_and_actions {
    use self::Action as _;
    use self::ActionId as _;
    use self::ActionMap as _;
    use self::ActionRef as _;
    use self::Axis2Binding as _;
    use self::AxisBinding as _;
    use self::ButtonBinding as _;
    use self::InputEvent as _;
    use self::UnknownAction as _;
    use ponko::actions::*;
    use ponko::*;
}

mod root_and_alloc_count {
    use self::AllocCounts as _;
    use self::CountingAllocator as _;
    use self::alloc_counts as _;
    use self::note_allocation as _;
    use ponko::alloc_count::*;
    use ponko::*;
}

mod root_and_assets {
    use self::AssetSource as _;
    use self::Assets as _;
    use self::MemorySource as _;
    use ponko::assets::*;
    use ponko::*;
}

mod root_and_attach {
    use self::Attached as _;
    use self::follow_attachments as _;
    use ponko::attach::*;
    use ponko::*;
}

mod root_and_audio {
    use self::Audio as _;
    use self::AudioBus as _;
    use self::AudioEffects as _;
    use self::Playing as _;
    use self::Sound as _;
    use self::SoundParams as _;
    use ponko::audio::*;
    use ponko::*;
}

mod root_and_camera {
    use self::Camera2D as _;
    use self::Rect as _;
    use self::camera_controls as _;
    use ponko::camera::*;
    use ponko::*;
}

mod root_and_color {
    use self::Color as _;
    use ponko::color::*;
    use ponko::*;
}

mod root_and_diagnostics {
    use self::ArchetypeDiagnostics as _;
    use self::ComponentDiagnostics as _;
    use self::ECS_DIAGNOSTICS_VERSION as _;
    use self::EcsDiagnostics as _;
    use self::MatchCount as _;
    use self::SystemDiagnostics as _;
    use ponko::diagnostics::*;
    use ponko::*;
}

mod root_and_error {
    use self::Error as _;
    use self::ErrorKind as _;
    use ponko::error::*;
    use ponko::*;
}

mod root_and_events {
    use self::Events as _;
    use ponko::events::*;
    use ponko::*;
}

mod root_and_flow {
    use self::FlowField as _;
    use self::NavGrid as _;
    use self::RawFlowView as _;
    use ponko::flow::*;
    use ponko::*;
}

mod root_and_game {
    use self::ConfiguredGame as _;
    use self::Game as _;
    use self::Phase as _;
    use self::Plugin as _;
    use self::SetupResult as _;
    use self::SystemId as _;
    use self::Systems as _;
    use ponko::game::*;
    use ponko::*;
}

mod root_and_gfx {
    use self::Graphics as _;
    use ponko::gfx::*;
    use ponko::*;
}

mod root_and_handle {
    use self::GfxHandle as _;
    use self::Handle as _;
    use self::LiveCounts as _;
    use ponko::handle::*;
    use ponko::*;
}

mod root_and_input {
    use self::Gamepad as _;
    use self::GamepadAxis as _;
    use self::GamepadButton as _;
    use self::GamepadStick as _;
    use self::Input as _;
    use self::MAX_GAMEPADS as _;
    use self::MouseButton as _;
    use ponko::input::*;
    use ponko::*;
}

mod root_and_inventory {
    use self::AssetClass as _;
    use self::AssetCount as _;
    use self::AssetGrowth as _;
    use self::AssetInventory as _;
    use self::DanglingHandle as _;
    use self::GpuResidency as _;
    use ponko::inventory::*;
    use ponko::*;
}

mod root_and_net {
    use self::Authority as _;
    use self::AuthorityCheck as _;
    use self::ClockSync as _;
    use self::Desync as _;
    use self::Interest as _;
    use self::InterestChange as _;
    use self::InterestChangeKind as _;
    use self::InterestConfig as _;
    use self::InterestEntity as _;
    use self::InterestEntry as _;
    use self::InterestTier as _;
    use self::InterestViewer as _;
    use self::Interpolate as _;
    use self::Lane as _;
    use self::LatestWins as _;
    use self::MAX_INPUT_BYTES as _;
    use self::MAX_INTEREST_TIERS as _;
    use self::MAX_MESSAGE_BYTES as _;
    use self::MAX_PLAYERS as _;
    use self::MessageEvent as _;
    use self::MessageStats as _;
    use self::Net as _;
    use self::NetMessages as _;
    #[cfg(feature = "3d")]
    use self::NetPose as _;
    use self::NetState as _;
    use self::NetStats as _;
    use self::PeerId as _;
    use self::Pose as _;
    use self::SnapshotBuffer as _;
    use self::SnapshotConfig as _;
    use self::SnapshotPush as _;
    use self::SnapshotSample as _;
    use self::SnapshotState as _;
    use self::SnapshotStream as _;
    #[cfg(feature = "steam")]
    use self::SteamLobby as _;
    #[cfg(feature = "steam")]
    use self::SteamLobbyVisibility as _;
    use self::UpdateStamp as _;
    #[cfg(feature = "3d")]
    use self::apply_net_poses as _;
    use self::seq_diff as _;
    use self::seq_newer as _;
    #[cfg(feature = "steam")]
    use self::steam_lobby_from_args as _;
    #[cfg(feature = "steam")]
    use self::steam_wait_for_invite as _;
    use ponko::net::*;
    use ponko::*;
}

mod root_and_perf {
    use self::AllocCounts as _;
    use self::CountingAllocator as _;
    use self::FrameStats as _;
    use self::GpuPass as _;
    use self::Instant as _;
    use self::Perf as _;
    use self::ProfileScope as _;
    use self::ScopeGuard as _;
    use self::ScopeTiming as _;
    use self::SystemTiming as _;
    use self::alloc_counts as _;
    use self::note_allocation as _;
    use self::scope_named as _;
    use ponko::perf::*;
    use ponko::*;
}

#[cfg(feature = "physics")]
mod root_and_physics {
    use self::Collider as _;
    use self::MAX_POLYGON_POINTS as _;
    use self::Physics as _;
    use self::RayHit as _;
    use self::RigidBody as _;
    use self::Shape as _;
    use self::Velocity as _;
    use self::step_physics as _;
    use ponko::physics::*;
    use ponko::*;
    // Different items on purpose: QueryFilter.
}

#[cfg(feature = "physics3d")]
mod root_and_physics3d {
    use self::Character3D as _;
    use self::Collider3D as _;
    use self::ColliderFocus as _;
    use self::CollisionGroups as _;
    use self::CollisionMesh as _;
    use self::CollisionMeshData as _;
    use self::ContactModifier3D as _;
    use self::ContactPair3D as _;
    use self::FrictionCurve as _;
    use self::Hit3D as _;
    use self::Joint3D as _;
    use self::JointAxis3D as _;
    use self::JointDrive3D as _;
    use self::JointMotion3D as _;
    use self::PHYSICS3D_HZ as _;
    use self::Physics3D as _;
    use self::QueryFilter3D as _;
    use self::Ragdoll3D as _;
    use self::RagdollBone as _;
    use self::RigidBody3D as _;
    use self::Shape3D as _;
    use self::TerrainCollider as _;
    use self::Vehicle3D as _;
    use self::Velocity3D as _;
    use self::Wheel3D as _;
    use self::interpolate_physics3d as _;
    use self::step_physics3d as _;
    use ponko::physics3d::*;
    use ponko::*;
}

mod root_and_random {
    use self::DEFAULT_SEED as _;
    use self::Random as _;
    use self::Rng as _;
    use ponko::random::*;
    use ponko::*;
}

mod root_and_reflect {
    use self::Field as _;
    use self::FieldType as _;
    use self::ReflectError as _;
    use self::ValueSchema as _;
    use self::WorldReflect as _;
    use self::json as _;
    use ponko::reflect::*;
    use ponko::*;
}

mod root_and_render {
    #[cfg(feature = "3d")]
    use self::AmbientOcclusion as _;
    #[cfg(feature = "3d")]
    use self::AntiAliasing as _;
    #[cfg(feature = "3d")]
    use self::Compute as _;
    #[cfg(feature = "3d")]
    use self::ComputePass as _;
    #[cfg(feature = "3d")]
    use self::Effect as _;
    #[cfg(feature = "3d")]
    use self::ExtractionBench as _;
    #[cfg(feature = "3d")]
    use self::ExtractionSample as _;
    #[cfg(feature = "3d")]
    use self::GlobalIllumination as _;
    use self::GpuAdapter as _;
    #[cfg(feature = "3d")]
    use self::GpuBuffer as _;
    use self::GpuCapabilities as _;
    use self::GpuFeature as _;
    use self::GpuFeatureStatus as _;
    use self::GpuLimits as _;
    use self::GpuTier as _;
    use self::GpuUploads as _;
    #[cfg(feature = "3d")]
    use self::GraphicsQuality as _;
    #[cfg(feature = "3d")]
    use self::InstanceBatch as _;
    #[cfg(feature = "3d")]
    use self::InstanceBatchOptions as _;
    #[cfg(feature = "3d")]
    use self::InstanceLevel as _;
    #[cfg(feature = "3d")]
    use self::Lighting as _;
    #[cfg(feature = "3d")]
    use self::LodGroup as _;
    #[cfg(feature = "3d")]
    use self::LodHysteresis as _;
    #[cfg(feature = "3d")]
    use self::LodLevel as _;
    #[cfg(feature = "3d")]
    use self::LodPick as _;
    #[cfg(feature = "3d")]
    use self::LodRenderer as _;
    #[cfg(feature = "3d")]
    use self::LodSettings as _;
    #[cfg(feature = "3d")]
    use self::LodShadow as _;
    #[cfg(feature = "3d")]
    use self::Material as _;
    #[cfg(feature = "3d")]
    use self::MaterialDesc as _;
    #[cfg(feature = "3d")]
    use self::Mesh as _;
    #[cfg(feature = "3d")]
    use self::MeshCost as _;
    #[cfg(feature = "3d")]
    use self::MeshData as _;
    #[cfg(feature = "3d")]
    use self::Model as _;
    #[cfg(feature = "3d")]
    use self::ModelMaterial as _;
    #[cfg(feature = "3d")]
    use self::ModelMaterials as _;
    #[cfg(feature = "3d")]
    use self::ParticleCurve as _;
    #[cfg(feature = "3d")]
    use self::ParticleGradient as _;
    use self::PipelineStats as _;
    #[cfg(feature = "3d")]
    use self::Post as _;
    #[cfg(feature = "3d")]
    use self::QualityLevel as _;
    #[cfg(feature = "3d")]
    use self::ReceivesDecals as _;
    use self::RenderStats as _;
    #[cfg(feature = "3d")]
    use self::ScreenEffect as _;
    #[cfg(feature = "3d")]
    use self::Shader as _;
    #[cfg(feature = "3d")]
    use self::ShaderDiagnostic as _;
    #[cfg(feature = "3d")]
    use self::ShaderKind as _;
    #[cfg(feature = "3d")]
    use self::ShaderOutcome as _;
    #[cfg(feature = "3d")]
    use self::ShaderStatus as _;
    #[cfg(feature = "3d")]
    use self::ShadowMesh as _;
    #[cfg(feature = "3d")]
    use self::Sky as _;
    #[cfg(feature = "3d")]
    use self::TargetCamera as _;
    #[cfg(feature = "3d")]
    use self::TargetEffect as _;
    #[cfg(feature = "3d")]
    use self::Tonemap as _;
    #[cfg(feature = "3d")]
    use self::look_at_joint as _;
    #[cfg(feature = "3d")]
    use self::two_bone_ik as _;
    use ponko::render::*;
    use ponko::*;
}

mod root_and_residency {
    use self::ResidencyKind as _;
    use self::ResidencyStats as _;
    use ponko::residency::*;
    use ponko::*;
}

mod root_and_scenario {
    use self::InputScenario as _;
    use self::MAX_SCENARIO_STEPS as _;
    use self::MAX_SCENARIO_TICKS as _;
    use self::SCENARIO_VERSION as _;
    use self::ScenarioCheck as _;
    use self::ScenarioError as _;
    use self::ScenarioFailure as _;
    use self::ScenarioReport as _;
    use self::ScenarioStatus as _;
    use ponko::scenario::*;
    use ponko::*;
}

mod root_and_scope {
    use self::ProfileScope as _;
    use self::ScopeGuard as _;
    use self::ScopeTiming as _;
    use self::scope_named as _;
    use ponko::scope::*;
    use ponko::*;
}

mod root_and_sheet {
    use self::Animation as _;
    use self::SpriteSheet as _;
    use self::animate_sprites as _;
    use ponko::sheet::*;
    use ponko::*;
}

mod root_and_spatial {
    use self::GridEntry as _;
    use self::RawGridView as _;
    use self::SpatialGrid as _;
    use self::rebuild_spatial_grid as _;
    use self::spatial_key as _;
    use ponko::spatial::*;
    use ponko::*;
}

mod root_and_stream {
    #[cfg(feature = "3d")]
    use self::CellStreamer as _;
    use self::Job as _;
    use ponko::stream::*;
    use ponko::*;
}

mod root_and_streaming {
    use self::AssetState as _;
    use self::DEFAULT_UPLOAD_BUDGET as _;
    use self::StreamedAsset as _;
    use self::UploadStats as _;
    use ponko::streaming::*;
    use ponko::*;
}

mod root_and_system_view {
    use self::SystemView as _;
    use self::ViewEvents as _;
    use ponko::system_view::*;
    use ponko::*;
}

mod root_and_systems {
    use self::animate_sprites as _;
    #[cfg(feature = "3d")]
    use self::apply_net_poses as _;
    use self::draw_sprites as _;
    use self::draw_tilemaps as _;
    use self::follow_attachments as _;
    #[cfg(feature = "physics3d")]
    use self::interpolate_physics3d as _;
    use self::rebuild_spatial_grid as _;
    #[cfg(feature = "navmesh")]
    use self::step_navmesh3d as _;
    #[cfg(feature = "physics")]
    use self::step_physics as _;
    #[cfg(feature = "physics3d")]
    use self::step_physics3d as _;
    use ponko::systems::*;
    use ponko::*;
}

mod root_and_text {
    use self::Font as _;
    use self::TextParams as _;
    use ponko::text::*;
    use ponko::*;
}

mod root_and_texture {
    use self::DrawParams as _;
    use self::Texture as _;
    use self::TextureFilter as _;
    use ponko::texture::*;
    use ponko::*;
}

mod root_and_tilemap {
    use self::TileMap as _;
    use self::draw_tilemaps as _;
    use ponko::tilemap::*;
    use ponko::*;
}

mod root_and_ui {
    use self::Ui as _;
    use self::UiAccessNode as _;
    use self::UiAccessTree as _;
    use self::UiAccessValue as _;
    use self::UiBuilder as _;
    use self::UiEvent as _;
    use self::UiId as _;
    use self::UiImmediate as _;
    use self::UiInput as _;
    use self::UiLayout as _;
    use self::UiPaint as _;
    #[cfg(feature = "3d")]
    use self::UiPanel3D as _;
    use self::UiRole as _;
    use self::UiStyle as _;
    use self::UiTextAlign as _;
    use self::UiTheme as _;
    use self::UiWidget as _;
    use ponko::ui::*;
    use ponko::*;
}

/// Every prelude item is also at the root, as the same item (the root glob
/// and the prelude glob above agree), and none of them is deprecated there.
#[deny(deprecated)]
mod root_holds_every_prelude_item {
    use ponko::Action as _;
    use ponko::ActionId as _;
    use ponko::ActionMap as _;
    use ponko::ActionRef as _;
    use ponko::Added as _;
    use ponko::AssetSource as _;
    use ponko::Assets as _;
    use ponko::AudioBus as _;
    use ponko::Axis2Binding as _;
    use ponko::AxisBinding as _;
    #[cfg(any(feature = "physics", feature = "physics3d"))]
    use ponko::BodyKind as _;
    use ponko::ButtonBinding as _;
    use ponko::Camera2D as _;
    #[cfg(feature = "3d")]
    use ponko::Camera3D as _;
    use ponko::Changed as _;
    #[cfg(any(feature = "physics", feature = "physics3d"))]
    use ponko::CollisionStarted as _;
    #[cfg(any(feature = "physics", feature = "physics3d"))]
    use ponko::CollisionStopped as _;
    use ponko::Color as _;
    use ponko::Commands as _;
    use ponko::Component as _;
    use ponko::ConfiguredGame as _;
    use ponko::Ctx as _;
    use ponko::DrawParams as _;
    use ponko::Entity as _;
    use ponko::Error as _;
    use ponko::ErrorKind as _;
    use ponko::Events as _;
    use ponko::Font as _;
    use ponko::Game as _;
    use ponko::Handle as _;
    use ponko::Key as _;
    #[cfg(feature = "3d")]
    use ponko::Mat4 as _;
    use ponko::MemorySource as _;
    #[cfg(feature = "3d")]
    use ponko::Model as _;
    use ponko::MouseButton as _;
    use ponko::Or as _;
    use ponko::Phase as _;
    use ponko::Placement as _;
    use ponko::Playing as _;
    use ponko::Plugin as _;
    use ponko::Quat as _;
    use ponko::Query as _;
    use ponko::Random as _;
    use ponko::Rect as _;
    use ponko::Rng as _;
    use ponko::SetupResult as _;
    use ponko::Sound as _;
    use ponko::SoundParams as _;
    use ponko::Sprite as _;
    use ponko::SpriteSheet as _;
    use ponko::SystemId as _;
    use ponko::Systems as _;
    use ponko::TextParams as _;
    use ponko::Texture as _;
    use ponko::TextureFilter as _;
    use ponko::Tick as _;
    use ponko::Time as _;
    use ponko::Transform2D as _;
    #[cfg(feature = "3d")]
    use ponko::Transform3D as _;
    use ponko::Turns as _;
    use ponko::UVec2 as _;
    use ponko::Ui as _;
    use ponko::UiId as _;
    use ponko::Vec2 as _;
    use ponko::Vec3 as _;
    #[cfg(feature = "3d")]
    use ponko::Vec4 as _;
    use ponko::With as _;
    use ponko::Without as _;
    use ponko::World as _;
    use ponko::WorldReflect as _;
    use ponko::actions as _;
    use ponko::d2 as _;
    #[cfg(feature = "3d")]
    use ponko::d3 as _;
    use ponko::json as _;
    use ponko::profile_scope as _;
    use ponko::systems as _;
    use ponko::uvec2 as _;
    use ponko::vec2 as _;
    use ponko::vec3 as _;
    #[cfg(feature = "3d")]
    use ponko::vec4 as _;
    #[allow(dead_code)]
    fn uses_the_names(
        _: Option<(
            ponko::Transform2D,
            ponko::Camera2D,
            ponko::Sprite,
            ponko::Ui,
            ponko::UiId,
        )>,
    ) {
    }
    #[cfg(feature = "3d")]
    #[allow(dead_code)]
    fn uses_the_3d_names(_: Option<(ponko::Transform3D, ponko::Camera3D, ponko::Model)>) {}
}

#[test]
fn the_root_glob_combines_with_the_prelude_and_dimension_globs() {
    // Compiling this file is the test.
}
