use crate::{fast_round, linear_f32_from_linear_u8, Rgba};

/// This format is used for space-efficient color representation (32 bits).
///
/// Instead of manipulating this directly it is often better
/// to first convert it to either [`Rgba`] or [`crate::Hsva`].
///
/// Internally this uses 0-255 gamma space `sRGBA` color with _premultiplied alpha_.
///
/// Premultiplied alpha means that the color values have been pre-multiplied with the alpha (opacity).
/// This is in contrast with "normal" RGBA, where the alpha is _separate_ (or "unmultiplied").
///
/// The color space is assumed to be [sRGB](https://en.wikipedia.org/wiki/SRGB).
///
/// All operations on `Color32` is done in "gamma space" (see <https://en.wikipedia.org/wiki/SRGB>).
/// This is not physically correct, but it is fast and sometimes more perceptually even than linear space.
/// If you want linear-space color, use [`Rgba`].
///
/// An `alpha=0` means the color is to be treated as an additive color.
#[repr(C)]
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Color32(pub(crate) [u8; 4]);

impl std::fmt::Debug for Color32 {
    /// Prints the contents with premultiplied alpha!
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [r, g, b, a] = self.0;
        write!(f, "#{r:02X}_{g:02X}_{b:02X}_{a:02X}")
    }
}

impl std::ops::Index<usize> for Color32 {
    type Output = u8;

