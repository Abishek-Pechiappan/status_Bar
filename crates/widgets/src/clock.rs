use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{mouse_area, row, text},
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
        // If show_seconds is on and the format doesn't already include seconds, append them.
        let time_fmt = if theme.clock_show_seconds && !theme.clock_format.contains("%S") {
            format!("{}:%S", theme.clock_format)
        } else {
            theme.clock_format.clone()
        };
        let time_str = state.time.format(&time_fmt).to_string();
        let date_str = state.time.format(&theme.date_format).to_string();
        let fg = theme.foreground.to_iced();

        mouse_area(
            row![
                text(date_str).size(theme.font_size).color(fg),
                text("  "),
                text(time_str).size(theme.font_size).color(fg),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(Message::CalendarToggle)
        .into()
    }
}
