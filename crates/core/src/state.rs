use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// Central application state — all widgets read from this snapshot.
#[derive(Debug, Clone)]
pub struct AppState {
    /// All known Hyprland workspaces on the active monitor.
    pub workspaces: Vec<WorkspaceInfo>,
    /// ID of the currently focused workspace.
    pub active_workspace: u32,
    /// Title of the currently focused window, if any.
    pub active_window: Option<String>,
    /// Whether any window is in fullscreen mode.
    pub is_fullscreen: bool,
    /// Latest system resource snapshot.
    pub system: SystemSnapshot,
    /// Current local time (updated every second).
    pub time: DateTime<Local>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            workspaces: Vec::new(),
            active_workspace: 1,
            active_window: None,
            is_fullscreen: false,
            system: SystemSnapshot::default(),
            time: Local::now(),
        }
    }
}

/// Information about a single Hyprland workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub id: u32,
    pub name: String,
    pub monitor: String,
    /// Number of windows currently in this workspace.
    pub windows: u32,
}

/// A point-in-time snapshot of system resource usage.
#[derive(Debug, Clone, Default)]
pub struct SystemSnapshot {
    /// Per-core CPU usage (0.0 – 100.0).
    pub cpu_per_core: Vec<f32>,
    /// Average CPU usage across all cores.
    pub cpu_average: f32,
    /// RAM used in bytes.
    pub ram_used: u64,
    /// Total RAM in bytes.
    pub ram_total: u64,
    /// Root filesystem: used bytes.
    pub disk_used: u64,
    /// Root filesystem: total bytes.
    pub disk_total: u64,
    /// Network receive rate in bytes/second.
    pub net_rx: u64,
    /// Network transmit rate in bytes/second.
    pub net_tx: u64,
    /// Battery charge level (0–100), `None` if no battery present.
    pub battery_percent: Option<u8>,
    /// `true` = charging / full, `false` = discharging, `None` = unknown.
    pub battery_charging: Option<bool>,
}

impl SystemSnapshot {
    /// RAM usage as a fraction in `[0, 1]`.
    #[must_use]
    pub fn ram_fraction(&self) -> f32 {
        if self.ram_total == 0 {
            return 0.0;
        }
        self.ram_used as f32 / self.ram_total as f32
    }

    /// Disk usage as a fraction in `[0, 1]`.
    #[must_use]
    pub fn disk_fraction(&self) -> f32 {
        if self.disk_total == 0 {
            return 0.0;
        }
        self.disk_used as f32 / self.disk_total as f32
    }
}
