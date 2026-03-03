use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::{row, text}, Alignment, Element};

/// Shows connected Bluetooth device name.  Hidden when no device is connected.
#[derive(Debug, Default)]
pub struct BluetoothWidget;

impl BluetoothWidget {
    pub fn new() -> Self { Self }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Option<Element<'a, Message>> {
        if !state.system.bt_connected { return None; }
        let icon = if theme.use_nerd_icons { "󰂱" } else { "BT" };
        let fg   = theme.foreground.to_iced();

        let label = match state.system.bt_device_name.as_deref() {
            Some(name) if !name.is_empty() => {
                let n: String = name.chars().take(16).collect();
                format!("{icon} {n}")
            }
            _ => icon.to_string(),
        };

        Some(
            row![text(label).size(theme.font_size).color(fg)]
                .align_y(Alignment::Center)
                .into(),
        )
    }
}
