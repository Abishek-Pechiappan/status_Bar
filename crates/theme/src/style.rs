use crate::colors::Color;

/// Visual settings for the bar surface itself.
#[derive(Debug, Clone)]
pub struct BarStyle {
    pub background: Color,
    /// Effective opacity (0.0 = transparent, 1.0 = opaque).
    pub opacity: f32,
}

/// Per-widget visual settings passed to each widget's `view()`.
#[derive(Debug, Clone)]
pub struct WidgetStyle {
    /// Optional widget background (None = transparent / inherits bar bg).
    pub background: Option<Color>,
    pub foreground: Color,
    pub accent:     Color,
    pub border_radius: f32,
    pub padding:    u16,
}
