//! Deprecated root aliases. Canonical module exports use original definitions.

#[doc(hidden)]
#[deprecated(note = "use ponko::access::AccessConflict")]
pub use crate::access::AccessConflict;

#[doc(hidden)]
#[deprecated(note = "use ponko::access::AccessReport")]
pub use crate::access::AccessReport;

#[doc(hidden)]
#[deprecated(note = "use ponko::access::ReportedPair")]
pub use crate::access::ReportedPair;

#[doc(hidden)]
#[deprecated(note = "use ponko::access::ReportedSystem")]
pub use crate::access::ReportedSystem;

#[doc(hidden)]
#[deprecated(note = "use ponko::access::Service")]
pub use crate::access::Service;

#[doc(hidden)]
#[deprecated(note = "use ponko::access::SystemAccess")]
pub use crate::access::SystemAccess;

#[doc(hidden)]
#[deprecated(note = "use ponko::actions::InputEvent")]
pub use crate::actions::InputEvent;

#[doc(hidden)]
#[deprecated(note = "use ponko::actions::UnknownAction")]
pub use crate::actions::UnknownAction;

#[doc(hidden)]
#[deprecated(note = "use ponko::system_view::SystemView")]
pub use crate::system_view::SystemView;

#[doc(hidden)]
#[deprecated(note = "use ponko::system_view::ViewEvents")]
pub use crate::system_view::ViewEvents;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::NavAgent")]
#[cfg(feature = "navmesh")]
pub use crate::d3::NavAgent as NavAgent3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::nav::NavBakeError")]
#[cfg(feature = "navmesh")]
pub use crate::nav::NavBakeError;

#[doc(hidden)]
#[deprecated(note = "use ponko::nav::NavBakeSettings")]
#[cfg(feature = "navmesh")]
pub use crate::nav::NavBakeSettings;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::NavMesh")]
#[cfg(feature = "navmesh")]
pub use crate::d3::NavMesh as NavMesh3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::NavObstacle")]
#[cfg(feature = "navmesh")]
pub use crate::d3::NavObstacle as NavObstacle3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::nav::NavSource")]
#[cfg(feature = "navmesh")]
pub use crate::nav::NavSource;

#[doc(hidden)]
#[deprecated(note = "use ponko::alloc_count::AllocCounts")]
pub use crate::alloc_count::AllocCounts;

#[doc(hidden)]
#[deprecated(note = "use ponko::alloc_count::CountingAllocator")]
pub use crate::alloc_count::CountingAllocator;

#[doc(hidden)]
#[deprecated(note = "use ponko::audio::Audio")]
pub use crate::audio::Audio;

#[doc(hidden)]
#[deprecated(note = "use ponko::audio::AudioEffects")]
pub use crate::audio::AudioEffects;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::AudioListener")]
#[cfg(feature = "3d")]
pub use crate::d3::AudioListener as AudioListener3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::audio::AudioRolloff")]
#[cfg(feature = "3d")]
pub use crate::audio::AudioRolloff;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::AudioSettings")]
#[cfg(feature = "3d")]
pub use crate::d3::AudioSettings as AudioSettings3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::AudioSource")]
#[cfg(feature = "3d")]
pub use crate::d3::AudioSource as AudioSource3D;

#[doc(hidden)]
pub use crate::d2::Camera as Camera2D;

#[doc(hidden)]
#[deprecated(note = "use ponko::diagnostics::ArchetypeDiagnostics")]
pub use crate::diagnostics::ArchetypeDiagnostics;

#[doc(hidden)]
#[deprecated(note = "use ponko::diagnostics::ComponentDiagnostics")]
pub use crate::diagnostics::ComponentDiagnostics;

#[doc(hidden)]
#[deprecated(note = "use ponko::diagnostics::EcsDiagnostics")]
pub use crate::diagnostics::EcsDiagnostics;

#[doc(hidden)]
#[deprecated(note = "use ponko::diagnostics::MatchCount")]
pub use crate::diagnostics::MatchCount;

#[doc(hidden)]
#[deprecated(note = "use ponko::diagnostics::SystemDiagnostics")]
pub use crate::diagnostics::SystemDiagnostics;

#[doc(hidden)]
pub use crate::d2::Sprite;

#[doc(hidden)]
pub use crate::d2::Transform as Transform2D;

#[doc(hidden)]
#[deprecated(note = "use ponko::flow::FlowField")]
pub use crate::flow::FlowField;

