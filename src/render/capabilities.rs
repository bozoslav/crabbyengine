//! What the GPU this game runs on can do, and what the renderer does where
//! it can't: one machine-readable report, [`GpuCapabilities`].
//!
//! The renderer's own feature checks live here too (`DeviceSupport`), so
//! the report and the passes the renderer actually builds can't disagree.
//! The report types are plain data (no GPU types inside), so they build
//! without the `window` feature and serialize to JSON for tools.

use serde::{Deserialize, Serialize};

/// What this GPU can do: the adapter, its main limits, and for every
/// renderer feature that depends on the device, whether it's available and
/// what the renderer does instead when it isn't.
///
/// From [`Ctx::gpu_capabilities`](crate::Ctx::gpu_capabilities). It is
/// decided once, when the GPU is opened, and doesn't change while the game
/// runs. Serializes to JSON (`serde_json::to_value`) for tools.
///
/// ```
/// # use ponko::*;
/// fn report(ctx: &mut Ctx) {
///     if let Some(caps) = ctx.gpu_capabilities() {
///         for f in caps.features.iter().filter(|f| !f.available) {
///             println!("{:?}: {} ({})", f.feature, f.fallback.as_deref().unwrap_or("-"), f.reason.as_deref().unwrap_or("-"));
///         }
///     }
/// }
/// # Game::new("t", 1, 1).system(report).run_headless(1, |_| {});
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuCapabilities {
    /// The GPU the renderer opened.
    pub adapter: GpuAdapter,
    /// The device's limits that the renderer's choices depend on.
    pub limits: GpuLimits,
    /// One entry per [`GpuFeature`], in declaration order.
    pub features: Vec<GpuFeatureStatus>,
}

impl GpuCapabilities {
    /// The entry for `feature`.
    #[must_use]
    pub fn feature(&self, feature: GpuFeature) -> Option<&GpuFeatureStatus> {
        self.features.iter().find(|f| f.feature == feature)
    }

    /// Whether `feature` is available on this GPU.
    #[must_use]
    pub fn available(&self, feature: GpuFeature) -> bool {
        self.feature(feature).is_some_and(|f| f.available)
    }
}

/// The GPU the renderer opened, as the graphics driver names it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuAdapter {
    /// The adapter's name, such as the GPU model.
    pub name: String,
    /// The graphics API: `vulkan`, `metal`, `dx12`, `gl` (WebGL2 in a
    /// browser), `browser_webgpu` or `noop`.
    pub backend: String,
    /// `discrete_gpu`, `integrated_gpu`, `virtual_gpu`, `cpu` or `other`.
    pub device_type: String,
    /// The driver's name, where the backend reports it (may be empty).
    pub driver: String,
    /// The driver's version details, where the backend reports them (may be empty).
    pub driver_info: String,
}

/// The device limits the renderer's feature choices depend on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuLimits {
    /// Largest texture width or height.
    pub max_texture_dimension_2d: u32,
    /// Most layers in a texture array.
    pub max_texture_array_layers: u32,
    /// Sampled textures one shader stage may read (decals need 17, global
    /// illumination 18).
    pub max_sampled_textures_per_shader_stage: u32,
    /// Storage buffers one shader stage may use (0 on WebGL2).
    pub max_storage_buffers_per_shader_stage: u32,
    /// Storage textures one shader stage may use (0 on WebGL2).
    pub max_storage_textures_per_shader_stage: u32,
    /// Largest storage buffer binding, in bytes.
    pub max_storage_buffer_binding_size: u64,
    /// Most compute workgroups per dispatch dimension (0: no compute shaders).
    pub max_compute_workgroups_per_dimension: u32,
}

