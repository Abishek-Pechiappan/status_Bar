use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    mouse::ScrollDelta,
    widget::{mouse_area, row, slider, text},
    Alignment, Element, Length,
};

/// Displays screen brightness as an interactive slider.
///
/// - Drag slider to set exact brightness.
/// - Scroll wheel adjusts ±5% via `brightnessctl`.
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
        let icon = if theme.use_nerd_icons { "󰃞" } else { "BRT" };

        let icon_el = mouse_area(
            text(icon).size(theme.font_size),
        )
        .on_scroll(|delta| {
            let step = match delta {
                ScrollDelta::Lines { y, .. } | ScrollDelta::Pixels { y, .. } => {
                    if y > 0.0 { 5 } else { -5 }
                }
            };
            Message::BrightnessAdjust(step)
        });

        if theme.brightness_show_slider {
            let brt_slider = slider(0.0f32..=100.0, pct as f32, Message::BrightnessSet)
                .step(1.0f32)
                .width(Length::Fixed(72.0));

            Some(
                row![
                    icon_el,
                    brt_slider,
                    text(format!("{pct}%")).size(theme.font_size - 1.0).width(Length::Fixed(32.0)),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .into(),
            )
        } else {
            Some(
                row![
                    icon_el,
                    text(format!("{pct}%")).size(theme.font_size - 1.0),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .into(),
            )
        }
    }
}