#[doc(hidden)]
#[deprecated(note = "use ponko::flow::NavGrid")]
pub use crate::flow::NavGrid;

#[doc(hidden)]
#[deprecated(note = "use ponko::handle::GfxHandle")]
pub use crate::handle::GfxHandle;

#[doc(hidden)]
#[deprecated(note = "use ponko::handle::LiveCounts")]
pub use crate::handle::LiveCounts;

#[doc(hidden)]
#[deprecated(note = "use ponko::input::Gamepad")]
pub use crate::input::Gamepad;

#[doc(hidden)]
#[deprecated(note = "use ponko::input::GamepadAxis")]
pub use crate::input::GamepadAxis;

#[doc(hidden)]
#[deprecated(note = "use ponko::input::GamepadButton")]
pub use crate::input::GamepadButton;

#[doc(hidden)]
#[deprecated(note = "use ponko::input::GamepadStick")]
pub use crate::input::GamepadStick;

#[doc(hidden)]
#[deprecated(note = "use ponko::input::Input")]
pub use crate::input::Input;

#[doc(hidden)]
#[deprecated(note = "use ponko::inventory::AssetClass")]
pub use crate::inventory::AssetClass;

#[doc(hidden)]
#[deprecated(note = "use ponko::inventory::AssetCount")]
pub use crate::inventory::AssetCount;

#[doc(hidden)]
#[deprecated(note = "use ponko::inventory::AssetGrowth")]
pub use crate::inventory::AssetGrowth;

#[doc(hidden)]
#[deprecated(note = "use ponko::inventory::AssetInventory")]
pub use crate::inventory::AssetInventory;

#[doc(hidden)]
#[deprecated(note = "use ponko::inventory::DanglingHandle")]
pub use crate::inventory::DanglingHandle;

#[doc(hidden)]
#[deprecated(note = "use ponko::inventory::GpuResidency")]
pub use crate::inventory::GpuResidency;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::Authority")]
pub use crate::net::Authority;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::AuthorityCheck")]
pub use crate::net::AuthorityCheck;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::ClockSync")]
pub use crate::net::ClockSync;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::Interest")]
pub use crate::net::Interest;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::InterestChange")]
pub use crate::net::InterestChange;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::InterestChangeKind")]
pub use crate::net::InterestChangeKind;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::InterestConfig")]
pub use crate::net::InterestConfig;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::InterestEntity")]
pub use crate::net::InterestEntity;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::InterestEntry")]
pub use crate::net::InterestEntry;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::InterestTier")]
pub use crate::net::InterestTier;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::InterestViewer")]
pub use crate::net::InterestViewer;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::LatestWins")]
pub use crate::net::LatestWins;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::Pose")]
pub use crate::net::Pose;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::SnapshotBuffer")]
pub use crate::net::SnapshotBuffer;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::SnapshotConfig")]
pub use crate::net::SnapshotConfig;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::SnapshotPush")]
pub use crate::net::SnapshotPush;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::SnapshotSample")]
pub use crate::net::SnapshotSample;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::SnapshotState")]
pub use crate::net::SnapshotState;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::SnapshotStream")]
pub use crate::net::SnapshotStream;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::UpdateStamp")]
pub use crate::net::UpdateStamp;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::Desync")]
pub use crate::net::Desync;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::Lane")]
pub use crate::net::Lane;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::MessageEvent")]
pub use crate::net::MessageEvent;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::MessageStats")]
pub use crate::net::MessageStats;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::Net")]
pub use crate::net::Net;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::NetMessages")]
pub use crate::net::NetMessages;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::NetState")]
pub use crate::net::NetState;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::NetStats")]
pub use crate::net::NetStats;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::PeerId")]
pub use crate::net::PeerId;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::SteamLobby")]
#[cfg(feature = "steam")]
pub use crate::net::SteamLobby;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::SteamLobbyVisibility")]
#[cfg(feature = "steam")]
pub use crate::net::SteamLobbyVisibility;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::Perf")]
pub use crate::perf::Perf;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::SystemTiming")]
pub use crate::perf::SystemTiming;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::AllocReport")]
pub use crate::perf::AllocReport;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::Budget")]
pub use crate::perf::Budget;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::BudgetViolation")]
pub use crate::perf::BudgetViolation;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::Contributor")]
pub use crate::perf::Contributor;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::FrameTimes")]
pub use crate::perf::FrameTimes;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::PassReport")]
pub use crate::perf::PassReport;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::PerfReport")]
pub use crate::perf::PerfReport;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::PerfWarning")]
pub use crate::perf::PerfWarning;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::Regression")]
pub use crate::perf::Regression;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::RenderReport")]
pub use crate::perf::RenderReport;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::ScopeReport")]
pub use crate::perf::ScopeReport;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::SystemReport")]
pub use crate::perf::SystemReport;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::WarningKind")]
pub use crate::perf::WarningKind;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::WorstFrame")]
pub use crate::perf::WorstFrame;

