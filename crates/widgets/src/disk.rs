use bar_core::{event::Message, state::AppState};
use bar_system::memory::format_bytes;
use bar_theme::Theme;
use iced::{
    widget::{row, text},
    Alignment, Element,
};

/// Displays root filesystem disk usage.
///
/// Returns `None` when disk info is unavailable — callers should skip rendering.
#[derive(Debug, Default)]
pub struct DiskWidget;

impl DiskWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(
        &'a self,
        state: &'a AppState,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        if state.system.disk_total == 0 {
            return None;
        }

        let used  = format_bytes(state.system.disk_used);
        let total = format_bytes(state.system.disk_total);
        let pct   = state.system.disk_fraction() * 100.0;

        Some(
            row![text(format!("󰋊 {used}/{total} ({pct:.0}%)")).size(theme.font_size)]
                .align_y(Alignment::Center)
                .into(),
        )
    }
}
