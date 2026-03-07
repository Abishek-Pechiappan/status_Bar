use serde::{Deserialize, Serialize};

/// Root configuration structure parsed from `bar.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DashConfig {
    /// Command to run for the Lock action in the power menu.
    pub lock_command: String,
    /// City name for wttr.in weather card (e.g. `"London"`).  Empty = disabled.
    pub weather_location: String,
    /// Theme / visual settings.
    pub theme: ThemeConfig,
    /// Bento dashboard overlay settings.
    pub dashboard: DashboardConfig,
}

impl Default for DashConfig {
    fn default() -> Self {
        Self {
            lock_command:     "loginctl lock-session".to_string(),
            weather_location: String::new(),
            theme:            ThemeConfig::default(),
            dashboard:        DashboardConfig::default(),
        }
    }
}

/// Configuration for the bento-style full-screen dashboard overlay.
///
/// Launch with `bar-dashboard` — bind it to a Hyprland key:
/// `bind = SUPER, D, exec, bar-dashboard`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DashboardConfig {
    /// When `false`, `bar-dashboard` is a no-op (exits immediately).
    pub enabled: bool,
    /// Visual card theme: `"minimal"`, `"cards"` (default), `"full"`, `"vivid"`.
    pub theme: String,
    /// Number of columns in the bento grid (2–4).  Default: 3.
    pub columns: u8,
    /// Ordered list of card types to display.
    /// Possible values: `"clock"`, `"network"`, `"battery"`, `"cpu"`, `"memory"`,
    /// `"disk"`, `"volume"`, `"brightness"`, `"media"`, `"power"`,
    /// `"uptime"`, `"temperature"`, `"updates"`,
    /// `"swap"`, `"load"`, `"gpu"`, `"bluetooth"`, `"weather"`.
    pub items: Vec<String>,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            theme:   "cards".to_string(),
            columns: 3,
            items:   default_dashboard_items(),
        }
    }
}

fn default_dashboard_items() -> Vec<String> {
    ["clock", "network", "battery", "cpu", "memory", "disk", "volume", "media", "power"]
        .iter().map(|s| s.to_string()).collect()
}

/// Theme / styling configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// Background color (hex, e.g. `"#1e1e2e"`).
    pub background: String,
    /// Primary text/foreground color.
    pub foreground: String,
    /// Accent / highlight color.
    pub accent: String,
    /// Font family name.
    pub font: String,
    /// Font size in points.
    pub font_size: f32,
    /// Corner radius for widget containers (pixels).
    pub border_radius: f32,
    /// Inner padding (pixels).
    pub padding: u16,
    /// Gap between widgets (pixels).
    pub gap: u16,
    /// Widget container background color (hex).  Empty string = transparent.
    pub widget_bg: String,
    /// Widget container border color (hex).  Empty string = no border.
    pub widget_border_color: String,
    /// Widget container border width in logical pixels (0 = no border).
    pub widget_border_width: u32,
    /// `strftime`-style time format string (default: `"%H:%M"`).
    pub clock_format: String,
    /// `strftime`-style date format string (default: `"%a %d %b"`).
    pub date_format: String,
    /// Icon style: `"nerd"` uses Nerd Font glyphs; `"ascii"` uses plain text labels.
    pub icon_style: String,
    /// Horizontal inner padding for each widget pill container (pixels).
    pub widget_padding_x: u16,
    /// Vertical inner padding for each widget pill container (pixels).
    pub widget_padding_y: u16,
    /// When `true`, the clock widget appends seconds to the time display.
    pub clock_show_seconds: bool,
    /// Battery percentage at which the battery widget shows a low-power glyph.
    pub battery_warn_percent: u8,
    /// Visual style for power menu action buttons.
    /// `"icon_label"` (default), `"icon_only"`, `"pill"`.
    pub power_button_style: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            background:          "#1e1e2e".to_string(), // Catppuccin Mocha — base
            foreground:          "#cdd6f4".to_string(), // Catppuccin Mocha — text
            accent:              "#cba6f7".to_string(), // Catppuccin Mocha — mauve
            font:                "JetBrains Mono".to_string(),
            font_size:           13.0,
            border_radius:       6.0,
            padding:             8,
            gap:                 4,
            widget_bg:           String::new(),
            widget_border_color: String::new(),
            widget_border_width: 0,
            clock_format:        "%H:%M".to_string(),
            date_format:         "%a %d %b".to_string(),
            icon_style:          "nerd".to_string(),
            widget_padding_x:    8,
            widget_padding_y:    4,
            clock_show_seconds:  false,
            battery_warn_percent: 20,
            power_button_style:  "icon_label".to_string(),
        }
    }
}
