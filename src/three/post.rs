//! Post-processing of the 3D frame: exposure, bloom, color grading, tone
//! mapping and vignette.

use crate::Texture;

/// How HDR light (which can be far brighter than white) is mapped to the
/// screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum Tonemap {
    /// Film-like: rich contrast, bright colors drift towards white.
    #[default]
    Aces = 0,
    /// Like a camera: keeps the hue of bright colors, softer contrast.
    AgX = 1,
    /// Khronos PBR Neutral: colors stay close to the material's own, only
    /// highlights roll off.
    Neutral = 2,
    /// No curve: everything above white clips.
    None = 3,
}

/// Antialiasing of the 3D frame. FXAA and SMAA work on the tone mapped image
/// before sprites/UI; [`Taa`](Self::Taa) works on the lit HDR frame.
/// ```
/// use ponko::render::AntiAliasing;
/// assert_eq!(ponko::render::AntiAliasing::default(), ponko::render::AntiAliasing::None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum AntiAliasing {
    /// Preserve the image (MSAA still operates on geometry).
    #[default]
    None = 0,
    /// Fast approximate antialiasing, one full-screen pass.
    Fxaa = 1,
    /// SMAA 1x, three full-screen passes.
    Smaa = 2,
    /// Temporal antialiasing: the camera jitters by a sub-pixel amount each
    /// frame, and the lit HDR frame (before bloom and tone mapping) is
    /// blended with last frame's, reprojected along per-pixel motion and
    /// clamped to the colors nearby, so edges and shading converge on
    /// smooth ones while still. Costs a motion pass, a resolve pass and
    /// three full-size HDR images.
    ///
    /// Moving models are tracked per entity; skinned models move with their
    /// model matrix only (limbs bending between frames can ghost slightly),
    /// and particles, transparent surfaces and [`LodGroup`](crate::render::LodGroup)s
    /// have no motion of their own (they reproject with the camera and what's
    /// behind them, and may smear a little when they move).
    /// Needs compute-capable devices: WebGL2 draws with FXAA instead. See
    /// [`Post::taa_sharpness`] and `Graphics::reset_temporal_history`.
    Taa = 3,
}

/// Smallest and largest [`Post::render_scale`].
/// ```
/// assert_eq!(ponko::MIN_RENDER_SCALE, 0.25);
/// assert_eq!(ponko::MAX_RENDER_SCALE, 2.0);
/// ```
pub const MIN_RENDER_SCALE: f32 = 0.25;
/// See [`GraphicsQuality::MIN_RENDER_SCALE`](crate::render::GraphicsQuality::MIN_RENDER_SCALE).
pub const MAX_RENDER_SCALE: f32 = 2.0;

/// Ground-truth ambient occlusion. Radius is in world units; quality controls
/// angular slices and horizon steps. Disabled by default to preserve scenes.
/// ```
/// use ponko::render::AmbientOcclusion;
/// let ao = ponko::render::AmbientOcclusion { enabled: true, radius: 1.0, ..Default::default() };
/// assert_eq!(ao.slices, 3);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmbientOcclusion {
    /// Enable screen-space occlusion of ambient light only. Ignored on
    /// WebGL2/downlevel devices without compute capabilities.
    pub enabled: bool,
    /// Maximum occluder distance in world units, default 1.
    pub radius: f32,
    /// Occlusion strength, default 1. Zero preserves ambient light.
    pub intensity: f32,
    /// Angular slices, clamped to 1..=8; default 3.
    pub slices: u32,
    /// Horizon samples in each direction, clamped to 1..=16; default 6.
    pub steps: u32,
}
impl Default for AmbientOcclusion {
    fn default() -> Self {
        Self {
            enabled: false,
            radius: 1.0,
            intensity: 1.0,
            slices: 3,
            steps: 6,
        }
    }
}

