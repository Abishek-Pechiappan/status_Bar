use serde::Deserialize;

/// All events emitted by the Hyprland IPC event socket (`socket2.sock`).
#[derive(Debug, Clone)]
pub enum HyprlandEvent {
    Workspace(WorkspaceEvent),
    ActiveWindow(ActiveWindowEvent),
    Fullscreen(bool),
    MonitorFocused(String),
    /// Active keyboard layout changed.  Carries the layout name string.
    ActiveLayout(String),
    /// An event we don't handle yet — carries the raw line for debugging.
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct WorkspaceEvent {
    pub id:   u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ActiveWindowEvent {
    pub class: String,
    pub title: String,
}

/// JSON shape returned by `hyprctl workspaces -j`.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceInfo {
    pub id:      i32,
    pub name:    String,
    pub monitor: String,
    pub windows: u32,
}

/// Parse a raw IPC event line into a typed [`HyprlandEvent`].
///
/// Hyprland events have the format `event_name>>event_data`.
pub fn parse_event(line: &str) -> HyprlandEvent {
    let Some((event, data)) = line.split_once(">>") else {
        return HyprlandEvent::Unknown(line.to_string());
    };

    match event {
        "workspace" | "workspacev2" => {
            // workspacev2 format: "id,name"
            let (id_str, name) = data
                .split_once(',')
                .unwrap_or((data, data));
            let id = id_str.trim().parse::<u32>().unwrap_or(0);
            HyprlandEvent::Workspace(WorkspaceEvent {
                id,
                name: name.trim().to_string(),
            })
        }
        "activewindow" | "activewindowv2" => {
            let mut parts = data.splitn(2, ',');
            let class = parts.next().unwrap_or("").trim().to_string();
            let title = parts.next().unwrap_or("").trim().to_string();
            HyprlandEvent::ActiveWindow(ActiveWindowEvent { class, title })
        }
        "fullscreen" => HyprlandEvent::Fullscreen(data.trim() == "1"),
        "monitoradded" | "monitorfocused" => {
            HyprlandEvent::MonitorFocused(data.trim().to_string())
        }
        "activelayout" => {
            // Format: "keyboard-name,layout-name"
            let layout = data.split_once(',')
                .map(|(_, l)| l.trim().to_string())
                .unwrap_or_else(|| data.trim().to_string());
            HyprlandEvent::ActiveLayout(layout)
        }
        _ => HyprlandEvent::Unknown(line.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_workspacev2_event() {
        let event = parse_event("workspacev2>>3,coding");
        assert!(matches!(
            event,
            HyprlandEvent::Workspace(WorkspaceEvent { id: 3, .. })
        ));
    }

    #[test]
    fn parse_active_window() {
        let event = parse_event("activewindow>>kitty,~/projects");
        if let HyprlandEvent::ActiveWindow(e) = event {
            assert_eq!(e.class, "kitty");
            assert_eq!(e.title, "~/projects");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_unknown_event() {
        let event = parse_event("somefutureevent>>data");
        assert!(matches!(event, HyprlandEvent::Unknown(_)));
    }
}