    #[inline]
    fn index(&self, index: usize) -> &u8 {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Color32 {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut u8 {
        &mut self.0[index]
    }
}

impl Color32 {
    // Mostly follows CSS names:

    pub const TRANSPARENT: Self = Self::from_rgba_premultiplied(0, 0, 0, 0);
    pub const BLACK: Self = Self::from_rgb(0, 0, 0);
    #[doc(alias = "DARK_GREY")]
    pub const DARK_GRAY: Self = Self::from_rgb(96, 96, 96);
    #[doc(alias = "GREY")]
    pub const GRAY: Self = Self::from_rgb(160, 160, 160);
    #[doc(alias = "LIGHT_GREY")]
    pub const LIGHT_GRAY: Self = Self::from_rgb(220, 220, 220);
    pub const WHITE: Self = Self::from_rgb(255, 255, 255);

    pub const BROWN: Self = Self::from_rgb(165, 42, 42);
    pub const DARK_RED: Self = Self::from_rgb(0x8B, 0, 0);
    pub const RED: Self = Self::from_rgb(255, 0, 0);
    pub const LIGHT_RED: Self = Self::from_rgb(255, 128, 128);

    pub const CYAN: Self = Self::from_rgb(0, 255, 255);
    pub const MAGENTA: Self = Self::from_rgb(255, 0, 255);
    pub const YELLOW: Self = Self::from_rgb(255, 255, 0);

    pub const ORANGE: Self = Self::from_rgb(255, 165, 0);
    pub const LIGHT_YELLOW: Self = Self::from_rgb(255, 255, 0xE0);
    pub const KHAKI: Self = Self::from_rgb(240, 230, 140);

    pub const DARK_GREEN: Self = Self::from_rgb(0, 0x64, 0);
    pub const GREEN: Self = Self::from_rgb(0, 255, 0);
    pub const LIGHT_GREEN: Self = Self::from_rgb(0x90, 0xEE, 0x90);

    pub const DARK_BLUE: Self = Self::from_rgb(0, 0, 0x8B);
    pub const BLUE: Self = Self::from_rgb(0, 0, 255);
    pub const LIGHT_BLUE: Self = Self::from_rgb(0xAD, 0xD8, 0xE6);

    pub const PURPLE: Self = Self::from_rgb(0x80, 0, 0x80);

    pub const GOLD: Self = Self::from_rgb(255, 215, 0);

    pub const DEBUG_COLOR: Self = Self::from_rgba_premultiplied(0, 200, 0, 128);

    /// An ugly color that is planned to be replaced before making it to the screen.
    ///
    /// This is an invalid color, in that it does not correspond to a valid multiplied color,
    /// nor to an additive color.
    ///
    /// This is used as a special color key,
    /// i.e. often taken to mean "no color".
    pub const PLACEHOLDER: Self = Self::from_rgba_premultiplied(64, 254, 0, 128);

    #[deprecated = "Renamed to PLACEHOLDER"]
    pub const TEMPORARY_COLOR: Self = Self::PLACEHOLDER;

    #[inline]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 255])
    }

    #[inline]
    pub const fn from_rgb_additive(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 0])
    }

    /// From `sRGBA` with premultiplied alpha.
    #[inline]
    pub const fn from_rgba_premultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }

    /// From `sRGBA` WITHOUT premultiplied alpha.
    #[inline]
    pub fn from_rgba_unmultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        use std::sync::OnceLock;
        match a {
            // common-case optimization:
            0 => Self::TRANSPARENT,

            // common-case optimization:
            255 => Self::from_rgb(r, g, b),

            a => {
                static LOOKUP_TABLE: OnceLock<Box<[u8]>> = OnceLock::new();
                let lut = LOOKUP_TABLE.get_or_init(|| {
                    (0..=u16::MAX)
                        .map(|i| {
                            let [value, alpha] = i.to_ne_bytes();
                            fast_round(value as f32 * linear_f32_from_linear_u8(alpha))
                        })
                        .collect()
                });

                let [r, g, b] =
                    [r, g, b].map(|value| lut[usize::from(u16::from_ne_bytes([value, a]))]);
                Self::from_rgba_premultiplied(r, g, b, a)
            }
        }
    }

    #[doc(alias = "from_grey")]
    #[inline]
    pub const fn from_gray(l: u8) -> Self {
        Self([l, l, l, 255])
    }

    #[inline]
    pub const fn from_black_alpha(a: u8) -> Self {
        Self([0, 0, 0, a])
    }

    #[inline]
    pub fn from_white_alpha(a: u8) -> Self {
        Self([a, a, a, a])
    }

    #[inline]
    pub const fn from_additive_luminance(l: u8) -> Self {
        Self([l, l, l, 0])
    }

    #[inline]
    pub const fn is_opaque(&self) -> bool {
        self.a() == 255
    }

    #[inline]
    pub const fn r(&self) -> u8 {
        self.0[0]
    }

    #[inline]
    pub const fn g(&self) -> u8 {
        self.0[1]
    }

    #[inline]
    pub const fn b(&self) -> u8 {
        self.0[2]
    }

    #[inline]
    pub const fn a(&self) -> u8 {
        self.0[3]
    }

    /// Returns an opaque version of self
    #[inline]
    pub fn to_opaque(self) -> Self {
        Rgba::from(self).to_opaque().into()
    }

    /// Returns an additive version of self
    #[inline]
    pub const fn additive(self) -> Self {
        let [r, g, b, _] = self.to_array();
        Self([r, g, b, 0])
    }

    /// Is the alpha=0 ?
    #[inline]
    pub fn is_additive(self) -> bool {
        self.a() == 0
    }

    /// Premultiplied RGBA
    #[inline]
    pub const fn to_array(&self) -> [u8; 4] {
        [self.r(), self.g(), self.b(), self.a()]
    }

    /// Premultiplied RGBA
    #[inline]
    pub const fn to_tuple(&self) -> (u8, u8, u8, u8) {
        (self.r(), self.g(), self.b(), self.a())
    }

    /// Convert to a normal "unmultiplied" RGBA color (i.e. with separate alpha).
    ///
    /// This will unmultiply the alpha.
    #[inline]
    pub fn to_srgba_unmultiplied(&self) -> [u8; 4] {
        let [r, g, b, a] = self.to_array();
        match a {
            // Common-case optimization.
            0 | 255 => self.to_array(),
            a => {
                let factor = 255.0 / a as f32;
                let r = fast_round(factor * r as f32);
                let g = fast_round(factor * g as f32);
                let b = fast_round(factor * b as f32);
                [r, g, b, a]
            }
        }
    }

    /// Multiply with 0.5 to make color half as opaque, perceptually.
    ///
    /// Fast multiplication in gamma-space.
    ///
    /// This is perceptually even, and faster that [`Self::linear_multiply`].
    #[inline]
    pub fn gamma_multiply(self, factor: f32) -> Self {
        debug_assert!(0.0 <= factor && factor.is_finite());
        let Self([r, g, b, a]) = self;
        Self([
            (r as f32 * factor + 0.5) as u8,
            (g as f32 * factor + 0.5) as u8,
            (b as f32 * factor + 0.5) as u8,
            (a as f32 * factor + 0.5) as u8,
        ])
    }

    /// Multiply with 127 to make color half as opaque, perceptually.
    ///
    /// Fast multiplication in gamma-space.
    ///
    /// This is perceptually even, and faster that [`Self::linear_multiply`].
    #[inline]
    pub fn gamma_multiply_u8(self, factor: u8) -> Self {
        let Self([r, g, b, a]) = self;
        let factor = factor as u32;
        Self([
            ((r as u32 * factor + 127) / 255) as u8,
            ((g as u32 * factor + 127) / 255) as u8,
            ((b as u32 * factor + 127) / 255) as u8,
            ((a as u32 * factor + 127) / 255) as u8,
        ])
    }

    /// Multiply with 0.5 to make color half as opaque in linear space.
    ///
    /// This is using linear space, which is not perceptually even.
    /// You likely want to use [`Self::gamma_multiply`] instead.
    #[inline]
    pub fn linear_multiply(self, factor: f32) -> Self {
        debug_assert!(0.0 <= factor && factor.is_finite());
        // As an unfortunate side-effect of using premultiplied alpha
        // we need a somewhat expensive conversion to linear space and back.
        Rgba::from(self).multiply(factor).into()
    }

    /// Converts to floating point values in the range 0-1 without any gamma space conversion.
    ///
    /// Use this with great care! In almost all cases, you want to convert to [`crate::Rgba`] instead
    /// in order to obtain linear space color values.
    #[inline]
    pub fn to_normalized_gamma_f32(self) -> [f32; 4] {
        let Self([r, g, b, a]) = self;
        [
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        ]
    }

    /// Lerp this color towards `other` by `t` in gamma space.
    pub fn lerp_to_gamma(&self, other: Self, t: f32) -> Self {
        use emath::lerp;

        Self::from_rgba_premultiplied(
            fast_round(lerp((self[0] as f32)..=(other[0] as f32), t)),
            fast_round(lerp((self[1] as f32)..=(other[1] as f32), t)),
            fast_round(lerp((self[2] as f32)..=(other[2] as f32), t)),
            fast_round(lerp((self[3] as f32)..=(other[3] as f32), t)),
        )
    }

    /// Blend two colors in gamma space, so that `self` is behind the argument.
    pub fn blend(self, on_top: Self) -> Self {
        self.gamma_multiply_u8(255 - on_top.a()) + on_top
    }

    /// Intensity of the color.
    ///
    /// Returns a value in the range 0-1.
    /// The brighter the color, the closer to 1.
    pub fn intensity(&self) -> f32 {
        (self.r() as f32 * 0.299 + self.g() as f32 * 0.587 + self.b() as f32 * 0.114) / 255.0
    }
}

