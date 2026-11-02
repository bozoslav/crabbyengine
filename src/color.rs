//! [`Color`].

/// An RGBA color. Components are 0.0 to 1.0, in sRGB (the values you'd pick
/// in a paint program or see in CSS).
///
/// ```
/// # use crabbyengine::*;
/// let sky = Color::rgb(0.4, 0.7, 1.0);
/// let ghost = Color::WHITE.with_alpha(0.5);
/// let orange = Color::hex(0xFF8800);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);
    pub const GRAY: Color = Color::rgb(0.5, 0.5, 0.5);
    pub const RED: Color = Color::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Color = Color::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);
    pub const YELLOW: Color = Color::rgb(1.0, 1.0, 0.0);
    pub const TRANSPARENT: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);
    /// A soft dark blue-gray, the default clear color.
    pub const CRABBY_DARK: Color = Color::rgb(0.11, 0.12, 0.15);

    /// An opaque color from red, green and blue (each 0.0 to 1.0).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let teal = Color::rgb(0.0, 0.5, 0.5);
    /// ```
    pub const fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b, a: 1.0 }
    }

    /// A color with transparency (`a`: 0.0 invisible, 1.0 opaque).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let shadow = Color::rgba(0.0, 0.0, 0.0, 0.3);
    /// ```
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }

    /// An opaque color from a `0xRRGGBB` hex number.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let crab_orange = Color::hex(0xE8743B);
    /// ```
    pub const fn hex(rgb: u32) -> Color {
        Color::rgb(
            ((rgb >> 16) & 0xFF) as f32 / 255.0,
            ((rgb >> 8) & 0xFF) as f32 / 255.0,
            (rgb & 0xFF) as f32 / 255.0,
        )
    }

    /// The same color with a different alpha.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let faded = Color::RED.with_alpha(0.25);
    /// ```
    pub const fn with_alpha(self, a: f32) -> Color {
        Color { a, ..self }
    }

    /// Packs into 4 bytes (sRGB), the format sprites are sent to the GPU in.
    pub(crate) fn to_rgba8(self) -> [u8; 4] {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        [q(self.r), q(self.g), q(self.b), q(self.a)]
    }

    /// Linear-light values, which is what an sRGB render target expects.
    pub(crate) fn to_linear_wgpu(self) -> wgpu::Color {
        fn lin(c: f32) -> f64 {
            let c = f64::from(c.clamp(0.0, 1.0));
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        wgpu::Color {
            r: lin(self.r),
            g: lin(self.g),
            b: lin(self.b),
            a: f64::from(self.a.clamp(0.0, 1.0)),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parses_channels() {
        assert_eq!(Color::hex(0xFF8000).to_rgba8(), [255, 128, 0, 255]);
    }

    #[test]
    fn rgba8_clamps() {
        assert_eq!(Color::rgba(2.0, -1.0, 0.5, 1.0).to_rgba8(), [255, 0, 128, 255]);
    }

    #[test]
    fn linear_conversion_endpoints_and_midpoint() {
        let c = Color::rgb(0.0, 1.0, 0.5).to_linear_wgpu();
        assert_eq!(c.r, 0.0);
        assert!((c.g - 1.0).abs() < 1e-9);
        assert!((c.b - 0.214).abs() < 0.001);
    }
}
