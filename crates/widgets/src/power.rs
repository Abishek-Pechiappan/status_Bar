use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::button, widget::text, Element};

/// A clickable power icon that opens the power panel.
///
/// Sends `Message::PowerPanelToggle`; the bar decides the display style based
/// on `config.global.power_menu_style` ("dropdown", "inline", or "overlay").
#[derive(Debug, Default)]
pub struct PowerWidget;

impl PowerWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let icon = if theme.use_nerd_icons { "󰤆" } else { "⏻" };
        let col = if state.power_panel_open {
            theme.accent.to_iced()
        } else {
            theme.foreground.to_iced()
        };

        button(text(icon).size(theme.font_size).color(col))
            .on_press(Message::PowerPanelToggle)
            .padding(0)
            .style(iced::widget::button::text)
            .into()
    }
}
