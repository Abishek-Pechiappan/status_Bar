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
    /// Widget container background.  `None` = transparent (no per-widget bg).
    pub widget_bg:     Option<Color>,
    /// Bar border color (used when `border_width > 0`).
    pub border_color:  Color,
    /// Bar border width in logical pixels.
    pub border_width:  u32,
    /// `strftime` format string for the clock time display.
    pub clock_format:  String,
    /// `strftime` format string for the clock date display.
    pub date_format:   String,
    /// When `true`, widgets render Nerd Font glyphs.  `false` → ASCII labels.
    pub use_nerd_icons: bool,
    /// Horizontal inner padding applied inside each widget pill container.
    pub widget_pad_x:  u16,
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
            widget_bg: if cfg.widget_bg.is_empty() {
                None
            } else {
                Color::from_hex(&cfg.widget_bg)
            },
            border_color: Color::from_hex(&cfg.border_color).unwrap_or(Color::DARK),
            border_width: cfg.border_width,
            clock_format:   cfg.clock_format.clone(),
            date_format:    cfg.date_format.clone(),
            use_nerd_icons: cfg.icon_style.to_lowercase() != "ascii",
            widget_pad_x:   cfg.widget_padding_x,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_config(&ThemeConfig::default())
    }
}
