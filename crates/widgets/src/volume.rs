use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    mouse::ScrollDelta,
    widget::{mouse_area, row, slider, text},
    Alignment, Element, Length,
};

/// Displays the default audio sink volume as an interactive slider.
///
/// - Drag slider to set exact volume.
/// - Scroll wheel adjusts ±5%.
/// - Click the icon to toggle mute.
/// Returns `None` when wpctl is unavailable.
#[derive(Debug, Default)]
pub struct VolumeWidget;

impl VolumeWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(
        &'a self,
        state: &'a AppState,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        let vol = state.system.volume?;

        let icon = if state.system.volume_muted {
            if theme.use_nerd_icons { "󰝟" } else { "[M]" }
        } else if theme.use_nerd_icons {
            if vol < 0.33 { "󰕿" } else if vol < 0.66 { "󰖀" } else { "󰕾" }
        } else {
            "VOL"
        };

        let pct = (vol * 100.0).round() as u32;

        // Icon is clickable (mute toggle); wrap in mouse_area for scroll too.
        let icon_el = mouse_area(
            text(icon).size(theme.font_size),
        )
        .on_press(Message::VolumeMuteToggle)
        .on_scroll(|delta| {
            let step = match delta {
                ScrollDelta::Lines { y, .. } | ScrollDelta::Pixels { y, .. } => {
                    if y > 0.0 { 5 } else { -5 }
                }
            };
            Message::VolumeAdjust(step)
        });

        let vol_slider = slider(0.0f32..=1.0, vol, Message::VolumeSet)
            .step(0.01f32)
            .width(Length::Fixed(72.0));

        Some(
            row![
                icon_el,
                vol_slider,
                text(format!("{pct}%")).size(theme.font_size - 1.0).width(Length::Fixed(32.0)),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
            .into(),
        )
    }
}
