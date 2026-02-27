//! Wayland layer-shell surface for `bar`.
//!
//! Owns the Iced application loop and wires together all background tasks:
//! - Hyprland IPC event stream (workspaces, active window, fullscreen)
//! - System resource monitor (CPU, RAM, disk)
//! - Config file watcher (live reload on change)
//! - 1-second timer (clock)

use bar_config::{default_path, load as load_config, BarConfig, ConfigWatcher, Position};
use bar_core::{
    event::Message as AppMessage,
    state::{AppState, WorkspaceInfo},
};
use bar_ipc::{fetch_workspaces, HyprlandEvent, HyprlandIpc};
use bar_theme::Theme;
use bar_widgets::{
    BatteryWidget, ClockWidget, CpuWidget, MemoryWidget, NetworkWidget, TitleWidget,
    WorkspaceWidget,
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
use std::time::Duration;
use tracing::{error, info, warn};

/// System monitor poll interval (milliseconds).
const SYSTEM_INTERVAL_MS: u64 = 2_000;

// ── Entry point ───────────────────────────────────────────────────────────────

/// Start the Wayland bar.  Never returns under normal operation.
pub fn run() -> iced_layershell::Result {
    let config = load_config(default_path()).unwrap_or_default();
    let height = config.global.height;
    let anchor = position_to_anchor(config.global.position);
    let exclusive_zone = if config.global.exclusive_zone {
        height as i32
    } else {
        0
    };

    application(Bar::new, Bar::namespace, Bar::update, Bar::view)
        .subscription(Bar::subscription)
        .style(Bar::style)
        .settings(Settings {
            layer_settings: LayerShellSettings {
                size: Some((0, height)), // width=0 + L|R anchor = full-width stretch
                exclusive_zone,
                anchor,
                layer: Layer::Top,
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}

// ── Message ───────────────────────────────────────────────────────────────────

/// Top-level application messages.
///
/// `#[to_layer_message]` injects layer-shell control variants (AnchorChange,
/// SizeChange, etc.).  Those are handled by the backend in 0.15 and never
/// reach `update()`.
#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    /// Propagate a core event-bus message.
    App(AppMessage),
    /// One-second timer tick — updates the clock display.
    Tick,
}

// ── State ─────────────────────────────────────────────────────────────────────

struct Bar {
    state:      AppState,
    config:     BarConfig,
    theme:      Theme,
    // Left
    workspaces: WorkspaceWidget,
    title:      TitleWidget,
    // Center
    clock:      ClockWidget,
    // Right
    network:    NetworkWidget,
    cpu:        CpuWidget,
    memory:     MemoryWidget,
    battery:    BatteryWidget,
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
            battery:    BatteryWidget::new(),
        };

        // Kick off an initial workspace list fetch so the widget isn't blank
        // until the first IPC event arrives.
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
                    Message::Tick // benign fallback
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
            // Layer-shell injected variants handled by backend in iced-layershell 0.15.
            _ => Task::none(),
        }
    }

    fn handle_app(&mut self, msg: AppMessage) -> Task<Message> {
        match msg {
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
            AppMessage::SystemSnapshot(snapshot) => {
                self.state.system = snapshot;
            }
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
            AppMessage::Tick | AppMessage::Shutdown => {}
        }
        Task::none()
    }

    // ── View ──────────────────────────────────────────────────────────────────

    fn view(&self) -> Element<'_, Message> {
        let gap = self.theme.gap as f32;
        let pad = self.theme.padding;

        // ── Left: workspaces + window title ──────────────────────────────────
        let left = {
            let ws    = self.workspaces.view(&self.state, &self.theme).map(Message::App);
            let title = self.title.view(&self.state, &self.theme).map(Message::App);
            row![ws, title].spacing(gap * 3.0).align_y(iced::Alignment::Center)
        };

        // ── Center: clock ─────────────────────────────────────────────────────
        let center = self.clock.view(&self.state, &self.theme).map(Message::App);

        // ── Right: network · cpu · memory · battery (optional) ───────────────
        let net = self.network.view(&self.state, &self.theme).map(Message::App);
        let cpu = self.cpu.view(&self.state, &self.theme).map(Message::App);
        let mem = self.memory.view(&self.state, &self.theme).map(Message::App);

        let right: Element<'_, Message> = match self.battery.view(&self.state, &self.theme) {
            Some(bat) => {
                let bat = bat.map(Message::App);
                row![net, cpu, mem, bat].spacing(gap * 2.0).align_y(iced::Alignment::Center).into()
            }
            None => {
                row![net, cpu, mem].spacing(gap * 2.0).align_y(iced::Alignment::Center).into()
            }
        };

        let bar = row![
            container(left)
                .width(Length::FillPortion(2))
                .padding(pad),
            container(center)
                .width(Length::FillPortion(1))
                .center_x(Length::Fill)
                .padding(pad),
            container(right)
                .width(Length::FillPortion(2))
                .align_right(Length::Fill)
                .padding(pad),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        container(bar)
            .width(Length::Fill)
            .height(Length::Fill)
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
//
// Each free function acts as both the stream builder AND the unique identity
// key for `Subscription::run(fn_ptr)`.  Iced uses the function pointer address
// to deduplicate subscriptions across redraws.

/// Connects to the Hyprland event socket, fetches the initial workspace list,
/// then streams live IPC events indefinitely (auto-reconnects on drop).
fn ipc_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(32, |mut sender: Sender<Message>| async move {
        let ipc = match HyprlandIpc::new() {
            Ok(c)  => c,
            Err(e) => {
                warn!("Hyprland IPC unavailable (not under Hyprland?): {e}");
                // Bar still runs without workspace data.
                loop {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
            }
        };

        // Send the initial workspace list immediately on connect.
        match fetch_workspaces(&ipc).await {
            Ok(ws) => {
                let workspaces: Vec<WorkspaceInfo> =
                    ws.into_iter().map(ipc_to_core_workspace).collect();
                let _ = sender.try_send(Message::App(
                    AppMessage::WorkspaceListUpdated(workspaces),
                ));
            }
            Err(e) => warn!("Could not fetch initial workspaces: {e}"),
        }

        // Stream live events; reconnect whenever the socket closes.
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

/// Polls system resources every [`SYSTEM_INTERVAL_MS`] ms and forwards snapshots.
fn system_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(4, |mut sender: Sender<Message>| async move {
        let mut rx = bar_system::spawn_monitor(SYSTEM_INTERVAL_MS);

        while let Some(snapshot) = rx.recv().await {
            let _ = sender.try_send(Message::App(AppMessage::SystemSnapshot(snapshot)));
        }

        // Monitor task exited — shouldn't happen; stall rather than crash.
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    })
}

/// Watches `~/.config/bar/bar.toml` for writes and sends `ConfigReloaded`.
fn config_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(1, |mut sender: Sender<Message>| async move {
        let (_watcher, mut rx) = ConfigWatcher::spawn(default_path());

        while rx.recv().await.is_some() {
            let _ = sender.try_send(Message::App(AppMessage::ConfigReloaded));
        }

        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn position_to_anchor(pos: Position) -> Anchor {
    match pos {
        Position::Top    => Anchor::Top    | Anchor::Left | Anchor::Right,
        Position::Bottom => Anchor::Bottom | Anchor::Left | Anchor::Right,
    }
}

/// Convert an `ipc::WorkspaceInfo` (i32 ids) to `core::WorkspaceInfo` (u32 ids).
fn ipc_to_core_workspace(w: bar_ipc::WorkspaceInfo) -> WorkspaceInfo {
    WorkspaceInfo {
        id:      w.id.unsigned_abs(),
        name:    w.name,
        monitor: w.monitor,
        windows: w.windows,
    }
}

/// Map a raw Hyprland event to an `AppMessage`, filtering out unhandled variants.
fn convert_hypr_event(event: HyprlandEvent) -> Option<AppMessage> {
    match event {
        HyprlandEvent::Workspace(ws) => Some(AppMessage::WorkspaceChanged(ws.id)),
        HyprlandEvent::ActiveWindow(aw) => {
            let title = if aw.title.is_empty() { None } else { Some(aw.title) };
            Some(AppMessage::ActiveWindowChanged(title))
        }
        HyprlandEvent::Fullscreen(fs) => Some(AppMessage::FullscreenStateChanged(fs)),
        HyprlandEvent::MonitorFocused(_) | HyprlandEvent::Unknown(_) => None,
    }
}
