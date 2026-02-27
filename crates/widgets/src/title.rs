use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::text, Element};

/// Maximum number of characters shown before truncating with `…`.
const MAX_CHARS: usize = 60;

/// Displays the currently focused window's title.
///
/// Shows a dimmed placeholder when no window is focused.
#[derive(Debug, Default)]
pub struct TitleWidget;

impl TitleWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        match &state.active_window {
            Some(title) => {
                let display = if title.chars().count() > MAX_CHARS {
                    let truncated: String = title.chars().take(MAX_CHARS).collect();
                    format!("{truncated}…")
                } else {
                    title.clone()
                };

                text(display)
                    .size(theme.font_size)
                    .color(theme.foreground.to_iced())
                    .into()
            }
            None => text("Desktop")
                .size(theme.font_size)
                .color(theme.foreground.with_alpha(0.4).to_iced())
                .into(),
        }
    }
}
