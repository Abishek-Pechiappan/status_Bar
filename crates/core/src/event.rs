use crate::state::{SystemSnapshot, WorkspaceInfo};

/// All messages (events) that can flow through the application event bus.
///
/// Sources:
/// - Hyprland IPC socket   → `Workspace*`, `ActiveWindow*`, `Fullscreen*`
/// - System monitor task   → `SystemSnapshot`
/// - Config watcher task   → `ConfigReloaded`
/// - Timer subscription    → `Tick`
#[derive(Debug, Clone)]
pub enum Message {
    // ── Hyprland IPC ──────────────────────────────────────────────────────────
    /// Active workspace changed (carries new workspace ID).
    WorkspaceChanged(u32),
    /// Full workspace list refreshed.
    WorkspaceListUpdated(Vec<WorkspaceInfo>),
    /// Focused window title changed (None = no window focused).
    ActiveWindowChanged(Option<String>),
    /// Fullscreen state toggled.
    FullscreenStateChanged(bool),

    // ── System monitor ────────────────────────────────────────────────────────
    /// Fresh system resource snapshot from the background monitor task.
    SystemSnapshot(SystemSnapshot),

    // ── Config ────────────────────────────────────────────────────────────────
    /// Config file changed on disk — triggers a live reload.
    ConfigReloaded,

    // ── Internal ──────────────────────────────────────────────────────────────
    /// One-second timer tick — used to update the clock.
    Tick,
    /// Graceful shutdown requested.
    Shutdown,
}