#[doc(hidden)]
#[deprecated(note = "use ponko::d2::Collider")]
#[cfg(feature = "physics")]
pub use crate::d2::Collider;

#[doc(hidden)]
#[deprecated(note = "use ponko::d2::Physics")]
#[cfg(feature = "physics")]
pub use crate::d2::Physics;

#[doc(hidden)]
#[deprecated(note = "use ponko::d2::Hit")]
#[cfg(feature = "physics")]
pub use crate::d2::Hit as RayHit;

#[doc(hidden)]
#[deprecated(note = "use ponko::d2::RigidBody")]
#[cfg(feature = "physics")]
pub use crate::d2::RigidBody;

#[doc(hidden)]
#[deprecated(note = "use ponko::d2::Shape")]
#[cfg(feature = "physics")]
pub use crate::d2::Shape;

#[doc(hidden)]
#[deprecated(note = "use ponko::d2::Velocity")]
#[cfg(feature = "physics")]
pub use crate::d2::Velocity;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Character")]
#[cfg(feature = "physics3d")]
pub use crate::d3::Character as Character3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Collider")]
#[cfg(feature = "physics3d")]
pub use crate::d3::Collider as Collider3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::physics3d::ColliderFocus")]
#[cfg(feature = "physics3d")]
pub use crate::physics3d::ColliderFocus;

#[doc(hidden)]
#[deprecated(note = "use ponko::physics3d::CollisionGroups")]
#[cfg(feature = "physics3d")]
pub use crate::physics3d::CollisionGroups;

#[doc(hidden)]
#[deprecated(note = "use ponko::physics3d::CollisionMesh")]
#[cfg(feature = "physics3d")]
pub use crate::physics3d::CollisionMesh;

#[doc(hidden)]
#[deprecated(note = "use ponko::physics3d::CollisionMeshData")]
#[cfg(feature = "physics3d")]
pub use crate::physics3d::CollisionMeshData;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::ContactModifier")]
#[cfg(feature = "physics3d")]
pub use crate::d3::ContactModifier as ContactModifier3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::ContactPair")]
#[cfg(feature = "physics3d")]
pub use crate::d3::ContactPair as ContactPair3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::physics3d::FrictionCurve")]
#[cfg(feature = "physics3d")]
pub use crate::physics3d::FrictionCurve;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Hit")]
#[cfg(feature = "physics3d")]
pub use crate::d3::Hit as Hit3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Joint")]
#[cfg(feature = "physics3d")]
pub use crate::d3::Joint as Joint3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::JointAxis")]
#[cfg(feature = "physics3d")]
pub use crate::d3::JointAxis as JointAxis3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::JointDrive")]
#[cfg(feature = "physics3d")]
pub use crate::d3::JointDrive as JointDrive3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::JointMotion")]
#[cfg(feature = "physics3d")]
pub use crate::d3::JointMotion as JointMotion3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Physics")]
#[cfg(feature = "physics3d")]
pub use crate::d3::Physics as Physics3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::QueryFilter")]
#[cfg(feature = "physics3d")]
pub use crate::d3::QueryFilter as QueryFilter3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Ragdoll")]
#[cfg(feature = "physics3d")]
pub use crate::d3::Ragdoll as Ragdoll3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::physics3d::RagdollBone")]
#[cfg(feature = "physics3d")]
pub use crate::physics3d::RagdollBone;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::RigidBody")]
#[cfg(feature = "physics3d")]
pub use crate::d3::RigidBody as RigidBody3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Shape")]
#[cfg(feature = "physics3d")]
pub use crate::d3::Shape as Shape3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::physics3d::TerrainCollider")]
#[cfg(feature = "physics3d")]
pub use crate::physics3d::TerrainCollider;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Vehicle")]
#[cfg(feature = "physics3d")]
pub use crate::d3::Vehicle as Vehicle3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Velocity")]
#[cfg(feature = "physics3d")]
pub use crate::d3::Velocity as Velocity3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Wheel")]
#[cfg(feature = "physics3d")]
pub use crate::d3::Wheel as Wheel3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::FrameStats")]
pub use crate::perf::FrameStats;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::GpuPass")]
pub use crate::perf::GpuPass;