/// What happens to the lit 3D frame before it reaches the screen. Sprites
/// and UI drawn on top are not changed.
///
/// ```no_run
/// # use ponko::prelude::*;
/// # fn f(ctx: &mut Ctx) {
/// ctx.post.exposure = 0.5; // half a stop brighter
/// ctx.post.bloom = 0.15;
/// ctx.post.tonemap = ponko::render::Tonemap::Neutral;
/// ctx.post.temperature = 0.2; // warmer
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Post {
    /// The 3D scene is drawn at this fraction of the window's size in each
    /// direction, then scaled to the window before sprites and UI, which stay
    /// at the window's resolution: below 1 is cheaper and softer, above 1
    /// (supersampling) sharper and costlier. From [`GraphicsQuality::MIN_RENDER_SCALE`](crate::render::GraphicsQuality::MIN_RENDER_SCALE) to
    /// [`GraphicsQuality::MAX_RENDER_SCALE`](crate::render::GraphicsQuality::MAX_RENDER_SCALE); anything else is clamped. Default 1 (no extra
    /// pass).
    pub render_scale: f32,
    /// How much the scaled image is sharpened, 0 to 1 (an adaptive
    /// contrast filter that only touches soft edges). Only used with a
    /// `render_scale` other than 1. Default 0.
    pub sharpen: f32,
    /// Ambient-only GTAO, before opaque lighting.
    pub ambient_occlusion: AmbientOcclusion,
    /// Trace GTAO at half width and height, then bilaterally upsample.
    /// Contact shadows remain full resolution. Default false.
    pub ao_half_resolution: bool,
    /// Accumulate GTAO over frames with rotating slices, following the
    /// camera and moving models (the same motion vectors as TAA) and
    /// rejecting history whose depth disagrees. Independent of TAA. Default
    /// false; enable to trade history cost for fewer samples.
    pub ao_temporal: bool,
    /// Contact shadows: how far, in world units, each pixel looks toward
    /// the sun across the depth buffer for something close that blocks
    /// it. They catch the small shadows the shadow map is too coarse for:
    /// under feet, pebbles and cups, in creases. 0 (the default) is off;
    /// 0.3 suits people and props. Only things on screen cast them. Shares
    /// a full-screen pass with [`AmbientOcclusion`] (one pass for both).
    pub contact_shadows: f32,
    /// Screen-space reflections: how strongly glossy surfaces (roughness
    /// below about 0.7, fully below 0.4) reflect what's on screen, 0 to 1,
    /// on top of the sky and reflection probes they reflect anyway. Where
    /// the reflected ray finds nothing on screen (it leaves the screen, goes
    /// behind something, or runs past
    /// [`ssr_max_distance`](Self::ssr_max_distance)), the environment's
    /// reflection stays. Wet streets, polished floors, still water. Things
    /// off screen or hidden aren't reflected, and normal maps don't bend the
    /// reflection. The colors come from the last frame's image, so the
    /// first frame after a camera cut reflects only the environment. A
    /// half-resolution pass after the depth prepass; [`AntiAliasing::Taa`]
    /// smooths its grain. Reflections use depth-aware filtered mips as roughness
    /// increases; the blur footprint is bounded in screen pixels. 0 (the default) is off.
    /// ```
    /// use ponko::prelude::*;
    /// assert_eq!(ponko::render::Post::default().ssr, 0.0);
    /// let wet_street = ponko::render::Post { ssr: 1.0, ..Default::default() };
    /// assert_eq!(wet_street.ssr_max_distance, 30.0);
    /// ```
    pub ssr: f32,
    /// How far, in world units, a screen-space reflection looks along the
    /// reflected ray; it fades out toward the end. Default 30.
    pub ssr_max_distance: f32,
    /// Post antialiasing. Default off to preserve existing scenes.
    pub anti_aliasing: AntiAliasing,
    /// How much [`AntiAliasing::Taa`] sharpens the result (it softens the
    /// image a little), 0 to 1. Default 0.25.
    pub taa_sharpness: f32,
    /// Brightness in stops: +1 doubles the light, -1 halves it. Default 0.
    pub exposure: f32,
    /// How light maps to the screen. Default [`Tonemap::Aces`].
    pub tonemap: Tonemap,
    /// How strongly bright light glows into its surroundings; 0 turns bloom
    /// off. Default 0.05.
    pub bloom: f32,
    /// How bright light must be to glow (1 is white). Default 1.
    pub bloom_threshold: f32,
    /// 1 keeps the contrast, higher is punchier, lower flatter. Default 1.
    pub contrast: f32,
    /// 1 keeps the colors, 0 is grey, higher is more vivid. Default 1.
    pub saturation: f32,
    /// White balance, -1 (cool, blue) to 1 (warm, orange). Default 0.
    pub temperature: f32,
    /// White balance, -1 (green) to 1 (magenta). Default 0.
    pub tint: f32,
    /// How much the corners darken, 0 to 1. Default 0.
    pub vignette: f32,
    /// Color fringes toward the edges, as a cheap lens splits red, green
    /// and blue apart, 0 to 1. Default 0.
    /// ```
    /// use ponko::prelude::*;
    /// let speed = ponko::render::Post { chromatic_aberration: 0.8, lens_distortion: -0.26, lens_scale: 1.04, ..Default::default() };
    /// assert_eq!(ponko::render::Post::default().chromatic_aberration, 0.0);
    /// # let _ = speed;
    /// ```
    pub chromatic_aberration: f32,
    /// Bends the picture as a lens would, -1 to 1: above 0 bulges the
    /// middle out (barrel), below 0 pinches it in (pincushion). Default 0.
    pub lens_distortion: f32,
    /// Zooms the picture in (above 1) or out after the distortion, to hide
    /// the edges it pulls in. Above zero; default 1.
    pub lens_scale: f32,
    /// A color lookup table for grading, applied after tone mapping: a
    /// strip of `n` squares of `n` x `n` pixels (for example 1024 x 32),
    /// red across each square, green down it, blue from square to square.
    /// A neutral one changes nothing; paint over it in an image editor to
    /// grade. Default none.
    pub lut: Option<Texture>,
}

