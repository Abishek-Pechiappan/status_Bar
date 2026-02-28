use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Element,
};

/// Displays the screen brightness as a percentage.
///
/// Returns `None` when no backlight device is found — callers should skip rendering.
#[derive(Debug, Default)]
pub struct BrightnessWidget;

impl BrightnessWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(
        &'a self,
        state: &'a AppState,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        let pct = state.system.brightness?;
        Some(
            row![text(format!("󰃞 {pct}%")).size(theme.font_size)]
                .align_y(Alignment::Center)
                .into(),
        )
    }
}
