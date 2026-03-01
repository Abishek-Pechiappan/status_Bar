use bar_core::{event::Message, state::AppState};
use bar_system::memory::format_bytes;
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Color, Element,
};

/// Displays RAM usage as `used / total  (X%)`.
///
/// Icon is rendered in the accent color.  The percentage turns amber above
/// 75 % and red above 90 % to give an at-a-glance pressure indicator.
#[derive(Debug, Default)]
pub struct MemoryWidget;

impl MemoryWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let used    = format_bytes(state.system.ram_used);
        let total   = format_bytes(state.system.ram_total);
        let pct     = (state.system.ram_fraction() * 100.0) as u8;
        let icon    = if theme.use_nerd_icons { "" } else { "RAM" };

        let pct_color = if pct >= 90 {
            Color::from_rgb8(0xf3, 0x8b, 0xa8) // red — critical
        } else if pct >= 75 {
            Color::from_rgb8(0xf9, 0xe2, 0xaf) // amber — warning
        } else {
            theme.foreground.with_alpha(0.75).to_iced()
        };

        let fg = theme.foreground.to_iced();
        let muted = theme.foreground.with_alpha(0.55).to_iced();

        row![
            text(icon).size(theme.font_size).color(theme.accent.to_iced()),
            text(format!("{used}")).size(theme.font_size).color(fg),
            text("/").size(theme.font_size - 1.0).color(muted),
            text(format!("{total}")).size(theme.font_size - 1.0).color(muted),
            text(format!("{pct}%")).size(theme.font_size - 1.0).color(pct_color),
        ]
        .spacing(3)
        .align_y(Alignment::Center)
        .into()
    }
}
