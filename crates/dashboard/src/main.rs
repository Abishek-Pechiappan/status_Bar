//! `bar-dashboard` — bento-grid system info overlay.
//!
//! Launch with a Hyprland keybind:
//!   `bind = SUPER, D, exec, bar-dashboard`
//! Press Escape or click the dim background to dismiss.

use bar_config::{default_path, load as load_config, schema::DashboardConfig};
use bar_theme::Theme;
use futures::channel::mpsc::Sender;
use iced::{
    widget::{canvas, column, container, row, stack, text},
    Alignment, Background, Border, Color, Element, Font, Length, Subscription, Task,
};
use iced_layershell::{
    build_pattern::application,
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::{LayerShellSettings, Settings},
    to_layer_message,
};
use std::{collections::VecDeque, time::Duration};

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> iced_layershell::Result {
    let config = load_config(default_path()).unwrap_or_default();
    if !config.dashboard.enabled {
        return Ok(());
    }

    let font_name: &'static str = Box::leak(config.theme.font.clone().into_boxed_str());
    let default_font = iced::Font {
        family: iced::font::Family::Name(font_name),
        weight: iced::font::Weight::Normal,
        stretch: iced::font::Stretch::Normal,
        style:  iced::font::Style::Normal,
    };

    application(Dashboard::new, Dashboard::namespace, Dashboard::update, Dashboard::view)
        .subscription(Dashboard::subscription)
        .style(Dashboard::style)
        .settings(Settings {
            default_font,
            layer_settings: LayerShellSettings {
                anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
                layer:  Layer::Overlay,
                exclusive_zone: -1,
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}

// ── System snapshot ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
struct DashSnapshot {
    cpu_pct:          f32,
    ram_used:         u64,
    ram_total:        u64,
    swap_used:        u64,
    swap_total:       u64,
    disk_used:        u64,
    disk_total:       u64,
    net_iface:        String,
    net_rx_bps:       u64,
    net_tx_bps:       u64,
    volume:           Option<f32>,
    volume_muted:     bool,
    brightness:       Option<u8>,
    battery_pct:      Option<u8>,
    battery_charging: bool,
    uptime_secs:      u64,
    temp_celsius:     Option<f32>,
    media_title:      Option<String>,
    media_artist:     Option<String>,
    media_playing:    bool,
    update_count:     Option<u32>,
    load_1:           f32,
    load_5:           f32,
    load_15:          f32,
    gpu_percent:      Option<f32>,
    gpu_temp:         Option<f32>,
    gpu_mem_used:     Option<u64>,
    gpu_mem_total:    Option<u64>,
    bt_connected:     bool,
    bt_device_name:   Option<String>,
    weather_text:     String,
    // Rolling history buffers — capped at 60 samples (~2 min at 2s poll)
    cpu_history:      VecDeque<f32>,
    net_rx_history:   VecDeque<f32>,
}

async fn read_sys_snapshot(weather_location: String) -> DashSnapshot {
    // Heavy sysinfo work in a blocking thread — CPU needs 150ms between samples.
    // Heavy sysinfo work in a blocking thread — CPU needs 150ms between samples.
    // Split into two smaller tuples (Rust Default only supports tuples up to 12).
    struct SysInfo {
        cpu_pct:    f32,
        ram_used:   u64,
        ram_total:  u64,
        swap_used:  u64,
        swap_total: u64,
        disk_used:  u64,
        disk_total: u64,
        net_iface:  String,
        net_rx_bps: u64,
        net_tx_bps: u64,
        uptime_secs: u64,
        temp_celsius: Option<f32>,
        load_1:     f32,
        load_5:     f32,
        load_15:    f32,
    }

    let info = tokio::task::spawn_blocking(|| {
        use sysinfo::System;
        let mut sys = System::new();
        sys.refresh_cpu_all();
        std::thread::sleep(Duration::from_millis(150));
        sys.refresh_cpu_all();
        sys.refresh_memory();

        let cpu_pct    = sys.global_cpu_usage();
        let ram_used   = sys.used_memory();
        let ram_total  = sys.total_memory();
        let swap_used  = sys.used_swap();
        let swap_total = sys.total_swap();
        let uptime     = System::uptime();

        let load = System::load_average();

        let disks = sysinfo::Disks::new_with_refreshed_list();
        let (disk_used, disk_total) = disks.iter()
            .find(|d| d.mount_point() == std::path::Path::new("/"))
            .map(|d| (d.total_space() - d.available_space(), d.total_space()))
            .unwrap_or((0, 1));

        // Network: sample twice with a short delay to get rate
        let mut nets = sysinfo::Networks::new_with_refreshed_list();
        std::thread::sleep(Duration::from_millis(200));
        nets.refresh(true);
        let (net_iface, net_rx_bps, net_tx_bps) = nets.iter()
            .find(|(n, _)| {
                let n = n.as_str();
                !n.starts_with("lo") && !n.starts_with("docker")
                    && !n.starts_with("virbr") && !n.starts_with("br-")
            })
            .map(|(n, d)| (n.clone(), d.received(), d.transmitted()))
            .unwrap_or_else(|| (String::new(), 0, 0));

        let comps = sysinfo::Components::new_with_refreshed_list();
        let temp = comps.iter()
            .find(|c| {
                let l = c.label().to_lowercase();
                l.contains("core 0") || l.contains("cpu temp")
                    || l.contains("tdie") || l.contains("package id")
            })
            .and_then(|c| c.temperature());

        SysInfo {
            cpu_pct, ram_used, ram_total,
            swap_used, swap_total,
            disk_used, disk_total,
            net_iface, net_rx_bps, net_tx_bps,
            uptime_secs: uptime, temp_celsius: temp,
            load_1: load.one as f32, load_5: load.five as f32, load_15: load.fifteen as f32,
        }
    })
    .await
    .unwrap_or_else(|_| SysInfo {
        cpu_pct: 0.0, ram_used: 0, ram_total: 0,
        swap_used: 0, swap_total: 0,
        disk_used: 0, disk_total: 1,
        net_iface: String::new(), net_rx_bps: 0, net_tx_bps: 0,
        uptime_secs: 0, temp_celsius: None,
        load_1: 0.0, load_5: 0.0, load_15: 0.0,
    });

    let SysInfo {
        cpu_pct, ram_used, ram_total,
        swap_used, swap_total,
        disk_used, disk_total,
        net_iface, net_rx_bps, net_tx_bps,
        uptime_secs, temp_celsius,
        load_1, load_5, load_15,
    } = info;

    // Parallel async reads for everything else.
    let (vol_out, bright, bat, title_out, artist_out, status_out, upd_out, gpu_out, bt_out, weather_out) = tokio::join!(
        tokio::process::Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output(),
        read_brightness(),
        tokio::task::spawn_blocking(read_battery),
        tokio::process::Command::new("playerctl")
            .args(["metadata", "--format", "{{title}}"])
            .output(),
        tokio::process::Command::new("playerctl")
            .args(["metadata", "--format", "{{artist}}"])
            .output(),
        tokio::process::Command::new("playerctl")
            .args(["status"])
            .output(),
        tokio::process::Command::new("checkupdates").output(),
        read_gpu(),
        read_bluetooth(),
        read_weather(weather_location),
    );

    // Volume: "Volume: 0.60" or "Volume: 0.60 [MUTED]"
    let (volume, volume_muted) = vol_out.ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            let muted = s.contains("[MUTED]");
            let vol = s.split_whitespace().nth(1)?.parse::<f32>().ok()?;
            Some((Some(vol), muted))
        })
        .unwrap_or((None, false));

    let media_title = title_out.ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let media_artist = artist_out.ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let media_playing = status_out.ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .map(|s| s == "Playing")
        .unwrap_or(false);

    let (battery_pct, battery_charging) = bat.unwrap_or_default();

    let update_count = upd_out.ok().map(|o| {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count() as u32
    });

    let (gpu_percent, gpu_temp, gpu_mem_used, gpu_mem_total) = gpu_out;

    let (bt_connected, bt_device_name) = bt_out;

    let weather_text = weather_out;

    DashSnapshot {
        cpu_pct, ram_used, ram_total,
        swap_used, swap_total,
        disk_used, disk_total,
        net_iface, net_rx_bps, net_tx_bps,
        volume, volume_muted, brightness: bright,
        battery_pct, battery_charging, uptime_secs, temp_celsius,
        media_title, media_artist, media_playing, update_count,
        load_1, load_5, load_15,
        gpu_percent, gpu_temp, gpu_mem_used, gpu_mem_total,
        bt_connected, bt_device_name,
        weather_text,
        // History buffers start empty — populated by Dashboard::merge_snapshot
        cpu_history: VecDeque::new(),
        net_rx_history: VecDeque::new(),
    }
}

fn read_battery() -> (Option<u8>, bool) {
    let dir = std::path::Path::new("/sys/class/power_supply");
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            if e.file_name().to_string_lossy().to_uppercase().starts_with("BAT") {
                let p = e.path();
                let pct = std::fs::read_to_string(p.join("capacity"))
                    .ok().and_then(|s| s.trim().parse::<u8>().ok());
                let status = std::fs::read_to_string(p.join("status"))
                    .ok().map(|s| s.trim().to_string()).unwrap_or_default();
                let charging = matches!(status.as_str(), "Charging" | "Full");
                return (pct, charging);
            }
        }
    }
    (None, false)
}

