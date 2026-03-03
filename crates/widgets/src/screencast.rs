use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::{row, text}, Alignment, Element};

/// Red indicator dot shown while Hyprland is screen-sharing or recording.
#[derive(Debug, Default)]
pub struct ScreencastWidget;

impl ScreencastWidget {
    pub fn new() -> Self { Self }

    /// Returns `None` when not screencasting.
    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Option<Element<'a, Message>> {
        if !state.screencasting { return None; }
        let icon = if theme.use_nerd_icons { "󰕧" } else { "REC" };
        Some(
            row![
                text(icon)
                    .size(theme.font_size)
                    .color(iced::Color::from_rgb8(0xf3, 0x8b, 0xa8)),
            ]
            .align_y(Alignment::Center)
            .into(),
        )
    }
}
