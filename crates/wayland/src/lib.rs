//! Wayland layer-shell surface for `bar`.
//!
//! Owns the Iced application loop and wires together all background tasks:
//! - Hyprland IPC event stream (workspaces, active window, fullscreen, keyboard layout)
//! - System resource monitor (CPU, RAM, disk, media, etc.)
//! - Config file watcher (live reload on change)
//! - D-Bus notification daemon (`org.freedesktop.Notifications`)
//! - 1-second timer (clock)

use bar_config::{default_path, load as load_config, BarConfig, ConfigWatcher, Position};
use bar_core::{
    event::Message as AppMessage,
    state::{AppState, ClientInfo, NotifEntry, WorkspaceInfo},
};
use bar_ipc::{fetch_active_window, fetch_workspaces, HyprlandEvent, HyprlandIpc};
use bar_theme::{Color as ThemeColor, Theme};
use bar_widgets::{
    BatteryWidget, BluetoothWidget, BrightnessWidget, ClockWidget, CpuWidget, CustomWidget,
    DiskWidget, GpuWidget, KeyboardWidget, LoadWidget, MediaWidget, MemoryWidget, NetworkWidget,
    NotifyWidget, PowerWidget, ScreencastWidget, SeparatorWidget, SubmapWidget, SwapWidget,
    TempWidget, TitleWidget, TrayWidget, UpdatesWidget, UptimeWidget, VolumeWidget,
    WorkspaceWidget,
};
use chrono::Local;
use futures::channel::mpsc::Sender;
use iced::{
    animation::{Animation, Easing},
    widget::{column, container, row},
    Element, Length, Subscription, Task,
};
use iced_layershell::{
    build_pattern::application,
    reexport::{Anchor, Layer},
    settings::{LayerShellSettings, Settings},
    to_layer_message,
};
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{error, info, warn};

/// Fallback system monitor poll interval when not set in config (milliseconds).
const DEFAULT_SYSTEM_INTERVAL_MS: u64 = 2_000;

/// Height of the notification panel that drops below the bar (pixels).
const NOTIFY_PANEL_HEIGHT: u32 = 300;
/// Height of the calendar panel that drops below the bar (pixels).
const CALENDAR_PANEL_HEIGHT: u32 = 250;
/// Height of the power panel that drops below the bar (pixels).
const POWER_PANEL_HEIGHT: u32 = 120;
/// Maximum number of power action buttons (lock/sleep/hibernate/logout/reboot/shutdown).
const MAX_POWER_ACTIONS: usize = 6;

/// Custom shell command set once from config at startup.
static CUSTOM_CMD: OnceLock<String> = OnceLock::new();
/// System poll interval in ms, set once from config.
static SYSTEM_INTERVAL_MS: OnceLock<u64> = OnceLock::new();

// ── Entry point ───────────────────────────────────────────────────────────────

