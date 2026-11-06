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

/// Post antialiasing, after tone mapping and before sprites/UI.
/// ```
/// use crabbyengine::AntiAliasing;
/// assert_eq!(AntiAliasing::default(), AntiAliasing::None);
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
}

/// Smallest and largest [`Post::render_scale`].
/// ```
/// assert_eq!(crabbyengine::MIN_RENDER_SCALE, 0.25);
/// assert_eq!(crabbyengine::MAX_RENDER_SCALE, 2.0);
/// ```
pub const MIN_RENDER_SCALE: f32 = 0.25;
/// See [`MIN_RENDER_SCALE`].
pub const MAX_RENDER_SCALE: f32 = 2.0;

/// Ground-truth ambient occlusion. Radius is in world units; quality controls
/// angular slices and horizon steps. Disabled by default to preserve scenes.
/// ```
/// use crabbyengine::AmbientOcclusion;
/// let ao = AmbientOcclusion { enabled: true, radius: 1.0, ..Default::default() };
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
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx) {
/// ctx.post.exposure = 0.5; // half a stop brighter
/// ctx.post.bloom = 0.15;
/// ctx.post.tonemap = Tonemap::Neutral;
/// ctx.post.temperature = 0.2; // warmer
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Post {
    /// The 3D scene is drawn at this fraction of the window's size in each
    /// direction, then scaled to the window before sprites and UI, which stay
    /// at the window's resolution: below 1 is cheaper and softer, above 1
    /// (supersampling) sharper and costlier. From [`MIN_RENDER_SCALE`] to
    /// [`MAX_RENDER_SCALE`]; anything else is clamped. Default 1 (no extra
    /// pass).
    pub render_scale: f32,
    /// How much the scaled image is sharpened, 0 to 1 (an adaptive
    /// contrast filter that only touches soft edges). Only used with a
    /// `render_scale` other than 1. Default 0.
    pub sharpen: f32,
    /// Ambient-only GTAO, before opaque lighting.
    pub ambient_occlusion: AmbientOcclusion,
    /// Post antialiasing. Default off to preserve existing scenes.
    pub anti_aliasing: AntiAliasing,
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
            anti_aliasing: AntiAliasing::None,
            exposure: 0.0,
            tonemap: Tonemap::Aces,
            bloom: 0.05,
            bloom_threshold: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            temperature: 0.0,
            tint: 0.0,
            vignette: 0.0,
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
    /// use crabbyengine::*;
    /// let post = Post { render_scale: 0.5, ..Default::default() };
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
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let (w, h, pixels) = Post::neutral_lut(32);
    /// ctx.post.lut = Some(ctx.create_texture(w, h, &pixels));
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