impl Default for Post {
    fn default() -> Self {
        Self {
            render_scale: 1.0,
            sharpen: 0.0,
            ambient_occlusion: AmbientOcclusion::default(),
            ao_half_resolution: false,
            ao_temporal: false,
            contact_shadows: 0.0,
            ssr: 0.0,
            ssr_max_distance: 30.0,
            anti_aliasing: AntiAliasing::None,
            taa_sharpness: 0.25,
            exposure: 0.0,
            tonemap: Tonemap::Aces,
            bloom: 0.05,
            bloom_threshold: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            temperature: 0.0,
            tint: 0.0,
            vignette: 0.0,
            chromatic_aberration: 0.0,
            lens_distortion: 0.0,
            lens_scale: 1.0,
            lut: None,
        }
    }
}

impl Post {
    /// The size the 3D scene is drawn at in a window of `window` pixels:
    /// scaled by [`render_scale`](Self::render_scale) (clamped; a NaN counts
    /// as 1), at least 1 pixel and at most `limit` wide (0: no limit).
    ///
    /// ```
    /// use ponko::prelude::*;
    /// let post = ponko::render::Post { render_scale: 0.5, ..Default::default() };
    /// assert_eq!(post.render_size((1920, 1080)), (960, 540));
    /// ```
    #[must_use]
    pub fn render_size(&self, window: (u32, u32)) -> (u32, u32) {
        scaled_size(window, self.render_scale, 0)
    }

    /// A neutral lookup table of `n` x `n` x `n` colors as RGBA pixels,
    /// `n * n` wide and `n` high: the starting point for a [`Post::lut`].
    ///
    /// ```no_run
    /// # use ponko::prelude::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let (w, h, pixels) = ponko::render::Post::neutral_lut(32);
    /// ctx.post.lut = Some(ctx.gfx.create_texture(w, h, &pixels));
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If `n` is not between 2 and 64.
    pub fn neutral_lut(n: u32) -> (u32, u32, Vec<u8>) {
        assert!(
            (2..=64).contains(&n),
            "a lookup table has 2 to 64 steps per color, not {n}"
        );
        let step = |i: u32| (i * 255 + (n - 1) / 2) / (n - 1);
        let mut pixels = Vec::with_capacity((n * n * n * 4) as usize);
        for g in 0..n {
            for b in 0..n {
                for r in 0..n {
                    pixels.extend_from_slice(&[step(r) as u8, step(g) as u8, step(b) as u8, 255]);
                }
            }
        }
        (n * n, n, pixels)
    }
}

/// `window` scaled by `scale` (clamped to the allowed range), at least 1 and
/// at most `limit` pixels per side (0: no limit).
pub(crate) fn scaled_size(window: (u32, u32), scale: f32, limit: u32) -> (u32, u32) {
    let scale = if scale.is_nan() {
        1.0
    } else {
        scale.clamp(MIN_RENDER_SCALE, MAX_RENDER_SCALE)
    };
    let limit = if limit == 0 { u32::MAX } else { limit };
    let side = |v: u32| ((v as f32 * scale).round() as u32).clamp(1, limit.max(1));
    (side(window.0), side(window.1))
}