#[doc(hidden)]
#[deprecated(note = "use ponko::reflect::ReflectError")]
pub use crate::reflect::ReflectError;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GpuAdapter")]
pub use crate::render::GpuAdapter;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GpuCapabilities")]
pub use crate::render::GpuCapabilities;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GpuFeature")]
pub use crate::render::GpuFeature;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GpuFeatureStatus")]
pub use crate::render::GpuFeatureStatus;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GpuLimits")]
pub use crate::render::GpuLimits;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GpuTier")]
pub use crate::render::GpuTier;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GpuUploads")]
pub use crate::render::GpuUploads;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::PipelineStats")]
pub use crate::render::PipelineStats;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::RenderStats")]
pub use crate::render::RenderStats;

#[doc(hidden)]
#[deprecated(note = "use ponko::residency::ResidencyKind")]
pub use crate::residency::ResidencyKind;

#[doc(hidden)]
#[deprecated(note = "use ponko::residency::ResidencyStats")]
pub use crate::residency::ResidencyStats;

#[doc(hidden)]
#[deprecated(note = "use ponko::scenario::InputScenario")]
pub use crate::scenario::InputScenario;

#[doc(hidden)]
#[deprecated(note = "use ponko::scenario::ScenarioCheck")]
pub use crate::scenario::ScenarioCheck;

#[doc(hidden)]
#[deprecated(note = "use ponko::scenario::ScenarioError")]
pub use crate::scenario::ScenarioError;

#[doc(hidden)]
#[deprecated(note = "use ponko::scenario::ScenarioFailure")]
pub use crate::scenario::ScenarioFailure;

#[doc(hidden)]
#[deprecated(note = "use ponko::scenario::ScenarioReport")]
pub use crate::scenario::ScenarioReport;

#[doc(hidden)]
#[deprecated(note = "use ponko::scenario::ScenarioStatus")]
pub use crate::scenario::ScenarioStatus;

#[doc(hidden)]
#[deprecated(note = "use ponko::scope::ProfileScope")]
pub use crate::scope::ProfileScope;

#[doc(hidden)]
#[deprecated(note = "use ponko::scope::ScopeGuard")]
pub use crate::scope::ScopeGuard;

#[doc(hidden)]
#[deprecated(note = "use ponko::scope::ScopeTiming")]
pub use crate::scope::ScopeTiming;

#[doc(hidden)]
#[deprecated(note = "use ponko::spatial::GridEntry")]
pub use crate::spatial::GridEntry;

#[doc(hidden)]
#[deprecated(note = "use ponko::spatial::SpatialGrid")]
pub use crate::spatial::SpatialGrid;

#[doc(hidden)]
#[deprecated(note = "use ponko::streaming::AssetState")]
pub use crate::streaming::AssetState;

#[doc(hidden)]
#[deprecated(note = "use ponko::streaming::StreamedAsset")]
pub use crate::streaming::StreamedAsset;

#[doc(hidden)]
#[deprecated(note = "use ponko::streaming::UploadStats")]
pub use crate::streaming::UploadStats;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::AmbientOcclusion")]
#[cfg(feature = "3d")]
pub use crate::render::AmbientOcclusion;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::AnimationCondition")]
#[cfg(feature = "3d")]
pub use crate::anim::AnimationCondition;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::AnimationEvent")]
#[cfg(feature = "3d")]
pub use crate::anim::AnimationEvent;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::AnimationGraph")]
#[cfg(feature = "3d")]
pub use crate::anim::AnimationGraph;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::AnimationLayer")]
#[cfg(feature = "3d")]
pub use crate::anim::AnimationLayer;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::AnimationParameter")]
#[cfg(feature = "3d")]
pub use crate::anim::AnimationParameter;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::AnimationState")]
#[cfg(feature = "3d")]
pub use crate::anim::AnimationState;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::AnimationTrack")]
#[cfg(feature = "3d")]
pub use crate::anim::AnimationTrack;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::AnimationTransition")]
#[cfg(feature = "3d")]
pub use crate::anim::AnimationTransition;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::Animator")]
#[cfg(feature = "3d")]
pub use crate::anim::Animator;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::AntiAliasing")]
#[cfg(feature = "3d")]
pub use crate::render::AntiAliasing;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::AttachToJoint")]
#[cfg(feature = "3d")]
pub use crate::anim::AttachToJoint;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::Blend")]
#[cfg(feature = "3d")]
pub use crate::anim::Blend;

