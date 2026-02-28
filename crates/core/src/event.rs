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

    // ── Hyprland IPC (continued) ──────────────────────────────────────────────
    /// Active keyboard layout changed (from Hyprland `activelayout` event).
    KeyboardLayoutChanged(String),

    // ── User actions ──────────────────────────────────────────────────────────
    /// User clicked a workspace button — request Hyprland to switch.
    WorkspaceSwitchRequested(u32),
    /// Scroll on volume widget — positive = louder, negative = quieter (% steps).
    VolumeAdjust(i32),
    /// Click on volume widget — toggle mute.
    VolumeMuteToggle,
    /// Scroll on brightness widget — positive = brighter, negative = dimmer (% steps).
    BrightnessAdjust(i32),
    /// Click on media widget — play/pause.
    MediaPlayPause,
    /// Scroll up on media widget — skip to next track.
    MediaNext,
    /// Scroll down on media widget — go to previous track.
    MediaPrev,
    /// Scroll up on keyboard widget — switch to next layout.
    KeyboardLayoutNext,
    /// Scroll down on keyboard widget — switch to previous layout.
    KeyboardLayoutPrev,

    // ── Internal ──────────────────────────────────────────────────────────────
    /// One-second timer tick — used to update the clock.
    Tick,
    /// Graceful shutdown requested.
    Shutdown,
}
