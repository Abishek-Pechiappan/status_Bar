use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Element,
};

/// Displays the CPU package temperature.
///
/// Returns `None` when the sensor is unavailable — callers should skip rendering.
#[derive(Debug, Default)]
pub struct TempWidget;

impl TempWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(
        &'a self,
        state: &'a AppState,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        let temp = state.system.cpu_temp?;
        let icon = if theme.use_nerd_icons { "" } else { "TMP" };
        Some(
            row![text(format!("{icon} {temp:.0}°C")).size(theme.font_size)]
                .align_y(Alignment::Center)
                .into(),
        )
    }
}