impl std::ops::Mul for Color32 {
    type Output = Self;

    /// Fast gamma-space multiplication.
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self([
            fast_round(self[0] as f32 * other[0] as f32 / 255.0),
            fast_round(self[1] as f32 * other[1] as f32 / 255.0),
            fast_round(self[2] as f32 * other[2] as f32 / 255.0),
            fast_round(self[3] as f32 * other[3] as f32 / 255.0),
        ])
    }
}

impl std::ops::Add for Color32 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self([
            self[0].saturating_add(other[0]),
            self[1].saturating_add(other[1]),
            self[2].saturating_add(other[2]),
            self[3].saturating_add(other[3]),
        ])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_color32_additive() {
        let opaque = Color32::from_rgb(40, 50, 60);
        let additive = Color32::from_rgb(255, 127, 10).additive();
        assert_eq!(additive.blend(opaque), opaque, "opaque on top of additive");
        assert_eq!(
            opaque.blend(additive),
            Color32::from_rgb(255, 177, 70),
            "additive on top of opaque"
        );
    }

    #[test]
    fn test_color32_blend_vs_gamma_blend() {
        let opaque = Color32::from_rgb(0x60, 0x60, 0x60);
        let transparent = Color32::from_rgba_unmultiplied(168, 65, 65, 79);
        assert_eq!(
            transparent.blend(opaque),
            opaque,
            "Opaque on top of transparent"
        );
        // Blending in gamma-space is the de-facto standard everywhere,
        // and it is what e.g. Chromium does.
        assert_eq!(
            opaque.blend(transparent),
            Color32::from_rgb(
                blend(0x60, 168, 79),
                blend(0x60, 65, 79),
                blend(0x60, 65, 79)
            ),
            "Transparent on top of opaque"
        );

        fn blend(dest: u8, src: u8, alpha: u8) -> u8 {
            let src = src as f32 / 255.0;
            let dest = dest as f32 / 255.0;
            let alpha = alpha as f32 / 255.0;
            fast_round((src * alpha + dest * (1.0 - alpha)) * 255.0)
        }
    }

    #[test]
    fn color32_unmultiplied_round_trip() {
        for in_rgba in [
            [10, 0, 30, 40],
            [10, 100, 200, 100],
            [10, 100, 200, 200],
            [10, 100, 200, 255],
            [10, 100, 200, 40],
            [10, 20, 0, 255],
            [10, 20, 30, 255],
            [10, 20, 30, 40],
        ] {
            let [r, g, b, a] = in_rgba;
            let c = Color32::from_rgba_unmultiplied(r, g, b, a);
            let out_rgba = c.to_srgba_unmultiplied();

            if a == 255 {
                assert_eq!(in_rgba, out_rgba);
            } else {
                // There will be small rounding errors whenever the alpha is not 0 or 255,
                // because we multiply and then unmultiply the alpha.
                for (&a, &b) in in_rgba.iter().zip(out_rgba.iter()) {
                    assert!(a.abs_diff(b) <= 3, "{in_rgba:?} != {out_rgba:?}");
                }
            }
        }
    }
}