/// Start the Wayland bar.  Never returns under normal operation.
pub fn run() -> iced_layershell::Result {
    let config      = load_config(default_path()).unwrap_or_default();
    let height      = config.global.height;
    let anchor      = position_to_anchor(config.global.position);
    let margin_side = config.global.margin as i32;
    let margin_edge = config.global.margin_top as i32;
    let (mt, mb)    = match config.global.position {
        Position::Top    => (margin_edge, 0),
        Position::Bottom => (0, margin_edge),
    };
    // Auto-hide bars never reserve compositor space — they collapse to 1 px.
    let exclusive_zone = if config.global.auto_hide {
        0
    } else if config.global.exclusive_zone {
        (height + config.global.margin_top) as i32
    } else {
        0
    };

    // Enable Hyprland backdrop blur for this layer surface (1-line change).
    let _ = std::process::Command::new("hyprctl")
        .args(["keyword", "layerrule", "blur,bar"])
        .output();

    let _ = CUSTOM_CMD.set(config.global.custom_command.clone());
    let interval_ms = (config.global.system_poll_secs as u64).max(1) * 1_000;
    let _ = SYSTEM_INTERVAL_MS.set(interval_ms);

    // Build a default_font from the configured font family so iced
    // actually uses the right typeface for all text widgets.
    // Leak the font name into a 'static str so iced::font::Family::Name can hold it.
    // This is a one-time allocation at startup — acceptable for a status bar.
    let font_name: &'static str = Box::leak(config.theme.font.clone().into_boxed_str());
    let default_font = iced::Font {
        family: iced::font::Family::Name(font_name),
        weight: iced::font::Weight::Normal,
        stretch: iced::font::Stretch::Normal,
        style:  iced::font::Style::Normal,
    };

    application(Bar::new, Bar::namespace, Bar::update, Bar::view)
        .subscription(Bar::subscription)
        .style(Bar::style)
        .settings(Settings {
            default_font,
            layer_settings: LayerShellSettings {
                size:           Some((0, height)),
                exclusive_zone,
                anchor,
                layer:          Layer::Top,
                margin:         (mt, margin_side, mb, margin_side),
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}

// ── Message ───────────────────────────────────────────────────────────────────

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    App(AppMessage),
    Tick,
}

// ── State ─────────────────────────────────────────────────────────────────────

struct Bar {
    state:      AppState,
    config:     BarConfig,
    theme:      Theme,
    // Widgets — always constructed, shown/hidden based on config layout
    workspaces: WorkspaceWidget,
    title:      TitleWidget,
    clock:      ClockWidget,
    network:    NetworkWidget,
    cpu:        CpuWidget,
    memory:     MemoryWidget,
    disk:       DiskWidget,
    temp:       TempWidget,
    volume:     VolumeWidget,
    brightness: BrightnessWidget,
    battery:    BatteryWidget,
    swap:       SwapWidget,
    uptime:     UptimeWidget,
    load:       LoadWidget,
    keyboard:   KeyboardWidget,
    media:      MediaWidget,
    custom:     CustomWidget,
    separator:  SeparatorWidget,
    notify:     NotifyWidget,
    tray:       TrayWidget,
    power:      PowerWidget,
    submap:     SubmapWidget,
    screencast: ScreencastWidget,
    gpu:        GpuWidget,
    bluetooth:  BluetoothWidget,
    updates:    UpdatesWidget,
    // EMA-smoothed system metrics
    ema_cpu:     f32,
    ema_net_rx:  f32,
    ema_net_tx:  f32,
    // Rolling histories for sparklines (newest at back, max 20)
    cpu_history:    std::collections::VecDeque<f32>,
    net_rx_history: std::collections::VecDeque<f32>,
    net_tx_history: std::collections::VecDeque<f32>,
    // Whether the calendar panel is currently open
    calendar_open: bool,
    // Month offset from current (0 = today's month, -1 = prev, +1 = next)
    calendar_month_offset: i32,
    // Notification card currently under the cursor (for hover highlight)
    hover_notif_id: Option<u32>,
    // ── Power panel ───────────────────────────────────────────────────────────
    /// Whether inline power mode is active (power buttons replace bar content).
    power_inline_open: bool,
    /// Smooth animation driving the power panel open/close transition.
    power_anim: Animation<bool>,
    /// Per-button hover animations (one per power action slot, max 6).
    power_hover_anim: [Animation<bool>; MAX_POWER_ACTIONS],
    // ── Auto-hide ─────────────────────────────────────────────────────────────
    /// `true` = bar at full height; `false` = collapsed to 1 px strip.
    bar_visible: bool,
    /// When the cursor left the bar — used to drive the hide countdown.
    hide_after: Option<std::time::Instant>,
}

impl Bar {
    fn new() -> (Self, Task<Message>) {
        let config = load_config(default_path()).unwrap_or_default();
        let theme  = Theme::from_config(&config.theme);

        let bar = Self {
            state:      AppState::default(),
            config,
            theme,
            workspaces: WorkspaceWidget::new(),
            title:      TitleWidget::new(),
            clock:      ClockWidget::new(),
            network:    NetworkWidget::new(),
            cpu:        CpuWidget::new(),
            memory:     MemoryWidget::new(),
            disk:       DiskWidget::new(),
            temp:       TempWidget::new(),
            volume:     VolumeWidget::new(),
            brightness: BrightnessWidget::new(),
            battery:    BatteryWidget::new(),
            swap:       SwapWidget::new(),
            uptime:     UptimeWidget::new(),
            load:       LoadWidget::new(),
            keyboard:   KeyboardWidget::new(),
            media:      MediaWidget::new(),
            custom:     CustomWidget::new(),
            separator:  SeparatorWidget::new(),
            notify:     NotifyWidget::new(),
            tray:       TrayWidget::new(),
            power:      PowerWidget::new(),
            submap:     SubmapWidget::new(),
            screencast: ScreencastWidget::new(),
            gpu:        GpuWidget::new(),
            bluetooth:  BluetoothWidget::new(),
            updates:    UpdatesWidget::new(),
            ema_cpu:     0.0,
            ema_net_rx:  0.0,
            ema_net_tx:  0.0,
            cpu_history:    std::collections::VecDeque::with_capacity(20),
            net_rx_history: std::collections::VecDeque::with_capacity(20),
            net_tx_history: std::collections::VecDeque::with_capacity(20),
            calendar_open:          false,
            calendar_month_offset:  0,
            hover_notif_id:         None,
            power_inline_open: false,
            power_anim:        Animation::new(false).slow().easing(Easing::EaseOutCubic),
            power_hover_anim:  std::array::from_fn(|_| {
                Animation::new(false).very_quick().easing(Easing::EaseOutQuad)
            }),
            bar_visible:  true,
            hide_after:   None,
        };

        let init_task = Task::perform(
            async {
                let ipc = HyprlandIpc::new()?;
                let raw = fetch_workspaces(&ipc).await?;
                let workspaces = raw.into_iter().map(ipc_to_core_workspace).collect();
                Ok::<Vec<WorkspaceInfo>, bar_core::BarError>(workspaces)
            },
            |result| match result {
                Ok(ws)  => Message::App(AppMessage::WorkspaceListUpdated(ws)),
                Err(e)  => {
                    warn!("Initial workspace fetch failed: {e}");
                    Message::Tick
                }
            },
        );

        (bar, init_task)
    }

    fn namespace() -> String {
        String::from("bar")
    }

    // ── Update ────────────────────────────────────────────────────────────────

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                self.state.time = Local::now();
                Task::none()
            }
            Message::App(msg) => self.handle_app(msg),
            _ => Task::none(),
        }
    }

    fn handle_app(&mut self, msg: AppMessage) -> Task<Message> {
        match msg {
            // ── IPC events ────────────────────────────────────────────────────
            AppMessage::WorkspaceChanged(id) => {
                self.state.active_workspace = id;
            }
            AppMessage::WorkspaceListUpdated(workspaces) => {
                self.state.workspaces = workspaces;
            }
            AppMessage::ActiveWindowChanged(title) => {
                self.state.active_window = title;
            }
            AppMessage::FullscreenStateChanged(fs) => {
                self.state.is_fullscreen = fs;
            }
            AppMessage::KeyboardLayoutChanged(layout) => {
                self.state.keyboard_layout = layout;
            }

            // ── System monitor ────────────────────────────────────────────────
            AppMessage::SystemSnapshot(mut snapshot) => {
                const ALPHA:   f32   = 0.35;
                const HISTORY: usize = 20;

                self.ema_cpu    = ALPHA * snapshot.cpu_average   + (1.0 - ALPHA) * self.ema_cpu;
                self.ema_net_rx = ALPHA * snapshot.net_rx as f32 + (1.0 - ALPHA) * self.ema_net_rx;
                self.ema_net_tx = ALPHA * snapshot.net_tx as f32 + (1.0 - ALPHA) * self.ema_net_tx;

                snapshot.cpu_average = self.ema_cpu;
                snapshot.net_rx      = self.ema_net_rx as u64;
                snapshot.net_tx      = self.ema_net_tx as u64;

                if self.cpu_history.len() >= HISTORY { self.cpu_history.pop_front(); }
                self.cpu_history.push_back(self.ema_cpu);

                // Scale net rates for sparkline (100 MB/s = 100%)
                const NET_MAX: f32 = 100_000_000.0;
                let rx_pct = (self.ema_net_rx / NET_MAX * 100.0).min(100.0);
                let tx_pct = (self.ema_net_tx / NET_MAX * 100.0).min(100.0);
                if self.net_rx_history.len() >= HISTORY { self.net_rx_history.pop_front(); }
                if self.net_tx_history.len() >= HISTORY { self.net_tx_history.pop_front(); }
                self.net_rx_history.push_back(rx_pct);
                self.net_tx_history.push_back(tx_pct);

                self.state.system = snapshot;
            }

            // ── Config live-reload ────────────────────────────────────────────
            AppMessage::ConfigReloaded => {
                match load_config(default_path()) {
                    Ok(cfg) => {
                        info!("Config reloaded");
                        self.theme  = Theme::from_config(&cfg.theme);
                        self.config = cfg;
                    }
                    Err(e) => warn!("Config reload failed: {e}"),
                }
            }

            // ── Notifications ─────────────────────────────────────────────────
            AppMessage::NotificationReceived { id, app_name, summary, body } => {
                if !self.state.dnd_enabled {
                    // Replace an existing entry with the same id (replaces_id flow).
                    self.state.notifications.retain(|n| n.id != id);
                    self.state.notifications.push(NotifEntry { id, app_name, summary, body });
                    // Cap history at 50 entries (drop oldest).
                    if self.state.notifications.len() > 50 {
                        self.state.notifications.remove(0);
                    }
                }
            }
            AppMessage::NotificationClosed(id) => {
                self.state.notifications.retain(|n| n.id != id);
                return self.maybe_close_panel();
            }
            AppMessage::NotifyPanelToggle => {
                if self.calendar_open { self.calendar_open = false; }
                self.state.notify_panel_open = !self.state.notify_panel_open;
                return self.sync_surface_size();
            }
            AppMessage::NotifyDismiss(id) => {
                self.state.notifications.retain(|n| n.id != id);
                return self.maybe_close_panel();
            }
            AppMessage::NotifyClearAll => {
                self.state.notifications.clear();
                if self.state.notify_panel_open {
                    self.state.notify_panel_open = false;
                    return self.sync_surface_size();
                }
            }

            // ── User interactions ─────────────────────────────────────────────
            AppMessage::WorkspaceSwitchRequested(id) => {
                return Task::perform(
                    async move {
                        let _ = tokio::process::Command::new("hyprctl")
                            .args(["dispatch", "workspace", &id.to_string()])
                            .output()
                            .await;
                    },
                    |_| Message::Tick,
                );
            }
            AppMessage::VolumeAdjust(delta) => {
                // Optimistic update: reflect change immediately without waiting for wpctl.
                if let Some(vol) = self.state.system.volume {
                    let step = delta as f32 / 100.0;
                    self.state.system.volume = Some((vol + step).clamp(0.0, 1.5));
                }
                let arg = if delta >= 0 {
                    format!("{delta}%+")
                } else {
                    format!("{}%-", delta.unsigned_abs())
                };
                tokio::spawn(async move {
                    let _ = tokio::process::Command::new("wpctl")
                        .args(["set-volume", "-l", "1.5", "@DEFAULT_AUDIO_SINK@", &arg])
                        .output()
                        .await;
                });
            }
            AppMessage::VolumeMuteToggle => {
                // Optimistic update: flip mute state immediately.
                self.state.system.volume_muted = !self.state.system.volume_muted;
                tokio::spawn(async {
                    let _ = tokio::process::Command::new("wpctl")
                        .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
                        .output()
                        .await;
                });
            }
            AppMessage::VolumeSet(val) => {
                // Optimistic update.
                self.state.system.volume = Some(val.clamp(0.0, 1.5));
                tokio::spawn(async move {
                    let _ = tokio::process::Command::new("wpctl")
                        .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{val:.2}")])
                        .output()
                        .await;
                });
            }
            AppMessage::BrightnessSet(pct) => {
                let clamped = (pct.round() as u8).clamp(1, 100);
                // Optimistic update.
                self.state.system.brightness = Some(clamped);
                let pct_u32 = clamped as u32;
                tokio::spawn(async move {
                    let _ = tokio::process::Command::new("brightnessctl")
                        .args(["set", &format!("{pct_u32}%")])
                        .output()
                        .await;
                });
            }
            AppMessage::BrightnessAdjust(delta) => {
                // Optimistic update.
                if let Some(b) = self.state.system.brightness {
                    let new = (b as i32 + delta).clamp(1, 100) as u8;
                    self.state.system.brightness = Some(new);
                }
                let arg = if delta >= 0 {
                    format!("{delta}%+")
                } else {
                    format!("{}%-", delta.unsigned_abs())
                };
                tokio::spawn(async move {
                    let _ = tokio::process::Command::new("brightnessctl")
                        .args(["set", &arg])
                        .output()
                        .await;
                });
            }
            AppMessage::MediaPlayPause => {
                return Task::perform(
                    async {
                        let _ = tokio::process::Command::new("playerctl")
                            .arg("play-pause")
                            .output()
                            .await;
                    },
                    |_| Message::Tick,
                );
            }
            AppMessage::MediaNext => {
                return Task::perform(
                    async {
                        let _ = tokio::process::Command::new("playerctl")
                            .arg("next")
                            .output()
                            .await;
                    },
                    |_| Message::Tick,
                );
            }
            AppMessage::MediaPrev => {
                return Task::perform(
                    async {
                        let _ = tokio::process::Command::new("playerctl")
                            .arg("previous")
                            .output()
                            .await;
                    },
                    |_| Message::Tick,
                );
            }

            AppMessage::KeyboardLayoutNext => {
                return Task::perform(
                    async {
                        let _ = tokio::process::Command::new("hyprctl")
                            .args(["switchxkblayout", "all", "next"])
                            .output()
                            .await;
                    },
                    |_| Message::Tick,
                );
            }
            AppMessage::KeyboardLayoutPrev => {
                return Task::perform(
                    async {
                        let _ = tokio::process::Command::new("hyprctl")
                            .args(["switchxkblayout", "all", "prev"])
                            .output()
                            .await;
                    },
                    |_| Message::Tick,
                );
            }

            // ── Power menu (legacy overlay — no longer sent by widget) ────────
            AppMessage::PowerMenuOpen => {
                tokio::spawn(async {
                    let _ = tokio::process::Command::new("bar-powermenu").spawn();
                });
            }

            // ── Power panel ───────────────────────────────────────────────────
            AppMessage::PowerPanelToggle => {
                let now = std::time::Instant::now();
                // Close other panels first.
                self.state.notify_panel_open = false;
                self.calendar_open = false;

                match self.config.global.power_menu_style.as_str() {
                    "overlay" => {
                        tokio::spawn(async {
                            let _ = tokio::process::Command::new("bar-powermenu").spawn();
                        });
                    }
                    "inline" => {
                        self.power_inline_open = !self.power_inline_open;
                        self.state.power_panel_open = self.power_inline_open;
                        if self.config.global.power_anim_style != "none" {
                            self.power_anim.go_mut(self.power_inline_open, now);
                        }
                    }
                    _ => {
                        // "dropdown" (default)
                        self.state.power_panel_open = !self.state.power_panel_open;
                        if self.config.global.power_anim_style != "none" {
                            self.power_anim.go_mut(self.state.power_panel_open, now);
                        }
                        return self.sync_surface_size();
                    }
                }
            }
            AppMessage::PowerActionTriggered(action) => {
                let was_open = self.state.power_panel_open;
                self.state.power_panel_open = false;
                self.power_inline_open = false;
                let now = std::time::Instant::now();
                self.power_anim.go_mut(false, now);
                let lock_cmd = self.config.global.lock_command.clone();
                let task = Task::perform(
                    async move { execute_power_action(action, lock_cmd).await },
                    |_| Message::Tick,
                );
                if was_open {
                    return Task::batch([self.sync_surface_size(), task]);
                }
                return task;
            }
            AppMessage::PowerHoverEnter(i) => {
                if i < MAX_POWER_ACTIONS {
                    self.power_hover_anim[i].go_mut(true, std::time::Instant::now());
                }
            }
            AppMessage::PowerHoverExit(i) => {
                if i < MAX_POWER_ACTIONS {
                    self.power_hover_anim[i].go_mut(false, std::time::Instant::now());
                }
            }
            AppMessage::PowerAnimFrame => {
                // For dropdown + slide: progressively update surface size each frame.
                if self.config.global.power_menu_style == "dropdown"
                    && self.config.global.power_anim_style == "slide"
                {
                    return self.sync_surface_size();
                }
                // Other animation styles just need a redraw (view() is called automatically).
            }

            // ── Tray / window list ────────────────────────────────────────────
            AppMessage::ClientsUpdated(clients) => {
                self.state.clients = clients;
            }
            AppMessage::WindowFocusRequested(addr) => {
                return Task::perform(
                    async move {
                        let _ = tokio::process::Command::new("hyprctl")
                            .args(["dispatch", "focuswindow", &format!("address:{addr}")])
                            .output()
                            .await;
                    },
                    |_| Message::Tick,
                );
            }

            // ── New feature messages ──────────────────────────────────────────
            AppMessage::SubMapChanged(name) => {
                self.state.active_submap = name;
            }
            AppMessage::ScreencastChanged(on) => {
                self.state.screencasting = on;
            }
            AppMessage::DndToggle => {
                self.state.dnd_enabled = !self.state.dnd_enabled;
            }
            AppMessage::CalendarToggle => {
                if self.state.notify_panel_open { self.state.notify_panel_open = false; }
                self.calendar_open = !self.calendar_open;
                return self.sync_surface_size();
            }
            AppMessage::UpdateCountRefreshed(count) => {
                self.state.update_count = count;
            }

            // ── Auto-hide ─────────────────────────────────────────────────────
            AppMessage::BarMouseEnter => {
                if self.config.global.auto_hide {
                    self.hide_after = None;
                    if !self.bar_visible {
                        self.bar_visible = true;
                        return self.sync_surface_size();
                    }
                }
            }
            AppMessage::BarMouseLeave => {
                // Only start the hide countdown when no panel is open.
                if self.config.global.auto_hide
                    && !self.state.notify_panel_open
                    && !self.calendar_open
                    && !self.state.power_panel_open
                    && !self.power_inline_open
                {
                    self.hide_after = Some(std::time::Instant::now());
                }
            }
            AppMessage::AutoHideTick => {
                if self.config.global.auto_hide {
                    if let Some(t) = self.hide_after {
                        let delay = Duration::from_millis(
                            self.config.global.auto_hide_delay_ms as u64,
                        );
                        if t.elapsed() >= delay
                            && self.bar_visible
                            && !self.state.notify_panel_open
                            && !self.calendar_open
                            && !self.state.power_panel_open
                            && !self.power_inline_open
                        {
                            self.bar_visible = false;
                            self.hide_after = None;
                            return self.sync_surface_size();
                        }
                    }
                }
            }

            // ── Panel management ──────────────────────────────────────────────
            AppMessage::CloseAllPanels => {
                let was_open = self.state.notify_panel_open
                    || self.calendar_open
                    || self.state.power_panel_open;
                self.state.notify_panel_open = false;
                self.calendar_open = false;
                self.calendar_month_offset = 0;
                if self.state.power_panel_open {
                    self.state.power_panel_open = false;
                    self.power_anim.go_mut(false, std::time::Instant::now());
                }
                if was_open {
                    return self.sync_surface_size();
                }
            }
            AppMessage::CalendarPrevMonth => {
                self.calendar_month_offset -= 1;
            }
            AppMessage::CalendarNextMonth => {
                self.calendar_month_offset += 1;
            }
            AppMessage::NotifyCardHover(id) => {
                self.hover_notif_id = id;
            }

            AppMessage::Tick | AppMessage::Shutdown => {}
        }
        Task::none()
    }

    // ── View ──────────────────────────────────────────────────────────────────

    fn render_widget<'a>(&'a self, kind: &str) -> Option<Element<'a, AppMessage>> {
        match kind {
            "workspaces"  => Some(self.workspaces.view(&self.state, &self.theme)),
            "title"       => Some(self.title.view(&self.state, &self.theme)),
            "clock"       => Some(self.clock.view(&self.state, &self.theme)),
            "cpu"         => Some(self.cpu.view(&self.state, &self.theme, &self.cpu_history)),
            "memory"      => Some(self.memory.view(&self.state, &self.theme)),
            "network"     => Some(self.network.view(&self.state, &self.theme, &self.net_rx_history, &self.net_tx_history)),
            "uptime"      => Some(self.uptime.view(&self.state, &self.theme)),
            "load"        => Some(self.load.view(&self.state, &self.theme)),
            "notify"      => Some(self.notify.view(&self.state, &self.theme)),
            "battery"     => self.battery.view(&self.state, &self.theme),
            "disk"        => self.disk.view(&self.state, &self.theme),
            "temperature" => self.temp.view(&self.state, &self.theme),
            "volume"      => self.volume.view(&self.state, &self.theme),
            "brightness"  => self.brightness.view(&self.state, &self.theme),
            "swap"        => self.swap.view(&self.state, &self.theme),
            "keyboard"    => self.keyboard.view(&self.state, &self.theme),
            "media"       => self.media.view(&self.state, &self.theme),
            "custom"      => self.custom.view(&self.state, &self.theme),
            "separator"   => Some(self.separator.view(&self.state, &self.theme)),
            "tray"        => Some(self.tray.view(&self.state, &self.theme)),
            "power"       => Some(self.power.view(&self.state, &self.theme)),
            "submap"      => self.submap.view(&self.state, &self.theme),
            "screencast"  => self.screencast.view(&self.state, &self.theme),
            "gpu"         => self.gpu.view(&self.state, &self.theme),
            "bluetooth"   => self.bluetooth.view(&self.state, &self.theme),
            "updates"     => self.updates.view(&self.state, &self.theme),
            other => {
                warn!("Unknown widget kind in config: {other}");
                None
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let now          = std::time::Instant::now();
        let power_style  = self.config.global.power_menu_style.as_str();
        let border_color = self.theme.border_color.to_iced();
        let border_width = self.theme.border_width as f32;
        let bar_h        = self.config.global.height as f32;
        let opacity      = self.config.global.opacity;
        let bar_bg_iced  = self.theme.bar_bg
            .map(|c| iced::Background::Color(c.with_alpha(opacity).to_iced()));

        // ── Auto-hide: 1 px strip ──────────────────────────────────────────────
        if self.config.global.auto_hide && !self.bar_visible {
            return iced::widget::mouse_area(
                container(iced::widget::Space::new())
                    .width(Length::Fill)
                    .height(Length::Fixed(1.0)),
            )
            .on_enter(Message::App(AppMessage::BarMouseEnter))
            .into();
        }

        // ── Bar inner content ─────────────────────────────────────────────────
        // Inline power mode replaces the widget row with power action buttons.
        let power_inline_active = power_style == "inline"
            && (self.power_inline_open || self.power_anim.is_animating(now));

        let bar_inner: Element<'_, Message> = if power_inline_active {
            self.view_power_inline()
        } else {
            let gap  = self.theme.gap as f32;
            let pad  = self.theme.padding;
            let fg   = self.theme.foreground.to_iced();
            let hpad = [0.0f32, pad as f32];

            let left_items: Vec<Element<'_, Message>> = self.config.left
                .iter()
                .filter_map(|w| {
                    self.render_widget(&w.kind)
                        .map(|e| pill_wrap(e.map(Message::App), fg))
                })
                .collect();
            let center_items: Vec<Element<'_, Message>> = self.config.center
                .iter()
                .filter_map(|w| {
                    self.render_widget(&w.kind)
                        .map(|e| pill_wrap(e.map(Message::App), fg))
                })
                .collect();
            let right_items: Vec<Element<'_, Message>> = self.config.right
                .iter()
                .filter_map(|w| {
                    self.render_widget(&w.kind)
                        .map(|e| pill_wrap(e.map(Message::App), fg))
                })
                .collect();

            row![
                container(iced::widget::Row::from_vec(left_items).spacing(gap).align_y(iced::Alignment::Center))
                    .width(Length::FillPortion(2)).height(Length::Fill).align_y(iced::Alignment::Center).padding(hpad),
                container(iced::widget::Row::from_vec(center_items).spacing(gap).align_y(iced::Alignment::Center))
                    .center_x(Length::FillPortion(1)).height(Length::Fill).align_y(iced::Alignment::Center).padding(hpad),
                container(iced::widget::Row::from_vec(right_items).spacing(gap).align_y(iced::Alignment::Center))
                    .align_right(Length::FillPortion(2)).height(Length::Fill).align_y(iced::Alignment::Center).padding(hpad),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        };

        let bar_outer: Element<'_, Message> = container(bar_inner)
            .width(Length::Fill)
            .height(Length::Fixed(bar_h))
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: bar_bg_iced,
                border: iced::Border {
                    color: border_color,
                    width: border_width,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into();

        // ── Panel selection (dropdown panels below the bar) ───────────────────
        let power_dropdown_active = power_style == "dropdown"
            && (self.state.power_panel_open || self.power_anim.is_animating(now));

        let any_panel_open = self.state.notify_panel_open
            || self.calendar_open
            || power_dropdown_active;

        // When a panel is open, clicking the bar area (outside panel content) closes it.
        let bar_clickable: Element<'_, Message> = if any_panel_open {
            iced::widget::mouse_area(bar_outer)
                .on_press(Message::App(AppMessage::CloseAllPanels))
                .into()
        } else {
            bar_outer
        };

        let content: Element<'_, Message> = if self.state.notify_panel_open {
            column![bar_clickable, self.view_notify_panel()]
                .width(Length::Fill)
                .into()
        } else if self.calendar_open {
            column![bar_clickable, self.view_calendar_panel()]
                .width(Length::Fill)
                .into()
        } else if power_dropdown_active {
            column![bar_clickable, self.view_power_panel()]
                .width(Length::Fill)
                .into()
        } else {
            bar_clickable
        };

        // When auto-hide is active, track cursor enter/leave on the whole surface.
        if self.config.global.auto_hide {
            iced::widget::mouse_area(content)
                .on_enter(Message::App(AppMessage::BarMouseEnter))
                .on_exit(Message::App(AppMessage::BarMouseLeave))
                .into()
        } else {
            content
        }
    }

    fn view_notify_panel(&self) -> Element<'_, Message> {
        let font_size  = self.theme.font_size;
        let fg_iced    = self.theme.foreground.to_iced();
        let dim_iced   = self.theme.foreground.with_alpha(0.55).to_iced();
        let accent_iced = self.theme.accent.to_iced();

        // Panel surface: blend 12 % of the foreground into the background.
        // This produces a subtly lighter (dark themes) or darker (light themes)
        // shade that is clearly different from the bar itself.
        let bg   = self.theme.background;
        let fg   = self.theme.foreground;
        let mix  = 0.12_f32;
        let panel_bg = ThemeColor {
            r: (bg.r + (fg.r - bg.r) * mix).clamp(0.0, 1.0),
            g: (bg.g + (fg.g - bg.g) * mix).clamp(0.0, 1.0),
            b: (bg.b + (fg.b - bg.b) * mix).clamp(0.0, 1.0),
            a: 0.98,
        };
        let bg_iced = panel_bg.to_iced();

        // ── Header row ───────────────────────────────────────────────────────
        let header = row![
            iced::widget::text("Notifications")
                .size(font_size)
                .color(fg_iced),
            iced::widget::Space::new().width(Length::Fill),
            iced::widget::button(
                iced::widget::text("Clear all").size(font_size - 2.0)
            )
            .on_press(Message::App(AppMessage::NotifyClearAll))
            .style(iced::widget::button::text),
        ]
        .align_y(iced::Alignment::Center)
        .padding([6.0, 12.0]);

        // ── Notification entries ──────────────────────────────────────────────
        let body: Element<'_, Message> = if self.state.notifications.is_empty() {
            container(
                iced::widget::text("No notifications")
                    .size(font_size)
                    .color(dim_iced),
            )
            .padding([16.0, 12.0])
            .width(Length::Fill)
            .into()
        } else {
            let hover_id   = self.hover_notif_id;
            let card_hover_bg = self.theme.foreground.with_alpha(0.06).to_iced();

            let items: Vec<Element<'_, Message>> = self.state.notifications
                .iter()
                .rev()
                .map(|n| {
                    let id = n.id;
                    let is_hovered = hover_id == Some(id);

                    let body_line: Element<'_, Message> = if n.body.is_empty() {
                        iced::widget::Space::new().height(0.0).into()
                    } else {
                        iced::widget::text(n.body.as_str())
                            .size(font_size - 2.0)
                            .color(dim_iced)
                            .into()
                    };

                    let card = row![
                        iced::widget::column![
                            iced::widget::text(n.app_name.as_str())
                                .size(font_size - 2.0)
                                .color(accent_iced),
                            iced::widget::text(n.summary.as_str())
                                .size(font_size),
                            body_line,
                        ]
                        .spacing(2.0)
                        .width(Length::Fill),
                        iced::widget::button(
                            iced::widget::text("×").size(font_size)
                        )
                        .on_press(Message::App(AppMessage::NotifyDismiss(id)))
                        .style(iced::widget::button::text),
                    ]
                    .align_y(iced::Alignment::Start)
                    .padding([6.0, 12.0]);

                    let card_container: Element<'_, Message> = container(card)
                        .width(Length::Fill)
                        .style(move |_: &iced::Theme| iced::widget::container::Style {
                            background: is_hovered
                                .then_some(iced::Background::Color(card_hover_bg)),
                            border: iced::Border {
                                radius: 6.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .into();

                    iced::widget::mouse_area(card_container)
                        .on_enter(Message::App(AppMessage::NotifyCardHover(Some(id))))
                        .on_exit(Message::App(AppMessage::NotifyCardHover(None)))
                        .into()
                })
                .collect();

            iced::widget::scrollable(
                iced::widget::Column::from_vec(items).spacing(1.0).width(Length::Fill),
            )
            .height(Length::Fill)
            .into()
        };

        // Thin accent strip at the very top — clear visual boundary between bar and panel.
        let top_border: Element<'_, Message> = container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(2.0))
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(accent_iced)),
                ..Default::default()
            })
            .into();

        container(
            column![
                top_border,
                header,
                iced::widget::rule::horizontal(1),
                body,
            ]
            .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fixed(NOTIFY_PANEL_HEIGHT as f32))
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(bg_iced)),
            ..Default::default()
        })
        .into()
    }

    fn view_calendar_panel(&self) -> Element<'_, Message> {
        use chrono::Datelike;

        let real_now  = self.state.time;
        let today_ymd = (real_now.year(), real_now.month(), real_now.day());

        // Apply month offset to determine which month to display.
        let offset = self.calendar_month_offset;
        let display_date = if offset >= 0 {
            real_now
                .date_naive()
                .checked_add_months(chrono::Months::new(offset as u32))
                .unwrap_or_else(|| real_now.date_naive())
        } else {
            real_now
                .date_naive()
                .checked_sub_months(chrono::Months::new((-offset) as u32))
                .unwrap_or_else(|| real_now.date_naive())
        };
        let year  = display_date.year();
        let month = display_date.month();

        let fg     = self.theme.foreground.to_iced();
        let dim    = self.theme.foreground.with_alpha(0.5).to_iced();
        let accent = self.theme.accent.to_iced();
        let fsize  = self.theme.font_size;

        // Panel background — same blend as notify panel.
        let bg  = self.theme.background;
        let fgc = self.theme.foreground;
        let mix = 0.12_f32;
        let panel_bg = ThemeColor {
            r: (bg.r + (fgc.r - bg.r) * mix).clamp(0.0, 1.0),
            g: (bg.g + (fgc.g - bg.g) * mix).clamp(0.0, 1.0),
            b: (bg.b + (fgc.b - bg.b) * mix).clamp(0.0, 1.0),
            a: 0.98,
        };
        let bg_iced = panel_bg.to_iced();

        // ── Month navigation header ───────────────────────────────────────────
        let month_name = display_date
            .format("%B %Y")
            .to_string();

        let nav = row![
            iced::widget::button(iced::widget::text("◀").size(fsize).color(fg))
                .on_press(Message::App(AppMessage::CalendarPrevMonth))
                .padding([2.0, 8.0])
                .style(iced::widget::button::text),
            container(iced::widget::text(month_name).size(fsize).color(fg))
                .width(Length::Fill)
                .center_x(Length::Fill),
            iced::widget::button(iced::widget::text("▶").size(fsize).color(fg))
                .on_press(Message::App(AppMessage::CalendarNextMonth))
                .padding([2.0, 8.0])
                .style(iced::widget::button::text),
        ]
        .align_y(iced::Alignment::Center)
        .padding([6.0, 8.0]);

        // ── Day-of-week header row ────────────────────────────────────────────
        let dow_row: Vec<Element<'_, Message>> = ["Mo","Tu","We","Th","Fr","Sa","Su"]
            .iter()
            .map(|&d| {
                container(iced::widget::text(d).size(fsize - 2.0).color(dim))
                    .width(Length::FillPortion(1))
                    .center_x(Length::FillPortion(1))
                    .into()
            })
            .collect();

        // First weekday offset (Mon=0 .. Sun=6)
        let first = chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let start_offset = first.weekday().num_days_from_monday() as usize;

        // Days in month
        let next_month_first = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
        };
        let days_in_month = (next_month_first.unwrap()
            - chrono::Duration::days(1))
            .day() as usize;

        let total_cells = start_offset + days_in_month;
        let num_rows = (total_cells + 6) / 7;
        let mut week_rows: Vec<Element<'_, Message>> = Vec::new();

        let viewing_current_month = (year, month) == (today_ymd.0, today_ymd.1);

        for row_i in 0..num_rows {
            let cells: Vec<Element<'_, Message>> = (0..7)
                .map(|col_i| {
                    let cell = row_i * 7 + col_i;
                    let day = if cell < start_offset || cell >= start_offset + days_in_month {
                        None
                    } else {
                        Some((cell - start_offset + 1) as u32)
                    };
                    let is_today = viewing_current_month
                        && day == Some(today_ymd.2);
                    let (label, color) = match day {
                        None    => ("  ".to_string(), dim),
                        Some(d) => {
                            let c = if is_today { accent } else { fg };
                            (format!("{d:2}"), c)
                        }
                    };

                    // Today gets an accent underline via a column with a dot.
                    let cell_elem: Element<'_, Message> = if is_today {
                        iced::widget::column![
                            iced::widget::text(label).size(fsize - 1.0).color(color),
                            container(iced::widget::Space::new())
                                .width(Length::Fixed(4.0))
                                .height(Length::Fixed(3.0))
                                .style(move |_: &iced::Theme| iced::widget::container::Style {
                                    background: Some(iced::Background::Color(accent)),
                                    border: iced::Border { radius: 2.0.into(), ..Default::default() },
                                    ..Default::default()
                                }),
                        ]
                        .align_x(iced::Alignment::Center)
                        .spacing(1.0)
                        .into()
                    } else {
                        iced::widget::text(label).size(fsize - 1.0).color(color).into()
                    };

                    container(cell_elem)
                        .width(Length::FillPortion(1))
                        .center_x(Length::FillPortion(1))
                        .padding([2.0, 0.0])
                        .into()
                })
                .collect();
            week_rows.push(iced::widget::Row::from_vec(cells).width(Length::Fill).into());
        }

        let mut grid_rows: Vec<Element<'_, Message>> = vec![
            iced::widget::Row::from_vec(dow_row).width(Length::Fill).into(),
        ];
        grid_rows.extend(week_rows);
        let grid = iced::widget::Column::from_vec(grid_rows)
            .spacing(2.0)
            .padding(iced::Padding { top: 0.0, right: 8.0, bottom: 8.0, left: 8.0 })
            .width(Length::Fill);

        let top_border: Element<'_, Message> = container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(2.0))
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(accent)),
                ..Default::default()
            })
            .into();

        container(
            column![top_border, nav, iced::widget::rule::horizontal(1), grid]
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fixed(CALENDAR_PANEL_HEIGHT as f32))
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(bg_iced)),
            ..Default::default()
        })
        .into()
    }

    fn view_power_panel(&self) -> Element<'_, Message> {
        let now        = std::time::Instant::now();
        let fsize      = self.theme.font_size;
        let fg         = self.theme.foreground.to_iced();
        let accent     = self.theme.accent.to_iced();
        let use_nerd   = self.theme.use_nerd_icons;
        let btn_style  = self.theme.power_button_style.as_str();
        let anim_style = self.config.global.power_anim_style.as_str();

        // Panel background — same blend as notify/calendar panels.
        let bg  = self.theme.background;
        let fgc = self.theme.foreground;
        let mix = 0.12_f32;
        let panel_bg = ThemeColor {
            r: (bg.r + (fgc.r - bg.r) * mix).clamp(0.0, 1.0),
            g: (bg.g + (fgc.g - bg.g) * mix).clamp(0.0, 1.0),
            b: (bg.b + (fgc.b - bg.b) * mix).clamp(0.0, 1.0),
            a: 0.98,
        };
        let bg_iced = panel_bg.to_iced();

        // Animation progress (0.0 = closed → 1.0 = fully open).
        let prog = if anim_style != "none" {
            self.power_anim.interpolate(0.0f32, 1.0f32, now)
        } else {
            1.0f32
        };

        // Slide: animate the container height.
        let panel_h = if anim_style == "slide" {
            (POWER_PANEL_HEIGHT as f32 * prog).max(0.0)
        } else {
            POWER_PANEL_HEIGHT as f32
        };

        // Fade: modulate alpha on all colors.
        let fade_alpha = if anim_style == "fade" { prog } else { 1.0f32 };

        // Scale: button padding shrinks from zero to full.
        let (pad_v, pad_h) = if anim_style == "scale" {
            (2.0 + 6.0 * prog, 6.0 + 8.0 * prog)
        } else {
            (8.0f32, 14.0f32)
        };

        let all_actions = ["lock", "sleep", "hibernate", "logout", "reboot", "shutdown"];

        let buttons: Vec<Element<'_, Message>> = self.config.global.power_actions
            .iter()
            .filter_map(|action| {
                let anim_idx = all_actions
                    .iter()
                    .position(|&a| a == action.as_str())
                    .unwrap_or(0);
                let hover_prog =
                    self.power_hover_anim[anim_idx].interpolate(0.0f32, 1.0f32, now);

                let (nerd_icon, label, ascii_icon) = power_action_info(action.as_str());
                let icon = if use_nerd { nerd_icon } else { ascii_icon };

                // Border interpolates foreground → accent on hover.
                let border_col = iced::Color {
                    r: fg.r + (accent.r - fg.r) * hover_prog,
                    g: fg.g + (accent.g - fg.g) * hover_prog,
                    b: fg.b + (accent.b - fg.b) * hover_prog,
                    a: fg.a * fade_alpha,
                };
                let text_col   = iced::Color { a: fg.a * fade_alpha, ..fg };
                let accent_col = iced::Color { a: accent.a * fade_alpha, ..accent };

                let btn_content: Element<'_, Message> = match btn_style {
                    "icon_only" => {
                        iced::widget::text(icon)
                            .size(fsize + 4.0)
                            .color(text_col)
                            .into()
                    }
                    "pill" => row![
                        iced::widget::text(icon).size(fsize + 2.0).color(text_col),
                        iced::widget::text(label).size(fsize - 1.0).color(text_col),
                    ]
                    .spacing(6.0)
                    .align_y(iced::Alignment::Center)
                    .into(),
                    _ => iced::widget::column![ // "icon_label" (default)
                        iced::widget::text(icon).size(fsize + 6.0).color(accent_col),
                        iced::widget::text(label).size(fsize - 2.0).color(text_col),
                    ]
                    .spacing(4.0)
                    .align_x(iced::Alignment::Center)
                    .into(),
                };

                let action_key = action.clone();
                let btn: Element<'_, Message> = iced::widget::button(btn_content)
                    .on_press(Message::App(AppMessage::PowerActionTriggered(action_key)))
                    .padding([pad_v, pad_h])
                    .style(move |_: &iced::Theme, status| {
                        let hovered = status == iced::widget::button::Status::Hovered
                            || status == iced::widget::button::Status::Pressed;
                        iced::widget::button::Style {
                            background: hovered.then_some(iced::Background::Color(
                                iced::Color { a: 0.08 * fade_alpha, ..accent }
                            )),
                            border: iced::Border {
                                color: border_col,
                                width: 1.5,
                                radius: 8.0.into(),
                            },
                            text_color: text_col,
                            ..Default::default()
                        }
                    })
                    .into();

                Some(
                    iced::widget::mouse_area(btn)
                        .on_enter(Message::App(AppMessage::PowerHoverEnter(anim_idx)))
                        .on_exit(Message::App(AppMessage::PowerHoverExit(anim_idx)))
                        .into(),
                )
            })
            .collect();

        let top_border: Element<'_, Message> = container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(2.0))
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(
                    iced::Color { a: accent.a * fade_alpha, ..accent },
                )),
                ..Default::default()
            })
            .into();

        let hint_col = self.theme.foreground.with_alpha(0.35 * fade_alpha).to_iced();
        let hint: Element<'_, Message> = container(
            iced::widget::text("Select an action  •  Esc to close")
                .size(fsize - 3.0)
                .color(hint_col),
        )
        .width(Length::Fill)
        .center_x(Length::Fill)
        .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 4.0, left: 0.0 })
        .into();

        let buttons_row: Element<'_, Message> =
            container(
                iced::widget::Row::from_vec(buttons)
                    .spacing(12.0)
                    .align_y(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .align_y(iced::Alignment::Center)
            .padding([8.0, 16.0])
            .into();

        let inner = iced::widget::column![top_border, buttons_row, hint].width(Length::Fill);

        container(inner)
            .width(Length::Fill)
            .height(Length::Fixed(panel_h))
            .clip(true)
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(
                    iced::Color { a: bg_iced.a * fade_alpha, ..bg_iced },
                )),
                ..Default::default()
            })
            .into()
    }

    fn view_power_inline(&self) -> Element<'_, Message> {
        let now        = std::time::Instant::now();
        let fsize      = self.theme.font_size;
        let fg         = self.theme.foreground.to_iced();
        let accent     = self.theme.accent.to_iced();
        let dim        = self.theme.foreground.with_alpha(0.5).to_iced();
        let use_nerd   = self.theme.use_nerd_icons;
        let btn_style  = self.theme.power_button_style.as_str();
        let anim_style = self.config.global.power_anim_style.as_str();

        // Animation progress (0 = hidden, 1 = fully visible).
        let prog = if anim_style != "none" {
            self.power_anim.interpolate(0.0f32, 1.0f32, now)
        } else {
            1.0f32
        };

        let fade_alpha = if anim_style == "fade" { prog } else { 1.0f32 };

        // Scale: button padding grows into view.
        let (pad_v, pad_h) = if anim_style == "scale" {
            (1.0 + 3.0 * prog, 4.0 + 6.0 * prog)
        } else {
            (4.0f32, 10.0f32)
        };

        let all_actions = ["lock", "sleep", "hibernate", "logout", "reboot", "shutdown"];

        let buttons: Vec<Element<'_, Message>> = self.config.global.power_actions
            .iter()
            .filter_map(|action| {
                let anim_idx = all_actions
                    .iter()
                    .position(|&a| a == action.as_str())
                    .unwrap_or(0);
                let hover_prog =
                    self.power_hover_anim[anim_idx].interpolate(0.0f32, 1.0f32, now);

                let (nerd_icon, label, ascii_icon) = power_action_info(action.as_str());
                let icon = if use_nerd { nerd_icon } else { ascii_icon };

                let text_col  = iced::Color { a: fg.a * fade_alpha, ..fg };
                let border_col = iced::Color {
                    r: fg.r + (accent.r - fg.r) * hover_prog,
                    g: fg.g + (accent.g - fg.g) * hover_prog,
                    b: fg.b + (accent.b - fg.b) * hover_prog,
                    a: fade_alpha,
                };

                let btn_content: Element<'_, Message> = match btn_style {
                    "pill" | "icon_label" => row![
                        iced::widget::text(icon).size(fsize).color(text_col),
                        iced::widget::text(label).size(fsize - 2.0).color(text_col),
                    ]
                    .spacing(5.0)
                    .align_y(iced::Alignment::Center)
                    .into(),
                    _ => iced::widget::text(icon).size(fsize).color(text_col).into(),
                };

                let action_key = action.clone();
                let btn: Element<'_, Message> = iced::widget::button(btn_content)
                    .on_press(Message::App(AppMessage::PowerActionTriggered(action_key)))
                    .padding([pad_v, pad_h])
                    .style(move |_: &iced::Theme, _| iced::widget::button::Style {
                        background: None,
                        border: iced::Border {
                            color: border_col,
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        text_color: text_col,
                        ..Default::default()
                    })
                    .into();

                Some(
                    iced::widget::mouse_area(btn)
                        .on_enter(Message::App(AppMessage::PowerHoverEnter(anim_idx)))
                        .on_exit(Message::App(AppMessage::PowerHoverExit(anim_idx)))
                        .into(),
                )
            })
            .collect();

        // Cancel / close button on the right.
        let cancel_icon = if use_nerd { "󰅖" } else { "x" };
        let cancel_col  = iced::Color { a: dim.a * fade_alpha, ..dim };
        let cancel_btn: Element<'_, Message> = iced::widget::button(
            iced::widget::text(cancel_icon).size(fsize).color(cancel_col),
        )
        .on_press(Message::App(AppMessage::PowerPanelToggle))
        .padding([4.0, 8.0])
        .style(move |_: &iced::Theme, _| iced::widget::button::Style {
            background: None,
            text_color: cancel_col,
            ..Default::default()
        })
        .into();

        // Slide: leading spacer shrinks so buttons appear to fly in from the right.
        let slide_lead = if anim_style == "slide" {
            ((1.0 - prog) * 120.0).max(0.0)
        } else {
            0.0f32
        };

        let buttons_row = iced::widget::Row::from_vec(buttons)
            .spacing(8.0)
            .align_y(iced::Alignment::Center);

        row![
            iced::widget::Space::new().width(Length::Fixed(12.0 + slide_lead)),
            buttons_row,
            iced::widget::Space::new().width(Length::Fill),
            cancel_btn,
            iced::widget::Space::new().width(Length::Fixed(8.0)),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(iced::Alignment::Center)
        .into()
    }

    // ── Subscriptions ─────────────────────────────────────────────────────────

    fn subscription(&self) -> Subscription<Message> {
        let tick = iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick);

        let mut subs = vec![
            tick,
            Subscription::run(ipc_stream),
            Subscription::run(system_stream),
            Subscription::run(config_stream),
            Subscription::run(notify_stream),
            Subscription::run(clients_stream),
            Subscription::run(updates_stream),
        ];

        // Faster poll for auto-hide countdown (200 ms granularity).
        if self.config.global.auto_hide {
            subs.push(
                iced::time::every(Duration::from_millis(200))
                    .map(|_| Message::App(AppMessage::AutoHideTick)),
            );
        }

        // 60 fps animation tick — only active while a power panel animation plays.
        let now = std::time::Instant::now();
        let power_animating = self.power_anim.is_animating(now)
            || self.power_hover_anim.iter().any(|a| a.is_animating(now));
        if power_animating {
            subs.push(
                iced::time::every(Duration::from_millis(16))
                    .map(|_| Message::App(AppMessage::PowerAnimFrame)),
            );
        }

        Subscription::batch(subs)
    }

    // ── Style ─────────────────────────────────────────────────────────────────

    fn style(&self, _theme: &iced::Theme) -> iced::theme::Style {
        // Keep the surface itself fully transparent so Wayland can composite the
        // wallpaper through the bar.  The actual background colour (with opacity)
        // is applied on the bar container in view(), where alpha works correctly.
        iced::theme::Style {
            background_color: iced::Color::TRANSPARENT,
            text_color: self.theme.foreground.to_iced(),
        }
    }

    // ── Panel helpers ─────────────────────────────────────────────────────────

    /// Resize the layer-shell surface to match current visibility / panel state.
    fn sync_surface_size(&self) -> Task<Message> {
        // When auto-hidden, collapse to a 1 px interactive strip.
        if self.config.global.auto_hide && !self.bar_visible {
            return Task::done(Message::SizeChange((0, 1)));
        }
        let bar_h = self.config.global.height;
        let now   = std::time::Instant::now();

        let panel_h = if self.state.notify_panel_open {
            NOTIFY_PANEL_HEIGHT
        } else if self.calendar_open {
            CALENDAR_PANEL_HEIGHT
        } else if self.config.global.power_menu_style == "dropdown"
            && (self.state.power_panel_open || self.power_anim.is_animating(now))
        {
            match self.config.global.power_anim_style.as_str() {
                "slide" => {
                    // Animate panel height during slide.
                    let prog = self.power_anim.interpolate(0.0f32, 1.0f32, now);
                    (POWER_PANEL_HEIGHT as f32 * prog).round() as u32
                }
                _ => POWER_PANEL_HEIGHT, // full height for fade/scale/none
            }
        } else {
            0
        };
        Task::done(Message::SizeChange((0, bar_h + panel_h)))
    }

    /// If no notifications remain and the panel is open, close the panel.
    fn maybe_close_panel(&mut self) -> Task<Message> {
        if self.state.notifications.is_empty() && self.state.notify_panel_open {
            self.state.notify_panel_open = false;
            return self.sync_surface_size();
        }
        Task::none()
    }
}

