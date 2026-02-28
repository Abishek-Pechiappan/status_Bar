use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    mouse::ScrollDelta,
    widget::{mouse_area, text},
    Element,
};

/// Displays screen brightness as a percentage.
///
/// Interactive: scroll wheel adjusts brightness ±5% via `brightnessctl`.
/// Returns `None` when no backlight device is found.
#[derive(Debug, Default)]
pub struct BrightnessWidget;

impl BrightnessWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(
        &'a self,
        state: &'a AppState,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        let pct = state.system.brightness?;
        let content = text(format!("󰃞 {pct}%")).size(theme.font_size);

        Some(
            mouse_area(content)
                .on_scroll(|delta| {
                    let step = match delta {
                        ScrollDelta::Lines { y, .. } | ScrollDelta::Pixels { y, .. } => {
                            if y > 0.0 { 5 } else { -5 }
                        }
                    };
                    Message::BrightnessAdjust(step)
                })
                .into(),
        )
    }
}
