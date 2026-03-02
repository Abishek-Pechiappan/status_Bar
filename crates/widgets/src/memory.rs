use crate::helpers::{mini_bar, usage_color};
use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Element,
};

/// Displays RAM usage with an inline fill bar and color-coded percentage.
#[derive(Debug, Default)]
pub struct MemoryWidget;

impl MemoryWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let frac    = state.system.ram_fraction();
        let percent = (frac * 100.0) as u8;
        let icon    = if theme.use_nerd_icons { "" } else { "RAM" };
        let col     = usage_color(frac, theme);
        let fg      = theme.foreground.to_iced();

        row![
            text(format!("{icon}  ")).size(theme.font_size).color(fg),
            mini_bar(frac, 44.0, theme),
            text(format!("  {percent}%")).size(theme.font_size).color(col),
        ]
        .align_y(Alignment::Center)
        .into()
    }
}
