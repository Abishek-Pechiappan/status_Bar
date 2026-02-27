use bar_core::{event::Message, state::AppState};
use bar_system::memory::format_bytes;
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Element,
};

/// Displays CPU usage (average) and RAM usage.
#[derive(Debug, Default)]
pub struct CpuWidget;

impl CpuWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let cpu_label = format!(" {:.0}%", state.system.cpu_average);
        let ram_used  = format_bytes(state.system.ram_used);
        let ram_total = format_bytes(state.system.ram_total);
        let ram_label = format!(" {ram_used}/{ram_total}");

        row![
            text(cpu_label).size(theme.font_size),
            text("  "),
            text(ram_label).size(theme.font_size),
        ]
        .spacing(theme.gap as f32)
        .align_y(Alignment::Center)
        .into()
    }
}
