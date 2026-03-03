use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::{row, text}, Alignment, Element};

/// Shows the number of available package updates.
///
/// Hidden when there are no updates or `checkupdates` is unavailable.
/// Color-coded: accent ≥10, red ≥50.
#[derive(Debug, Default)]
pub struct UpdatesWidget;

impl UpdatesWidget {
    pub fn new() -> Self { Self }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Option<Element<'a, Message>> {
        let count = state.update_count?;
        if count == 0 { return None; }

        let icon = if theme.use_nerd_icons { "󰚰" } else { "UPD" };
        let col = if count >= 50 {
            iced::Color::from_rgb8(0xf3, 0x8b, 0xa8) // red — many updates pending
        } else if count >= 10 {
            theme.accent.to_iced()
        } else {
            theme.foreground.to_iced()
        };

        Some(
            row![
                text(format!("{icon} {count}"))
                    .size(theme.font_size)
                    .color(col),
            ]
            .align_y(Alignment::Center)
            .into(),
        )
    }
}
