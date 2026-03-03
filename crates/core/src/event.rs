use crate::state::{ClientInfo, SystemSnapshot, WorkspaceInfo};

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
    /// Drag volume slider to an exact level (0.0 = mute, 1.0 = 100%).
    VolumeSet(f32),
    /// Scroll on brightness widget — positive = brighter, negative = dimmer (% steps).
    BrightnessAdjust(i32),
    /// Drag brightness slider to an exact percentage (0–100).
    BrightnessSet(f32),
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

    // ── Notifications ─────────────────────────────────────────────────────────
    /// A new notification arrived via D-Bus `org.freedesktop.Notifications`.
    NotificationReceived {
        id: u32,
        app_name: String,
        summary: String,
        body: String,
    },
    /// A notification was closed by the sender application.
    NotificationClosed(u32),
    /// User toggled the notification panel open/closed.
    NotifyPanelToggle,
    /// User dismissed a single notification entry from the panel.
    NotifyDismiss(u32),
    /// User pressed "Clear all" in the notification panel.
    NotifyClearAll,

    // ── Power menu ───────────────────────────────────────────────────────────
    /// User clicked the power widget — spawn `bar-powermenu`.
    PowerMenuOpen,

    // ── Tray / window list ────────────────────────────────────────────────────
    /// Fresh list of all open clients from `hyprctl clients -j`.
    ClientsUpdated(Vec<ClientInfo>),
    /// User clicked a tray entry — focus the window at the given address.
    WindowFocusRequested(String),

    // ── Hyprland IPC (extended) ───────────────────────────────────────────────
    /// Hyprland submap changed (`None` = back to default binds).
    SubMapChanged(Option<String>),
    /// Screen-share / recording state changed.
    ScreencastChanged(bool),

    // ── UI state ──────────────────────────────────────────────────────────────
    /// User toggled Do-Not-Disturb mode.
    DndToggle,
    /// User clicked the clock widget — toggle the calendar popup panel.
    CalendarToggle,

    // ── Package updates ───────────────────────────────────────────────────────
    /// Package update count refreshed (`None` = `checkupdates` unavailable).
    UpdateCountRefreshed(Option<u32>),

    // ── Power panel ───────────────────────────────────────────────────────────
    /// User clicked the power widget — open/close the power panel
    /// (style depends on `power_menu_style` config; overlay mode spawns process instead).
    PowerPanelToggle,
    /// User selected a power action from the panel.
    /// Payload is the action key: `"lock"`, `"sleep"`, `"hibernate"`, `"logout"`, `"reboot"`, `"shutdown"`.
    PowerActionTriggered(String),
    /// Cursor entered a power action button (index 0–5) — start hover animation.
    PowerHoverEnter(usize),
    /// Cursor left a power action button (index 0–5) — reverse hover animation.
    PowerHoverExit(usize),
    /// 60 fps animation tick — active only while a power panel animation is in progress.
    PowerAnimFrame,

    // ── Auto-hide ─────────────────────────────────────────────────────────────
    /// Cursor entered the bar surface — cancel any pending hide timer.
    BarMouseEnter,
    /// Cursor left the bar surface — start the hide countdown.
    BarMouseLeave,
    /// Internal 200 ms timer tick used to check the auto-hide countdown.
    AutoHideTick,

    // ── Internal ──────────────────────────────────────────────────────────────
    /// One-second timer tick — used to update the clock.
    Tick,
    /// Graceful shutdown requested.
    Shutdown,
}
