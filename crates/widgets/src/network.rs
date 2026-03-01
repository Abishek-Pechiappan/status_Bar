use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Color, Element,
};

/// Displays configurable network stats: speed, interface name, and/or WiFi signal.
///
/// When speed is shown, ↓ (download) is rendered in the accent color and
/// ↑ (upload) in a muted foreground so you can tell them apart at a glance.
#[derive(Debug, Default)]
pub struct NetworkWidget;

impl NetworkWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let accent = theme.accent.to_iced();
        let fg     = theme.foreground.to_iced();
        let muted  = theme.foreground.with_alpha(0.55).to_iced();
        let upload_color = Color::from_rgb8(0x89, 0xdc, 0xeb); // sky-blue for ↑

        let mut items: Vec<Element<'_, Message>> = Vec::new();

        // Optional: interface name
        if theme.network_show_name && !state.system.net_interface.is_empty() {
            items.push(text(state.system.net_interface.clone()).size(theme.font_size - 1.0).color(muted).into());
        }

        // Optional: WiFi signal
        if theme.network_show_signal {
            let (icon, label) = signal_parts(state.system.net_signal, theme.use_nerd_icons);
            items.push(text(icon).size(theme.font_size).color(accent).into());
            if !label.is_empty() {
                items.push(text(label).size(theme.font_size - 1.0).color(muted).into());
            }
        }

        // Speed: ↓rx ↑tx with colored arrows
        let show_speed = theme.network_show_speed || (!theme.network_show_name && !theme.network_show_signal);
        if show_speed {
            let rx = format_rate(state.system.net_rx);
            let tx = format_rate(state.system.net_tx);
            items.push(text("↓").size(theme.font_size).color(accent).into());
            items.push(text(rx).size(theme.font_size).color(fg).into());
            items.push(text("↑").size(theme.font_size).color(upload_color).into());
            items.push(text(tx).size(theme.font_size).color(fg).into());
        }

        if items.is_empty() {
            // Absolute fallback
            let rx = format_rate(state.system.net_rx);
            let tx = format_rate(state.system.net_tx);
            return row![
                text("↓").size(theme.font_size).color(accent),
                text(rx).size(theme.font_size).color(fg),
                text("↑").size(theme.font_size).color(upload_color),
                text(tx).size(theme.font_size).color(fg),
            ]
            .spacing(3)
            .align_y(Alignment::Center)
            .into();
        }

        iced::widget::Row::from_vec(items)
            .spacing(3)
            .align_y(Alignment::Center)
            .into()
    }
}

/// Split a signal level into an icon string and a dBm label string.
fn signal_parts(dbm: Option<i32>, nerd: bool) -> (&'static str, String) {
    match dbm {
        None => {
            if nerd { ("󰤭", String::new()) } else { ("--", String::new()) }
        }
        Some(level) => {
            let icon = if nerd {
                if level >= -50 { "󰤨" } else if level >= -60 { "󰤥" } else if level >= -70 { "󰤢" } else { "󰤟" }
            } else {
                if level >= -50 { "▂▄▆█" } else if level >= -60 { "▂▄▆_" } else if level >= -70 { "▂▄__" } else { "▂___" }
            };
            (icon, format!("{level}dBm"))
        }
    }
}

/// Format a bytes-per-second rate into a compact human-readable string.
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