/// A renderer feature that depends on what the GPU can do.
///
/// New features may be added; match with a wildcard arm.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuFeature {
    /// GPU timestamp queries: per-pass GPU times and `RenderStats::gpu_ms`.
    Timestamps,
    /// Compute shaders and storage textures, which the effects below build on.
    Compute,
    /// Storage buffers in shaders: `GpuBuffer`s and material shaders that
    /// read `items`.
    StorageBuffers,
    /// Static instance batches culled per instance by a compute pass.
    ComputeCulling,
    /// Skinned and morphed meshes posed by a compute pass.
    GpuSkinning,
    /// Hiding models behind others with the previous frame's depth.
    OcclusionCulling,
    /// Temporal antialiasing (`AntiAliasing::Taa`).
    TemporalAntialiasing,
    /// Ground-truth ambient occlusion.
    AmbientOcclusion,
    /// Screen-space contact shadows.
    ContactShadows,
    /// Screen-space reflections.
    ScreenSpaceReflections,
    /// Probe-based global illumination.
    GlobalIllumination,
    /// Projected decals (`Decal3D`).
    Decals,
    /// The game's multisampling count for the 3D frame.
    Multisampling,
}

impl GpuFeature {
    /// Every feature, in declaration order (the report's order).
    pub const ALL: [GpuFeature; 13] = [
        GpuFeature::Timestamps,
        GpuFeature::Compute,
        GpuFeature::StorageBuffers,
        GpuFeature::ComputeCulling,
        GpuFeature::GpuSkinning,
        GpuFeature::OcclusionCulling,
        GpuFeature::TemporalAntialiasing,
        GpuFeature::AmbientOcclusion,
        GpuFeature::ContactShadows,
        GpuFeature::ScreenSpaceReflections,
        GpuFeature::GlobalIllumination,
        GpuFeature::Decals,
        GpuFeature::Multisampling,
    ];

    /// Whether the feature belongs to the 3D renderer (the `3d` Cargo feature).
    #[must_use]
    pub fn is_3d(self) -> bool {
        !matches!(self, GpuFeature::Timestamps | GpuFeature::Compute)
    }
}

/// Whether one [`GpuFeature`] is available, and if not, what the renderer
/// does instead and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuFeatureStatus {
    /// The feature.
    pub feature: GpuFeature,
    /// Whether the renderer uses it where the game asks for it.
    pub available: bool,
    /// What the renderer does instead (`None` when available).
    pub fallback: Option<String>,
    /// Why it isn't available: the missing GPU feature or limit (`None`
    /// when available).
    pub reason: Option<String>,
}

/// Sampled textures a fragment shader needs for the mesh pipelines with
/// decals: the 16 WebGL2 and WebGPU's defaults allow, and the decal images.
pub(crate) const DECAL_SAMPLED_TEXTURES: u32 = 17;
/// Storage buffers the per-instance culling pass binds at once.
const CULLING_STORAGE_BUFFERS: u32 = 4;
/// Storage buffers the skinning pass binds at once (see `skinning.rs`).
const SKINNING_STORAGE_BUFFERS: u32 = 6;

impl GpuLimits {
    /// Whether the device runs compute passes: WebGL2 has no compute
    /// shaders or storage textures.
    pub(crate) fn compute(&self) -> bool {
        self.max_compute_workgroups_per_dimension > 0 && self.max_storage_textures_per_shader_stage > 0
    }

    /// Whether shaders can read and write storage buffers.
    pub(crate) fn storage_buffers(&self) -> bool {
        self.compute() && self.max_storage_buffers_per_shader_stage >= 2
    }

    /// Whether the per-instance culling pass can bind its buffers.
    pub(crate) fn compute_culling(&self) -> bool {
        self.compute() && self.max_storage_buffers_per_shader_stage >= CULLING_STORAGE_BUFFERS
    }

    /// Whether the skinning pass can bind its buffers.
    pub(crate) fn gpu_skinning(&self) -> bool {
        self.compute() && self.max_storage_buffers_per_shader_stage >= SKINNING_STORAGE_BUFFERS
    }

    /// Whether the mesh shaders have a texture slot to spare for decals.
    pub(crate) fn decals(&self) -> bool {
        self.max_sampled_textures_per_shader_stage >= DECAL_SAMPLED_TEXTURES
    }

    /// Whether global illumination fits: compute shaders, and a texture
    /// slot past the decals' for the probes (the 18th).
    pub(crate) fn gi(&self) -> bool {
        self.compute() && self.max_sampled_textures_per_shader_stage > DECAL_SAMPLED_TEXTURES
    }
}