// ── Power helpers ─────────────────────────────────────────────────────────────

/// Returns `(nerd_icon, label, ascii_fallback)` for a power action key.
fn power_action_info(action: &str) -> (&'static str, &'static str, &'static str) {
    match action {
        "lock"      => ("\u{f033e}", "Lock",      "\u{1f512}"),
        "sleep"     => ("\u{f0904}", "Sleep",     "\u{1f4a4}"),
        "hibernate" => ("\u{f04b2}", "Hibernate", "\u{1f319}"),
        "logout"    => ("\u{f05fd}", "Log Out",   "\u{1f6aa}"),
        "reboot"    => ("\u{f0453}", "Reboot",    "\u{1f504}"),
        "shutdown"  => ("\u{f0425}", "Shutdown",  "\u{23fb}"),
        _           => ("?",         "?",         "?"),
    }
}

/// Execute a power action command in the background.
async fn execute_power_action(action: String, lock_cmd: String) {
    let cmd_str: &str = match action.as_str() {
        "lock"      => &lock_cmd,
        "sleep"     => "systemctl suspend",
        "hibernate" => "systemctl hibernate",
        "logout"    => "hyprctl dispatch exit",
        "reboot"    => "systemctl reboot",
        "shutdown"  => "systemctl poweroff",
        _           => return,
    };
    let mut parts = cmd_str.split_whitespace();
    if let Some(prog) = parts.next() {
        let _ = tokio::process::Command::new(prog).args(parts).spawn();
    }
}

