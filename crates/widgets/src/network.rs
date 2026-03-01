use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::{row, text}, Alignment, Element};

/// Displays configurable network stats: speed, interface name, and/or WiFi signal.
#[derive(Debug, Default)]
pub struct NetworkWidget;

impl NetworkWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let mut parts: Vec<String> = Vec::new();

        if theme.network_show_name && !state.system.net_interface.is_empty() {
            parts.push(state.system.net_interface.clone());
        }

        if theme.network_show_signal {
            parts.push(signal_label(state.system.net_signal, theme.use_nerd_icons));
        }

        if theme.network_show_speed {
            let rx = format_rate(state.system.net_rx);
            let tx = format_rate(state.system.net_tx);
            parts.push(format!("↓{rx}  ↑{tx}"));
        }

        // Fallback: always show speed if nothing is selected
        let label = if parts.is_empty() {
            let rx = format_rate(state.system.net_rx);
            let tx = format_rate(state.system.net_tx);
            format!("↓{rx}  ↑{tx}")
        } else {
            parts.join("  ")
        };

        row![
            text(label).size(theme.font_size),
        ]
        .align_y(Alignment::Center)
        .into()
    }
}

/// Convert a dBm signal level to a human-readable label with signal bars.
fn signal_label(dbm: Option<i32>, nerd: bool) -> String {
    match dbm {
        None => {
            if nerd { "󰤭".to_string() } else { "-- dBm".to_string() }
        }
        Some(level) => {
            if nerd {
                // Nerd Font WiFi icons: full, high, medium, low, none
                let icon = if level >= -50 {
                    "󰤨"
                } else if level >= -60 {
                    "󰤥"
                } else if level >= -70 {
                    "󰤢"
                } else {
                    "󰤟"
                };
                format!("{icon} {level} dBm")
            } else {
                let bars = if level >= -50 {
                    "▂▄▆█"
                } else if level >= -60 {
                    "▂▄▆_"
                } else if level >= -70 {
                    "▂▄__"
                } else {
                    "▂___"
                };
                format!("{bars} {level} dBm")
            }
        }
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
