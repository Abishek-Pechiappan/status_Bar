use bar_core::{event::Message, state::AppState};
use bar_system::memory::format_bytes;
use bar_theme::Theme;
use iced::{widget::text, Element};

/// Displays swap (virtual memory) usage.
///
/// Hidden when the system has no swap configured.
#[derive(Debug, Default)]
pub struct SwapWidget;

impl SwapWidget {
    pub fn new() -> Self {
        Self
    }

    /// Returns `None` when swap is unavailable / not configured.
    pub fn view<'a>(
        &'a self,
        state: &'a AppState,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        if state.system.swap_total == 0 {
            return None;
        }
        let used  = format_bytes(state.system.swap_used);
        let total = format_bytes(state.system.swap_total);
        Some(text(format!("󰓡 {used}/{total}")).size(theme.font_size).into())
    }
}
