use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::{row, text}, Alignment, Element};

/// Shows the active Hyprland submap name.  Hidden when no submap is active.
#[derive(Debug, Default)]
pub struct SubmapWidget;

impl SubmapWidget {
    pub fn new() -> Self { Self }

    /// Returns `None` when no submap is active.
    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Option<Element<'a, Message>> {
        let name = state.active_submap.as_deref()?;
        let icon = if theme.use_nerd_icons { "󰌌" } else { "MAP" };
        Some(
            row![
                text(format!("{icon} {name}"))
                    .size(theme.font_size)
                    .color(theme.accent.to_iced()),
            ]
            .align_y(Alignment::Center)
            .into(),
        )
    }
}
