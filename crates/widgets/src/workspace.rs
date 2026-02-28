use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{button, text},
    Alignment, Element,
};

/// Displays Hyprland workspaces as clickable buttons.
///
/// Appearance is controlled by two theme flags:
///
/// | `workspace_dots` | `workspace_show_all` | Result |
/// |---|---|---|
/// | false | true  | `1  2  3`  — all workspaces as numbers (default) |
/// | true  | true  | `●  ○  ○`  — all workspaces as filled/empty dots |
/// | false | false | `2`        — active workspace number only |
/// | true  | false | `●`        — single filled dot |
#[derive(Debug, Default)]
pub struct WorkspaceWidget;

impl WorkspaceWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        if !theme.workspace_show_all {
            return self.view_active_only(state, theme);
        }
        self.view_all(state, theme)
    }

    /// Show every open workspace.
    fn view_all<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let mut workspaces = state.workspaces.clone();
        workspaces.sort_by_key(|w| w.id);

        let items: Vec<Element<'a, Message>> = workspaces
            .into_iter()
            .map(|ws| {
                let is_active = ws.id == state.active_workspace;
                let id = ws.id;

                let (label, color) = if theme.workspace_dots {
                    let dot = if is_active { "●" } else { "○" };
                    let color = if is_active {
                        theme.accent.to_iced()
                    } else {
                        theme.foreground.with_alpha(0.45).to_iced()
                    };
                    (dot.to_string(), color)
                } else {
                    let color = if is_active {
                        theme.accent.to_iced()
                    } else {
                        theme.foreground.with_alpha(0.6).to_iced()
                    };
                    (ws.name.clone(), color)
                };

                button(text(label).size(theme.font_size).color(color))
                    .on_press(Message::WorkspaceSwitchRequested(id))
                    .padding(0)
                    .style(button::text)
                    .into()
            })
            .collect();

        if items.is_empty() {
            // Fallback: Hyprland hasn't sent workspace info yet
            let fallback = if theme.workspace_dots { "●" } else { "1" };
            return text(fallback)
                .size(theme.font_size)
                .color(theme.accent.to_iced())
                .into();
        }

        iced::widget::Row::from_vec(items)
            .spacing(theme.gap as f32)
            .align_y(Alignment::Center)
            .into()
    }

    /// Show only the active workspace (no click target needed).
    fn view_active_only<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let label = if theme.workspace_dots {
            "●".to_string()
        } else {
            // Try to find the workspace name; fall back to ID
            state.workspaces
                .iter()
                .find(|w| w.id == state.active_workspace)
                .map(|w| w.name.clone())
                .unwrap_or_else(|| state.active_workspace.to_string())
        };

        text(label)
            .size(theme.font_size)
            .color(theme.accent.to_iced())
            .into()
    }
}
