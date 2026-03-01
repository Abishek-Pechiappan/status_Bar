use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::button, widget::text, Element};

/// A clickable power icon in the bar that opens the power menu overlay.
///
/// Clicking sends `Message::PowerMenuOpen` which the bar handles by spawning
/// the `bar-powermenu` process.
#[derive(Debug, Default)]
pub struct PowerWidget;

impl PowerWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, _state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let icon = if theme.use_nerd_icons { "󰤆" } else { "⏻" };
        let fg   = theme.foreground.to_iced();

        button(text(icon).size(theme.font_size).color(fg))
            .on_press(Message::PowerMenuOpen)
            .padding(0)
            .style(iced::widget::button::text)
            .into()
    }
}
