use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::text, Element};

/// Displays the active Hyprland keyboard layout.
///
/// Updated via the `activelayout` IPC event.
/// Hidden until the first layout event is received.
#[derive(Debug, Default)]
pub struct KeyboardWidget;

impl KeyboardWidget {
    pub fn new() -> Self {
        Self
    }

    /// Returns `None` until a keyboard layout event has been received.
    pub fn view<'a>(
        &'a self,
        state: &'a AppState,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        if state.keyboard_layout.is_empty() {
            return None;
        }
        Some(
            text(format!("󰌌 {}", state.keyboard_layout))
                .size(theme.font_size)
                .into(),
        )
    }
}