#[doc(hidden)]
#[cfg(feature = "3d")]
pub use crate::d3::Camera as Camera3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::Compute")]
#[cfg(feature = "3d")]
pub use crate::render::Compute;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ComputePass")]
#[cfg(feature = "3d")]
pub use crate::render::ComputePass;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Decal")]
#[cfg(feature = "3d")]
pub use crate::d3::Decal as Decal3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::Effect")]
#[cfg(feature = "3d")]
pub use crate::render::Effect;

#[doc(hidden)]
#[deprecated(note = "use ponko::terrain::FarTerrain")]
#[cfg(feature = "3d")]
pub use crate::terrain::FarTerrain;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GlobalIllumination")]
#[cfg(feature = "3d")]
pub use crate::render::GlobalIllumination;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GpuBuffer")]
#[cfg(feature = "3d")]
pub use crate::render::GpuBuffer;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GraphicsQuality")]
#[cfg(feature = "3d")]
pub use crate::render::GraphicsQuality;

#[doc(hidden)]
#[deprecated(note = "use ponko::terrain::Heightfield")]
#[cfg(feature = "3d")]
pub use crate::terrain::Heightfield;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::InstanceBatch")]
#[cfg(feature = "3d")]
pub use crate::render::InstanceBatch;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::InstanceBatchOptions")]
#[cfg(feature = "3d")]
pub use crate::render::InstanceBatchOptions;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::InstanceLevel")]
#[cfg(feature = "3d")]
pub use crate::render::InstanceLevel;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::Interpolation")]
#[cfg(feature = "3d")]
pub use crate::anim::Interpolation;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::Joint")]
#[cfg(feature = "3d")]
pub use crate::anim::Joint;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::JointAnimation")]
#[cfg(feature = "3d")]
pub use crate::anim::JointAnimation;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::LayerBlend")]
#[cfg(feature = "3d")]
pub use crate::anim::LayerBlend;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::Lighting")]
#[cfg(feature = "3d")]
pub use crate::render::Lighting;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Line")]
#[cfg(feature = "3d")]
pub use crate::d3::Line as Line3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::LodGroup")]
#[cfg(feature = "3d")]
pub use crate::render::LodGroup;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::LodHysteresis")]
#[cfg(feature = "3d")]
pub use crate::render::LodHysteresis;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::LodLevel")]
#[cfg(feature = "3d")]
pub use crate::render::LodLevel;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::LodPick")]
#[cfg(feature = "3d")]
pub use crate::render::LodPick;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::LodRenderer")]
#[cfg(feature = "3d")]
pub use crate::render::LodRenderer;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::LodSettings")]
#[cfg(feature = "3d")]
pub use crate::render::LodSettings;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::LodShadow")]
#[cfg(feature = "3d")]
pub use crate::render::LodShadow;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::Material")]
#[cfg(feature = "3d")]
pub use crate::render::Material;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::MaterialDesc")]
#[cfg(feature = "3d")]
pub use crate::render::MaterialDesc;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::Mesh")]
#[cfg(feature = "3d")]
pub use crate::render::Mesh;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::MeshCost")]
#[cfg(feature = "3d")]
pub use crate::render::MeshCost;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::MeshData")]
#[cfg(feature = "3d")]
pub use crate::render::MeshData;

