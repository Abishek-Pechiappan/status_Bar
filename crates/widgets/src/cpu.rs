use crate::helpers::{mini_sparkline, usage_color};
use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Element,
};
use std::collections::VecDeque;

/// Displays average CPU usage with a rolling sparkline and color-coded percentage.
#[derive(Debug, Default)]
pub struct CpuWidget;

impl CpuWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(
        &'a self,
        state:   &'a AppState,
        theme:   &'a Theme,
        history: &'a VecDeque<f32>,
    ) -> Element<'a, Message> {
        let pct  = state.system.cpu_average;
        let icon = if theme.use_nerd_icons { "" } else { "CPU" };
        let col  = usage_color(pct / 100.0, theme);

        row![
            text(format!("{icon} ")).size(theme.font_size).color(theme.foreground.to_iced()),
            mini_sparkline(history, theme),
            text(format!("  {pct:.0}%")).size(theme.font_size).color(col),
        ]
        .align_y(Alignment::Center)
        .into()
    }
}