/// Builds the report from what the device has: the inputs a GPU gives
/// (`timestamps`: whether the device has timestamp queries; `msaa`: the
/// samples the game asked for and the ones used), without the GPU itself.
pub(crate) fn report(
    adapter: GpuAdapter,
    limits: GpuLimits,
    timestamps: bool,
    msaa: (u32, u32),
    three: bool,
) -> GpuCapabilities {
    let no_compute = "the device has no compute shaders or storage textures (max_compute_workgroups_per_dimension or max_storage_textures_per_shader_stage is 0, as on WebGL2)";
    let features = GpuFeature::ALL
        .iter()
        .map(|&feature| {
            let (available, fallback, reason): (bool, String, String) = if feature.is_3d() && !three {
                (false, "not drawn".into(), "built without the `3d` Cargo feature".into())
            } else {
                match feature {
                    GpuFeature::Timestamps => (
                        timestamps,
                        "no GPU pass times; RenderStats::gpu_ms is None".into(),
                        "the device lacks the TIMESTAMP_QUERY feature".into(),
                    ),
                    GpuFeature::Compute => (
                        limits.compute(),
                        "compute passes and the effects that need them are skipped (see the other entries)".into(),
                        no_compute.into(),
                    ),
                    GpuFeature::StorageBuffers => (
                        limits.storage_buffers(),
                        "GPU buffers are not created and compute passes using them do nothing; material shaders that read `items` use the built-in shader".into(),
                        if limits.compute() {
                            storage_reason(&limits, 2, no_compute)
                        } else {
                            no_compute.into()
                        },
                    ),
                    GpuFeature::ComputeCulling => (
                        limits.compute_culling(),
                        "static instance batches use CPU cell culling and choose one distance level per cell".into(),
                        storage_reason(&limits, CULLING_STORAGE_BUFFERS, no_compute),
                    ),
                    GpuFeature::GpuSkinning => (
                        limits.gpu_skinning(),
                        "skinned and morphed meshes are posed on the CPU".into(),
                        storage_reason(&limits, SKINNING_STORAGE_BUFFERS, no_compute),
                    ),
                    GpuFeature::OcclusionCulling => (
                        limits.compute(),
                        "off: hidden models are drawn".into(),
                        no_compute.into(),
                    ),
                    GpuFeature::TemporalAntialiasing => {
                        (limits.compute(), "FXAA instead of TAA".into(), no_compute.into())
                    }
                    GpuFeature::AmbientOcclusion => (limits.compute(), "off".into(), no_compute.into()),
                    GpuFeature::ContactShadows => (limits.compute(), "off".into(), no_compute.into()),
                    GpuFeature::ScreenSpaceReflections => (limits.compute(), "off".into(), no_compute.into()),
                    GpuFeature::GlobalIllumination => (
                        limits.gi(),
                        "off: sky and ambient light only".into(),
                        if limits.compute() {
                            format!(
                                "max_sampled_textures_per_shader_stage is {}; global illumination needs {}",
                                limits.max_sampled_textures_per_shader_stage,
                                DECAL_SAMPLED_TEXTURES + 1
                            )
                        } else {
                            no_compute.into()
                        },
                    ),
                    GpuFeature::Decals => (
                        limits.decals(),
                        "decals are not drawn".into(),
                        format!(
                            "max_sampled_textures_per_shader_stage is {}; decals need {DECAL_SAMPLED_TEXTURES}",
                            limits.max_sampled_textures_per_shader_stage
                        ),
                    ),
                    GpuFeature::Multisampling => (
                        msaa.0 == msaa.1,
                        format!("{}x multisampling", msaa.1),
                        format!("the GPU can't multisample the 3D frame's color and depth {}x", msaa.0),
                    ),
                }
            };
            GpuFeatureStatus {
                feature,
                available,
                fallback: (!available).then_some(fallback),
                reason: (!available).then_some(reason),
            }
        })
        .collect();
    GpuCapabilities {
        adapter,
        limits,
        features,
    }
}

