use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::text, Element};

/// Displays battery level, charging state, and estimated time remaining.
///
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

        let icon  = battery_icon(pct, charging);
        let time  = format_time(state.system.battery_time_min);
        let label = if time.is_empty() {
            format!("{icon} {pct}%")
        } else {
            format!("{icon} {pct}% ({time})")
        };

        let color = if pct <= 15 && !charging {
            theme.accent.to_iced()
        } else {
            theme.foreground.to_iced()
        };

        Some(text(label).size(theme.font_size).color(color).into())
    }
}

fn battery_icon(pct: u8, charging: bool) -> &'static str {
    if charging { return "⚡"; }
    match pct {
        80..=100 => "█",
        60..=79  => "▊",
        40..=59  => "▌",
        20..=39  => "▎",
        _        => "▏",
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