// ── Subscription streams ──────────────────────────────────────────────────────

fn ipc_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(32, |mut sender: Sender<Message>| async move {
        let ipc = match HyprlandIpc::new() {
            Ok(c)  => c,
            Err(e) => {
                warn!("Hyprland IPC unavailable (not under Hyprland?): {e}");
                loop { tokio::time::sleep(Duration::from_secs(3600)).await; }
            }
        };

        match fetch_workspaces(&ipc).await {
            Ok(ws) => {
                let workspaces: Vec<WorkspaceInfo> =
                    ws.into_iter().map(ipc_to_core_workspace).collect();
                let _ = sender.try_send(Message::App(AppMessage::WorkspaceListUpdated(workspaces)));
            }
            Err(e) => warn!("Could not fetch initial workspaces: {e}"),
        }

        let title = fetch_active_window(&ipc).await;
        let _ = sender.try_send(Message::App(AppMessage::ActiveWindowChanged(title)));

        loop {
            match tokio::net::UnixStream::connect(ipc.event_socket()).await {
                Ok(stream) => {
                    info!("Connected to Hyprland event socket");
                    use tokio::io::AsyncBufReadExt;
                    let mut lines = tokio::io::BufReader::new(stream).lines();

                    while let Ok(Some(line)) = lines.next_line().await {
                        let event = bar_ipc::events::parse_event(&line);
                        // Fetch clients immediately when window list changes.
                        if matches!(event, HyprlandEvent::WindowListChanged) {
                            if let Ok(out) = tokio::process::Command::new("hyprctl")
                                .args(["clients", "-j"])
                                .output()
                                .await
                            {
                                if out.status.success() {
                                    let json = String::from_utf8_lossy(&out.stdout);
                                    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&json) {
                                        let clients: Vec<ClientInfo> = arr.iter().filter_map(|c| {
                                            let class = c.get("class")?.as_str()?.to_string();
                                            if class.is_empty() { return None; }
                                            Some(ClientInfo {
                                                address: c.get("address").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                                class,
                                                title: c.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                                workspace_id: c.get("workspace").and_then(|v| v.get("id")).and_then(|v| v.as_i64()).unwrap_or(0).max(0) as u32,
                                            })
                                        }).collect();
                                        let _ = sender.try_send(Message::App(AppMessage::ClientsUpdated(clients)));
                                    }
                                }
                            }
                        }
                        if let Some(msg) = convert_hypr_event(event) {
                            let _ = sender.try_send(Message::App(msg));
                        }
                    }

                    warn!("IPC connection dropped; reconnecting in 2s");
                }
                Err(e) => {
                    error!("Cannot connect to IPC socket: {e}; retrying in 2s");
                }
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    })
}

