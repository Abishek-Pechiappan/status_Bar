use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::text, Element};

/// Displays 1/5/15-minute load averages from `/proc/loadavg`.
#[derive(Debug, Default)]
pub struct LoadWidget;

impl LoadWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let l1  = state.system.load_1;
        let l5  = state.system.load_5;
        let l15 = state.system.load_15;
        let icon = if theme.use_nerd_icons { "" } else { "LD" };
        text(format!("{icon} {l1:.2} {l5:.2} {l15:.2}"))
            .size(theme.font_size)
            .into()
    }
}
