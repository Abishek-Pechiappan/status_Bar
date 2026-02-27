use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::text,
    Alignment, Element,
};

/// Displays the list of Hyprland workspaces.
///
/// The active workspace is highlighted with the accent colour.
#[derive(Debug, Default)]
pub struct WorkspaceWidget;

impl WorkspaceWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let items: Vec<Element<'a, Message>> = state
            .workspaces
            .iter()
            .map(|ws| {
                let is_active = ws.id == state.active_workspace;
                let label = ws.name.clone();

                if is_active {
                    text(label)
                        .size(theme.font_size)
                        .color(theme.accent.to_iced())
                        .into()
                } else {
                    text(label)
                        .size(theme.font_size)
                        .color(theme.foreground.with_alpha(0.6).to_iced())
                        .into()
                }
            })
            .collect();

        if items.is_empty() {
            // Fallback when Hyprland hasn't sent workspace info yet.
            return text("1")
                .size(theme.font_size)
                .color(theme.accent.to_iced())
                .into();
        }

        iced::widget::Row::from_vec(items)
            .spacing(theme.gap as f32)
            .align_y(Alignment::Center)
            .into()
    }
}
