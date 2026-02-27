pub mod colors;
pub mod style;

pub use colors::Color;
pub use style::{BarStyle, WidgetStyle};

use bar_config::ThemeConfig;

/// Compiled theme derived from [`ThemeConfig`].
///
/// All colors are pre-parsed from hex strings into normalised `[0, 1]` RGBA.
/// Calling [`Theme::from_config`] is infallible — invalid color strings fall
/// back to safe defaults.
#[derive(Debug, Clone)]
pub struct Theme {
    pub background:    Color,
    pub foreground:    Color,
    pub accent:        Color,
    pub font_size:     f32,
    pub border_radius: f32,
    pub padding:       u16,
    pub gap:           u16,
}

impl Theme {
    /// Build a [`Theme`] from the config file's `[theme]` section.
    pub fn from_config(cfg: &ThemeConfig) -> Self {
        Self {
            background:    Color::from_hex(&cfg.background).unwrap_or(Color::DARK),
            foreground:    Color::from_hex(&cfg.foreground).unwrap_or(Color::WHITE),
            accent:        Color::from_hex(&cfg.accent).unwrap_or(Color::PURPLE),
            font_size:     cfg.font_size,
            border_radius: cfg.border_radius,
            padding:       cfg.padding,
            gap:           cfg.gap,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_config(&ThemeConfig::default())
    }
}
