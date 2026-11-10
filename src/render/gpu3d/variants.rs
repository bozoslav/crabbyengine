//! The pipeline variants every material shader is built in, as data.
//!
//! A material shader (the engine's prelude, the game's `vertex`/`surface`
//! hooks and the engine's entry points, see `mesh.wgsl`) is compiled once
//! into one module. From that module the renderer builds a fixed set of
//! render pipelines: one [`PipelineSet`] per [`ShaderVariant`], and in each
//! set one pipeline per [`PipelineRole`]. These enums are the whole list: the
//! builder, the draw loop's lookup and the offscreen tests all go through them,
//! so a new variant or role is added here first.
//!
//! What picks the variant of a draw:
//!
//! | Key | From | Effect on the pipelines |
//! |---|---|---|
//! | [`Sides`] | `MaterialDesc::double_sided` | back-face culling, or none |
//! | [`Instancing::Plain`] | ordinary models and CPU-culled batches | per-instance records |
//! | [`Instancing::Crossfade`] | a model crossfading between LOD levels | every fragment entry point dithers (`fade_keeps`) |
//! | [`Instancing::Shared`] | a mesh with a GPU buffer (`set_mesh_gpu_buffer`) or GPU-culled batches | a zero-stride instance record; no crossfade |
//!
//! What picks the role inside a set is the material's blend
//! ([`crate::three::Blend`]), the pass being drawn and multisampling (see
//! [`PipelineRole::for_pass`]). Skinned meshes are not a variant: they are
//! posed by a compute pass into a vertex buffer of the same layout, so the
//! same pipelines draw them. There is no unlit variant either: an unlit look
//! is a `surface` hook that writes `emissive` and a black `albedo`.
//!
//! The plain variants are built with the shader, the others the first time a
//! frame draws them (or all up front by `Renderer3D::warm_pipelines`). The
//! sets are cached by shader code, composed features, target formats and
//! samples (`pipeline_cache.rs`), which also lists the renderer's fixed
//! pipelines.

/// Which faces of a material's triangles are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Sides {
    /// Back faces culled: the default.
    One,
    /// Both faces drawn (`MaterialDesc::double_sided`); `surface` sees the
    /// normal flipped on the back.
    Both,
}

impl Sides {
    pub(crate) const ALL: [Sides; 2] = [Sides::One, Sides::Both];

    pub(crate) fn of(double_sided: bool) -> Self {
        if double_sided { Sides::Both } else { Sides::One }
    }

    /// The face the rasterizer culls.
    pub(crate) fn cull(self) -> Option<wgpu::Face> {
        match self {
            Sides::One => Some(wgpu::Face::Back),
            Sides::Both => None,
        }
    }

    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

/// How a draw's instances reach the vertex shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Instancing {
    /// One instance record per model (transform, tint, decal layers).
    Plain,
    /// Like `Plain`, for models crossfading between two levels of detail:
    /// every fragment entry point discards the pixels the dither drops.
    Crossfade,
    /// Every instance of the draw reads the same record (zero stride): a
    /// mesh drawn `count` times from a GPU buffer, read as `items` in
    /// `vertex`. Buffer-driven draws never crossfade.
    Shared,
}

impl Instancing {
    #[cfg(test)]
    pub(crate) const ALL: [Instancing; 3] = [Instancing::Plain, Instancing::Crossfade, Instancing::Shared];

    /// The instancing of a draw: buffer-driven wins over crossfading.
    pub(crate) fn of(fading: bool, shared: bool) -> Self {
        if shared {
            Instancing::Shared
        } else if fading {
            Instancing::Crossfade
        } else {
            Instancing::Plain
        }
    }

    pub(crate) fn fades(self) -> bool {
        self == Instancing::Crossfade
    }

    pub(crate) fn shared(self) -> bool {
        self == Instancing::Shared
    }
}

