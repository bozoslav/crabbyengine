//! Graphics quality presets: the settings that trade looks for speed,
//! gathered in one struct a game's options menu can switch at run time.

use super::collect::MAX_CASCADES;
use super::{
    AntiAliasing, MAX_LIGHT_SHADOW_ATLAS, MAX_LIGHT_SHADOW_RESOLUTION, MAX_LIGHT_SHADOW_VIEWS, MAX_RENDER_SCALE,
    MAX_SHADOW_RESOLUTION, MIN_LIGHT_SHADOW_ATLAS, MIN_LIGHT_SHADOW_RESOLUTION, MIN_RENDER_SCALE,
    MIN_SHADOW_RESOLUTION,
};
use crate::Ctx;

/// Which of the [`GraphicsQuality`] presets some settings are.
/// ```
/// use crabbyengine::*;
/// assert_eq!(GraphicsQuality::high().level(), QualityLevel::High);
/// let tweaked = GraphicsQuality { shadow_cascades: 2, ..GraphicsQuality::high() };
/// assert_eq!(tweaked.level(), QualityLevel::Custom);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum QualityLevel {
    /// [`GraphicsQuality::low`].
    Low = 0,
    /// [`GraphicsQuality::medium`].
    Medium = 1,
    /// [`GraphicsQuality::high`].
    High = 2,
    /// [`GraphicsQuality::ultra`].
    Ultra = 3,
    /// None of the presets: some setting was changed by hand.
    Custom = 4,
}

impl QualityLevel {
    /// The preset's settings, or `None` for [`Custom`](Self::Custom).
    /// ```
    /// use crabbyengine::*;
    /// assert_eq!(QualityLevel::Low.quality(), Some(GraphicsQuality::low()));
    /// assert_eq!(QualityLevel::Custom.quality(), None);
    /// ```
    #[must_use]
    pub fn quality(self) -> Option<GraphicsQuality> {
        match self {
            Self::Low => Some(GraphicsQuality::low()),
            Self::Medium => Some(GraphicsQuality::medium()),
            Self::High => Some(GraphicsQuality::high()),
            Self::Ultra => Some(GraphicsQuality::ultra()),
            Self::Custom => None,
        }
    }
}

