use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{mouse_area, text},
    Element,
};

/// Displays the current media player track via playerctl.
///
/// Click to play/pause.  Hidden when no player is active.
#[derive(Debug, Default)]
pub struct MediaWidget;

impl MediaWidget {
    pub fn new() -> Self {
        Self
    }

    /// Returns `None` when no media player is running.
    pub fn view<'a>(
        &'a self,
        state: &'a AppState,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        let title = state.system.media_title.as_deref()?;

        let icon = if state.system.media_playing { "▶" } else { "⏸" };

        let label = match state.system.media_artist.as_deref() {
            Some(artist) if !artist.is_empty() => {
                // Truncate combined string at 40 chars to keep the bar tidy
                let combined = format!("{artist} - {title}");
                if combined.chars().count() > 40 {
                    let truncated: String = combined.chars().take(38).collect();
                    format!("{icon} {truncated}…")
                } else {
                    format!("{icon} {combined}")
                }
            }
            _ => {
                let t: String = title.chars().take(38).collect();
                if title.chars().count() > 38 {
                    format!("{icon} {t}…")
                } else {
                    format!("{icon} {t}")
                }
            }
        };

        Some(
            mouse_area(text(label).size(theme.font_size))
                .on_press(Message::MediaPlayPause)
                .into(),
        )
    }
}