/// [`Post`] as the post shader reads it.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PostUniform {
    /// x: exposure multiplier, y: bloom, z: contrast, w: saturation.
    pub grade: [f32; 4],
    /// White balance multipliers in LMS space; w: vignette.
    pub balance: [f32; 4],
    /// x: tone map, y: lookup table size (0 for none), z: bloom on (1/0).
    pub mode: [u32; 4],
    /// x: bloom threshold, y: soft knee.
    pub bloom: [f32; 4],
    /// For the scaling pass: one output pixel in uv (x, y), the sharpness
    /// (z) and the scene's size over the output's (w).
    pub output: [f32; 4],
    /// Lens distortion: x theta (or its inverse when pinching), y sigma,
    /// z one over the scale, w 1 barrel, -1 pincushion, 0 off. See [`lens`].
    pub lens: [f32; 4],
    /// x: chromatic aberration amount (0 off).
    pub fringe: [f32; 4],
}

/// The lens distortion terms, as Unity's and other engines' "lens
/// distortion" effects compute them: an angle `theta` that grows with the
/// strength (capped at 160 degrees), and `sigma = 2 tan(theta / 2)`.
fn lens(post: &Post) -> [f32; 4] {
    let intensity = post.lens_distortion;
    if !intensity.is_finite() || intensity == 0.0 {
        return [0.0, 0.0, 1.0, 0.0];
    }
    let intensity = intensity.clamp(-1.0, 1.0);
    let amount = 1.6 * (intensity * 100.0).abs().max(1.0);
    let theta = amount.min(160.0).to_radians();
    let sigma = 2.0 * (theta * 0.5).tan();
    let scale = if post.lens_scale.is_finite() && post.lens_scale > 0.0 {
        post.lens_scale
    } else {
        1.0
    };
    [
        if intensity > 0.0 { theta } else { 1.0 / theta },
        sigma,
        1.0 / scale,
        if intensity > 0.0 { 1.0 } else { -1.0 },
    ]
}

impl PostUniform {
    pub fn new(post: &Post, lut_size: Option<u32>) -> Self {
        let [l, m, s] = white_balance(post.temperature, post.tint);
        let bloom = post.bloom.max(0.0);
        Self {
            grade: [
                post.exposure.exp2(),
                bloom,
                post.contrast.max(0.0),
                post.saturation.max(0.0),
            ],
            balance: [l, m, s, post.vignette.clamp(0.0, 1.0)],
            mode: [post.tonemap as u32, lut_size.unwrap_or(0), u32::from(bloom > 0.0), 0],
            bloom: [post.bloom_threshold.max(0.0), 0.5, 0.0, 0.0],
            output: [0.0; 4],
            lens: lens(post),
            fringe: [
                if post.chromatic_aberration.is_finite() {
                    post.chromatic_aberration.clamp(0.0, 1.0) * 0.05
                } else {
                    0.0
                },
                0.0,
                0.0,
                0.0,
            ],
        }
    }

    /// The same, for a scene drawn at `scene` pixels and scaled to
    /// `output` ones.
    pub fn scaled(mut self, post: &Post, scene: (u32, u32), output: (u32, u32)) -> Self {
        let sharpen = if post.sharpen.is_nan() {
            0.0
        } else {
            post.sharpen.clamp(0.0, 1.0)
        };
        self.output = [
            1.0 / output.0.max(1) as f32,
            1.0 / output.1.max(1) as f32,
            sharpen,
            scene.0 as f32 / output.0.max(1) as f32,
        ];
        self
    }
}

