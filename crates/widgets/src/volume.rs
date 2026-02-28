use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Element,
};

/// Displays the default audio sink volume.
///
/// Shows a mute icon when muted.  Returns `None` when wpctl is unavailable.
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

        Some(
            row![text(label).size(theme.font_size)]
                .align_y(Alignment::Center)
                .into(),
        )
    }
}