fn system_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(4, |mut sender: Sender<Message>| async move {
        let custom_cmd   = CUSTOM_CMD.get().cloned().unwrap_or_default();
        let interval_ms  = *SYSTEM_INTERVAL_MS.get().unwrap_or(&DEFAULT_SYSTEM_INTERVAL_MS);
        let mut rx = bar_system::spawn_monitor(interval_ms, custom_cmd);

        while let Some(snapshot) = rx.recv().await {
            let _ = sender.try_send(Message::App(AppMessage::SystemSnapshot(snapshot)));
        }

        loop { tokio::time::sleep(Duration::from_secs(3600)).await; }
    })
}

fn config_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(1, |mut sender: Sender<Message>| async move {
        let (_watcher, mut rx) = ConfigWatcher::spawn(default_path());

        while rx.recv().await.is_some() {
            let _ = sender.try_send(Message::App(AppMessage::ConfigReloaded));
        }

        loop { tokio::time::sleep(Duration::from_secs(3600)).await; }
    })
}

/// D-Bus `org.freedesktop.Notifications` listener.
///
/// First tries to register as the notification daemon.
/// If another daemon is already running (dunst, mako, swaync …) the
/// registration fails and we fall back to polling `dunstctl history` every
/// 2 s instead — so the notify widget still works with dunst.
fn notify_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(32, |mut iced_tx: Sender<Message>| async move {
        // ── Try to become the D-Bus notification daemon ───────────────────────
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
        let daemon = NotifDaemon { sender: tx, next_id: 0 };

        let conn_result = zbus::connection::Builder::session()
            .and_then(|b| b.name("org.freedesktop.Notifications"))
            .and_then(|b| b.serve_at("/org/freedesktop/Notifications", daemon));

        match conn_result {
            Ok(builder) => match builder.build().await {
                Ok(_conn) => {
                    info!("Registered as org.freedesktop.Notifications daemon");
                    while let Some(msg) = rx.recv().await {
                        let _ = iced_tx.try_send(msg);
                    }
                    loop { tokio::time::sleep(Duration::from_secs(3600)).await; }
                }
                Err(e) => {
                    warn!("D-Bus build failed ({e}) — falling back to dunstctl polling");
                    dunstctl_poll_loop(iced_tx).await;
                }
            },
            Err(e) => {
                warn!("Could not register notification daemon ({e}) — falling back to dunstctl polling");
                dunstctl_poll_loop(iced_tx).await;
            }
        }
    })
}