fn storage_reason(limits: &GpuLimits, needed: u32, no_compute: &str) -> String {
    if limits.compute() {
        format!(
            "max_storage_buffers_per_shader_stage is {}; {needed} are needed",
            limits.max_storage_buffers_per_shader_stage
        )
    } else {
        no_compute.into()
    }
}

#[cfg(feature = "window")]
impl GpuLimits {
    /// The limits `device` was opened with.
    pub(crate) fn of(device: &wgpu::Device) -> Self {
        let l = device.limits();
        Self {
            max_texture_dimension_2d: l.max_texture_dimension_2d,
            max_texture_array_layers: l.max_texture_array_layers,
            max_sampled_textures_per_shader_stage: l.max_sampled_textures_per_shader_stage,
            max_storage_buffers_per_shader_stage: l.max_storage_buffers_per_shader_stage,
            max_storage_textures_per_shader_stage: l.max_storage_textures_per_shader_stage,
            max_storage_buffer_binding_size: l.max_storage_buffer_binding_size,
            max_compute_workgroups_per_dimension: l.max_compute_workgroups_per_dimension,
        }
    }
}

#[cfg(feature = "window")]
impl GpuAdapter {
    pub(crate) fn of(info: &wgpu::AdapterInfo) -> Self {
        let device_type = match info.device_type {
            wgpu::DeviceType::DiscreteGpu => "discrete_gpu",
            wgpu::DeviceType::IntegratedGpu => "integrated_gpu",
            wgpu::DeviceType::VirtualGpu => "virtual_gpu",
            wgpu::DeviceType::Cpu => "cpu",
            wgpu::DeviceType::Other => "other",
        };
        Self {
            name: info.name.clone(),
            backend: info.backend.to_str().to_owned(),
            device_type: device_type.to_owned(),
            driver: info.driver.clone(),
            driver_info: info.driver_info.clone(),
        }
    }
}

