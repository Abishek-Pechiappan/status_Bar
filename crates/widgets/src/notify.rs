use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{button, text},
    Element,
};

/// Notification count badge — shows a bell icon with the number of pending
/// notifications.  Clicking it sends `Message::NotifyPanelToggle` to expand
/// the notification panel that is rendered by the bar itself.
#[derive(Debug, Default)]
pub struct NotifyWidget;

impl NotifyWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let count = state.notifications.len();
        let icon = if theme.use_nerd_icons { "󰂚" } else { "🔔" };
        let label = if count > 0 {
            format!("{icon} {count}")
        } else {
            icon.to_string()
        };

        let fg = if state.notify_panel_open {
            theme.accent.to_iced()
        } else {
            theme.foreground.to_iced()
        };

        button(text(label).size(theme.font_size).color(fg))
            .on_press(Message::NotifyPanelToggle)
            .style(iced::widget::button::text)
            .into()
    }
}
