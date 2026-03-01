use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Color, Element,
};

/// Displays average CPU usage as a percentage.
///
/// Icon is rendered in the accent color.  The percentage turns amber above
/// 70 % and red above 90 % to give an at-a-glance load indicator.
#[derive(Debug, Default)]
pub struct CpuWidget;

impl CpuWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let pct  = state.system.cpu_average;
        let icon = if theme.use_nerd_icons { "" } else { "CPU" };

        let value_color = if pct >= 90.0 {
            Color::from_rgb8(0xf3, 0x8b, 0xa8) // red — critical
        } else if pct >= 70.0 {
            Color::from_rgb8(0xf9, 0xe2, 0xaf) // amber — warning
        } else {
            theme.foreground.to_iced()
        };

        row![
            text(icon).size(theme.font_size).color(theme.accent.to_iced()),
            text(format!("{pct:.0}%")).size(theme.font_size).color(value_color),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into()
    }
}
