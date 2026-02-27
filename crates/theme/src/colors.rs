/// Normalised RGBA colour (each channel in `[0.0, 1.0]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const DARK:        Self = Self { r: 0.118, g: 0.118, b: 0.180, a: 1.0 }; // #1e1e2e
    pub const WHITE:       Self = Self { r: 0.804, g: 0.839, b: 0.957, a: 1.0 }; // #cdd6f4
    pub const PURPLE:      Self = Self { r: 0.796, g: 0.651, b: 0.969, a: 1.0 }; // #cba6f7
    pub const TRANSPARENT: Self = Self { r: 0.0,   g: 0.0,   b: 0.0,   a: 0.0 };

    /// Parse a CSS-style hex color string (`#RRGGBB` or `#RRGGBBAA`).
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');

        let byte = |s: &str| -> Option<u8> { u8::from_str_radix(s, 16).ok() };

        match hex.len() {
            6 => Some(Self {
                r: byte(&hex[0..2])? as f32 / 255.0,
                g: byte(&hex[2..4])? as f32 / 255.0,
                b: byte(&hex[4..6])? as f32 / 255.0,
                a: 1.0,
            }),
            8 => Some(Self {
                r: byte(&hex[0..2])? as f32 / 255.0,
                g: byte(&hex[2..4])? as f32 / 255.0,
                b: byte(&hex[4..6])? as f32 / 255.0,
                a: byte(&hex[6..8])? as f32 / 255.0,
            }),
            _ => None,
        }
    }

    /// Convert to an [`iced::Color`] for use in Iced widgets.
    #[inline]
    pub fn to_iced(self) -> iced::Color {
        iced::Color::from_rgba(self.r, self.g, self.b, self.a)
    }

    /// Return a copy with the alpha channel set to `alpha`.
    #[inline]
    #[must_use]
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.a = alpha.clamp(0.0, 1.0);
        self
    }
}
