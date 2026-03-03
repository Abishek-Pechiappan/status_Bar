use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{button, row, text},
    Alignment, Element,
};

/// Notification count badge with DND toggle.
///
/// Shows a bell icon with pending notification count; clicking it sends
/// `Message::NotifyPanelToggle`.  A second button toggles Do-Not-Disturb mode.
#[derive(Debug, Default)]
pub struct NotifyWidget;

impl NotifyWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let count = state.notifications.len();
        let bell_icon = if theme.use_nerd_icons { "󰂚" } else { "🔔" };
        let dnd_icon  = if theme.use_nerd_icons { "󰂛" } else { "DND" };

        let bell_col = if state.notify_panel_open {
            theme.accent.to_iced()
        } else {
            theme.foreground.to_iced()
        };
        let dnd_col = if state.dnd_enabled {
            theme.accent.to_iced()
        } else {
            theme.foreground.with_alpha(0.45).to_iced()
        };

        let count_str = if count > 0 { format!(" {count}") } else { String::new() };

        let bell_btn = button(
            text(format!("{bell_icon}{count_str}"))
                .size(theme.font_size)
                .color(bell_col),
        )
        .on_press(Message::NotifyPanelToggle)
        .style(iced::widget::button::text);

        let dnd_btn = button(
            text(dnd_icon)
                .size(theme.font_size - 1.0)
                .color(dnd_col),
        )
        .on_press(Message::DndToggle)
        .style(iced::widget::button::text);

        row![bell_btn, dnd_btn]
            .spacing(2)
            .align_y(Alignment::Center)
            .into()
    }
}
