//! Three-dimensional game types. Pair with `ponko::prelude::*`.
#[doc(inline)]
pub use crate::audio::AudioListener3D as AudioListener;
#[doc(inline)]
pub use crate::audio::AudioSettings3D as AudioSettings;
#[doc(inline)]
pub use crate::audio::AudioSource3D as AudioSource;
#[cfg(feature = "navmesh")]
#[doc(inline)]
pub use crate::nav3d::NavAgent3D as NavAgent;
#[cfg(feature = "navmesh")]
#[doc(inline)]
pub use crate::nav3d::NavMesh3D as NavMesh;
#[cfg(feature = "navmesh")]
#[doc(inline)]
pub use crate::nav3d::NavObstacle3D as NavObstacle;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::Character3D as Character;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::Collider3D as Collider;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::ContactModifier3D as ContactModifier;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::ContactPair3D as ContactPair;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::Hit3D as Hit;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::Joint3D as Joint;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::JointAxis3D as JointAxis;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::JointDrive3D as JointDrive;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::JointMotion3D as JointMotion;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::Physics3D as Physics;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::QueryFilter3D as QueryFilter;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::Ragdoll3D as Ragdoll;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::RigidBody3D as RigidBody;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::Shape3D as Shape;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::Vehicle3D as Vehicle;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::Velocity3D as Velocity;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::Wheel3D as Wheel;
#[cfg(feature = "physics3d")]
#[doc(inline)]
pub use crate::physics3d::{
    ColliderFocus, CollisionGroups, CollisionMesh, CollisionMeshData, FrictionCurve, RagdollBone, TerrainCollider,
};
#[doc(inline)]
pub use crate::three::AmbientOcclusion;
#[doc(inline)]
pub use crate::three::AnimationClip;
#[doc(inline)]
pub use crate::three::AnimationCondition;
#[doc(inline)]
pub use crate::three::AnimationEvent;
#[doc(inline)]
pub use crate::three::AnimationGraph;
#[doc(inline)]
pub use crate::three::AnimationLayer;
#[doc(inline)]
pub use crate::three::AnimationParameter;
#[doc(inline)]
pub use crate::three::AnimationState;
#[doc(inline)]
pub use crate::three::AnimationTrack;
#[doc(inline)]
pub use crate::three::AnimationTransition;
#[doc(inline)]
pub use crate::three::Animator;
#[doc(inline)]
pub use crate::three::AntiAliasing;
#[doc(inline)]
pub use crate::three::AttachToJoint;
#[doc(inline)]
pub use crate::three::Blend;
#[doc(inline)]
pub use crate::three::Camera3D as Camera;
#[doc(inline)]
pub use crate::three::Compute;
#[doc(inline)]
pub use crate::three::ComputePass;
#[doc(inline)]
pub use crate::three::Decal3D as Decal;
#[doc(inline)]
pub use crate::three::Effect;
#[doc(inline)]
pub use crate::three::FarTerrain;
#[doc(inline)]
pub use crate::three::GlobalIllumination;
#[doc(inline)]
pub use crate::three::GpuBuffer;
#[doc(inline)]
pub use crate::three::GraphicsQuality;
#[doc(inline)]
pub use crate::three::Heightfield;
#[doc(inline)]
pub use crate::three::InstanceBatch;
#[doc(inline)]
pub use crate::three::InstanceBatchOptions;
#[doc(inline)]
pub use crate::three::InstanceLevel;
#[doc(inline)]
pub use crate::three::Interpolation;
#[doc(inline)]
pub use crate::three::JointAnimation;
#[doc(inline)]
pub use crate::three::JointMask;
#[doc(inline)]
pub use crate::three::LayerBlend;
#[doc(inline)]
pub use crate::three::Lighting;
#[doc(inline)]
pub use crate::three::Line3D as Line;
#[doc(inline)]
pub use crate::three::LodGroup;
#[doc(inline)]
pub use crate::three::LodHysteresis;
#[doc(inline)]
pub use crate::three::LodLevel;
#[doc(inline)]
pub use crate::three::LodPick;
#[doc(inline)]
pub use crate::three::LodRenderer;
#[doc(inline)]
pub use crate::three::LodSettings;
#[doc(inline)]
pub use crate::three::LodShadow;
#[doc(inline)]
pub use crate::three::Material;
#[doc(inline)]
pub use crate::three::MaterialDesc;
#[doc(inline)]
pub use crate::three::Mesh;
#[doc(inline)]
pub use crate::three::MeshCost;
#[doc(inline)]
pub use crate::three::MeshData;
#[doc(inline)]
pub use crate::three::Model;
#[doc(inline)]
pub use crate::three::ModelMaterial;
#[doc(inline)]
pub use crate::three::ModelMaterials;
#[doc(inline)]
pub use crate::three::MorphAnimation;
#[doc(inline)]
pub use crate::three::MorphFrame;
#[doc(inline)]
pub use crate::three::MorphTarget;
#[doc(inline)]
pub use crate::three::MorphWeights;
#[doc(inline)]
pub use crate::three::Motion;
#[doc(inline)]
pub use crate::three::MotionChild;
#[doc(inline)]
pub use crate::three::ParticleBlend3D as ParticleBlend;
#[doc(inline)]
pub use crate::three::ParticleCurve;
#[doc(inline)]
pub use crate::three::ParticleEmitter3D as ParticleEmitter;
#[doc(inline)]
pub use crate::three::ParticleGradient;
#[doc(inline)]
pub use crate::three::ParticleSettings3D as ParticleSettings;
#[doc(inline)]
pub use crate::three::ParticleShape3D as ParticleShape;
#[doc(inline)]
pub use crate::three::ParticleSpace3D as ParticleSpace;
#[doc(inline)]
pub use crate::three::PointLight3D as PointLight;
#[doc(inline)]
pub use crate::three::PointLight3D as Light;
#[doc(inline)]
pub use crate::three::Post;
#[doc(inline)]
pub use crate::three::QualityLevel;
#[doc(inline)]
pub use crate::three::ReceivesDecals;
#[doc(inline)]
pub use crate::three::ReflectionProbe3D as ReflectionProbe;
#[doc(inline)]
pub use crate::three::RootMotion;
#[doc(inline)]
pub use crate::three::ScreenEffect;
#[doc(inline)]
pub use crate::three::Shader;
#[doc(inline)]
pub use crate::three::ShaderDiagnostic;
#[doc(inline)]
pub use crate::three::ShaderKind;
#[doc(inline)]
pub use crate::three::ShaderOutcome;
#[doc(inline)]
pub use crate::three::ShaderStatus;
#[doc(inline)]
pub use crate::three::ShadowMesh;
#[doc(inline)]
pub use crate::three::Skeleton;
#[doc(inline)]
pub use crate::three::SkinOf;
#[doc(inline)]
pub use crate::three::Sky;
#[doc(inline)]
pub use crate::three::SpotLight3D as SpotLight;
#[doc(inline)]
pub use crate::three::TargetCamera;
#[doc(inline)]
pub use crate::three::TargetEffect;
#[doc(inline)]
pub use crate::three::TerrainMaterialDesc;
#[doc(inline)]
pub use crate::three::TerrainTile;
#[doc(inline)]
pub use crate::three::TerrainTileData;
#[doc(inline)]
pub use crate::three::Text3D as Text;
#[doc(inline)]
pub use crate::three::TextAlign3D as TextAlign;
#[doc(inline)]
pub use crate::three::TextureArray;
#[doc(inline)]
pub use crate::three::TextureKind;
#[doc(inline)]
pub use crate::three::Tonemap;
#[doc(inline)]
pub use crate::three::Trail3D as Trail;
#[doc(inline)]
pub use crate::three::Transform3D as Transform;
#[doc(inline)]
pub use crate::ui::UiPanel3D as UiPanel;

#[cfg(any(feature = "physics", feature = "physics3d"))]
#[doc(inline)]
pub use crate::collision::{BodyKind, CollisionStarted, CollisionStopped};
#[doc(inline)]
pub use crate::three::StreamedLod;
