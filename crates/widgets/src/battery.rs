use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::text, Element};

/// Displays battery level and charging state.
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
        let pct = state.system.battery_percent?;
        let charging = state.system.battery_charging.unwrap_or(false);

        let icon = battery_icon(pct, charging);
        let label = format!("{icon} {pct}%");

        let color = if pct <= 15 && !charging {
            // Low battery warning — use accent color as a soft alert
            theme.accent.to_iced()
        } else {
            theme.foreground.to_iced()
        };

        Some(text(label).size(theme.font_size).color(color).into())
    }
}

/// Pick a simple ASCII/Unicode battery icon based on charge level.
fn battery_icon(pct: u8, charging: bool) -> &'static str {
    if charging {
        return "⚡";
    }
    match pct {
        80..=100 => "█",
        60..=79  => "▊",
        40..=59  => "▌",
        20..=39  => "▎",
        _        => "▏", // ≤ 19 % — critically low
    }
}
