use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root configuration structure parsed from `bar.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BarConfig {
    /// Global settings applied to all monitors.
    pub global: GlobalConfig,
    /// Per-monitor overrides (key = output name, e.g. `"DP-1"`).
    pub monitors: HashMap<String, MonitorConfig>,
    /// Widgets on the left side of the bar.
    pub left: Vec<WidgetConfig>,
    /// Widgets in the centre of the bar.
    pub center: Vec<WidgetConfig>,
    /// Widgets on the right side of the bar.
    pub right: Vec<WidgetConfig>,
    /// Theme / visual settings.
    pub theme: ThemeConfig,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            monitors: HashMap::new(),
            left: vec![WidgetConfig::new("workspaces")],
            center: vec![WidgetConfig::new("clock")],
            right: vec![WidgetConfig::new("cpu"), WidgetConfig::new("memory")],
            theme: ThemeConfig::default(),
        }
    }
}

/// Global bar settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    /// Bar height in logical pixels.
    pub height: u32,
    /// Whether the bar sits at the top or the bottom.
    pub position: Position,
    /// Reserve an exclusive zone so windows don't overlap the bar.
    pub exclusive_zone: bool,
    /// Overall background opacity (0.0 – 1.0).
    pub opacity: f32,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            height: 40,
            position: Position::Top,
            exclusive_zone: true,
            opacity: 0.95,
        }
    }
}

/// Bar position on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    #[default]
    Top,
    Bottom,
}

/// Per-monitor overrides; unset fields fall back to `GlobalConfig`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MonitorConfig {
    pub height: Option<u32>,
    pub position: Option<Position>,
}

/// Config block for a single widget instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    /// Widget type identifier, e.g. `"clock"`, `"workspaces"`, `"cpu"`.
    pub kind: String,
    /// Optional display label override.
    #[serde(default)]
    pub label: Option<String>,
    /// Arbitrary extra options forwarded to the widget at construction.
    #[serde(default, flatten)]
    pub options: toml::Table,
}

impl WidgetConfig {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            label: None,
            options: toml::Table::new(),
        }
    }
}

/// Theme / styling configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// Bar background color (hex, e.g. `"#1e1e2e"`).
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
    /// Inner padding for each widget (pixels).
    pub padding: u16,
    /// Gap between widgets (pixels).
    pub gap: u16,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            background:    "#1e1e2e".to_string(), // Catppuccin Mocha — base
            foreground:    "#cdd6f4".to_string(), // Catppuccin Mocha — text
            accent:        "#cba6f7".to_string(), // Catppuccin Mocha — mauve
            font:          "JetBrains Mono".to_string(),
            font_size:     13.0,
            border_radius: 6.0,
            padding:       8,
            gap:           4,
        }
    }
}
