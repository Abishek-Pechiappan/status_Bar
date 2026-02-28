use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::text, Element};

/// Displays system uptime in a compact human-readable format.
#[derive(Debug, Default)]
pub struct UptimeWidget;

impl UptimeWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let icon = if theme.use_nerd_icons { "󰔛" } else { "UP" };
        text(format!("{icon} {}", fmt_uptime(state.system.uptime_secs)))
            .size(theme.font_size)
            .into()
    }
}

fn fmt_uptime(secs: u64) -> String {
    let mins  = secs / 60;
    let hours = mins / 60;
    let days  = hours / 24;

    if days > 0 {
        format!("{}d {}h", days, hours % 24)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins % 60)
    } else {
        format!("{}m", mins.max(1))
    }
}
