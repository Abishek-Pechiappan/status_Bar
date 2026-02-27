use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Element,
};

/// Displays the current time and date.
///
/// Reads `state.time` which is updated every second via `Message::Tick`.
#[derive(Debug, Default)]
pub struct ClockWidget;

impl ClockWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let time_str = state.time.format("%H:%M").to_string();
        let date_str = state.time.format("%a %d %b").to_string();

        row![
            text(date_str).size(theme.font_size - 1.0),
            text("  "),
            text(time_str).size(theme.font_size),
        ]
        .align_y(Alignment::Center)
        .into()
    }
}
