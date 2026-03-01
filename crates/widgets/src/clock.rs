use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Element,
};

/// Displays the current time and date.
///
/// - Time is rendered in the accent color at full font size.
/// - Date is rendered in a muted foreground at a slightly smaller size.
///
/// Reads `state.time` which is updated every second via `Message::Tick`.
/// Format strings come from `theme.clock_format` and `theme.date_format`.
#[derive(Debug, Default)]
pub struct ClockWidget;

impl ClockWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let time_str = state.time.format(&theme.clock_format).to_string();
        let date_str = state.time.format(&theme.date_format).to_string();

        let muted = theme.foreground.with_alpha(0.5).to_iced();

        row![
            text(date_str).size(theme.font_size - 1.0).color(muted),
            text(time_str).size(theme.font_size).color(theme.accent.to_iced()),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    }
}