#[doc(hidden)]
#[cfg(feature = "3d")]
pub use crate::render::Model;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ModelMaterial")]
#[cfg(feature = "3d")]
pub use crate::render::ModelMaterial;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::MorphAnimation")]
#[cfg(feature = "3d")]
pub use crate::anim::MorphAnimation;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::MorphFrame")]
#[cfg(feature = "3d")]
pub use crate::anim::MorphFrame;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::MorphTarget")]
#[cfg(feature = "3d")]
pub use crate::anim::MorphTarget;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::Motion")]
#[cfg(feature = "3d")]
pub use crate::anim::Motion;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::MotionChild")]
#[cfg(feature = "3d")]
pub use crate::anim::MotionChild;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::ParticleBlend")]
#[cfg(feature = "3d")]
pub use crate::d3::ParticleBlend as ParticleBlend3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::ParticleEmitter")]
#[cfg(feature = "3d")]
pub use crate::d3::ParticleEmitter as ParticleEmitter3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::ParticleSettings")]
#[cfg(feature = "3d")]
pub use crate::d3::ParticleSettings as ParticleSettings3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::ParticleShape")]
#[cfg(feature = "3d")]
pub use crate::d3::ParticleShape as ParticleShape3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::ParticleSpace")]
#[cfg(feature = "3d")]
pub use crate::d3::ParticleSpace as ParticleSpace3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::PointLight")]
#[cfg(feature = "3d")]
pub use crate::d3::PointLight as PointLight3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::Post")]
#[cfg(feature = "3d")]
pub use crate::render::Post;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::QualityLevel")]
#[cfg(feature = "3d")]
pub use crate::render::QualityLevel;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ReceivesDecals")]
#[cfg(feature = "3d")]
pub use crate::render::ReceivesDecals;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::ReflectionProbe")]
#[cfg(feature = "3d")]
pub use crate::d3::ReflectionProbe as ReflectionProbe3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::RootMotion")]
#[cfg(feature = "3d")]
pub use crate::anim::RootMotion;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ScreenEffect")]
#[cfg(feature = "3d")]
pub use crate::render::ScreenEffect;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::Shader")]
#[cfg(feature = "3d")]
pub use crate::render::Shader;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ShaderDiagnostic")]
#[cfg(feature = "3d")]
pub use crate::render::ShaderDiagnostic;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ShaderKind")]
#[cfg(feature = "3d")]
pub use crate::render::ShaderKind;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ShaderOutcome")]
#[cfg(feature = "3d")]
pub use crate::render::ShaderOutcome;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ShaderStatus")]
#[cfg(feature = "3d")]
pub use crate::render::ShaderStatus;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::Skeleton")]
#[cfg(feature = "3d")]
pub use crate::anim::Skeleton;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::SkinOf")]
#[cfg(feature = "3d")]
pub use crate::anim::SkinOf;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::Sky")]
#[cfg(feature = "3d")]
pub use crate::render::Sky;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::SpotLight")]
#[cfg(feature = "3d")]
pub use crate::d3::SpotLight as SpotLight3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::TargetCamera")]
#[cfg(feature = "3d")]
pub use crate::render::TargetCamera;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::TargetEffect")]
#[cfg(feature = "3d")]
pub use crate::render::TargetEffect;

#[doc(hidden)]
#[deprecated(note = "use ponko::terrain::TerrainMaterialDesc")]
#[cfg(feature = "3d")]
pub use crate::terrain::TerrainMaterialDesc;

#[doc(hidden)]
#[deprecated(note = "use ponko::terrain::TerrainTile")]
#[cfg(feature = "3d")]
pub use crate::terrain::TerrainTile;

#[doc(hidden)]
#[deprecated(note = "use ponko::terrain::TerrainTileData")]
#[cfg(feature = "3d")]
pub use crate::terrain::TerrainTileData;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Text")]
#[cfg(feature = "3d")]
pub use crate::d3::Text as Text3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::TextAlign")]
#[cfg(feature = "3d")]
pub use crate::d3::TextAlign as TextAlign3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::terrain::TextureArray")]
#[cfg(feature = "3d")]
pub use crate::terrain::TextureArray;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::Tonemap")]
#[cfg(feature = "3d")]
pub use crate::render::Tonemap;

#[doc(hidden)]
#[deprecated(note = "use ponko::terrain::TextureKind")]
#[cfg(feature = "3d")]
pub use crate::terrain::TextureKind;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Trail")]
#[cfg(feature = "3d")]
pub use crate::d3::Trail as Trail3D;

#[doc(hidden)]
#[cfg(feature = "3d")]
pub use crate::d3::Transform as Transform3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ExtractionBench")]
#[cfg(feature = "3d")]
pub use crate::render::ExtractionBench;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ExtractionSample")]
#[cfg(feature = "3d")]
pub use crate::render::ExtractionSample;

#[doc(hidden)]
pub use crate::ui::Ui;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiAccessNode")]
pub use crate::ui::UiAccessNode;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiAccessTree")]
pub use crate::ui::UiAccessTree;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiAccessValue")]
pub use crate::ui::UiAccessValue;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiBuilder")]
pub use crate::ui::UiBuilder;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiEvent")]
pub use crate::ui::UiEvent;

#[doc(hidden)]
pub use crate::ui::UiId;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiImmediate")]
pub use crate::ui::UiImmediate;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiInput")]
pub use crate::ui::UiInput;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiLayout")]
pub use crate::ui::UiLayout;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiPaint")]
pub use crate::ui::UiPaint;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiRole")]
pub use crate::ui::UiRole;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiStyle")]
pub use crate::ui::UiStyle;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiTextAlign")]
pub use crate::ui::UiTextAlign;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiTheme")]
pub use crate::ui::UiTheme;

