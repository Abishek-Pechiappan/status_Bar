use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    mouse::ScrollDelta,
    widget::{mouse_area, text},
    Element,
};

/// Displays the active Hyprland keyboard layout.
///
/// Interactive: scroll to cycle through available layouts.
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
        let icon = if theme.use_nerd_icons { "󰌌" } else { "KB" };
        let content = text(format!("{icon} {}", state.keyboard_layout)).size(theme.font_size);

        Some(
            mouse_area(content)
                .on_scroll(|delta| {
                    let y = match delta {
                        ScrollDelta::Lines { y, .. } | ScrollDelta::Pixels { y, .. } => y,
                    };
                    if y > 0.0 { Message::KeyboardLayoutNext } else { Message::KeyboardLayoutPrev }
                })
                .into(),
        )
    }
}
