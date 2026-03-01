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
    state::{AppState, NotifEntry, WorkspaceInfo},
};
use bar_ipc::{fetch_active_window, fetch_workspaces, HyprlandEvent, HyprlandIpc};
use bar_theme::{Color as ThemeColor, Theme};
use bar_widgets::{
    BatteryWidget, BrightnessWidget, ClockWidget, CpuWidget, CustomWidget, DiskWidget,
    KeyboardWidget, LoadWidget, MediaWidget, MemoryWidget, NetworkWidget, NotifyWidget,
    SeparatorWidget, SwapWidget, TempWidget, TitleWidget, UptimeWidget, VolumeWidget,
    WorkspaceWidget,
};
use chrono::Local;
use futures::channel::mpsc::Sender;
use iced::{
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

/// System monitor poll interval (milliseconds).
const SYSTEM_INTERVAL_MS: u64 = 2_000;

/// Height of the notification panel that drops below the bar (pixels).
const NOTIFY_PANEL_HEIGHT: u32 = 300;

/// Custom shell command set once from config at startup.
static CUSTOM_CMD: OnceLock<String> = OnceLock::new();

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
    let exclusive_zone = if config.global.exclusive_zone {
        (height + config.global.margin_top) as i32
    } else {
        0
    };

    let _ = CUSTOM_CMD.set(config.global.custom_command.clone());

    application(Bar::new, Bar::namespace, Bar::update, Bar::view)
        .subscription(Bar::subscription)
        .style(Bar::style)
        .settings(Settings {
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
            AppMessage::SystemSnapshot(snapshot) => {
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
                // Replace an existing entry with the same id (replaces_id flow).
                self.state.notifications.retain(|n| n.id != id);
                self.state.notifications.push(NotifEntry { id, app_name, summary, body });
                // Cap history at 50 entries (drop oldest).
                if self.state.notifications.len() > 50 {
                    self.state.notifications.remove(0);
                }
            }
            AppMessage::NotificationClosed(id) => {
                self.state.notifications.retain(|n| n.id != id);
                return self.maybe_close_panel();
            }
            AppMessage::NotifyPanelToggle => {
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
                let arg = if delta >= 0 {
                    format!("{delta}%+")
                } else {
                    format!("{}%-", delta.unsigned_abs())
                };
                return Task::perform(
                    async move {
                        let _ = tokio::process::Command::new("wpctl")
                            .args(["set-volume", "-l", "1.5", "@DEFAULT_AUDIO_SINK@", &arg])
                            .output()
                            .await;
                    },
                    |_| Message::Tick,
                );
            }
            AppMessage::VolumeMuteToggle => {
                return Task::perform(
                    async {
                        let _ = tokio::process::Command::new("wpctl")
                            .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
                            .output()
                            .await;
                    },
                    |_| Message::Tick,
                );
            }
            AppMessage::BrightnessAdjust(delta) => {
                let arg = if delta >= 0 {
                    format!("{delta}%+")
                } else {
                    format!("{}%-", delta.unsigned_abs())
                };
                return Task::perform(
                    async move {
                        let _ = tokio::process::Command::new("brightnessctl")
                            .args(["set", &arg])
                            .output()
                            .await;
                    },
                    |_| Message::Tick,
                );
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
            "cpu"         => Some(self.cpu.view(&self.state, &self.theme)),
            "memory"      => Some(self.memory.view(&self.state, &self.theme)),
            "network"     => Some(self.network.view(&self.state, &self.theme)),
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
            other => {
                warn!("Unknown widget kind in config: {other}");
                None
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let gap    = self.theme.gap as f32;
        let pad    = self.theme.padding;
        let radius = self.theme.border_radius;
        let wbg    = self.theme.widget_bg;
        let pad_x  = self.theme.widget_pad_x;
        let pad_y  = self.theme.widget_pad_y;

        let left_items: Vec<Element<'_, Message>> = self.config.left
            .iter()
            .filter_map(|w| {
                self.render_widget(&w.kind)
                    .map(|e| pill_wrap(e.map(Message::App), radius, wbg, pad_x, pad_y))
            })
            .collect();
        let left = iced::widget::Row::from_vec(left_items)
            .spacing(gap)
            .align_y(iced::Alignment::Center);

        let center_items: Vec<Element<'_, Message>> = self.config.center
            .iter()
            .filter_map(|w| {
                self.render_widget(&w.kind)
                    .map(|e| pill_wrap(e.map(Message::App), radius, wbg, pad_x, pad_y))
            })
            .collect();
        let center = iced::widget::Row::from_vec(center_items)
            .spacing(gap)
            .align_y(iced::Alignment::Center);

        let right_items: Vec<Element<'_, Message>> = self.config.right
            .iter()
            .filter_map(|w| {
                self.render_widget(&w.kind)
                    .map(|e| pill_wrap(e.map(Message::App), radius, wbg, pad_x, pad_y))
            })
            .collect();
        let right = iced::widget::Row::from_vec(right_items)
            .spacing(gap)
            .align_y(iced::Alignment::Center);

        let hpad = [0.0f32, pad as f32];

        let bar = row![
            container(left)
                .width(Length::FillPortion(2))
                .height(Length::Fill)
                .align_y(iced::Alignment::Center)
                .padding(hpad),
            container(center)
                .center_x(Length::FillPortion(1))
                .height(Length::Fill)
                .align_y(iced::Alignment::Center)
                .padding(hpad),
            container(right)
                .align_right(Length::FillPortion(2))
                .height(Length::Fill)
                .align_y(iced::Alignment::Center)
                .padding(hpad),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        let border_color = self.theme.border_color.to_iced();
        let border_width = self.theme.border_width as f32;
        let bar_h        = self.config.global.height as f32;

        let bar_outer: Element<'_, Message> = container(bar)
            .width(Length::Fill)
            .height(Length::Fixed(bar_h))
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                border: iced::Border {
                    color: border_color,
                    width: border_width,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into();

        if self.state.notify_panel_open {
            column![bar_outer, self.view_notify_panel()]
                .width(Length::Fill)
                .into()
        } else {
            bar_outer
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
            let items: Vec<Element<'_, Message>> = self.state.notifications
                .iter()
                .rev()
                .map(|n| {
                    let id = n.id;
                    let body_line: Element<'_, Message> = if n.body.is_empty() {
                        iced::widget::Space::new().height(0.0).into()
                    } else {
                        iced::widget::text(n.body.as_str())
                            .size(font_size - 2.0)
                            .color(dim_iced)
                            .into()
                    };

                    row![
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
                    .padding([6.0, 12.0])
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

    // ── Subscriptions ─────────────────────────────────────────────────────────

    fn subscription(&self) -> Subscription<Message> {
        let tick = iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick);

        Subscription::batch([
            tick,
            Subscription::run(ipc_stream),
            Subscription::run(system_stream),
            Subscription::run(config_stream),
            Subscription::run(notify_stream),
        ])
    }

    // ── Style ─────────────────────────────────────────────────────────────────

    fn style(&self, _theme: &iced::Theme) -> iced::theme::Style {
        let bg = self.theme.background.with_alpha(self.config.global.opacity);
        iced::theme::Style {
            background_color: bg.to_iced(),
            text_color: self.theme.foreground.to_iced(),
        }
    }

    // ── Panel helpers ─────────────────────────────────────────────────────────

    /// Resize the layer-shell surface to match whether the panel is open.
    fn sync_surface_size(&self) -> Task<Message> {
        let bar_h   = self.config.global.height;
        let total_h = if self.state.notify_panel_open {
            bar_h + NOTIFY_PANEL_HEIGHT
        } else {
            bar_h
        };
        Task::done(Message::SizeChange((0, total_h)))
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
                        if let Some(msg) =
                            convert_hypr_event(bar_ipc::events::parse_event(&line))
                        {
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
        let custom_cmd = CUSTOM_CMD.get().cloned().unwrap_or_default();
        let mut rx = bar_system::spawn_monitor(SYSTEM_INTERVAL_MS, custom_cmd);

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

fn pill_wrap<'a>(
    elem: Element<'a, Message>,
    radius: f32,
    bg: Option<ThemeColor>,
    pad_x: u16,
    pad_y: u16,
) -> Element<'a, Message> {
    container(elem)
        .padding([pad_y as f32, pad_x as f32])
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: bg.map(|c| iced::Background::Color(c.to_iced())),
            border: iced::Border { radius: radius.into(), ..Default::default() },
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
        HyprlandEvent::MonitorFocused(_) | HyprlandEvent::Unknown(_) => None,
    }
}