async fn read_brightness() -> Option<u8> {
    let dir = std::path::Path::new("/sys/class/backlight");
    if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
        if let Ok(Some(e)) = entries.next_entry().await {
            let p = e.path();
            let cur: u64 = tokio::fs::read_to_string(p.join("brightness"))
                .await.ok()?.trim().parse().ok()?;
            let max: u64 = tokio::fs::read_to_string(p.join("max_brightness"))
                .await.ok()?.trim().parse().ok()?;
            if max > 0 {
                return Some(((cur as f64 / max as f64 * 100.0).round()) as u8);
            }
        }
    }
    None
}

/// Try nvidia-smi first, then radeontop (AMD) for GPU stats.
async fn read_gpu() -> (Option<f32>, Option<f32>, Option<u64>, Option<u64>) {
    // Try nvidia-smi
    if let Ok(out) = tokio::process::Command::new("nvidia-smi")
        .args(["--query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total",
               "--format=csv,noheader,nounits"])
        .output()
        .await
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = s.trim().split(',').map(str::trim).collect();
            if parts.len() == 4 {
                let pct  = parts[0].parse::<f32>().ok();
                let temp = parts[1].parse::<f32>().ok();
                let used  = parts[2].parse::<u64>().ok().map(|m| m * 1_048_576);
                let total = parts[3].parse::<u64>().ok().map(|m| m * 1_048_576);
                return (pct, temp, used, total);
            }
        }
    }

    // Try AMD via reading sysfs
    let amdgpu_path = std::path::Path::new("/sys/class/drm/card0/device");
    let gpu_pct = tokio::fs::read_to_string(amdgpu_path.join("gpu_busy_percent"))
        .await.ok()
        .and_then(|s| s.trim().parse::<f32>().ok());

    let gpu_temp = tokio::fs::read_to_string(amdgpu_path.join("hwmon/hwmon0/temp1_input"))
        .await.ok()
        .and_then(|s| s.trim().parse::<f32>().ok())
        .map(|m| m / 1000.0); // millidegrees to degrees

    (gpu_pct, gpu_temp, None, None)
}

/// Query bluetooth via bluetoothctl.
async fn read_bluetooth() -> (bool, Option<String>) {
    let out = tokio::process::Command::new("bluetoothctl")
        .args(["info"])
        .output()
        .await;

    if let Ok(o) = out {
        let s = String::from_utf8_lossy(&o.stdout);
        if s.contains("Connected: yes") {
            let name = s.lines()
                .find(|l| l.trim_start().starts_with("Name:"))
                .map(|l| l.trim_start().trim_start_matches("Name:").trim().to_string());
            return (true, name);
        }
    }
    (false, None)
}

/// Fetch weather from wttr.in using curl — no new dependency.
async fn read_weather(location: String) -> String {
    if location.is_empty() {
        return String::new();
    }
    let url = format!("https://wttr.in/{location}?format=3");
    let out = tokio::process::Command::new("curl")
        .args(["--silent", "--max-time", "5", &url])
        .output()
        .await;

    out.ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
}

// ── Sparkline canvas ──────────────────────────────────────────────────────────

/// A mini sparkline chart rendered via iced canvas.
struct Sparkline<'a> {
    history:   &'a VecDeque<f32>,
    color:     Color,
    width:     f32,
    height:    f32,
}

impl<'a> canvas::Program<Message> for Sparkline<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let _ = (renderer, self.width, self.height); // suppress unused warnings
        if self.history.len() < 2 {
            return vec![];
        }

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;
        let n = self.history.len() as f32;

        // Find max for normalization — always at least 1.0 to avoid div/zero
        let max_val = self.history.iter().cloned().fold(1.0_f32, f32::max);

        let x_step = w / (n - 1.0);

        // Build path for filled area under sparkline
        let mut path_builder = canvas::path::Builder::new();

        // Start at bottom-left
        path_builder.move_to(iced::Point::new(0.0, h));

        // Draw each data point
        for (i, &val) in self.history.iter().enumerate() {
            let x = i as f32 * x_step;
            let y = h - (val / max_val * h).clamp(0.0, h);
            path_builder.line_to(iced::Point::new(x, y));
        }

        // Close back to bottom-right then bottom-left
        let last_x = (self.history.len() - 1) as f32 * x_step;
        path_builder.line_to(iced::Point::new(last_x, h));
        path_builder.close();

        let fill_color = Color { a: 0.30, ..self.color };
        frame.fill(&path_builder.build(), canvas::Fill {
            style: canvas::Style::Solid(fill_color),
            ..Default::default()
        });

        // Draw the line on top
        let mut line_builder = canvas::path::Builder::new();
        for (i, &val) in self.history.iter().enumerate() {
            let x = i as f32 * x_step;
            let y = h - (val / max_val * h).clamp(0.0, h);
            if i == 0 {
                line_builder.move_to(iced::Point::new(x, y));
            } else {
                line_builder.line_to(iced::Point::new(x, y));
            }
        }
        let line_color = Color { a: 0.85, ..self.color };
        frame.stroke(
            &line_builder.build(),
            canvas::Stroke {
                style: canvas::Style::Solid(line_color),
                width: 1.5,
                ..Default::default()
            },
        );

        vec![frame.into_geometry()]
    }
}

// ── Noise overlay canvas ──────────────────────────────────────────────────────

/// Pseudo-random film-grain dots drawn over the overlay background.
/// Uses a fixed LCG seed so the pattern never flickers between frames.
struct NoiseOverlay;

