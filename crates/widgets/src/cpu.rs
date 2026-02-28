use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::text, Element};

/// Displays average CPU usage as a percentage.
#[derive(Debug, Default)]
pub struct CpuWidget;

impl CpuWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let icon = if theme.use_nerd_icons { "" } else { "CPU" };
        text(format!("{icon} {:.0}%", state.system.cpu_average))
            .size(theme.font_size)
            .into()
    }
}