/// Multipliers in LMS (cone response) space that move the white point, as
/// Unity and others do it: shift along the daylight curve by `temperature`,
/// across it by `tint`.
fn white_balance(temperature: f32, tint: f32) -> [f32; 3] {
    let t1 = temperature.clamp(-1.0, 1.0) * 100.0 / 65.0;
    let t2 = tint.clamp(-1.0, 1.0) * 100.0 / 65.0;
    let x = 0.31271 - t1 * if t1 < 0.0 { 0.1 } else { 0.05 };
    let y = 2.87 * x - 3.0 * x * x - 0.275_095_07 + t2 * 0.05;
    let big_x = x / y;
    let big_z = (1.0 - x - y) / y;
    let lms = [
        0.7328 * big_x + 0.4296 - 0.1624 * big_z,
        -0.7036 * big_x + 1.6975 + 0.0061 * big_z,
        0.0030 * big_x + 0.0136 + 0.9834 * big_z,
    ];
    let d65 = [0.949_237, 1.035_42, 1.087_28];
    [d65[0] / lms[0], d65[1] / lms[1], d65[2] / lms[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_sizes_round_clamp_and_never_reach_zero() {
        assert_eq!(scaled_size((1920, 1080), 1.0, 0), (1920, 1080));
        assert_eq!(scaled_size((1920, 1080), 0.5, 0), (960, 540));
        assert_eq!(scaled_size((1921, 1081), 0.5, 0), (961, 541));
        assert_eq!(scaled_size((100, 100), 0.01, 0), (25, 25), "clamped to the minimum");
        assert_eq!(scaled_size((100, 100), 9.0, 0), (200, 200), "clamped to the maximum");
        assert_eq!(scaled_size((100, 100), f32::NAN, 0), (100, 100));
        assert_eq!(scaled_size((100, 100), f32::INFINITY, 0), (200, 200));
        assert_eq!(scaled_size((1, 1), 0.25, 0), (1, 1));
        assert_eq!(scaled_size((0, 0), 1.0, 0), (1, 1));
        assert_eq!(
            scaled_size((4096, 4096), 2.0, 4096),
            (4096, 4096),
            "within the GPU's limit"
        );
    }

    #[test]
    fn the_scaling_uniform_holds_the_pixel_size_and_ratio() {
        let post = Post {
            sharpen: 2.0,
            ..Default::default()
        };
        let u = PostUniform::new(&post, None).scaled(&post, (960, 540), (1920, 1080));
        assert_eq!(u.output, [1.0 / 1920.0, 1.0 / 1080.0, 1.0, 0.5], "sharpness is clamped");
    }

    #[test]
    fn lens_terms_follow_the_strength_and_stay_off_at_zero() {
        let off = PostUniform::new(&Post::default(), None);
        assert_eq!((off.lens[3], off.fringe[0]), (0.0, 0.0));
        let bulge = PostUniform::new(
            &Post {
                lens_distortion: 0.5,
                chromatic_aberration: 2.0,
                ..Default::default()
            },
            None,
        );
        let theta = 80f32.to_radians();
        assert!((bulge.lens[0] - theta).abs() < 1e-6);
        assert!((bulge.lens[1] - 2.0 * (theta / 2.0).tan()).abs() < 1e-5);
        assert_eq!((bulge.lens[2], bulge.lens[3]), (1.0, 1.0));
        assert_eq!(bulge.fringe[0], 0.05, "clamped to 1");
        let pinch = PostUniform::new(
            &Post {
                lens_distortion: -0.26,
                lens_scale: 1.04,
                ..Default::default()
            },
            None,
        );
        let theta = (1.6f32 * 26.0).to_radians();
        assert!((pinch.lens[0] - 1.0 / theta).abs() < 1e-5);
        assert!((pinch.lens[2] - 1.0 / 1.04).abs() < 1e-6);
        assert_eq!(pinch.lens[3], -1.0);
        let strong = lens(&Post {
            lens_distortion: 1.0,
            ..Default::default()
        });
        assert!((strong[0] - 160f32.to_radians()).abs() < 1e-6, "capped at 160 degrees");
        assert_eq!(size_of::<PostUniform>() % 16, 0);
    }

    #[test]
    fn neutral_white_balance_changes_nothing() {
        for v in white_balance(0.0, 0.0) {
            assert!((v - 1.0).abs() < 0.01, "{v}");
        }
    }

    #[test]
    fn warm_white_balance_boosts_long_wavelengths() {
        let [l, _, s] = white_balance(0.5, 0.0);
        assert!(l > s, "warm: {l} vs {s}");
        let [l, _, s] = white_balance(-0.5, 0.0);
        assert!(l < s, "cool: {l} vs {s}");
    }

    #[test]
    fn neutral_lut_corners_are_black_and_white() {
        let (w, h, px) = Post::neutral_lut(4);
        assert_eq!((w, h, px.len()), (16, 4, 16 * 4 * 4));
        assert_eq!(&px[..4], &[0, 0, 0, 255]);
        // Last square (blue full), last row (green full), last column (red full).
        let last = ((h - 1) * w + w - 1) as usize * 4;
        assert_eq!(&px[last..last + 4], &[255, 255, 255, 255]);
    }
}
