use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    mouse::ScrollDelta,
    widget::{mouse_area, text},
    Element,
};

/// Displays the default audio sink volume.
///
/// Interactive: scroll wheel adjusts volume ±5%, left-click toggles mute.
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
            "󰝟"
        } else if vol < 0.33 {
            "󰕿"
        } else if vol < 0.66 {
            "󰖀"
        } else {
            "󰕾"
        };

        let pct   = (vol * 100.0).round() as u32;
        let label = if state.system.volume_muted {
            format!("{icon} muted")
        } else {
            format!("{icon} {pct}%")
        };

        let content = text(label).size(theme.font_size);

        Some(
            mouse_area(content)
                .on_scroll(|delta| {
                    let step = match delta {
                        ScrollDelta::Lines { y, .. } | ScrollDelta::Pixels { y, .. } => {
                            if y > 0.0 { 5 } else { -5 }
                        }
                    };
                    Message::VolumeAdjust(step)
                })
                .on_press(Message::VolumeMuteToggle)
                .into(),
        )
    }
}
