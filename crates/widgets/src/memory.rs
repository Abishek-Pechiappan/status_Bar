use bar_core::{event::Message, state::AppState};
use bar_system::memory::format_bytes;
use bar_theme::Theme;
use iced::{widget::text, Element};

/// Displays RAM usage as `used / total  (X%)`.
#[derive(Debug, Default)]
pub struct MemoryWidget;

impl MemoryWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let used    = format_bytes(state.system.ram_used);
        let total   = format_bytes(state.system.ram_total);
        let percent = (state.system.ram_fraction() * 100.0) as u8;
        let icon    = if theme.use_nerd_icons { "" } else { "RAM" };

        text(format!("{icon} {used}/{total}  {percent}%"))
            .size(theme.font_size)
            .color(theme.foreground.to_iced())
            .into()
    }
}
