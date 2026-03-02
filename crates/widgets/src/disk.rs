use crate::helpers::{mini_bar, usage_color};
use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Element,
};

/// Displays root filesystem disk usage with an inline fill bar.
///
/// Returns `None` when disk info is unavailable — callers should skip rendering.
#[derive(Debug, Default)]
pub struct DiskWidget;

impl DiskWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(
        &'a self,
        state: &'a AppState,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        if state.system.disk_total == 0 {
            return None;
        }

        let frac = state.system.disk_fraction();
        let pct  = (frac * 100.0) as u8;
        let icon = if theme.use_nerd_icons { "󰋊" } else { "DSK" };
        let col  = usage_color(frac, theme);
        let fg   = theme.foreground.to_iced();

        Some(
            row![
                text(format!("{icon}  ")).size(theme.font_size).color(fg),
                mini_bar(frac, 44.0, theme),
                text(format!("  {pct}%")).size(theme.font_size).color(col),
            ]
            .align_y(Alignment::Center)
            .into(),
        )
    }
}
