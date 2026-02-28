//! Wayland layer-shell surface for `bar`.
//!
//! Owns the Iced application loop and wires together all background tasks:
//! - Hyprland IPC event stream (workspaces, active window, fullscreen, keyboard layout)
//! - System resource monitor (CPU, RAM, disk, media, etc.)
//! - Config file watcher (live reload on change)
//! - 1-second timer (clock)

use bar_config::{default_path, load as load_config, BarConfig, ConfigWatcher, Position};
use bar_core::{
    event::Message as AppMessage,
    state::{AppState, WorkspaceInfo},
};
use bar_ipc::{fetch_workspaces, HyprlandEvent, HyprlandIpc};
use bar_theme::{Color as ThemeColor, Theme};
use bar_widgets::{
    BatteryWidget, BrightnessWidget, ClockWidget, CpuWidget, CustomWidget, DiskWidget,
    KeyboardWidget, LoadWidget, MediaWidget, MemoryWidget, NetworkWidget, SwapWidget, TempWidget,
    TitleWidget, UptimeWidget, VolumeWidget, WorkspaceWidget,
};
use chrono::Local;
use futures::channel::mpsc::Sender;
use iced::{
    widget::{container, row},
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

/// Custom shell command set once from config at startup.
/// Shared with the system monitor subscription (which is a free function and
/// cannot receive parameters directly).
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

    // Initialise custom command before the app starts so system_stream can read it.
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
            "battery"     => self.battery.view(&self.state, &self.theme),
            "disk"        => self.disk.view(&self.state, &self.theme),
            "temperature" => self.temp.view(&self.state, &self.theme),
            "volume"      => self.volume.view(&self.state, &self.theme),
            "brightness"  => self.brightness.view(&self.state, &self.theme),
            "swap"        => self.swap.view(&self.state, &self.theme),
            "keyboard"    => self.keyboard.view(&self.state, &self.theme),
            "media"       => self.media.view(&self.state, &self.theme),
            "custom"      => self.custom.view(&self.state, &self.theme),
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

        let left_items: Vec<Element<'_, Message>> = self.config.left
            .iter()
            .filter_map(|w| {
                self.render_widget(&w.kind)
                    .map(|e| pill_wrap(e.map(Message::App), radius, wbg))
            })
            .collect();
        let left = iced::widget::Row::from_vec(left_items)
            .spacing(gap)
            .align_y(iced::Alignment::Center);

        let center_items: Vec<Element<'_, Message>> = self.config.center
            .iter()
            .filter_map(|w| {
                self.render_widget(&w.kind)
                    .map(|e| pill_wrap(e.map(Message::App), radius, wbg))
            })
            .collect();
        let center = iced::widget::Row::from_vec(center_items)
            .spacing(gap)
            .align_y(iced::Alignment::Center);

        let right_items: Vec<Element<'_, Message>> = self.config.right
            .iter()
            .filter_map(|w| {
                self.render_widget(&w.kind)
                    .map(|e| pill_wrap(e.map(Message::App), radius, wbg))
            })
            .collect();
        let right = iced::widget::Row::from_vec(right_items)
            .spacing(gap)
            .align_y(iced::Alignment::Center);

        let bar = row![
            container(left)
                .width(Length::FillPortion(2))
                .padding(pad),
            container(center)
                .center_x(Length::FillPortion(1))
                .padding(pad),
            container(right)
                .align_right(Length::FillPortion(2))
                .padding(pad),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        let border_color = self.theme.border_color.to_iced();
        let border_width = self.theme.border_width as f32;

        container(bar)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                border: iced::Border {
                    color: border_color,
                    width: border_width,
                    radius: 0.0.into(),
                },
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

// ── Helpers ───────────────────────────────────────────────────────────────────

fn pill_wrap<'a>(
    elem: Element<'a, Message>,
    radius: f32,
    bg: Option<ThemeColor>,
) -> Element<'a, Message> {
    container(elem)
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
