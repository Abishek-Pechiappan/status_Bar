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
    /// Horizontal gap between bar and screen edges in logical pixels (floating look).
    pub margin: u32,
    /// Vertical gap between bar and screen edge in logical pixels (floating look).
    pub margin_top: u32,
    /// Shell command to run every poll cycle, displayed by the `custom` widget.
    /// Empty string disables the custom widget.
    pub custom_command: String,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            height:         40,
            position:       Position::Top,
            exclusive_zone: true,
            opacity:        0.95,
            margin:         0,
            margin_top:     0,
            custom_command: String::new(),
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
    /// Widget container background color (hex).  Empty string = transparent.
    pub widget_bg: String,
    /// Bar border color (hex).  Empty string = no border.
    pub border_color: String,
    /// Bar border width in logical pixels (0 = no border).
    pub border_width: u32,
    /// `strftime`-style time format string (default: `"%H:%M"`).
    pub clock_format: String,
    /// `strftime`-style date format string (default: `"%a %d %b"`).
    pub date_format: String,
    /// Icon style: `"nerd"` uses Nerd Font glyphs; `"ascii"` uses plain text labels.
    /// Use `"ascii"` if your terminal / font shows question marks for icons.
    pub icon_style: String,
    /// Horizontal inner padding for each widget pill container (pixels).
    pub widget_padding_x: u16,
    /// Vertical inner padding for each widget pill container (pixels).
    pub widget_padding_y: u16,
    /// Workspace display style: `"numbers"` shows workspace names/IDs;
    /// `"dots"` shows ● for active and ○ for inactive workspaces.
    pub workspace_style: String,
    /// When `true` (default), all open workspaces are shown.
    /// When `false`, only the active workspace is shown.
    pub workspace_show_all: bool,
    /// What the network widget displays.  Comma-separated list of:
    /// `"speed"` (↓rx ↑tx), `"name"` (interface name), `"signal"` (WiFi dBm/bars).
    /// Default: `"speed"`.  Example: `"speed,signal"` or `"name,speed"`.
    pub network_show: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            background:        "#1e1e2e".to_string(), // Catppuccin Mocha — base
            foreground:        "#cdd6f4".to_string(), // Catppuccin Mocha — text
            accent:            "#cba6f7".to_string(), // Catppuccin Mocha — mauve
            font:              "JetBrains Mono".to_string(),
            font_size:         13.0,
            border_radius:     6.0,
            padding:           8,
            gap:               4,
            widget_bg:         String::new(), // transparent by default
            border_color:      String::new(), // no border by default
            border_width:      0,
            clock_format:      "%H:%M".to_string(),
            date_format:       "%a %d %b".to_string(),
            icon_style:        "nerd".to_string(),
            widget_padding_x:  8,
            widget_padding_y:  4,
            workspace_style:   "numbers".to_string(),
            workspace_show_all: true,
            network_show:      "speed".to_string(),
        }
    }
}
