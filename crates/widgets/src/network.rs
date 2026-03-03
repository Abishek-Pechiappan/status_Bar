use crate::helpers::{mini_sparkline, mini_sparkline_colored};
use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::{column, row, text}, Alignment, Element};
use std::collections::VecDeque;

/// Displays configurable network stats with dual RX/TX sparklines.
#[derive(Debug, Default)]
pub struct NetworkWidget;

impl NetworkWidget {
    pub fn new() -> Self { Self }

    pub fn view<'a>(
        &'a self,
        state:      &'a AppState,
        theme:      &'a Theme,
        history_rx: &'a VecDeque<f32>,
        history_tx: &'a VecDeque<f32>,
    ) -> Element<'a, Message> {
        let fg     = theme.foreground.to_iced();
        let tx_col = theme.foreground.with_alpha(0.55).to_iced();
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

        let label = if parts.is_empty() {
            let rx = format_rate(state.system.net_rx);
            let tx = format_rate(state.system.net_tx);
            format!("↓{rx}  ↑{tx}")
        } else {
            parts.join("  ")
        };

        // Only show sparklines once we have enough history.
        if history_rx.len() >= 3 {
            let spark_rx = mini_sparkline(history_rx, theme);
            let spark_tx = mini_sparkline_colored(history_tx, theme, tx_col);

            column![
                row![spark_rx, iced::widget::Space::new().width(2.0), spark_tx]
                    .align_y(Alignment::Center),
                row![text(label).size(theme.font_size).color(fg)]
                    .align_y(Alignment::Center),
            ]
            .align_x(iced::Alignment::Center)
            .into()
        } else {
            row![text(label).size(theme.font_size).color(fg)]
                .align_y(Alignment::Center)
                .into()
        }
    }
}

fn signal_label(dbm: Option<i32>, nerd: bool) -> String {
    match dbm {
        None => {
            if nerd { "󰤭".to_string() } else { "-- dBm".to_string() }
        }
        Some(level) => {
            let icon = if nerd {
                if level >= -50 { "󰤨" } else if level >= -60 { "󰤥" } else if level >= -70 { "󰤢" } else { "󰤟" }
            } else if level >= -50 { "▂▄▆█" } else if level >= -60 { "▂▄▆_" } else if level >= -70 { "▂▄__" } else { "▂___" };
            format!("{icon} {level} dBm")
        }
    }
}

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
