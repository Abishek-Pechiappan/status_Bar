use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::text, Element};

/// Displays the output of a user-configured shell command.
///
/// The command is set via `custom_command` in `[global]` of `bar.toml`.
/// Hidden when the command is empty or produces no output.
#[derive(Debug, Default)]
pub struct CustomWidget;

impl CustomWidget {
    pub fn new() -> Self {
        Self
    }

    /// Returns `None` when no custom command is configured or it produced no output.
    pub fn view<'a>(
        &'a self,
        state: &'a AppState,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        let output = &state.system.custom_output;
        if output.is_empty() {
            return None;
        }
        Some(text(output.as_str()).size(theme.font_size).into())
    }
}