/// The report for `device`, opened on the adapter described by `info`;
/// `msaa`: the 3D frame's samples the game asked for, and the ones used.
#[cfg(feature = "window")]
pub(crate) fn of_device(info: &wgpu::AdapterInfo, device: &wgpu::Device, msaa: (u32, u32)) -> GpuCapabilities {
    report(
        GpuAdapter::of(info),
        GpuLimits::of(device),
        device.features().contains(wgpu::Features::TIMESTAMP_QUERY),
        msaa,
        cfg!(feature = "3d"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WebGL2's limits, as wgpu's `Limits::downlevel_webgl2_defaults` has them.
    fn webgl2() -> GpuLimits {
        GpuLimits {
            max_texture_dimension_2d: 2048,
            max_texture_array_layers: 256,
            max_sampled_textures_per_shader_stage: 16,
            max_storage_buffers_per_shader_stage: 0,
            max_storage_textures_per_shader_stage: 0,
            max_storage_buffer_binding_size: 0,
            max_compute_workgroups_per_dimension: 0,
        }
    }

    /// WebGPU's default limits, with `slots` sampled textures.
    fn webgpu(slots: u32) -> GpuLimits {
        GpuLimits {
            max_texture_dimension_2d: 8192,
            max_texture_array_layers: 256,
            max_sampled_textures_per_shader_stage: slots,
            max_storage_buffers_per_shader_stage: 8,
            max_storage_textures_per_shader_stage: 4,
            max_storage_buffer_binding_size: 128 << 20,
            max_compute_workgroups_per_dimension: 65535,
        }
    }

    fn caps(limits: GpuLimits, timestamps: bool) -> GpuCapabilities {
        report(GpuAdapter::default(), limits, timestamps, (4, 4), true)
    }

    #[test]
    fn every_feature_is_reported_once_in_order() {
        let r = caps(webgpu(16), false);
        let order: Vec<_> = r.features.iter().map(|f| f.feature).collect();
        assert_eq!(order, GpuFeature::ALL);
    }

    #[test]
    fn unavailable_features_name_a_fallback_and_a_reason() {
        for r in [caps(webgl2(), false), caps(webgpu(16), false), caps(webgpu(24), true)] {
            for f in &r.features {
                assert_eq!(f.fallback.is_none(), f.available, "{f:?}");
                assert_eq!(f.reason.is_none(), f.available, "{f:?}");
            }
        }
    }

    #[test]
    fn webgl2_limits_fall_back_from_every_compute_feature() {
        let r = caps(webgl2(), false);
        assert!(!r.available(GpuFeature::Compute));
        let taa = r.feature(GpuFeature::TemporalAntialiasing).unwrap();
        assert!(!taa.available);
        assert_eq!(taa.fallback.as_deref(), Some("FXAA instead of TAA"));
        assert!(taa.reason.as_deref().unwrap().contains("compute"), "{taa:?}");
        for f in [
            GpuFeature::StorageBuffers,
            GpuFeature::ComputeCulling,
            GpuFeature::GpuSkinning,
            GpuFeature::OcclusionCulling,
            GpuFeature::AmbientOcclusion,
            GpuFeature::ContactShadows,
            GpuFeature::ScreenSpaceReflections,
            GpuFeature::GlobalIllumination,
            GpuFeature::Decals,
            GpuFeature::Timestamps,
        ] {
            assert!(!r.available(f), "{f:?}");
        }
    }

    #[test]
    fn default_texture_slots_leave_out_decals_and_gi_only() {
        let r = caps(webgpu(16), true);
        for f in GpuFeature::ALL {
            let expected = !matches!(f, GpuFeature::Decals | GpuFeature::GlobalIllumination);
            assert_eq!(r.available(f), expected, "{f:?}");
        }
        assert!(
            r.feature(GpuFeature::Decals)
                .unwrap()
                .reason
                .as_deref()
                .unwrap()
                .contains("16")
        );
        // One more slot: decals, but no room for GI's probes yet.
        let r = caps(webgpu(17), true);
        assert!(r.available(GpuFeature::Decals));
        assert!(!r.available(GpuFeature::GlobalIllumination));
        assert!(caps(webgpu(18), true).available(GpuFeature::GlobalIllumination));
    }

    #[test]
    fn few_storage_buffers_turn_off_skinning_before_culling() {
        let limits = GpuLimits {
            max_storage_buffers_per_shader_stage: 4,
            ..webgpu(16)
        };
        let r = caps(limits, false);
        assert!(r.available(GpuFeature::ComputeCulling));
        let skinning = r.feature(GpuFeature::GpuSkinning).unwrap();
        assert!(!skinning.available);
        assert_eq!(
            skinning.reason.as_deref(),
            Some("max_storage_buffers_per_shader_stage is 4; 6 are needed")
        );
        assert!(!r.available(GpuFeature::Timestamps));
    }

    #[test]
    fn a_lowered_multisample_count_is_a_fallback() {
        let r = report(GpuAdapter::default(), webgpu(24), true, (8, 4), true);
        let msaa = r.feature(GpuFeature::Multisampling).unwrap();
        assert!(!msaa.available);
        assert_eq!(msaa.fallback.as_deref(), Some("4x multisampling"));
        assert!(msaa.reason.as_deref().unwrap().contains("8x"));
    }

    #[test]
    fn without_the_3d_renderer_its_features_are_off() {
        let r = report(GpuAdapter::default(), webgpu(24), true, (4, 4), false);
        for f in GpuFeature::ALL {
            assert_eq!(r.available(f), !f.is_3d(), "{f:?}");
        }
    }

    #[test]
    fn the_report_is_snake_case_json_and_round_trips() {
        let r = caps(webgl2(), false);
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["features"][6]["feature"], "temporal_antialiasing");
        assert_eq!(json["features"][6]["available"], false);
        assert_eq!(json["features"][6]["fallback"], "FXAA instead of TAA");
        assert_eq!(json["limits"]["max_compute_workgroups_per_dimension"], 0);
        let back: GpuCapabilities = serde_json::from_value(json).unwrap();
        assert_eq!(back, r);
    }
}