#[doc(hidden)]
#[deprecated(note = "use ponko::ui::UiWidget")]
pub use crate::ui::UiWidget;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Decal::MAX")]
#[cfg(feature = "3d")]
pub const MAX_DECALS: usize = crate::d3::Decal::MAX;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::Decal::MAX_PER_CLUSTER")]
#[cfg(feature = "3d")]
pub const MAX_DECALS_PER_CLUSTER: usize = crate::d3::Decal::MAX_PER_CLUSTER;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GlobalIllumination::MAX_PROBES")]
#[cfg(feature = "3d")]
pub const MAX_GI_PROBES: u32 = crate::render::GlobalIllumination::MAX_PROBES;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GlobalIllumination::MAX_PROBES_PER_FRAME")]
#[cfg(feature = "3d")]
pub const MAX_GI_PROBES_PER_FRAME: u32 = crate::render::GlobalIllumination::MAX_PROBES_PER_FRAME;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::PointLight::MAX_SHADOW_VIEWS")]
#[cfg(feature = "3d")]
pub const MAX_LIGHT_SHADOW_VIEWS: usize = crate::d3::PointLight::MAX_SHADOW_VIEWS;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::PointLight::SHADOW_SIZE")]
#[cfg(feature = "3d")]
pub const LIGHT_SHADOW_SIZE: u32 = crate::d3::PointLight::SHADOW_SIZE;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::PointLight::SHADOW_ATLAS_SIZE")]
#[cfg(feature = "3d")]
pub const LIGHT_SHADOW_ATLAS_SIZE: u32 = crate::d3::PointLight::SHADOW_ATLAS_SIZE;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GraphicsQuality::MIN_RENDER_SCALE")]
#[cfg(feature = "3d")]
pub const MIN_RENDER_SCALE: f32 = crate::render::GraphicsQuality::MIN_RENDER_SCALE;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::GraphicsQuality::MAX_RENDER_SCALE")]
#[cfg(feature = "3d")]
pub const MAX_RENDER_SCALE: f32 = crate::render::GraphicsQuality::MAX_RENDER_SCALE;

#[doc(hidden)]
#[deprecated(note = "use ponko::terrain::TerrainTile::LODS")]
#[cfg(feature = "3d")]
pub const TERRAIN_LODS: usize = crate::terrain::TerrainTile::LODS;

#[doc(hidden)]
#[deprecated(note = "use ponko::terrain::TerrainTile::LAYERS")]
#[cfg(feature = "3d")]
pub const TERRAIN_LAYERS: usize = crate::terrain::TerrainTile::LAYERS;

#[doc(hidden)]
#[deprecated(note = "use ponko::spatial::SpatialGrid::<()>::SORT_EVERY")]
pub const SPATIAL_SORT_EVERY: u64 = crate::spatial::SpatialGrid::<()>::SORT_EVERY;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::alloc_counts")]
pub use crate::perf::alloc_counts;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::note_allocation")]
pub use crate::perf::note_allocation;

#[doc(hidden)]
#[deprecated(note = "use ponko::camera::camera_controls")]
pub use crate::camera::camera_controls;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::seq_diff")]
pub use crate::net::seq_diff;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::seq_newer")]
pub use crate::net::seq_newer;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::scope_named")]
pub use crate::perf::scope_named;

#[doc(hidden)]
#[deprecated(note = "use ponko::spatial::spatial_key")]
pub use crate::spatial::spatial_key;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::two_bone_ik")]
#[cfg(feature = "3d")]
pub use crate::anim::two_bone_ik;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::look_at_joint")]
#[cfg(feature = "3d")]
pub use crate::anim::look_at_joint;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::steam_lobby_from_args")]
#[cfg(feature = "steam")]
pub use crate::net::steam_lobby_from_args;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::steam_wait_for_invite")]
#[cfg(feature = "steam")]
pub use crate::net::steam_wait_for_invite;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::AnimationClip")]
#[cfg(feature = "3d")]
pub use crate::anim::AnimationClip;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::JointMask")]
#[cfg(feature = "3d")]
pub use crate::anim::JointMask;

