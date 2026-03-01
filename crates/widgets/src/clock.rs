use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Element,
};

/// Displays the current time and date.
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
        let fg = theme.foreground.to_iced();

        row![
            text(date_str).size(theme.font_size - 1.0).color(fg),
            text("  "),
            text(time_str).size(theme.font_size).color(fg),
        ]
        .align_y(Alignment::Center)
        .into()
    }
}