/// One pipeline set of a material shader: the key the renderer builds and
/// looks pipelines up by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ShaderVariant {
    pub sides: Sides,
    pub instancing: Instancing,
}

impl ShaderVariant {
    /// Every variant, plain ones first (those are built with the shader).
    pub(crate) const ALL: [ShaderVariant; 6] = [
        ShaderVariant::new(Sides::One, Instancing::Plain),
        ShaderVariant::new(Sides::Both, Instancing::Plain),
        ShaderVariant::new(Sides::One, Instancing::Crossfade),
        ShaderVariant::new(Sides::Both, Instancing::Crossfade),
        ShaderVariant::new(Sides::One, Instancing::Shared),
        ShaderVariant::new(Sides::Both, Instancing::Shared),
    ];

    pub(crate) const fn new(sides: Sides, instancing: Instancing) -> Self {
        Self { sides, instancing }
    }
}

/// A pass of the frame, as the draw loop sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pass {
    Shadow,
    Prepass,
    /// Cut-outs' own prepass with alpha to coverage (multisampling).
    PrepassCoverage,
    Opaque,
    Transparent,
    Velocity,
    /// Global illumination's probe views.
    Capture,
}

impl Pass {
    pub(crate) const ALL: [Pass; 7] = [
        Pass::Shadow,
        Pass::Prepass,
        Pass::PrepassCoverage,
        Pass::Opaque,
        Pass::Transparent,
        Pass::Velocity,
        Pass::Capture,
    ];
}

/// Every (cutout, transparent, full_prepass) combination.
fn blend_and_prepass() -> impl Iterator<Item = (bool, bool, bool)> {
    (0..8u8).map(|i| (i & 1 != 0, i & 2 != 0, i & 4 != 0))
}

/// One pipeline of a [`ShaderVariant`]'s set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PipelineRole {
    /// Depth only, no fragment shader (dither only when crossfading).
    Prepass,
    /// Depth minus the pixels `surface` gives alpha < 0.5.
    PrepassCutout,
    /// Cut-outs' depth with alpha to coverage. Multisampling only.
    PrepassCoverage,
    /// Lit, over the prepass's depth (`Equal`).
    Opaque,
    /// Lit cut-outs over their coverage prepass. Multisampling only.
    OpaqueCoverage,
    /// Lit, writing its own depth (no prepass for solid surfaces).
    OpaqueSolid,
    /// Lit and blended (premultiplied alpha), no depth write.
    Transparent,
    /// Sun and light shadow maps; both faces, depth bias.
    Shadow,
    /// Shadow maps minus cut-out pixels.
    ShadowCutout,
    /// Motion vectors of moved models (temporal antialiasing).
    Velocity,
}

impl PipelineRole {
    pub(crate) const ALL: [PipelineRole; 10] = [
        PipelineRole::Prepass,
        PipelineRole::PrepassCutout,
        PipelineRole::PrepassCoverage,
        PipelineRole::Opaque,
        PipelineRole::OpaqueCoverage,
        PipelineRole::OpaqueSolid,
        PipelineRole::Transparent,
        PipelineRole::Shadow,
        PipelineRole::ShadowCutout,
        PipelineRole::Velocity,
    ];

    /// Which roles [`PipelineRole::for_pass`] picks for some pass, blend
    /// and prepass setting, with or without multisampling (`coverage`).
    fn picked(coverage: bool) -> [bool; Self::ALL.len()] {
        let mut used = [false; Self::ALL.len()];
        for pass in Pass::ALL {
            for (cutout, transparent, full_prepass) in blend_and_prepass() {
                if let Some(role) = Self::for_pass(pass, cutout, transparent, coverage, full_prepass) {
                    used[Self::ALL.iter().position(|&r| r == role).expect("listed in ALL")] = true;
                }
            }
        }
        used
    }