#[doc(hidden)]
#[deprecated(note = "use ponko::anim::MorphWeights")]
#[cfg(feature = "3d")]
pub use crate::anim::MorphWeights;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ModelMaterials")]
#[cfg(feature = "3d")]
pub use crate::render::ModelMaterials;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ParticleCurve")]
#[cfg(feature = "3d")]
pub use crate::render::ParticleCurve;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ParticleGradient")]
#[cfg(feature = "3d")]
pub use crate::render::ParticleGradient;

#[doc(hidden)]
#[deprecated(note = "use ponko::render::ShadowMesh")]
#[cfg(feature = "3d")]
pub use crate::render::ShadowMesh;

#[doc(hidden)]
#[deprecated(note = "use ponko::stream::CellStreamer")]
#[cfg(feature = "3d")]
pub use crate::stream::CellStreamer;

#[doc(hidden)]
#[deprecated(note = "use ponko::stream::Job")]
pub use crate::stream::Job;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::StreamedLod")]
#[cfg(feature = "3d")]
pub use crate::d3::StreamedLod;

#[doc(hidden)]
#[deprecated(note = "use ponko::d3::UiPanel")]
#[cfg(feature = "3d")]
pub use crate::d3::UiPanel as UiPanel3D;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::NetPose")]
#[cfg(feature = "3d")]
pub use crate::net::NetPose;

#[doc(hidden)]
#[deprecated(note = "use ponko::diagnostics::ECS_DIAGNOSTICS_VERSION")]
pub use crate::diagnostics::ECS_DIAGNOSTICS_VERSION;

#[doc(hidden)]
#[deprecated(note = "use ponko::input::MAX_GAMEPADS")]
pub use crate::input::MAX_GAMEPADS;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::MAX_INTEREST_TIERS")]
pub use crate::net::MAX_INTEREST_TIERS;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::MAX_INPUT_BYTES")]
pub use crate::net::MAX_INPUT_BYTES;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::MAX_MESSAGE_BYTES")]
pub use crate::net::MAX_MESSAGE_BYTES;

#[doc(hidden)]
#[deprecated(note = "use ponko::net::MAX_PLAYERS")]
pub use crate::net::MAX_PLAYERS;

#[doc(hidden)]
#[deprecated(note = "use ponko::physics::MAX_POLYGON_POINTS")]
#[cfg(feature = "physics")]
pub use crate::physics::MAX_POLYGON_POINTS;

#[doc(hidden)]
#[deprecated(note = "use ponko::physics3d::PHYSICS3D_HZ")]
#[cfg(feature = "physics3d")]
pub use crate::physics3d::PHYSICS3D_HZ;

#[doc(hidden)]
#[deprecated(note = "use ponko::random::DEFAULT_SEED")]
pub use crate::random::DEFAULT_SEED;

#[doc(hidden)]
#[deprecated(note = "use ponko::scenario::MAX_SCENARIO_STEPS")]
pub use crate::scenario::MAX_SCENARIO_STEPS;

#[doc(hidden)]
#[deprecated(note = "use ponko::scenario::MAX_SCENARIO_TICKS")]
pub use crate::scenario::MAX_SCENARIO_TICKS;

#[doc(hidden)]
#[deprecated(note = "use ponko::scenario::SCENARIO_VERSION")]
pub use crate::scenario::SCENARIO_VERSION;

#[doc(hidden)]
#[deprecated(note = "use ponko::streaming::DEFAULT_UPLOAD_BUDGET")]
pub use crate::streaming::DEFAULT_UPLOAD_BUDGET;

#[doc(hidden)]
#[deprecated(note = "use ponko::d2::Attached")]
pub use crate::attach::Attached;

#[doc(hidden)]
#[deprecated(note = "use ponko::gfx::Graphics")]
pub use crate::gfx::Graphics;

#[doc(hidden)]
#[deprecated(note = "use ponko::reflect::Field")]
pub use crate::reflect::Field;

#[doc(hidden)]
#[deprecated(note = "use ponko::d2::TileMap")]
pub use crate::tilemap::TileMap;

#[doc(hidden)]
#[deprecated(note = "use ponko::ecs_core::ComponentId")]
pub use ponko_ecs::ComponentId;

#[doc(hidden)]
#[deprecated(note = "use ponko::ecs_core::WorldView")]
pub use ponko_ecs::WorldView;

#[doc(hidden)]
#[deprecated(note = "use ponko::perf::Instant")]
pub use web_time::Instant;

#[doc(hidden)]
#[deprecated(note = "use ponko::d2::Animation")]
pub use crate::sheet::Animation;