/// The settings that decide how heavy the 3D picture is, in one place: the
/// resolution it is drawn at, shadows, ambient occlusion, screen-space
/// reflections and antialiasing.
/// [`low`](Self::low), [`medium`](Self::medium), [`high`](Self::high) and
/// [`ultra`](Self::ultra) are ready-made sets; apply one with
/// [`Ctx::set_graphics_quality`] and read the current one back with
/// [`Ctx::graphics_quality`].
///
/// A `GraphicsQuality` holds no state of its own: applying it writes the
/// matching fields of [`Ctx::post`] and [`Ctx::lighting`], and reading it
/// gathers them again. So a game can change any of those fields afterwards
/// (the AO radius, `shadow_distance`, ...), and
/// [`level`](Self::level) then says [`Custom`](QualityLevel::Custom).
///
/// | | low | medium | high | ultra |
/// |---|---|---|---|---|
/// | `render_scale` | 0.67 | 1 | 1 | 1 |
/// | `sharpen` | 0.5 | 0 | 0 | 0 |
/// | `shadow_distance` | 30 | 50 | 80 | 120 |
/// | `shadow_cascades` | 2 | 3 | 4 | 4 |
/// | `shadow_resolution` | 1024 | 2048 | 2048 | 4096 |
/// | `shadow_refresh` | 4 | 2 | 2 | 1 |
/// | `light_shadow_views` | 0 | 4 | 8 | 24 |
/// | `light_shadow_resolution` | 512 | 512 | 1024 | 2048 |
/// | `light_shadow_atlas` | 1024 | 2048 | 4096 | 8192 |
/// | `ambient_occlusion` (slices, steps) | off | off | 2, 4 | 4, 8 |
/// | AO half resolution / temporal | off | off | on | on |
/// | `ssr` | 0 | 0 | 1 | 1 |
/// | `anti_aliasing` | none | FXAA | SMAA | SMAA |
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx) {
/// ctx.set_graphics_quality(GraphicsQuality::medium());
/// // Then change one thing: sharper shadows, the rest as it was.
/// let mut q = ctx.graphics_quality();
/// q.shadow_resolution = 4096;
/// ctx.set_graphics_quality(q);
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphicsQuality {
    /// [`Post::render_scale`](crate::Post::render_scale): the 3D scene's
    /// resolution relative to the window, 0.25 to 2.
    pub render_scale: f32,
    /// [`Post::sharpen`](crate::Post::sharpen): sharpening of the scaled
    /// image, 0 to 1.
    pub sharpen: f32,
    /// [`Lighting::shadows`](crate::Lighting::shadows): every shadow on or off.
    pub shadows: bool,
    /// [`Lighting::shadow_distance`](crate::Lighting::shadow_distance).
    pub shadow_distance: f32,
    /// [`Lighting::shadow_cascades`](crate::Lighting::shadow_cascades), 1 to 4.
    pub shadow_cascades: u32,
    /// [`Lighting::shadow_resolution`](crate::Lighting::shadow_resolution),
    /// 256 to 4096.
    pub shadow_resolution: u32,
    /// [`Lighting::shadow_refresh`](crate::Lighting::shadow_refresh): how
    /// often the far cascades are drawn again, in frames, 1 to 8.
    pub shadow_refresh: u32,
    /// [`Lighting::light_shadow_views`](crate::Lighting::light_shadow_views):
    /// how many shadow views point and spot lights share, 0 (no shadows
    /// from them) to 24.
    pub light_shadow_views: u32,
    /// [`Lighting::light_shadow_resolution`](crate::Lighting::light_shadow_resolution):
    /// the largest light shadow view, 128 to 2048.
    pub light_shadow_resolution: u32,
    /// [`Lighting::light_shadow_atlas`](crate::Lighting::light_shadow_atlas):
    /// the atlas the light shadow views share, 1024 to 8192 (a power of two).
    pub light_shadow_atlas: u32,
    /// [`AmbientOcclusion::enabled`](crate::AmbientOcclusion::enabled).
    pub ambient_occlusion: bool,
    /// Trace at half resolution, with full-resolution bilateral upsampling.
    pub ao_half_resolution: bool,
    /// Reproject and accumulate GTAO history independently of TAA.
    pub ao_temporal: bool,
    /// [`AmbientOcclusion::slices`](crate::AmbientOcclusion::slices), 1 to 8.
    pub ao_slices: u32,
    /// [`AmbientOcclusion::steps`](crate::AmbientOcclusion::steps), 1 to 16.
    pub ao_steps: u32,
    /// [`Post::ssr`](crate::Post::ssr): screen-space reflections' strength,
    /// 0 (off) to 1.
    pub ssr: f32,
    /// [`Post::anti_aliasing`](crate::Post::anti_aliasing).
    pub anti_aliasing: AntiAliasing,
}

impl Default for GraphicsQuality {
    /// What a new game starts with: full resolution, the engine's default
    /// shadows, no AO and no post antialiasing. Between medium and high, and
    /// none of the presets.
    fn default() -> Self {
        Self {
            render_scale: 1.0,
            sharpen: 0.0,
            shadows: true,
            shadow_distance: 60.0,
            shadow_cascades: 4,
            shadow_resolution: 2048,
            shadow_refresh: 2,
            light_shadow_views: 12,
            light_shadow_resolution: 1024,
            light_shadow_atlas: 4096,
            ambient_occlusion: false,
            ao_half_resolution: false,
            ao_temporal: false,
            ao_slices: 3,
            ao_steps: 6,
            ssr: 0.0,
            anti_aliasing: AntiAliasing::None,
        }
    }
}

impl GraphicsQuality {
    /// For weak GPUs: the scene is drawn at two thirds of the window's size
    /// and sharpened on the way up, two 1024² shadow cascades reach 30 units
    /// (the far one redrawn every fourth frame),
    /// point and spot lights cast no shadows, no AO, no post antialiasing.
    /// ```
    /// use crabbyengine::*;
    /// let q = GraphicsQuality::low();
    /// assert_eq!((q.render_scale, q.shadow_cascades, q.light_shadow_views), (0.67, 2, 0));
    /// ```
    #[must_use]
    pub fn low() -> Self {
        Self {
            render_scale: 0.67,
            sharpen: 0.5,
            shadows: true,
            shadow_distance: 30.0,
            shadow_cascades: 2,
            shadow_resolution: 1024,
            shadow_refresh: 4,
            light_shadow_views: 0,
            light_shadow_resolution: 512,
            light_shadow_atlas: 1024,
            ambient_occlusion: false,
            ao_half_resolution: false,
            ao_temporal: false,
            ao_slices: 2,
            ao_steps: 4,
            ssr: 0.0,
            anti_aliasing: AntiAliasing::None,
        }
    }

    /// Full resolution, three 2048² cascades reaching 50 units, four
    /// views of up to 512² for point and spot lights in a 2048² atlas, FXAA,
    /// no AO.
    /// ```
    /// use crabbyengine::*;
    /// let q = GraphicsQuality::medium();
    /// assert_eq!((q.render_scale, q.anti_aliasing), (1.0, AntiAliasing::Fxaa));
    /// ```
    #[must_use]
    pub fn medium() -> Self {
        Self {
            render_scale: 1.0,
            sharpen: 0.0,
            shadows: true,
            shadow_distance: 50.0,
            shadow_cascades: 3,
            shadow_resolution: 2048,
            shadow_refresh: 2,
            light_shadow_views: 4,
            light_shadow_resolution: 512,
            light_shadow_atlas: 2048,
            ambient_occlusion: false,
            ao_half_resolution: false,
            ao_temporal: false,
            ao_slices: 2,
            ao_steps: 4,
            ssr: 0.0,
            anti_aliasing: AntiAliasing::Fxaa,
        }
    }

    /// Four 2048² cascades reaching 80 units, eight views of up to 1024² for
    /// point and spot lights in a 4096² atlas, light AO (2 slices, 4
    /// steps), screen-space reflections, SMAA.
    /// ```
    /// use crabbyengine::*;
    /// let q = GraphicsQuality::high();
    /// assert!(q.ambient_occlusion && q.ssr == 1.0 && q.anti_aliasing == AntiAliasing::Smaa);
    /// ```
    #[must_use]
    pub fn high() -> Self {
        Self {
            render_scale: 1.0,
            sharpen: 0.0,
            shadows: true,
            shadow_distance: 80.0,
            shadow_cascades: 4,
            shadow_resolution: 2048,
            shadow_refresh: 2,
            light_shadow_views: 8,
            light_shadow_resolution: 1024,
            light_shadow_atlas: 4096,
            ambient_occlusion: true,
            ao_half_resolution: true,
            ao_temporal: true,
            ao_slices: 2,
            ao_steps: 4,
            ssr: 1.0,
            anti_aliasing: AntiAliasing::Smaa,
        }
    }

    /// Four 4096² cascades (256 MB of shadow maps) reaching 120 units, all
    /// redrawn every frame, 24
    /// views of up to 2048² for point and spot lights in an 8192² atlas
    /// (another 256 MB), finer AO (4 slices, 8 steps), screen-space
    /// reflections, SMAA.
    /// ```
    /// use crabbyengine::*;
    /// let q = GraphicsQuality::ultra();
    /// assert_eq!((q.shadow_resolution, q.ao_steps), (4096, 8));
    /// ```
    #[must_use]
    pub fn ultra() -> Self {
        Self {
            render_scale: 1.0,
            sharpen: 0.0,
            shadows: true,
            shadow_distance: 120.0,
            shadow_cascades: 4,
            shadow_resolution: 4096,
            shadow_refresh: 1,
            light_shadow_views: MAX_LIGHT_SHADOW_VIEWS as u32,
            light_shadow_resolution: 2048,
            light_shadow_atlas: 8192,
            ambient_occlusion: true,
            ao_half_resolution: true,
            ao_temporal: true,
            ao_slices: 4,
            ao_steps: 8,
            ssr: 1.0,
            anti_aliasing: AntiAliasing::Smaa,
        }
    }

    /// Which preset these are, or [`Custom`](QualityLevel::Custom). Values
    /// outside the allowed ranges are compared as they would be applied.
    /// ```
    /// use crabbyengine::*;
    /// assert_eq!(GraphicsQuality::ultra().level(), QualityLevel::Ultra);
    /// assert_eq!(GraphicsQuality::default().level(), QualityLevel::Custom);
    /// ```
    #[must_use]
    pub fn level(&self) -> QualityLevel {
        let me = self.clamped();
        [
            QualityLevel::Low,
            QualityLevel::Medium,
            QualityLevel::High,
            QualityLevel::Ultra,
        ]
        .into_iter()
        .find(|l| l.quality() == Some(me))
        .unwrap_or(QualityLevel::Custom)
    }

    /// These settings with every value brought into its allowed range (as
    /// [`Ctx::set_graphics_quality`] applies them); a NaN becomes the
    /// [default](Self::default).
    /// ```
    /// use crabbyengine::*;
    /// let q = GraphicsQuality { render_scale: 9.0, shadow_cascades: 0, ..Default::default() }.clamped();
    /// assert_eq!((q.render_scale, q.shadow_cascades), (2.0, 1));
    /// ```
    #[must_use]
    pub fn clamped(&self) -> Self {
        let d = Self::default();
        let float = |v: f32, lo: f32, hi: f32, fallback: f32| if v.is_nan() { fallback } else { v.clamp(lo, hi) };
        Self {
            render_scale: float(self.render_scale, MIN_RENDER_SCALE, MAX_RENDER_SCALE, d.render_scale),
            sharpen: float(self.sharpen, 0.0, 1.0, d.sharpen),
            shadows: self.shadows,
            shadow_distance: float(self.shadow_distance, 0.0, f32::MAX, d.shadow_distance),
            shadow_cascades: self.shadow_cascades.clamp(1, MAX_CASCADES as u32),
            shadow_resolution: self
                .shadow_resolution
                .clamp(MIN_SHADOW_RESOLUTION, MAX_SHADOW_RESOLUTION),
            shadow_refresh: self.shadow_refresh.clamp(1, super::collect::MAX_SHADOW_REFRESH),
            light_shadow_views: self.light_shadow_views.min(MAX_LIGHT_SHADOW_VIEWS as u32),
            light_shadow_resolution: self
                .light_shadow_resolution
                .clamp(MIN_LIGHT_SHADOW_RESOLUTION, MAX_LIGHT_SHADOW_RESOLUTION),
            light_shadow_atlas: super::shadow_atlas::floor_pow2(
                self.light_shadow_atlas
                    .clamp(MIN_LIGHT_SHADOW_ATLAS, MAX_LIGHT_SHADOW_ATLAS),
            ),
            ambient_occlusion: self.ambient_occlusion,
            ao_half_resolution: self.ao_half_resolution,
            ao_temporal: self.ao_temporal,
            ao_slices: self.ao_slices.clamp(1, 8),
            ao_steps: self.ao_steps.clamp(1, 16),
            ssr: float(self.ssr, 0.0, 1.0, d.ssr),
            anti_aliasing: self.anti_aliasing,
        }
    }
}

impl Ctx {
    /// The current graphics quality, gathered from [`Ctx::post`] and
    /// [`Ctx::lighting`]. Use [`GraphicsQuality::level`] to see which
    /// preset it is.
    ///
    /// ```
    /// use crabbyengine::*;
    /// Game::new("quality", 64, 64).run_headless(0, |ctx| {
    ///     ctx.set_graphics_quality(GraphicsQuality::medium());
    ///     assert_eq!(ctx.graphics_quality().level(), QualityLevel::Medium);
    /// });
    /// ```
    #[must_use]
    pub fn graphics_quality(&self) -> GraphicsQuality {
        GraphicsQuality {
            render_scale: self.post.render_scale,
            sharpen: self.post.sharpen,
            shadows: self.lighting.shadows,
            shadow_distance: self.lighting.shadow_distance,
            shadow_cascades: self.lighting.shadow_cascades,
            shadow_resolution: self.lighting.shadow_resolution,
            shadow_refresh: self.lighting.shadow_refresh,
            light_shadow_views: self.lighting.light_shadow_views,
            light_shadow_resolution: self.lighting.light_shadow_resolution,
            light_shadow_atlas: self.lighting.light_shadow_atlas,
            ambient_occlusion: self.post.ambient_occlusion.enabled,
            ao_half_resolution: self.post.ao_half_resolution,
            ao_temporal: self.post.ao_temporal,
            ao_slices: self.post.ambient_occlusion.slices,
            ao_steps: self.post.ambient_occlusion.steps,
            ssr: self.post.ssr,
            anti_aliasing: self.post.anti_aliasing,
        }
        .clamped()
    }

    /// Switches the 3D picture to `quality` (values outside their ranges are
    /// clamped, see [`GraphicsQuality::clamped`]), from any frame, without
    /// restarting. It writes the matching fields of [`Ctx::post`] and
    /// [`Ctx::lighting`] and leaves the others (AO radius, tone mapping,
    /// ...) alone. The next frame draws the scene at the new resolution and
    /// with new shadow maps where their size or count changed: nothing is
    /// drawn from stale targets.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.set_graphics_quality(GraphicsQuality::low());
    /// ctx.post.ambient_occlusion.radius = 2.0; // a tweak on top
    /// # }
    /// ```
    pub fn set_graphics_quality(&mut self, quality: GraphicsQuality) {
        let q = quality.clamped();
        self.post.render_scale = q.render_scale;
        self.post.sharpen = q.sharpen;
        self.post.anti_aliasing = q.anti_aliasing;
        self.post.ambient_occlusion.enabled = q.ambient_occlusion;
        self.post.ao_half_resolution = q.ao_half_resolution;
        self.post.ao_temporal = q.ao_temporal;
        self.post.ambient_occlusion.slices = q.ao_slices;
        self.post.ambient_occlusion.steps = q.ao_steps;
        self.post.ssr = q.ssr;
        self.lighting.shadows = q.shadows;
        self.lighting.shadow_distance = q.shadow_distance;
        self.lighting.shadow_cascades = q.shadow_cascades;
        self.lighting.shadow_resolution = q.shadow_resolution;
        self.lighting.shadow_refresh = q.shadow_refresh;
        self.lighting.light_shadow_views = q.light_shadow_views;
        self.lighting.light_shadow_resolution = q.light_shadow_resolution;
        self.lighting.light_shadow_atlas = q.light_shadow_atlas;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_get_heavier_from_low_to_ultra() {
        let p = [
            GraphicsQuality::low(),
            GraphicsQuality::medium(),
            GraphicsQuality::high(),
            GraphicsQuality::ultra(),
        ];
        for w in p.windows(2) {
            let (a, b) = (w[0], w[1]);
            assert!(a.render_scale <= b.render_scale);
            assert!(a.shadow_distance < b.shadow_distance);
            assert!(a.shadow_cascades <= b.shadow_cascades);
            assert!(a.shadow_resolution <= b.shadow_resolution);
            assert!(
                a.shadow_refresh >= b.shadow_refresh,
                "heavier redraws far shadows more often"
            );
            assert!(a.light_shadow_views < b.light_shadow_views);
            assert!(a.light_shadow_resolution <= b.light_shadow_resolution);
            assert!(a.light_shadow_atlas < b.light_shadow_atlas);
            assert!(a.ambient_occlusion <= b.ambient_occlusion);
            assert!(a.ao_slices * a.ao_steps <= b.ao_slices * b.ao_steps);
            assert!(a.ssr <= b.ssr);
            assert!(a.anti_aliasing as u32 <= b.anti_aliasing as u32);
        }
    }

    #[test]
    fn presets_are_valid_and_distinct() {
        for l in [
            QualityLevel::Low,
            QualityLevel::Medium,
            QualityLevel::High,
            QualityLevel::Ultra,
        ] {
            let q = l.quality().unwrap();
            assert_eq!(q, q.clamped(), "{l:?} is inside every range");
            assert_eq!(q.level(), l);
        }
        assert_eq!(GraphicsQuality::default().level(), QualityLevel::Custom);
        assert_eq!(GraphicsQuality::default(), GraphicsQuality::default().clamped());
    }

    #[test]
    fn the_documented_values() {
        let low = GraphicsQuality::low();
        assert_eq!((low.render_scale, low.sharpen, low.shadow_distance), (0.67, 0.5, 30.0));
        assert_eq!((low.shadow_cascades, low.shadow_resolution), (2, 1024));
        assert_eq!(low.shadow_refresh, 4);
        assert_eq!(GraphicsQuality::ultra().shadow_refresh, 1);
        assert_eq!(
            (
                low.light_shadow_views,
                low.light_shadow_resolution,
                low.light_shadow_atlas
            ),
            (0, 512, 1024)
        );
        assert_eq!((low.ambient_occlusion, low.anti_aliasing), (false, AntiAliasing::None));
        let medium = GraphicsQuality::medium();
        assert_eq!(
            (medium.shadow_distance, medium.shadow_cascades, medium.shadow_resolution),
            (50.0, 3, 2048)
        );
        assert_eq!(
            (
                medium.light_shadow_views,
                medium.light_shadow_resolution,
                medium.light_shadow_atlas
            ),
            (4, 512, 2048)
        );
        assert_eq!(
            (medium.ambient_occlusion, medium.anti_aliasing),
            (false, AntiAliasing::Fxaa)
        );
        let high = GraphicsQuality::high();
        assert_eq!(
            (high.shadow_distance, high.shadow_cascades, high.shadow_resolution),
            (80.0, 4, 2048)
        );
        assert_eq!(
            (
                high.light_shadow_views,
                high.light_shadow_resolution,
                high.light_shadow_atlas
            ),
            (8, 1024, 4096)
        );
        assert_eq!((high.ambient_occlusion, high.ao_slices, high.ao_steps), (true, 2, 4));
        assert!(high.ao_half_resolution && high.ao_temporal);
        assert_eq!(high.anti_aliasing, AntiAliasing::Smaa);
        assert_eq!((low.ssr, medium.ssr, high.ssr), (0.0, 0.0, 1.0));
        let ultra = GraphicsQuality::ultra();
        assert_eq!(
            (ultra.shadow_distance, ultra.shadow_cascades, ultra.shadow_resolution),
            (120.0, 4, 4096)
        );
        assert_eq!(
            (
                ultra.light_shadow_views,
                ultra.light_shadow_resolution,
                ultra.light_shadow_atlas
            ),
            (24, 2048, 8192)
        );
        assert_eq!((ultra.ao_slices, ultra.ao_steps, ultra.ssr), (4, 8, 1.0));
    }

    #[test]
    fn clamping_fixes_out_of_range_and_nan_values() {
        let q = GraphicsQuality {
            render_scale: 0.0,
            sharpen: f32::NAN,
            shadow_distance: -5.0,
            shadow_cascades: 9,
            shadow_resolution: 1 << 20,
            light_shadow_views: 99,
            light_shadow_resolution: 1,
            light_shadow_atlas: 5000,
            ao_slices: 0,
            ao_steps: 99,
            ssr: 7.0,
            ..Default::default()
        }
        .clamped();
        assert_eq!(q.render_scale, MIN_RENDER_SCALE);
        assert_eq!(q.sharpen, 0.0);
        assert_eq!(q.shadow_distance, 0.0);
        assert_eq!(q.shadow_cascades, 4);
        assert_eq!(q.shadow_resolution, 4096);
        assert_eq!(q.light_shadow_views, 24);
        assert_eq!(q.light_shadow_resolution, 128);
        assert_eq!(q.light_shadow_atlas, 4096);
        assert_eq!((q.ao_slices, q.ao_steps), (1, 16));
        assert_eq!(q.ssr, 1.0);
    }

    #[test]
    fn applying_a_preset_and_reading_it_back_round_trips() {
        crate::Game::new("quality", 64, 64).run_headless(0, |ctx| {
            assert_eq!(ctx.graphics_quality(), GraphicsQuality::default());
            for l in [
                QualityLevel::Ultra,
                QualityLevel::Low,
                QualityLevel::High,
                QualityLevel::Medium,
            ] {
                let q = l.quality().unwrap();
                ctx.set_graphics_quality(q);
                assert_eq!(ctx.graphics_quality(), q);
                assert_eq!(ctx.graphics_quality().level(), l);
                assert_eq!(ctx.post.render_scale, q.render_scale);
                assert_eq!(ctx.lighting.shadow_resolution, q.shadow_resolution);
                assert_eq!(ctx.post.ambient_occlusion.enabled, q.ambient_occlusion);
                assert_eq!(ctx.post.ao_half_resolution, q.ao_half_resolution);
                assert_eq!(ctx.post.ao_temporal, q.ao_temporal);
                assert_eq!(ctx.post.ssr, q.ssr);
            }
        });
    }

    #[test]
    fn tweaks_after_a_preset_stick_and_make_it_custom() {
        crate::Game::new("quality", 64, 64).run_headless(0, |ctx| {
            ctx.post.ambient_occlusion.radius = 3.5;
            ctx.set_graphics_quality(GraphicsQuality::high());
            assert_eq!(
                ctx.post.ambient_occlusion.radius, 3.5,
                "settings outside the struct stay"
            );
            ctx.lighting.shadow_cascades = 2;
            assert_eq!(ctx.graphics_quality().level(), QualityLevel::Custom);
            ctx.set_graphics_quality(GraphicsQuality {
                render_scale: 5.0,
                ..GraphicsQuality::high()
            });
            assert_eq!(ctx.post.render_scale, MAX_RENDER_SCALE, "clamped when applied");
        });
    }
}
