pub mod colors;
pub mod style;

pub use colors::Color;
pub use style::{BarStyle, WidgetStyle};

use bar_config::ThemeConfig;

/// Compiled theme derived from [`ThemeConfig`].
#[derive(Debug, Clone)]
pub struct Theme {
    pub background:    Color,
    pub foreground:    Color,
    pub accent:        Color,
    pub font_name:     String,
    pub font_size:     f32,
    pub border_radius: f32,
    pub padding:       u16,
    pub gap:           u16,
    /// Widget container background.  `None` = transparent.
    pub widget_bg:     Option<Color>,
    /// Widget container border color.
    pub widget_border_color: Color,
    /// Widget container border width in logical pixels (0 = no border).
    pub widget_border_width: u32,
    /// `strftime` format string for the clock time display.
    pub clock_format:  String,
    /// `strftime` format string for the clock date display.
    pub date_format:   String,
    /// When `true`, widgets render Nerd Font glyphs.  `false` → ASCII labels.
    pub use_nerd_icons: bool,
    /// Horizontal inner padding applied inside each widget pill container.
    pub widget_pad_x:  u16,
    /// Vertical inner padding applied inside each widget pill container.
    pub widget_pad_y:  u16,
    /// When `true`, the clock widget appends seconds to the time display.
    pub clock_show_seconds: bool,
    /// Battery percent at which the battery icon switches to a low-power glyph.
    pub battery_warn_percent: u8,
    /// Visual style for power menu buttons: `"icon_label"`, `"icon_only"`, or `"pill"`.
    pub power_button_style: String,
}

impl Theme {
    /// Build a [`Theme`] from the config file's `[theme]` section.
    pub fn from_config(cfg: &ThemeConfig) -> Self {
        Self {
            background:    Color::from_hex(&cfg.background).unwrap_or(Color::DARK),
            foreground:    Color::from_hex(&cfg.foreground).unwrap_or(Color::WHITE),
            accent:        Color::from_hex(&cfg.accent).unwrap_or(Color::PURPLE),
            font_name:     cfg.font.clone(),
            font_size:     cfg.font_size,
            border_radius: cfg.border_radius,
            padding:       cfg.padding,
            gap:           cfg.gap,
            widget_bg: if cfg.widget_bg.is_empty() {
                None
            } else {
                Color::from_hex(&cfg.widget_bg)
            },
            widget_border_color: Color::from_hex(&cfg.widget_border_color)
                .unwrap_or(Color::DARK),
            widget_border_width: cfg.widget_border_width,
            clock_format:        cfg.clock_format.clone(),
            date_format:         cfg.date_format.clone(),
            use_nerd_icons:      cfg.icon_style.to_lowercase() != "ascii",
            widget_pad_x:        cfg.widget_padding_x,
            widget_pad_y:        cfg.widget_padding_y,
            clock_show_seconds:  cfg.clock_show_seconds,
            battery_warn_percent: cfg.battery_warn_percent,
            power_button_style:  cfg.power_button_style.clone(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_config(&ThemeConfig::default())
    }
}
