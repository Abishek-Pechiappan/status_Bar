use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::{row, text}, Alignment, Element};

/// Displays network RX / TX rates.
#[derive(Debug, Default)]
pub struct NetworkWidget;

impl NetworkWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let rx = format_rate(state.system.net_rx);
        let tx = format_rate(state.system.net_tx);
        let label = format!("↓{rx}  ↑{tx}");

        row![
            text(label).size(theme.font_size),
        ]
        .align_y(Alignment::Center)
        .into()
    }
}

/// Format a bytes-per-second rate into a human-readable string.
fn format_rate(bps: u64) -> String {
    const MB: u64 = 1_000_000;
    const KB: u64 = 1_000;

    if bps >= MB {
        format!("{:.1}M", bps as f64 / MB as f64)
    } else if bps >= KB {
        format!("{:.0}K", bps as f64 / KB as f64)
    } else {
        format!("{}B", bps)
    }
}
