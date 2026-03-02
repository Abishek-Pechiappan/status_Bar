use crate::helpers::mini_bar;
use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Element,
};

/// Displays battery level with a fill bar and state-colored icon/text.
///
/// Green when charging, red when at or below the configured warn threshold.
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
        let frac     = pct as f32 / 100.0;

        let col = if charging {
            iced::Color::from_rgb8(0xa6, 0xe3, 0xa1)   // green
        } else if pct <= theme.battery_warn_percent {
            iced::Color::from_rgb8(0xf3, 0x8b, 0xa8)   // red
        } else {
            theme.foreground.to_iced()
        };

        let icon = battery_icon(pct, charging, theme.use_nerd_icons);
        let time = format_time(state.system.battery_time_min);

        let time_el: Element<'a, Message> = if time.is_empty() {
            iced::widget::Space::new().width(0).into()
        } else {
            text(format!("  {time}")).size(theme.font_size).color(col).into()
        };

        Some(
            row![
                text(format!("{icon}  ")).size(theme.font_size).color(col),
                mini_bar(frac, 36.0, theme),
                text(format!("  {pct}%")).size(theme.font_size).color(col),
                time_el,
            ]
            .align_y(Alignment::Center)
            .into(),
        )
    }
}

fn battery_icon(pct: u8, charging: bool, nerd: bool) -> &'static str {
    if nerd {
        if charging { return "󰂄"; }
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