/// Polls `dunstctl history` every 2 s and forwards new notifications to iced.
/// Used automatically when another notification daemon (e.g. dunst) is running.
async fn dunstctl_poll_loop(mut sender: Sender<Message>) {
    let mut known_ids: std::collections::HashSet<u32> = std::collections::HashSet::new();

    loop {
        if let Ok(out) = tokio::process::Command::new("dunstctl")
            .arg("history")
            .output()
            .await
        {
            if out.status.success() {
                let json = String::from_utf8_lossy(&out.stdout);
                for (id, app_name, summary, body) in parse_dunstctl_history(&json) {
                    if known_ids.insert(id) {
                        let _ = sender.try_send(Message::App(AppMessage::NotificationReceived {
                            id,
                            app_name,
                            summary,
                            body,
                        }));
                    }
                }
                // Prevent the set from growing indefinitely.
                if known_ids.len() > 500 {
                    known_ids.clear();
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Parse the JSON output of `dunstctl history` into a flat list of entries.
///
/// Format: `{"data": [[{notification}, …], …]}`
/// Each notification field is `{"data": <value>, "type": "string"|"int"|…}`.
fn parse_dunstctl_history(json: &str) -> Vec<(u32, String, String, String)> {
    let mut out = Vec::new();
    let Ok(root) = serde_json::from_str::<serde_json::Value>(json) else {
        return out;
    };
    let Some(stacks) = root.get("data").and_then(|v| v.as_array()) else {
        return out;
    };
    for stack in stacks {
        let Some(entries) = stack.as_array() else { continue };
        for entry in entries {
            let id = entry.get("id")
                .and_then(|v| v.get("data"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            if id == 0 { continue; }
            let app_name = dunst_str(entry, "appname");
            let summary  = dunst_str(entry, "summary");
            let body     = dunst_str(entry, "body");
            out.push((id, app_name, summary, body));
        }
    }
    out
}

/// Extract a string-typed field from a dunstctl history notification object.
fn dunst_str(entry: &serde_json::Value, key: &str) -> String {
    entry.get(key)
        .and_then(|v| v.get("data"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Polls `hyprctl clients -j` every 10 s as a fallback — real-time updates come from ipc_stream.
fn clients_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(4, |mut sender: Sender<Message>| async move {
        loop {
            if let Ok(out) = tokio::process::Command::new("hyprctl")
                .args(["clients", "-j"])
                .output()
                .await
            {
                if out.status.success() {
                    let json = String::from_utf8_lossy(&out.stdout);
                    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&json) {
                        let clients: Vec<ClientInfo> = arr
                            .iter()
                            .filter_map(|c| {
                                let class = c.get("class")?.as_str()?.to_string();
                                if class.is_empty() {
                                    return None;
                                }
                                Some(ClientInfo {
                                    address: c.get("address")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    class,
                                    title: c.get("title")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    workspace_id: c.get("workspace")
                                        .and_then(|v| v.get("id"))
                                        .and_then(|v| v.as_i64())
                                        .unwrap_or(0)
                                        .max(0) as u32,
                                })
                            })
                            .collect();
                        let _ = sender.try_send(Message::App(AppMessage::ClientsUpdated(clients)));
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    })
}

/// Polls `checkupdates` every 5 minutes and sends the count to the bar.
///
/// `checkupdates` (pacman-contrib) prints one pending update per line and
/// exits 0 when updates exist, 2 when up-to-date, or non-zero on error.
fn updates_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(4, |mut sender: Sender<Message>| async move {
        loop {
            let count = tokio::process::Command::new("checkupdates")
                .output()
                .await
                .ok()
                .map(|out| {
                    String::from_utf8_lossy(&out.stdout)
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .count() as u32
                });
            let _ = sender.try_send(Message::App(AppMessage::UpdateCountRefreshed(count)));
            tokio::time::sleep(Duration::from_secs(300)).await;
        }
    })
}

// ── D-Bus notification interface ──────────────────────────────────────────────

struct NotifDaemon {
    sender:  tokio::sync::mpsc::UnboundedSender<Message>,
    next_id: u32,
}

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl NotifDaemon {
    /// Called by applications to display a notification.
    async fn notify(
        &mut self,
        app_name:       String,
        replaces_id:    u32,
        _app_icon:      String,
        summary:        String,
        body:           String,
        _actions:       Vec<String>,
        _hints:         std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        _expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id != 0 {
            replaces_id
        } else {
            self.next_id += 1;
            self.next_id
        };
        let _ = self.sender.send(Message::App(AppMessage::NotificationReceived {
            id,
            app_name,
            summary,
            body,
        }));
        id
    }

    /// Called by applications to close a specific notification.
    fn close_notification(&self, id: u32) {
        let _ = self.sender.send(Message::App(AppMessage::NotificationClosed(id)));
    }

    /// Returns the capabilities this server supports.
    fn get_capabilities(&self) -> Vec<&'static str> {
        vec!["body", "persistence"]
    }

    /// Returns server identity information.
    fn get_server_information(&self) -> (&'static str, &'static str, &'static str, &'static str) {
        ("bar", "bar", "0.1.0", "1.2")
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn pill_wrap<'a>(elem: Element<'a, Message>, fg: iced::Color) -> Element<'a, Message> {
    let border_col = iced::Color { a: 0.18, ..fg };
    container(elem)
        .padding([4.0, 10.0])
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            border: iced::Border {
                radius: 99.0.into(),
                color:  border_col,
                width:  1.0,
            },
            ..Default::default()
        })
        .into()
}

fn position_to_anchor(pos: Position) -> Anchor {
    match pos {
        Position::Top    => Anchor::Top    | Anchor::Left | Anchor::Right,
        Position::Bottom => Anchor::Bottom | Anchor::Left | Anchor::Right,
    }
}

fn ipc_to_core_workspace(w: bar_ipc::WorkspaceInfo) -> WorkspaceInfo {
    WorkspaceInfo {
        id:      w.id.unsigned_abs(),
        name:    w.name,
        monitor: w.monitor,
        windows: w.windows,
    }
}

fn convert_hypr_event(event: HyprlandEvent) -> Option<AppMessage> {
    match event {
        HyprlandEvent::Workspace(ws) => Some(AppMessage::WorkspaceChanged(ws.id)),
        HyprlandEvent::ActiveWindow(aw) => {
            let title = if aw.title.is_empty() { None } else { Some(aw.title) };
            Some(AppMessage::ActiveWindowChanged(title))
        }
        HyprlandEvent::Fullscreen(fs) => Some(AppMessage::FullscreenStateChanged(fs)),
        HyprlandEvent::ActiveLayout(layout) => Some(AppMessage::KeyboardLayoutChanged(layout)),
        HyprlandEvent::SubMap(name) => {
            let opt = if name.is_empty() { None } else { Some(name) };
            Some(AppMessage::SubMapChanged(opt))
        }
        HyprlandEvent::Screencast(on) => Some(AppMessage::ScreencastChanged(on)),
        // WindowListChanged is handled inline in ipc_stream — return None here.
        HyprlandEvent::WindowListChanged
        | HyprlandEvent::MonitorFocused(_)
        | HyprlandEvent::Unknown(_) => None,
    }
}