    /// How many pipelines a set has: every role some pass draws with
    /// without multisampling, and with it (`coverage`) the alpha-to-coverage
    /// ones too. A multisampled set keeps the plain cut-out prepass, which
    /// the passes then don't pick.
    pub(crate) fn per_set(coverage: bool) -> usize {
        let (plain, multisampled) = (Self::picked(false), Self::picked(coverage));
        plain.iter().zip(multisampled).filter(|&(&p, m)| p || m).count()
    }

    /// The vertex entry point in the composed shader.
    pub(crate) fn vertex_entry(self) -> &'static str {
        match self {
            PipelineRole::Shadow | PipelineRole::ShadowCutout => "vs_shadow",
            PipelineRole::Velocity => "vs_velocity",
            _ => "vs_main",
        }
    }

    /// The fragment entry point in the composed shader, `None` for a
    /// depth-only pipeline without one.
    pub(crate) fn fragment_entry(self, instancing: Instancing) -> Option<&'static str> {
        let fade = instancing.fades();
        let pick = |plain, faded| Some(if fade { faded } else { plain });
        match self {
            PipelineRole::Prepass | PipelineRole::Shadow => fade.then_some("fs_fade"),
            PipelineRole::PrepassCutout | PipelineRole::ShadowCutout => pick("fs_depth", "fs_depth_fade"),
            PipelineRole::PrepassCoverage => pick("fs_depth_coverage", "fs_depth_coverage_fade"),
            PipelineRole::Opaque | PipelineRole::OpaqueSolid => pick("fs_main", "fs_main_fade"),
            PipelineRole::OpaqueCoverage => pick("fs_main_coverage", "fs_main_coverage_fade"),
            PipelineRole::Transparent => pick("fs_transparent", "fs_transparent_fade"),
            // Motion is per model; a crossfade's dither is not part of it.
            PipelineRole::Velocity => Some("fs_velocity"),
        }
    }

    /// The role a material draws with in `pass`, or `None` when it is not
    /// drawn there.
    ///
    /// `cutout` and `transparent` come from its blend; `coverage` is
    /// whether the renderer multisamples (cut-outs then use alpha to
    /// coverage); `full_prepass` whether solid surfaces get a depth prepass
    /// too (otherwise they write their own depth in the opaque pass).
    pub(crate) fn for_pass(
        pass: Pass,
        cutout: bool,
        transparent: bool,
        coverage: bool,
        full_prepass: bool,
    ) -> Option<Self> {
        // Transparent surfaces are drawn last, cast no shadows and write no
        // depth or motion.
        if transparent != (pass == Pass::Transparent) {
            return None;
        }
        let coverage = coverage && cutout;
        Some(match pass {
            Pass::Shadow if cutout => PipelineRole::ShadowCutout,
            Pass::Shadow => PipelineRole::Shadow,
            Pass::PrepassCoverage if coverage => PipelineRole::PrepassCoverage,
            Pass::PrepassCoverage => return None,
            // With coverage, cut-outs have a prepass of their own.
            Pass::Prepass if coverage => return None,
            Pass::Prepass if cutout => PipelineRole::PrepassCutout,
            Pass::Prepass if !full_prepass => return None,
            Pass::Prepass => PipelineRole::Prepass,
            Pass::Opaque if !cutout && !full_prepass => PipelineRole::OpaqueSolid,
            Pass::Opaque | Pass::Capture if coverage => PipelineRole::OpaqueCoverage,
            Pass::Opaque => PipelineRole::Opaque,
            Pass::Capture if !cutout => PipelineRole::OpaqueSolid,
            Pass::Capture => PipelineRole::Opaque,
            Pass::Transparent => PipelineRole::Transparent,
            Pass::Velocity => PipelineRole::Velocity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_is_listed_once() {
        for (i, a) in ShaderVariant::ALL.iter().enumerate() {
            for b in &ShaderVariant::ALL[i + 1..] {
                assert_ne!(a, b);
            }
        }
        for sides in Sides::ALL {
            for instancing in Instancing::ALL {
                assert!(ShaderVariant::ALL.contains(&ShaderVariant::new(sides, instancing)));
            }
        }
    }

    #[test]
    fn buffer_driven_draws_never_crossfade() {
        assert_eq!(Instancing::of(true, true), Instancing::Shared);
        assert_eq!(Instancing::of(true, false), Instancing::Crossfade);
        assert_eq!(Instancing::of(false, false), Instancing::Plain);
        assert!(!Instancing::Shared.fades());
    }

    #[test]
    fn every_fragment_entry_point_exists_in_the_composed_shader() {
        let module = wgpu::naga::front::wgsl::parse_str(&super::super::compose_shader("")).unwrap();
        let entries: Vec<&str> = module.entry_points.iter().map(|e| e.name.as_str()).collect();
        for role in PipelineRole::ALL {
            assert!(entries.contains(&role.vertex_entry()), "{role:?}");
            for instancing in Instancing::ALL {
                if let Some(fragment) = role.fragment_entry(instancing) {
                    assert!(entries.contains(&fragment), "{role:?} {instancing:?}: {fragment}");
                }
            }
        }
    }

    #[test]
    fn crossfading_dithers_every_fragment_but_motion() {
        for role in PipelineRole::ALL {
            let faded = role.fragment_entry(Instancing::Crossfade);
            if role == PipelineRole::Velocity {
                assert_eq!(faded, role.fragment_entry(Instancing::Plain));
            } else {
                assert!(faded.is_some_and(|f| f.ends_with("fade")), "{role:?}");
            }
            assert_eq!(
                role.fragment_entry(Instancing::Shared),
                role.fragment_entry(Instancing::Plain)
            );
        }
    }

    #[test]
    fn a_set_has_every_role_with_multisampling_and_no_coverage_ones_without() {
        assert_eq!(PipelineRole::per_set(true), PipelineRole::ALL.len());
        assert_eq!(PipelineRole::per_set(false), PipelineRole::ALL.len() - 2);
    }

    #[test]
    fn blends_pick_their_passes() {
        use PipelineRole as R;
        let roles = |cutout, transparent, coverage, full_prepass| {
            [
                Pass::Shadow,
                Pass::Prepass,
                Pass::PrepassCoverage,
                Pass::Opaque,
                Pass::Transparent,
                Pass::Velocity,
                Pass::Capture,
            ]
            .map(|p| PipelineRole::for_pass(p, cutout, transparent, coverage, full_prepass))
        };
        // Opaque without a full prepass: writes its own depth.
        assert_eq!(
            roles(false, false, false, false),
            [
                Some(R::Shadow),
                None,
                None,
                Some(R::OpaqueSolid),
                None,
                Some(R::Velocity),
                Some(R::OpaqueSolid)
            ]
        );
        assert_eq!(
            roles(false, false, true, true),
            [
                Some(R::Shadow),
                Some(R::Prepass),
                None,
                Some(R::Opaque),
                None,
                Some(R::Velocity),
                Some(R::OpaqueSolid)
            ]
        );
        // Cut-outs: their shader runs in every depth pass.
        assert_eq!(
            roles(true, false, false, false),
            [
                Some(R::ShadowCutout),
                Some(R::PrepassCutout),
                None,
                Some(R::Opaque),
                None,
                Some(R::Velocity),
                Some(R::Opaque)
            ]
        );
        assert_eq!(
            roles(true, false, true, false),
            [
                Some(R::ShadowCutout),
                None,
                Some(R::PrepassCoverage),
                Some(R::OpaqueCoverage),
                None,
                Some(R::Velocity),
                Some(R::OpaqueCoverage)
            ]
        );
        // Transparent: only its own pass.
        assert_eq!(
            roles(false, true, true, true),
            [None, None, None, None, Some(R::Transparent), None, None]
        );
    }
}
