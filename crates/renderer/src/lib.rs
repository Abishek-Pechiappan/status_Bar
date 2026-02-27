//! Layout and drawing engine for the status bar.
//!
//! Phase 1: Basic three-column layout is handled directly in `bar-wayland`.
//!
//! This crate will grow to contain:
//! - Advanced layout algorithms (per-monitor overrides, dynamic sizing)
//! - Drawing primitives and compositing helpers
//! - Widget ordering and spacing engine

use bar_config::BarConfig;

/// Describes which widget kinds should appear in each bar section.
#[derive(Debug, Clone, Default)]
pub struct BarLayout {
    pub left:   Vec<String>,
    pub center: Vec<String>,
    pub right:  Vec<String>,
}

impl BarLayout {
    /// Build a [`BarLayout`] from the loaded configuration.
    pub fn from_config(config: &BarConfig) -> Self {
        Self {
            left:   config.left.iter().map(|w| w.kind.clone()).collect(),
            center: config.center.iter().map(|w| w.kind.clone()).collect(),
            right:  config.right.iter().map(|w| w.kind.clone()).collect(),
        }
    }
}
