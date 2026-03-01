use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Color, Element,
};

/// Displays battery level, charging state, and estimated time remaining.
///
/// - Icon uses Nerd Font glyphs when enabled, block chars otherwise.
/// - Icon color: green when charging, red when ≤ 15 %, amber when ≤ 30 %.
/// Hidden entirely when no battery is present (desktop / VM).
#[derive(Debug, Default)]
pub struct BatteryWidget;

impl BatteryWidget {
    pub fn new() -> Self {
        Self
    }

    /// Returns `None` when there is no battery — callers should skip rendering.
    pub fn view<'a>(
        &'a self,
        state: &'a AppState,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        let pct      = state.system.battery_percent?;
        let charging = state.system.battery_charging.unwrap_or(false);

        let icon        = battery_icon(pct, charging, theme.use_nerd_icons);
        let icon_color  = battery_icon_color(pct, charging);
        let time        = format_time(state.system.battery_time_min);

        let value_str = if time.is_empty() {
            format!("{pct}%")
        } else {
            format!("{pct}% {time}")
        };

        Some(
            row![
                text(icon).size(theme.font_size + 2.0).color(icon_color),
                text(value_str).size(theme.font_size).color(theme.foreground.to_iced()),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
            .into(),
        )
    }
}

fn battery_icon(pct: u8, charging: bool, nerd: bool) -> &'static str {
    if nerd {
        if charging {
            return "󰂄";
        }
        match pct {
            91..=100 => "󰁹",
            81..=90  => "󰂂",
            71..=80  => "󰂁",
            61..=70  => "󰂀",
            51..=60  => "󰁿",
            41..=50  => "󰁾",
            31..=40  => "󰁽",
            21..=30  => "󰁼",
            11..=20  => "󰁻",
            1..=10   => "󰁺",
            _        => "󰂃",
        }
    } else {
        if charging { return "⚡"; }
        match pct {
            80..=100 => "█",
            60..=79  => "▊",
            40..=59  => "▌",
            20..=39  => "▎",
            _        => "▏",
        }
    }
}

/// Returns a color for the battery icon based on level and charging state.
fn battery_icon_color(pct: u8, charging: bool) -> Color {
    if charging {
        Color::from_rgb8(0xa6, 0xe3, 0xa1) // green — charging
    } else if pct <= 15 {
        Color::from_rgb8(0xf3, 0x8b, 0xa8) // red — critical
    } else if pct <= 30 {
        Color::from_rgb8(0xf9, 0xe2, 0xaf) // amber — low
    } else {
        Color::from_rgb8(0xa6, 0xe3, 0xa1) // green — healthy
    }
}

/// Format minutes into a compact human-readable string: "1h 23m" or "45m".
fn format_time(mins: Option<u32>) -> String {
    let m = match mins {
        Some(m) if m > 0 => m,
        _ => return String::new(),
    };
    if m >= 60 {
        format!("{}h {}m", m / 60, m % 60)
    } else {
        format!("{m}m")
    }
}