impl canvas::Program<Message> for NoiseOverlay {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let mut seed: u32 = 0xdead_beef;
        let dot_color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.018 };
        for _ in 0..300 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let x = (seed >> 17) as f32 / 32768.0 * bounds.width;
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let y = (seed >> 17) as f32 / 32768.0 * bounds.height;
            let dot = canvas::Path::circle(iced::Point { x, y }, 0.6);
            frame.fill(&dot, canvas::Fill {
                style: canvas::Style::Solid(dot_color),
                ..Default::default()
            });
        }

        vec![frame.into_geometry()]
    }
}

// ── Message ───────────────────────────────────────────────────────────────────

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    SysReady(DashSnapshot),
    Dismiss,
    VolumeSet(f32),
    BrightnessSet(u8),
    MediaAction(&'static str),
    PowerAction(&'static str),
    AnimFrame,
    KeyEvent(iced::keyboard::Event),
}

// ── State ─────────────────────────────────────────────────────────────────────

struct Dashboard {
    theme:            Theme,
    dash_config:      DashboardConfig,
    lock_command:     String,
    weather_location: String,
    sys:              DashSnapshot,
    eq_tick:          u64,
    /// Entrance animation progress: 0.0 (hidden) → 1.0 (fully revealed).
    /// Incremented each AnimFrame tick by DT_PER_FRAME, reaching 1.0 in ~300ms.
    intro_t:          f32,
}

/// Per-tick increment so intro_t reaches 1.0 in ~18 ticks (~300ms at 60fps).
const INTRO_DT: f32 = 1.0 / 18.0;

impl Dashboard {
    fn new() -> (Self, Task<Message>) {
        let config           = load_config(default_path()).unwrap_or_default();
        let theme            = Theme::from_config(&config.theme);
        let dash_config      = config.dashboard.clone();
        let lock_command     = config.lock_command.clone();
        let weather_location = config.weather_location.clone();

        let loc = weather_location.clone();
        let dash = Self {
            theme, dash_config, lock_command, weather_location,
            sys: DashSnapshot::default(),
            eq_tick: 0,
            intro_t: 0.0,
        };
        let task = Task::perform(
            async move { read_sys_snapshot(loc).await },
            Message::SysReady,
        );
        (dash, task)
    }

    fn namespace() -> String {
        "bar-dashboard".to_string()
    }

    /// Merge a fresh snapshot into `self.sys` — preserving the rolling history buffers.
    fn merge_snapshot(&mut self, mut snap: DashSnapshot) {
        const MAX_HISTORY: usize = 60;

        // Carry over the existing history from the stored snapshot
        let mut cpu_hist = std::mem::take(&mut self.sys.cpu_history);
        let mut rx_hist  = std::mem::take(&mut self.sys.net_rx_history);

        cpu_hist.push_back(snap.cpu_pct);
        if cpu_hist.len() > MAX_HISTORY { cpu_hist.pop_front(); }

        rx_hist.push_back(snap.net_rx_bps as f32);
        if rx_hist.len() > MAX_HISTORY { rx_hist.pop_front(); }

        snap.cpu_history    = cpu_hist;
        snap.net_rx_history = rx_hist;

        self.sys = snap;
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::SysReady(snap) => { self.merge_snapshot(snap); }
            Message::Dismiss => std::process::exit(0),
            Message::KeyEvent(iced::keyboard::Event::KeyPressed { key, .. }) => {
                if key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) {
                    std::process::exit(0);
                }
            }
            Message::VolumeSet(v) => {
                let clamped = v.clamp(0.0, 1.5);
                self.sys.volume = Some(clamped);
                let arg = format!("{clamped:.2}");
                tokio::spawn(async move {
                    let _ = tokio::process::Command::new("wpctl")
                        .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &arg])
                        .output().await;
                });
            }
            Message::BrightnessSet(pct) => {
                self.sys.brightness = Some(pct);
                tokio::spawn(async move {
                    let _ = tokio::process::Command::new("brightnessctl")
                        .args(["set", &format!("{pct}%")])
                        .output().await;
                });
            }
            Message::MediaAction(cmd) => {
                if cmd == "play-pause" {
                    self.sys.media_playing = !self.sys.media_playing;
                }
                tokio::spawn(async move {
                    let _ = tokio::process::Command::new("playerctl").arg(cmd).output().await;
                });
            }
            Message::PowerAction(action) => {
                let cmd_str = match action {
                    "lock"      => self.lock_command.clone(),
                    "sleep"     => "systemctl suspend".to_string(),
                    "hibernate" => "systemctl hibernate".to_string(),
                    "logout"    => "hyprctl dispatch exit".to_string(),
                    "reboot"    => "systemctl reboot".to_string(),
                    "shutdown"  => "systemctl poweroff".to_string(),
                    _           => return Task::none(),
                };
                let mut parts = cmd_str.split_whitespace();
                if let Some(prog) = parts.next() {
                    let args: Vec<String> = parts.map(String::from).collect();
                    let _ = std::process::Command::new(prog).args(&args).spawn();
                }
                std::process::exit(0);
            }
            Message::AnimFrame => {
                self.eq_tick = self.eq_tick.wrapping_add(1);
                // Advance entrance animation until fully visible
                if self.intro_t < 1.0 {
                    self.intro_t = (self.intro_t + INTRO_DT).min(1.0);
                }
            }
            _ => {}
        }
        Task::none()
    }

    // ── View ──────────────────────────────────────────────────────────────────

    fn view(&self) -> Element<'_, Message> {
        let t     = &self.theme;
        let fg    = t.foreground.to_iced();
        let fsize = t.font_size;

        // Improved overlay background: use theme background color (tinted dark),
        // rather than pure black, for a cohesive frosted-glass feel.
        let bg_iced = t.background.to_iced();
        let overlay_bg = Color {
            r: bg_iced.r * 0.55,
            g: bg_iced.g * 0.55,
            b: bg_iced.b * 0.55,
            a: 0.88,
        };

        // Span-aware bento grid — wide cards (clock/media/power) span 2 columns.
        let cols = self.dash_config.columns.clamp(2, 4) as usize;
        // Increased gap for better whitespace between cards
        let gap  = 18.0f32;

        let mut grid_rows: Vec<Element<'_, Message>> = Vec::new();
        let mut row_items: Vec<Element<'_, Message>> = Vec::new();
        let mut row_span = 0usize;
        let mut card_idx = 0usize;

        for item in &self.dash_config.items {
            let kind = item.kind.as_str();
            // Use col_span from config if > 1, otherwise fall back to card_span() default.
            let span = if item.col_span > 1 {
                (item.col_span as usize).min(cols)
            } else {
                card_span(kind).min(cols)
            };
            if row_span + span > cols && !row_items.is_empty() {
                grid_rows.push(
                    iced::widget::Row::from_vec(std::mem::take(&mut row_items))
                        .spacing(gap).align_y(Alignment::Center).into(),
                );
                row_span = 0;
            }
            if let Some(card) = self.make_card(kind, span, card_idx) {
                row_items.push(card);
                row_span += span;
                card_idx += 1;
            }
            if row_span >= cols {
                grid_rows.push(
                    iced::widget::Row::from_vec(std::mem::take(&mut row_items))
                        .spacing(gap).align_y(Alignment::Center).into(),
                );
                row_span = 0;
            }
        }
        if !row_items.is_empty() {
            grid_rows.push(
                iced::widget::Row::from_vec(row_items)
                    .spacing(gap).align_y(Alignment::Center).into(),
            );
        }

        let grid = iced::widget::Column::from_vec(grid_rows)
            .spacing(gap)
            .align_x(Alignment::Center);

        // ESC hint — pill-style keyboard chip followed by muted label text
        let hint_col = Color { a: 0.38, ..fg };
        let esc_chip = container(
            text("ESC")
                .size(fsize - 3.0)
                .color(Color { a: 0.65, ..fg }),
        )
        .padding(iced::Padding { top: 3.0, right: 8.0, bottom: 3.0, left: 8.0 })
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color { a: 0.15, ..fg })),
            border: Border {
                radius: 4.0.into(),
                color: Color { a: 0.25, ..fg },
                width: 1.0,
            },
            ..Default::default()
        });
        let hint = row![
            esc_chip,
            text(" to close").size(fsize - 4.0).color(hint_col),
        ]
        .align_y(Alignment::Center)
        .spacing(0.0);

        let content_col = column![grid, hint]
            .spacing(28.0)
            .align_x(Alignment::Center);

        let inner_container = container(content_col)
            .width(Length::Fill).height(Length::Fill)
            .align_x(Alignment::Center).align_y(Alignment::Center)
            .padding(iced::Padding { top: 32.0, right: 48.0, bottom: 32.0, left: 48.0 });

        let noise_canvas = canvas(NoiseOverlay)
            .width(Length::Fill)
            .height(Length::Fill);

        container(
            stack![noise_canvas, inner_container],
        )
        .width(Length::Fill).height(Length::Fill)
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(overlay_bg)),
            ..Default::default()
        })
        .into()
    }

    // ── Card builder ──────────────────────────────────────────────────────────

    /// Returns a staggered per-card opacity based on card index and global intro_t.
    /// Card i starts animating when intro_t > i * 0.08, reaching 1.0 at intro_t = i * 0.08 + 0.35.
    fn card_opacity(&self, card_idx: usize) -> f32 {
        let start = (card_idx as f32) * 0.08;
        let end   = start + 0.35;
        let t     = self.intro_t;
        if t <= start {
            0.0
        } else if t >= end {
            1.0
        } else {
            // EaseOutCubic: smooth deceleration into final position
            let p = (t - start) / (end - start);
            1.0 - (1.0 - p).powi(3)
        }
    }

    fn make_card(&self, item: &str, span: usize, card_idx: usize) -> Option<Element<'_, Message>> {
        let t      = &self.theme;
        let fsize  = t.font_size;
        let fg     = t.foreground.to_iced();
        let accent = t.accent.to_iced();
        let theme  = self.dash_config.theme.as_str();
        let nerd   = t.use_nerd_icons;

        // Per-card entrance opacity (staggered)
        let opacity = self.card_opacity(card_idx);

        // Bold font for primary values
        let bold_font = Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        };

        // Base card size per theme
        let (base_w, base_h) = match theme {
            "minimal"        => (130.0f32, 66.0f32),
            "full" | "vivid" => (175.0f32, 128.0f32),
            _                => (172.0f32, 118.0f32),
        };
        let gap = 18.0f32;
        let card_w = if span >= 2 { base_w * span as f32 + gap * (span - 1) as f32 } else { base_w };
        let card_h = match item {
            "clock" | "media" => base_h * 1.25,
            "load"            => base_h * 1.10,
            _                 => base_h,
        };

        let bg_iced = t.background.to_iced();
        // Glassmorphism: lighter card bg with moderate transparency
        let card_bg_base = Color {
            r: (bg_iced.r + 0.06).min(1.0),
            g: (bg_iced.g + 0.06).min(1.0),
            b: (bg_iced.b + 0.08).min(1.0),
            a: 0.75 * opacity,
        };
        // Per-item semantic tint — barely perceptible, just enough to hint at
        // the card's "zone" without clashing with the overall palette.
        let card_bg = match item {
            "cpu" | "temperature" => Color {
                r: (card_bg_base.r + 0.02).min(1.0),
                ..card_bg_base
            },
            "memory" | "swap" => Color {
                b: (card_bg_base.b + 0.02).min(1.0),
                ..card_bg_base
            },
            "network" => Color {
                b: (card_bg_base.b + 0.015).min(1.0),
                g: (card_bg_base.g + 0.005).min(1.0),
                ..card_bg_base
            },
            "battery" => Color {
                g: (card_bg_base.g + 0.01).min(1.0),
                ..card_bg_base
            },
            _ => card_bg_base,
        };
        let bar_w = card_w - 44.0;

        // Muted label color for typography hierarchy (dimmer)
        let label_col = Color { a: 0.55 * opacity, ..fg };
        // Secondary text — slightly brighter than label
        let sec_col = Color { a: 0.70 * opacity, ..fg };
        // Primary value color (full opacity modulated by intro animation)
        let val_col = Color { a: opacity, ..fg };

        let (inner, card_color): (Element<'_, Message>, Color) = match item {

            // ── Clock ─────────────────────────────────────────────────────────
            "clock" => {
                let now = chrono::Local::now();
                let time_str = now.format(t.clock_format.as_str()).to_string();
                let date_str = now.format(t.date_format.as_str()).to_string();
                let accent_cap = accent;
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(time_str).size(fsize + 4.0).color(val_col),
                    ].into()
                } else if theme == "full" || theme == "vivid" {
                    let accent_line = container(iced::widget::Space::new())
                        .width(Length::Fixed(48.0))
                        .height(Length::Fixed(2.0))
                        .style(move |_: &iced::Theme| iced::widget::container::Style {
                            background: Some(Background::Color(Color {
                                a: 0.6 * opacity,
                                ..accent_cap
                            })),
                            border: Border { radius: 1.0.into(), ..Default::default() },
                            ..Default::default()
                        });
                    column![
                        text(time_str)
                            .size(fsize + 14.0)
                            .font(bold_font)
                            .color(Color { a: opacity, ..fg }),
                        text(date_str).size(fsize - 1.0).color(sec_col),
                        accent_line,
                    ].spacing(4.0).align_x(Alignment::Center).into()
                } else {
                    column![
                        text(time_str)
                            .size(fsize + 14.0)
                            .font(bold_font)
                            .color(Color { a: opacity, ..fg }),
                        text(date_str).size(fsize - 1.0).color(sec_col),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, accent)
            }

            // ── Network ───────────────────────────────────────────────────────
            "network" => {
                let blue = Color::from_rgba(0.54, 0.71, 0.98, opacity);
                let iface = if self.sys.net_iface.is_empty() {
                    "No network".to_string()
                } else {
                    self.sys.net_iface.clone()
                };
                let icon = if nerd { "\u{f05a9}" } else { "NET" };
                let rx_str = format!("↓ {}", fmt_bytes(self.sys.net_rx_bps));
                let tx_str = format!("↑ {}", fmt_bytes(self.sys.net_tx_bps));

                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(blue),
                        text(iface).size(fsize - 1.0).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else if theme == "full" || theme == "vivid" {
                    let spark: Element<'_, Message> = if self.sys.net_rx_history.len() >= 2 {
                        canvas(Sparkline {
                            history: &self.sys.net_rx_history,
                            color:   blue,
                            width:   bar_w,
                            height:  28.0,
                        })
                        .width(Length::Fixed(bar_w))
                        .height(Length::Fixed(28.0))
                        .into()
                    } else {
                        iced::widget::Space::new().height(Length::Fixed(28.0)).into()
                    };
                    column![
                        text(icon).size(fsize + 10.0).color(blue),
                        text(iface).size(fsize - 2.0).color(label_col),
                        text(rx_str).size(fsize - 1.0).font(bold_font).color(val_col),
                        text(tx_str).size(fsize - 2.5).color(sec_col),
                        spark,
                    ].spacing(4.0).align_x(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(blue),
                        text(iface).size(fsize - 2.0).color(label_col),
                        text(rx_str).size(fsize - 1.0).font(bold_font).color(val_col),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, blue)
            }

            // ── Battery ───────────────────────────────────────────────────────
            "battery" => {
                let pct = self.sys.battery_pct?;
                let charging = self.sys.battery_charging;
                let warn = t.battery_warn_percent;
                let fill_col = if charging {
                    Color::from_rgba(0.67, 0.88, 0.63, opacity)
                } else if pct < warn {
                    Color::from_rgba(0.96, 0.54, 0.67, opacity)
                } else {
                    Color { a: 0.85 * opacity, ..fg }
                };
                let icon = if charging {
                    if nerd { "\u{f0e7}" } else { "⚡" }
                } else if nerd { "\u{f0079}" } else { "BAT" };
                let frac = pct as f32 / 100.0;
                let pct_str = format!("{pct}%");
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(fill_col),
                        text(pct_str).size(fsize).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(fill_col),
                        text("Battery").size(fsize - 2.0).color(label_col),
                        text(pct_str).size(fsize + 4.0).font(bold_font).color(fill_col),
                        self.mini_bar(frac, fill_col, fg, bar_w),
                    ].spacing(6.0).align_x(Alignment::Center).into()
                };
                (content, fill_col)
            }

            // ── CPU ───────────────────────────────────────────────────────────
            "cpu" => {
                let frac = self.sys.cpu_pct / 100.0;
                let cpu_col = lerp_color(
                    Color::from_rgba(0.67, 0.88, 0.63, opacity),
                    Color::from_rgba(0.96, 0.54, 0.67, opacity),
                    (frac * 2.0 - 1.0).max(0.0),
                );
                let icon = if nerd { "\u{f4bc}" } else { "CPU" };
                let val  = format!("{:.0}%", self.sys.cpu_pct);

                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(cpu_col),
                        text(val).size(fsize).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else if theme == "full" || theme == "vivid" {
                    let spark: Element<'_, Message> = if self.sys.cpu_history.len() >= 2 {
                        canvas(Sparkline {
                            history: &self.sys.cpu_history,
                            color:   cpu_col,
                            width:   bar_w,
                            height:  28.0,
                        })
                        .width(Length::Fixed(bar_w))
                        .height(Length::Fixed(28.0))
                        .into()
                    } else {
                        iced::widget::Space::new().height(Length::Fixed(28.0)).into()
                    };
                    column![
                        text(icon).size(fsize + 10.0).color(cpu_col),
                        text("CPU").size(fsize - 2.0).color(label_col),
                        text(val).size(fsize + 4.0).font(bold_font).color(cpu_col),
                        self.mini_bar(frac, cpu_col, fg, bar_w),
                        spark,
                    ].spacing(4.0).align_x(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(cpu_col),
                        text("CPU").size(fsize - 2.0).color(label_col),
                        text(val).size(fsize + 4.0).font(bold_font).color(cpu_col),
                        self.mini_bar(frac, cpu_col, fg, bar_w),
                    ].spacing(6.0).align_x(Alignment::Center).into()
                };
                (content, cpu_col)
            }

            // ── Memory ────────────────────────────────────────────────────────
            "memory" => {
                let frac = if self.sys.ram_total > 0 {
                    self.sys.ram_used as f32 / self.sys.ram_total as f32
                } else { 0.0 };
                let mem_col = Color::from_rgba(0.79, 0.65, 0.97, opacity);
                let icon = if nerd { "\u{f035b}" } else { "RAM" };
                let val  = fmt_bytes(self.sys.ram_used);
                let sub  = format!("/ {}", fmt_bytes(self.sys.ram_total));
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(mem_col),
                        text(format!("{val} {sub}")).size(fsize - 1.0).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(mem_col),
                        text("Memory").size(fsize - 2.0).color(label_col),
                        text(val).size(fsize + 4.0).font(bold_font).color(mem_col),
                        text(sub).size(fsize - 2.0).color(sec_col),
                        self.mini_bar(frac, mem_col, fg, bar_w),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, mem_col)
            }

            // ── Swap ──────────────────────────────────────────────────────────
            "swap" => {
                if self.sys.swap_total == 0 { return None; }
                let frac = self.sys.swap_used as f32 / self.sys.swap_total as f32;
                let swap_col = Color::from_rgba(0.96, 0.69, 0.98, opacity);
                let icon = if nerd { "\u{f0552}" } else { "SWP" };
                let val  = format!("{} / {}", fmt_bytes(self.sys.swap_used), fmt_bytes(self.sys.swap_total));
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(swap_col),
                        text(val).size(fsize - 1.0).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(swap_col),
                        text("Swap").size(fsize - 2.0).color(label_col),
                        text(val).size(fsize - 1.0).font(bold_font).color(val_col),
                        self.mini_bar(frac, swap_col, fg, bar_w),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, swap_col)
            }

            // ── Load average ──────────────────────────────────────────────────
            "load" => {
                let load_col = Color::from_rgba(0.98, 0.81, 0.68, opacity);
                let icon = if nerd { "\u{f080}" } else { "LOAD" };
                let l1  = format!("{:.2}", self.sys.load_1);
                let l5  = format!("{:.2}", self.sys.load_5);
                let l15 = format!("{:.2}", self.sys.load_15);

                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(load_col),
                        text(l1.clone()).size(fsize).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(load_col),
                        text("Load avg").size(fsize - 2.0).color(label_col),
                        row![
                            column![
                                text("1m").size(fsize - 3.0).color(sec_col),
                                text(l1).size(fsize - 0.5).font(bold_font).color(val_col),
                            ].align_x(Alignment::Center).spacing(2.0),
                            column![
                                text("5m").size(fsize - 3.0).color(sec_col),
                                text(l5).size(fsize - 0.5).font(bold_font).color(val_col),
                            ].align_x(Alignment::Center).spacing(2.0),
                            column![
                                text("15m").size(fsize - 3.0).color(sec_col),
                                text(l15).size(fsize - 0.5).font(bold_font).color(val_col),
                            ].align_x(Alignment::Center).spacing(2.0),
                        ].spacing(12.0).align_y(Alignment::Center),
                    ].spacing(6.0).align_x(Alignment::Center).into()
                };
                (content, load_col)
            }

            // ── GPU ───────────────────────────────────────────────────────────
            "gpu" => {
                let pct = self.sys.gpu_percent?;
                let frac = pct / 100.0;
                let gpu_col = Color::from_rgba(0.54, 0.87, 0.75, opacity);
                let icon = if nerd { "\u{f071b}" } else { "GPU" };

                let pct_str  = format!("{pct:.0}%");
                let temp_str = self.sys.gpu_temp
                    .map(|t| format!("{t:.0}°C"))
                    .unwrap_or_default();
                let mem_str = match (self.sys.gpu_mem_used, self.sys.gpu_mem_total) {
                    (Some(u), Some(t)) if t > 0 => {
                        format!("{} / {}", fmt_bytes(u), fmt_bytes(t))
                    }
                    _ => String::new(),
                };

                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(gpu_col),
                        text(pct_str.clone()).size(fsize).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    let mut items: Vec<Element<'_, Message>> = vec![
                        text(icon).size(fsize + 10.0).color(gpu_col).into(),
                        text("GPU").size(fsize - 2.0).color(label_col).into(),
                        text(pct_str).size(fsize + 4.0).font(bold_font).color(gpu_col).into(),
                        self.mini_bar(frac, gpu_col, fg, bar_w),
                    ];
                    if !temp_str.is_empty() {
                        items.push(
                            text(temp_str).size(fsize - 2.0).color(sec_col).into()
                        );
                    }
                    if !mem_str.is_empty() {
                        items.push(
                            text(mem_str).size(fsize - 2.5).color(sec_col).into()
                        );
                    }
                    iced::widget::Column::from_vec(items)
                        .spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, gpu_col)
            }

            // ── Disk ──────────────────────────────────────────────────────────
            "disk" => {
                let frac = if self.sys.disk_total > 0 {
                    self.sys.disk_used as f32 / self.sys.disk_total as f32
                } else { 0.0 };
                let disk_col = Color::from_rgba(0.98, 0.89, 0.68, opacity);
                let icon = if nerd { "\u{f01bc}" } else { "DSK" };
                let val  = fmt_bytes(self.sys.disk_used);
                let sub  = format!("/ {}", fmt_bytes(self.sys.disk_total));
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(disk_col),
                        text(format!("{val} {sub}")).size(fsize - 1.0).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(disk_col),
                        text("Disk").size(fsize - 2.0).color(label_col),
                        text(val).size(fsize + 4.0).font(bold_font).color(disk_col),
                        text(sub).size(fsize - 2.0).color(sec_col),
                        self.mini_bar(frac, disk_col, fg, bar_w),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, disk_col)
            }

            // ── Volume ────────────────────────────────────────────────────────
            "volume" => {
                let vol     = self.sys.volume.unwrap_or(0.0);
                let _frac   = (vol / 1.5).clamp(0.0, 1.0);
                let vol_col = Color::from_rgba(0.58, 0.89, 0.84, opacity);
                let icon = if self.sys.volume_muted {
                    if nerd { "\u{f075f}" } else { "M" }
                } else if nerd { "\u{f057e}" } else { "V" };
                let val = format!("{:.0}%", vol * 100.0);
                let vol_cap = vol_col;
                let fg_cap  = fg;
                let slider_elem: Element<'_, Message> = if theme != "minimal" {
                    iced::widget::slider(0.0f32..=1.5, vol, Message::VolumeSet)
                        .width(Length::Fixed(bar_w))
                        .style(move |_: &iced::Theme, _| iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail {
                                backgrounds: (
                                    Background::Color(Color { a: 0.85, ..vol_cap }),
                                    Background::Color(Color { a: 0.15, ..fg_cap }),
                                ),
                                width: 4.0,
                                border: Border { radius: 99.0.into(), ..Default::default() },
                            },
                            handle: iced::widget::slider::Handle {
                                shape: iced::widget::slider::HandleShape::Circle { radius: 0.0 },
                                background: Background::Color(Color::TRANSPARENT),
                                border_color: Color::TRANSPARENT,
                                border_width: 0.0,
                            },
                        })
                        .into()
                } else {
                    iced::widget::Space::new().height(Length::Fixed(0.0)).into()
                };
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(vol_col),
                        text(val).size(fsize).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(vol_col),
                        text("Volume").size(fsize - 2.0).color(label_col),
                        text(val).size(fsize + 4.0).font(bold_font).color(vol_col),
                        slider_elem,
                    ].spacing(6.0).align_x(Alignment::Center).into()
                };
                (content, vol_col)
            }

            // ── Brightness ────────────────────────────────────────────────────
            "brightness" => {
                let bright  = self.sys.brightness.unwrap_or(50);
                let _frac   = bright as f32 / 100.0;
                let br_col  = Color::from_rgba(0.98, 0.89, 0.55, opacity);
                let icon = if nerd { "\u{f00e0}" } else { "BRT" };
                let val  = format!("{bright}%");
                let br_cap = br_col;
                let fg_cap = fg;
                let slider_elem: Element<'_, Message> = if theme != "minimal" {
                    iced::widget::slider(
                        0.0f32..=100.0,
                        bright as f32,
                        |v| Message::BrightnessSet(v.round() as u8),
                    )
                    .width(Length::Fixed(bar_w))
                    .style(move |_: &iced::Theme, _| iced::widget::slider::Style {
                        rail: iced::widget::slider::Rail {
                            backgrounds: (
                                Background::Color(Color { a: 0.85, ..br_cap }),
                                Background::Color(Color { a: 0.15, ..fg_cap }),
                            ),
                            width: 4.0,
                            border: Border { radius: 99.0.into(), ..Default::default() },
                        },
                        handle: iced::widget::slider::Handle {
                            shape: iced::widget::slider::HandleShape::Circle { radius: 0.0 },
                            background: Background::Color(Color::TRANSPARENT),
                            border_color: Color::TRANSPARENT,
                            border_width: 0.0,
                        },
                    })
                    .into()
                } else {
                    iced::widget::Space::new().height(Length::Fixed(0.0)).into()
                };
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(br_col),
                        text(val).size(fsize).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(br_col),
                        text("Brightness").size(fsize - 2.0).color(label_col),
                        text(val).size(fsize + 4.0).font(bold_font).color(br_col),
                        slider_elem,
                    ].spacing(6.0).align_x(Alignment::Center).into()
                };
                (content, br_col)
            }

            // ── Media ─────────────────────────────────────────────────────────
            "media" => {
                let pink = Color::from_rgba(0.96, 0.54, 0.84, opacity);
                let play_icon = if nerd {
                    if self.sys.media_playing { "\u{f03e4}" } else { "\u{f040a}" }
                } else if self.sys.media_playing { "⏸" } else { "▶" };
                let prev_icon = if nerd { "\u{f0602}" } else { "⏮" };
                let next_icon = if nerd { "\u{f0604}" } else { "⏭" };

                let title = self.sys.media_title.as_deref().unwrap_or("Nothing playing");
                let trunc = if title.len() > 20 { &title[..20] } else { title };
                let trunc = trunc.to_string();

                let artist = self.sys.media_artist.as_deref().unwrap_or("").to_string();

                // Smooth sine-wave equalizer bars
                let eq: Element<'_, Message> = if self.sys.media_playing && theme != "minimal" {
                    let tick = self.eq_tick as f32;
                    let pink_cap = pink;
                    let bars: Vec<Element<'_, Message>> = (0..5).map(|i| {
                        // Smooth sine wave: each bar has a different phase offset
                        let phase = (tick * 0.15 + i as f32 * 0.8).sin() * 0.5 + 0.5;
                        let h = 4.0 + phase * 16.0;
                        container(iced::widget::Space::new())
                            .width(Length::Fixed(4.0))
                            .height(Length::Fixed(h))
                            .style(move |_: &iced::Theme| iced::widget::container::Style {
                                background: Some(Background::Color(Color { a: 0.8, ..pink_cap })),
                                border: Border { radius: 2.0.into(), ..Default::default() },
                                ..Default::default()
                            })
                            .into()
                    }).collect();
                    iced::widget::Row::from_vec(bars)
                        .spacing(3.0)
                        .align_y(Alignment::End)
                        .into()
                } else {
                    iced::widget::Space::new().height(Length::Fixed(6.0)).into()
                };

                let fg_dim = Color { a: 0.7 * opacity, ..fg };
                let pink_cap = pink;
                let fg_dim_cap = fg_dim;
                let controls: Element<'_, Message> = row![
                    iced::widget::button(text(prev_icon).size(fsize + 2.0).color(fg_dim))
                        .on_press(Message::MediaAction("previous"))
                        .padding([2.0, 5.0])
                        .style(move |_: &iced::Theme, status| {
                            let hov = status == iced::widget::button::Status::Hovered
                                || status == iced::widget::button::Status::Pressed;
                            iced::widget::button::Style {
                                background: if hov {
                                    Some(Background::Color(Color { a: 0.15, ..pink_cap }))
                                } else { None },
                                border: Border { radius: 8.0.into(), ..Default::default() },
                                text_color: fg_dim_cap,
                                ..Default::default()
                            }
                        }),
                    iced::widget::button(text(play_icon).size(fsize + 5.0).color(pink))
                        .on_press(Message::MediaAction("play-pause"))
                        .padding([2.0, 5.0])
                        .style(move |_: &iced::Theme, status| {
                            let pressed = status == iced::widget::button::Status::Pressed;
                            let hov = status == iced::widget::button::Status::Hovered || pressed;
                            iced::widget::button::Style {
                                background: if pressed {
                                    Some(Background::Color(Color { a: 0.28, ..pink_cap }))
                                } else if hov {
                                    Some(Background::Color(Color { a: 0.18, ..pink_cap }))
                                } else { None },
                                border: Border { radius: 8.0.into(), ..Default::default() },
                                text_color: pink_cap,
                                ..Default::default()
                            }
                        }),
                    iced::widget::button(text(next_icon).size(fsize + 2.0).color(fg_dim))
                        .on_press(Message::MediaAction("next"))
                        .padding([2.0, 5.0])
                        .style(move |_: &iced::Theme, status| {
                            let hov = status == iced::widget::button::Status::Hovered
                                || status == iced::widget::button::Status::Pressed;
                            iced::widget::button::Style {
                                background: if hov {
                                    Some(Background::Color(Color { a: 0.15, ..pink_cap }))
                                } else { None },
                                border: Border { radius: 8.0.into(), ..Default::default() },
                                text_color: fg_dim_cap,
                                ..Default::default()
                            }
                        }),
                ].spacing(4.0).align_y(Alignment::Center).into();

                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(play_icon).size(fsize).color(pink),
                        text(trunc).size(fsize - 1.0).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    let mut col_items: Vec<Element<'_, Message>> = vec![
                        text(trunc).size(fsize - 1.0).font(bold_font).color(val_col).into(),
                    ];
                    if !artist.is_empty() {
                        col_items.push(
                            text(artist).size(fsize - 2.5).color(sec_col).into()
                        );
                    }
                    col_items.push(eq);
                    col_items.push(controls);
                    iced::widget::Column::from_vec(col_items)
                        .spacing(5.0).align_x(Alignment::Center).into()
                };
                (content, pink)
            }

            // ── Power quick-actions ───────────────────────────────────────────
            "power" => {
                let orange = Color::from_rgba(0.98, 0.70, 0.53, opacity);
                let actions: &[(&str, &str, &'static str)] = &[
                    ("\u{f033e}", "🔒", "lock"),
                    ("\u{f0904}", "💤", "sleep"),
                    ("\u{f0453}", "🔄", "reboot"),
                    ("\u{f0425}", "⏻",  "shutdown"),
                ];
                let fg_cap  = fg;
                let or_cap  = orange;
                let btn_sty = move |_: &iced::Theme, status: iced::widget::button::Status| {
                    let hov = status == iced::widget::button::Status::Hovered
                        || status == iced::widget::button::Status::Pressed;
                    iced::widget::button::Style {
                        background: if hov {
                            Some(Background::Color(Color { a: 0.14, ..or_cap }))
                        } else { None },
                        border: Border {
                            radius: 8.0.into(),
                            color: if hov { or_cap } else { Color { a: 0.0, ..or_cap } },
                            width: 1.0,
                        },
                        text_color: fg_cap,
                        ..Default::default()
                    }
                };
                let buttons: Vec<Element<'_, Message>> = actions.iter().map(|(ni, ai, action)| {
                    let icon = if nerd { *ni } else { *ai };
                    iced::widget::button(text(icon).size(fsize + 6.0).color(orange))
                        .on_press(Message::PowerAction(action))
                        .padding([4.0, 8.0])
                        .style(btn_sty)
                        .into()
                }).collect();

                let content: Element<'_, Message> = if theme == "minimal" {
                    iced::widget::Row::from_vec(buttons)
                        .spacing(2.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text("Power").size(fsize - 2.0).color(label_col),
                        iced::widget::Row::from_vec(buttons)
                            .spacing(2.0).align_y(Alignment::Center),
                    ].spacing(8.0).align_x(Alignment::Center).into()
                };
                (content, orange)
            }

            // ── Uptime ────────────────────────────────────────────────────────
            "uptime" => {
                let teal = Color::from_rgba(0.58, 0.89, 0.84, opacity);
                let icon = if nerd { "\u{f150e}" } else { "UP" };
                let val  = fmt_uptime(self.sys.uptime_secs);
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(teal),
                        text(val).size(fsize).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(teal),
                        text("Uptime").size(fsize - 2.0).color(label_col),
                        text(val).size(fsize + 4.0).font(bold_font).color(val_col),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, teal)
            }

            // ── Temperature ───────────────────────────────────────────────────
            "temperature" => {
                let temp = self.sys.temp_celsius?;
                let heat = ((temp - 40.0) / 50.0).clamp(0.0, 1.0);
                let temp_col = lerp_color(
                    Color::from_rgba(0.67, 0.88, 0.63, opacity),
                    Color::from_rgba(0.96, 0.54, 0.67, opacity),
                    heat,
                );
                let icon = if nerd { "\u{f050f}" } else { "TMP" };
                let val  = format!("{temp:.0}°C");
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(temp_col),
                        text(val).size(fsize).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(temp_col),
                        text("Temp").size(fsize - 2.0).color(label_col),
                        text(val).size(fsize + 4.0).font(bold_font).color(temp_col),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, temp_col)
            }

            // ── Updates ───────────────────────────────────────────────────────
            "updates" => {
                let yellow = Color::from_rgba(0.98, 0.70, 0.53, opacity);
                let icon = if nerd { "\u{f0954}" } else { "UPD" };
                let val = match self.sys.update_count {
                    Some(0) => "Up to date".to_string(),
                    Some(n) => format!("{n} updates"),
                    None    => "Checking\u{2026}".to_string(),
                };
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(yellow),
                        text(val).size(fsize - 1.0).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(yellow),
                        text("Updates").size(fsize - 2.0).color(label_col),
                        text(val).size(fsize - 1.0).font(bold_font).color(val_col),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, yellow)
            }

            // ── Bluetooth ─────────────────────────────────────────────────────
            "bluetooth" => {
                let bt_col = Color::from_rgba(0.49, 0.72, 0.97, opacity);
                let icon = if nerd { "\u{f00af}" } else { "BT" };
                let (status_str, device_str) = if self.sys.bt_connected {
                    let dev = self.sys.bt_device_name.as_deref()
                        .unwrap_or("Connected");
                    let dev_trunc = if dev.len() > 14 { &dev[..14] } else { dev };
                    ("Connected".to_string(), dev_trunc.to_string())
                } else {
                    ("Disconnected".to_string(), String::new())
                };
                let status_col = if self.sys.bt_connected {
                    bt_col
                } else {
                    Color { a: 0.40 * opacity, ..fg }
                };
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(bt_col),
                        text(status_str).size(fsize - 1.0).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    let mut items: Vec<Element<'_, Message>> = vec![
                        text(icon).size(fsize + 10.0).color(bt_col).into(),
                        text("Bluetooth").size(fsize - 2.0).color(label_col).into(),
                        text(status_str).size(fsize - 1.0).font(bold_font).color(status_col).into(),
                    ];
                    if !device_str.is_empty() {
                        items.push(
                            text(device_str).size(fsize - 2.5).color(sec_col).into()
                        );
                    }
                    iced::widget::Column::from_vec(items)
                        .spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, bt_col)
            }

            // ── Weather ───────────────────────────────────────────────────────
            "weather" => {
                // Hide if location not configured or weather not yet fetched
                if self.weather_location.is_empty() { return None; }

                let sky_col = Color::from_rgba(0.53, 0.82, 0.96, opacity);
                let icon = if nerd { "\u{f0599}" } else { "WX" };

                let (weather_main, weather_detail) = if self.sys.weather_text.is_empty() {
                    ("Fetching\u{2026}".to_string(), String::new())
                } else {
                    // wttr.in format 3 returns: "CityName: ConditionIcon Temp"
                    // e.g. "London: ⛅️  +12°C"
                    let raw = &self.sys.weather_text;
                    if let Some(colon_pos) = raw.find(':') {
                        let detail = raw[colon_pos + 1..].trim().to_string();
                        let city   = raw[..colon_pos].trim().to_string();
                        (city, detail)
                    } else {
                        (raw.clone(), String::new())
                    }
                };

                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(sky_col),
                        text(weather_main).size(fsize - 1.0).color(val_col),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    let mut items: Vec<Element<'_, Message>> = vec![
                        text(icon).size(fsize + 10.0).color(sky_col).into(),
                        text("Weather").size(fsize - 2.0).color(label_col).into(),
                        text(weather_main).size(fsize - 1.0).font(bold_font).color(val_col).into(),
                    ];
                    if !weather_detail.is_empty() {
                        items.push(
                            text(weather_detail).size(fsize - 0.5).color(sky_col).into()
                        );
                    }
                    iced::widget::Column::from_vec(items)
                        .spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, sky_col)
            }

            _ => return None,
        };

        // ── Card border & glow logic ──────────────────────────────────────────
        //
        // Normal border based on theme, then check for "danger" state to
        // override with a colored accent glow border.
        //
        let card_radius = match theme {
            "full" | "vivid" => 16.0f32,
            "minimal"        => 8.0,
            _                => 12.0,
        };

        // Detect high-value / alert state for accent glow
        let is_alert = match item {
            "cpu"         => self.sys.cpu_pct > 80.0,
            "memory"      => self.sys.ram_total > 0
                && (self.sys.ram_used as f32 / self.sys.ram_total as f32) > 0.85,
            "temperature" => self.sys.temp_celsius.map(|c| c > 75.0).unwrap_or(false),
            "battery"     => self.sys.battery_pct
                .map(|p| p < t.battery_warn_percent && !self.sys.battery_charging)
                .unwrap_or(false),
            "gpu"         => self.sys.gpu_percent.map(|p| p > 85.0).unwrap_or(false),
            _ => false,
        };

        let (border_col, border_w) = if is_alert {
            // Colored glow border for alert state
            (Color { a: 0.65 * opacity, ..card_color }, 2.0f32)
        } else {
            match theme {
                "minimal" => (Color::TRANSPARENT, 0.0f32),
                "vivid"   => (Color { a: 0.55 * opacity, ..card_color }, 1.5),
                "full"    => (Color { a: 0.30 * opacity, ..card_color }, 1.0),
                // "cards" and others — subtle white-tinted top-highlight border
                _         => (Color { a: 0.18 * opacity, r: 1.0, g: 1.0, b: 1.0 }, 1.0),
            }
        };

        // For "vivid" theme, prepend a 3px accent strip at the top of the card
        // to simulate a thick top-edge accent border (iced applies borders uniformly).
        let final_inner: Element<'_, Message> = if theme == "vivid" {
            let card_color_cap = card_color;
            let accent_strip = container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fixed(3.0))
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.65 * opacity,
                        ..card_color_cap
                    })),
                    ..Default::default()
                });
            // For vivid, reduce top padding since the accent strip takes 3px of space
            let padded_inner = container(inner)
                .padding(iced::Padding { top: 10.0, right: 16.0, bottom: 14.0, left: 16.0 });
            column![accent_strip, padded_inner]
                .spacing(0.0)
                .into()
        } else {
            // Standard inner padding so content breathes inside the card
            container(inner)
                .padding(iced::Padding { top: 14.0, right: 16.0, bottom: 14.0, left: 16.0 })
                .into()
        };

        Some(
            container(final_inner)
                .width(Length::Fixed(card_w))
                .height(Length::Fixed(card_h))
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(card_bg)),
                    border: Border {
                        radius: card_radius.into(),
                        color: border_col,
                        width: border_w,
                    },
                    ..Default::default()
                })
                .into(),
        )
    }

    // ── Mini progress bar (used by full/vivid themes) ──────────────────────────

    fn mini_bar(&self, frac: f32, fill_col: Color, fg: Color, width: f32) -> Element<'_, Message> {
        let fill_w    = (frac.clamp(0.0, 1.0) * width).max(2.0);
        let empty_w   = (width - fill_w).max(0.0);
        let fill_solid = Color { a: 0.90, ..fill_col };
        // More transparent track so the fill color reads with higher contrast
        let track_col  = Color { a: 0.12, ..fg };

        iced::widget::Row::from_vec(vec![
            container(iced::widget::Space::new())
                .width(Length::Fixed(fill_w))
                .height(Length::Fixed(6.0))
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(fill_solid)),
                    border: Border { radius: 3.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .into(),
            container(iced::widget::Space::new())
                .width(Length::Fixed(empty_w))
                .height(Length::Fixed(6.0))
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(track_col)),
                    border: Border { radius: 3.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .into(),
        ])
        .spacing(0.0)
        .into()
    }

    // ── Subscriptions ─────────────────────────────────────────────────────────

    fn subscription(&self) -> Subscription<Message> {
        // Always run at 60fps while intro animation is playing, or when media
        // is playing (for equalizer animation). Otherwise step down to 1fps.
        let tick_ms = if self.intro_t < 1.0 || self.sys.media_playing { 16 } else { 1000 };
        Subscription::batch([
            iced::keyboard::listen().map(Message::KeyEvent),
            Subscription::run(sys_stream),
            iced::time::every(Duration::from_millis(tick_ms))
                .map(|_| Message::AnimFrame),
        ])
    }

    fn style(&self, _theme: &iced::Theme) -> iced::theme::Style {
        iced::theme::Style {
            background_color: Color::TRANSPARENT,
            text_color: self.theme.foreground.to_iced(),
        }
    }
}

// ── Live update stream ────────────────────────────────────────────────────────

fn sys_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(4, |mut sender: Sender<Message>| async move {
        // Load weather_location once at stream startup
        let weather_location = {
            let config = load_config(default_path()).unwrap_or_default();
            config.weather_location
        };
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let snap = read_sys_snapshot(weather_location.clone()).await;
            let _ = sender.try_send(Message::SysReady(snap));
        }
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn fmt_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1}G", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.0}M", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.0}K", bytes as f64 / 1024.0)
    } else {
        format!("{bytes}B")
    }
}

fn fmt_uptime(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 { format!("{h}h {m:02}m") } else { format!("{m}m") }
}

/// How many grid columns this card type spans (1 = normal, 2 = wide).
fn card_span(item: &str) -> usize {
    match item {
        "clock" | "media" | "power" | "load" => 2,
        _ => 1,
    }
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}
