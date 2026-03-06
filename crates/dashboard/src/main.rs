//! `bar-dashboard` — bento-grid system info overlay.
//!
//! Launch with a Hyprland keybind:
//!   `bind = SUPER, D, exec, bar-dashboard`
//! Press Escape or click the dim background to dismiss.

use bar_config::{default_path, load as load_config, schema::DashboardConfig};
use bar_theme::Theme;
use futures::channel::mpsc::Sender;
use iced::{
    animation::{Animation, Easing},
    widget::{column, container, mouse_area, row, text},
    Alignment, Background, Border, Color, Element, Length, Subscription, Task,
};
use iced_layershell::{
    build_pattern::application,
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::{LayerShellSettings, Settings},
    to_layer_message,
};
use std::time::{Duration, Instant};

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
    disk_used:        u64,
    disk_total:       u64,
    net_iface:        String,
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
}

async fn read_sys_snapshot() -> DashSnapshot {
    // Heavy sysinfo work in a blocking thread — CPU needs 150ms between samples.
    let (cpu_pct, ram_used, ram_total, disk_used, disk_total, net_iface, uptime_secs, temp_celsius) =
        tokio::task::spawn_blocking(|| {
            use sysinfo::System;
            let mut sys = System::new();
            sys.refresh_cpu_all();
            std::thread::sleep(Duration::from_millis(150));
            sys.refresh_cpu_all();
            sys.refresh_memory();

            let cpu_pct   = sys.global_cpu_usage();
            let ram_used  = sys.used_memory();
            let ram_total = sys.total_memory();
            let uptime    = System::uptime();

            let disks = sysinfo::Disks::new_with_refreshed_list();
            let (disk_used, disk_total) = disks.iter()
                .find(|d| d.mount_point() == std::path::Path::new("/"))
                .map(|d| (d.total_space() - d.available_space(), d.total_space()))
                .unwrap_or((0, 1));

            let nets = sysinfo::Networks::new_with_refreshed_list();
            let net_iface = nets.iter()
                .find(|(n, _)| {
                    let n = n.as_str();
                    !n.starts_with("lo") && !n.starts_with("docker")
                        && !n.starts_with("virbr") && !n.starts_with("br-")
                })
                .map(|(n, _)| n.clone())
                .unwrap_or_default();

            let comps = sysinfo::Components::new_with_refreshed_list();
            let temp = comps.iter()
                .find(|c| {
                    let l = c.label().to_lowercase();
                    l.contains("core 0") || l.contains("cpu temp")
                        || l.contains("tdie") || l.contains("package id")
                })
                .and_then(|c| c.temperature());

            (cpu_pct, ram_used, ram_total, disk_used, disk_total, net_iface, uptime, temp)
        })
        .await
        .unwrap_or_default();

    // Parallel async reads for everything else.
    let (vol_out, bright, bat, title_out, artist_out, status_out, upd_out) = tokio::join!(
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

    DashSnapshot {
        cpu_pct, ram_used, ram_total, disk_used, disk_total,
        net_iface, volume, volume_muted, brightness: bright,
        battery_pct, battery_charging, uptime_secs, temp_celsius,
        media_title, media_artist, media_playing, update_count,
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
    theme:        Theme,
    dash_config:  DashboardConfig,
    lock_command: String,
    sys:          DashSnapshot,
    enter_anim:   Animation<bool>,
    eq_tick:      u64,
}

impl Dashboard {
    fn new() -> (Self, Task<Message>) {
        let config       = load_config(default_path()).unwrap_or_default();
        let theme        = Theme::from_config(&config.theme);
        let dash_config  = config.dashboard.clone();
        let lock_command = config.global.lock_command.clone();

        let mut enter_anim = Animation::new(false).slow().easing(Easing::EaseOutCubic);
        enter_anim.go_mut(true, Instant::now());

        let dash = Self {
            theme, dash_config, lock_command,
            sys: DashSnapshot::default(),
            enter_anim, eq_tick: 0,
        };
        let task = Task::perform(async { read_sys_snapshot().await }, Message::SysReady);
        (dash, task)
    }

    fn namespace() -> String {
        "bar-dashboard".to_string()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::SysReady(snap) => { self.sys = snap; }
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
            }
            _ => {}
        }
        Task::none()
    }

    // ── View ──────────────────────────────────────────────────────────────────

    fn view(&self) -> Element<'_, Message> {
        let now  = Instant::now();
        let t    = &self.theme;
        let prog = self.enter_anim.interpolate(0.0f32, 1.0f32, now);
        let slide = (1.0 - prog) * 40.0;

        let bg  = t.background;
        let fgc = t.foreground;
        let fg  = fgc.to_iced();
        let fsize = t.font_size;

        // Modal: blend 18% fg into bg → lifted look
        let mix = 0.18f32;
        let modal_bg = Color::from_rgba(
            (bg.r + (fgc.r - bg.r) * mix).clamp(0.0, 1.0),
            (bg.g + (fgc.g - bg.g) * mix).clamp(0.0, 1.0),
            (bg.b + (fgc.b - bg.b) * mix).clamp(0.0, 1.0),
            0.97 * prog,
        );
        let modal_border = Color { a: 0.22 * prog, ..fg };
        let overlay_bg   = Color::from_rgba(0.0, 0.0, 0.0, 0.72 * prog);

        // Build bento grid rows
        let cols = self.dash_config.columns.clamp(2, 4) as usize;
        let gap  = 14.0f32;

        let grid_rows: Vec<Element<'_, Message>> = self.dash_config.items
            .chunks(cols)
            .filter_map(|chunk: &[String]| {
                let row_cards: Vec<Element<'_, Message>> = chunk.iter()
                    .filter_map(|item: &String| self.make_card(item.as_str(), prog))
                    .collect();
                if row_cards.is_empty() { return None; }
                Some(
                    iced::widget::Row::from_vec(row_cards)
                        .spacing(gap)
                        .align_y(Alignment::Center)
                        .into(),
                )
            })
            .collect();

        let grid = iced::widget::Column::from_vec(grid_rows)
            .spacing(gap)
            .align_x(Alignment::Center);

        let hint_col = Color::from_rgba(fgc.r, fgc.g, fgc.b, 0.38 * prog);
        let hint = text("Esc or click outside to close")
            .size(fsize - 2.0).color(hint_col);

        let modal = container(
            column![grid, hint].spacing(24.0).align_x(Alignment::Center),
        )
        .padding(iced::Padding {
            top:    36.0,
            right:  48.0,
            bottom: 36.0 + slide,
            left:   48.0,
        })
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(modal_bg)),
            border: Border { radius: 24.0.into(), color: modal_border, width: 1.0 },
            ..Default::default()
        });

        mouse_area(
            container(modal)
                .width(Length::Fill).height(Length::Fill)
                .align_x(Alignment::Center).align_y(Alignment::Center)
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(overlay_bg)),
                    ..Default::default()
                }),
        )
        .on_press(Message::Dismiss)
        .into()
    }

    // ── Card builder ──────────────────────────────────────────────────────────

    fn make_card(&self, item: &str, prog: f32) -> Option<Element<'_, Message>> {
        let t      = &self.theme;
        let fsize  = t.font_size;
        let fg     = t.foreground.to_iced();
        let accent = t.accent.to_iced();
        let bg     = t.background;
        let fgc    = t.foreground;
        let theme  = self.dash_config.theme.as_str();
        let nerd   = t.use_nerd_icons;

        // Card dimensions by theme
        let (card_w, card_h) = match theme {
            "minimal"        => (130.0f32, 66.0f32),
            "full" | "vivid" => (175.0f32, 128.0f32),
            _                => (160.0f32, 108.0f32),  // "cards"
        };

        // Card background — slightly lifted from modal background
        let mix2 = if theme == "vivid" { 0.14f32 } else { 0.07f32 };
        let card_bg = Color::from_rgba(
            (bg.r + (fgc.r - bg.r) * mix2).clamp(0.0, 1.0),
            (bg.g + (fgc.g - bg.g) * mix2).clamp(0.0, 1.0),
            (bg.b + (fgc.b - bg.b) * mix2).clamp(0.0, 1.0),
            0.90 * prog,
        );
        let bar_w = card_w - 44.0;

        // Build content + get the card's semantic accent color
        let (inner, card_color): (Element<'_, Message>, Color) = match item {

            // ── Clock ─────────────────────────────────────────────────────────
            "clock" => {
                let now = chrono::Local::now();
                let time_str = now.format(t.clock_format.as_str()).to_string();
                let date_str = now.format(t.date_format.as_str()).to_string();
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(time_str).size(fsize + 4.0).color(fg),
                    ].into()
                } else {
                    column![
                        text(time_str).size(fsize + 14.0).color(fg),
                        text(date_str).size(fsize - 1.0).color(Color { a: 0.55, ..fg }),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, accent)
            }

            // ── Network ───────────────────────────────────────────────────────
            "network" => {
                let blue = Color::from_rgba(0.54, 0.71, 0.98, prog);
                let iface = if self.sys.net_iface.is_empty() {
                    "No network".to_string()
                } else {
                    self.sys.net_iface.clone()
                };
                let icon = if nerd { "\u{f05a9}" } else { "NET" };
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(blue),
                        text(iface).size(fsize - 1.0).color(fg),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(blue),
                        text(iface).size(fsize - 1.0).color(fg),
                        text("Connected").size(fsize - 2.5).color(Color { a: 0.45, ..fg }),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, blue)
            }

            // ── Battery ───────────────────────────────────────────────────────
            "battery" => {
                let pct = self.sys.battery_pct?;  // hide card if no battery
                let charging = self.sys.battery_charging;
                let warn = t.battery_warn_percent;
                let fill_col = if charging {
                    Color::from_rgba(0.67, 0.88, 0.63, prog)
                } else if pct < warn {
                    Color::from_rgba(0.96, 0.54, 0.67, prog)
                } else {
                    Color { a: 0.85 * prog, ..fg }
                };
                let icon = if charging {
                    if nerd { "\u{f0e7}" } else { "⚡" }
                } else {
                    if nerd { "\u{f0079}" } else { "BAT" }
                };
                let frac = pct as f32 / 100.0;
                let pct_str = format!("{pct}%");
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(fill_col),
                        text(pct_str).size(fsize).color(fg),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(fill_col),
                        text(pct_str).size(fsize).color(fg),
                        self.mini_bar(frac, fill_col, fg, bar_w),
                    ].spacing(6.0).align_x(Alignment::Center).into()
                };
                (content, fill_col)
            }

            // ── CPU ───────────────────────────────────────────────────────────
            "cpu" => {
                let frac = self.sys.cpu_pct / 100.0;
                let cpu_col = lerp_color(
                    Color::from_rgba(0.67, 0.88, 0.63, prog),
                    Color::from_rgba(0.96, 0.54, 0.67, prog),
                    (frac * 2.0 - 1.0).max(0.0),
                );
                let icon = if nerd { "\u{f4bc}" } else { "CPU" };
                let val  = format!("{:.0}%", self.sys.cpu_pct);
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(cpu_col),
                        text(val).size(fsize).color(fg),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(cpu_col),
                        text(val).size(fsize).color(fg),
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
                let mem_col = Color::from_rgba(0.79, 0.65, 0.97, prog);
                let icon = if nerd { "\u{f035b}" } else { "RAM" };
                let val  = format!("{} / {}", fmt_bytes(self.sys.ram_used), fmt_bytes(self.sys.ram_total));
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(mem_col),
                        text(val).size(fsize - 1.0).color(fg),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(mem_col),
                        text(val).size(fsize - 1.0).color(fg),
                        self.mini_bar(frac, mem_col, fg, bar_w),
                    ].spacing(6.0).align_x(Alignment::Center).into()
                };
                (content, mem_col)
            }

            // ── Disk ──────────────────────────────────────────────────────────
            "disk" => {
                let frac = if self.sys.disk_total > 0 {
                    self.sys.disk_used as f32 / self.sys.disk_total as f32
                } else { 0.0 };
                let disk_col = Color::from_rgba(0.98, 0.89, 0.68, prog);
                let icon = if nerd { "\u{f01bc}" } else { "DSK" };
                let val  = format!("{} / {}", fmt_bytes(self.sys.disk_used), fmt_bytes(self.sys.disk_total));
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(disk_col),
                        text(val).size(fsize - 1.0).color(fg),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(disk_col),
                        text(val).size(fsize - 1.0).color(fg),
                        self.mini_bar(frac, disk_col, fg, bar_w),
                    ].spacing(6.0).align_x(Alignment::Center).into()
                };
                (content, disk_col)
            }

            // ── Volume ────────────────────────────────────────────────────────
            "volume" => {
                let vol     = self.sys.volume.unwrap_or(0.0);
                let _frac   = (vol / 1.5).clamp(0.0, 1.0);
                let vol_col = Color::from_rgba(0.58, 0.89, 0.84, prog);
                let icon = if self.sys.volume_muted {
                    if nerd { "\u{f075f}" } else { "M" }
                } else {
                    if nerd { "\u{f057e}" } else { "V" }
                };
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
                        text(val).size(fsize).color(fg),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(vol_col),
                        text(val).size(fsize).color(fg),
                        slider_elem,
                    ].spacing(6.0).align_x(Alignment::Center).into()
                };
                (content, vol_col)
            }

            // ── Brightness ────────────────────────────────────────────────────
            "brightness" => {
                let bright  = self.sys.brightness.unwrap_or(50);
                let _frac   = bright as f32 / 100.0;
                let br_col  = Color::from_rgba(0.98, 0.89, 0.55, prog);
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
                        text(val).size(fsize).color(fg),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(br_col),
                        text(val).size(fsize).color(fg),
                        slider_elem,
                    ].spacing(6.0).align_x(Alignment::Center).into()
                };
                (content, br_col)
            }

            // ── Media ─────────────────────────────────────────────────────────
            "media" => {
                let pink = Color::from_rgba(0.96, 0.54, 0.84, prog);
                let play_icon = if nerd {
                    if self.sys.media_playing { "\u{f03e4}" } else { "\u{f040a}" }
                } else {
                    if self.sys.media_playing { "⏸" } else { "▶" }
                };
                let prev_icon = if nerd { "\u{f0602}" } else { "⏮" };
                let next_icon = if nerd { "\u{f0604}" } else { "⏭" };

                let title = self.sys.media_title.as_deref().unwrap_or("Nothing playing");
                let trunc = if title.len() > 20 { &title[..20] } else { title };
                let trunc = trunc.to_string();

                let artist = self.sys.media_artist.as_deref().unwrap_or("").to_string();

                // Equalizer bars — animated when playing
                let eq: Element<'_, Message> = if self.sys.media_playing && theme != "minimal" {
                    let tick = self.eq_tick as f32;
                    let pink_cap = pink;
                    let bars: Vec<Element<'_, Message>> = (0..5).map(|i| {
                        let phase = (tick * 0.10 + i as f32 * 1.257).sin() * 0.5 + 0.5;
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

                let fg_dim = Color { a: 0.7, ..fg };
                let controls: Element<'_, Message> = row![
                    iced::widget::button(text(prev_icon).size(fsize + 2.0).color(fg_dim))
                        .on_press(Message::MediaAction("previous"))
                        .padding([2.0, 5.0])
                        .style(move |_: &iced::Theme, _| iced::widget::button::Style {
                            background: None, text_color: fg_dim, ..Default::default()
                        }),
                    iced::widget::button(text(play_icon).size(fsize + 5.0).color(pink))
                        .on_press(Message::MediaAction("play-pause"))
                        .padding([2.0, 5.0])
                        .style(move |_: &iced::Theme, _| iced::widget::button::Style {
                            background: None, text_color: pink, ..Default::default()
                        }),
                    iced::widget::button(text(next_icon).size(fsize + 2.0).color(fg_dim))
                        .on_press(Message::MediaAction("next"))
                        .padding([2.0, 5.0])
                        .style(move |_: &iced::Theme, _| iced::widget::button::Style {
                            background: None, text_color: fg_dim, ..Default::default()
                        }),
                ].spacing(4.0).align_y(Alignment::Center).into();

                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(play_icon).size(fsize).color(pink),
                        text(trunc).size(fsize - 1.0).color(fg),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    let mut col_items: Vec<Element<'_, Message>> = vec![
                        text(trunc).size(fsize - 1.0).color(fg).into(),
                    ];
                    if !artist.is_empty() {
                        col_items.push(
                            text(artist).size(fsize - 2.5).color(Color { a: 0.5, ..fg }).into()
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
                let orange = Color::from_rgba(0.98, 0.70, 0.53, prog);
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

                let dim = Color { a: 0.50, ..fg };
                let content: Element<'_, Message> = if theme == "minimal" {
                    iced::widget::Row::from_vec(buttons)
                        .spacing(2.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text("Power").size(fsize - 2.0).color(dim),
                        iced::widget::Row::from_vec(buttons)
                            .spacing(2.0).align_y(Alignment::Center),
                    ].spacing(8.0).align_x(Alignment::Center).into()
                };
                (content, orange)
            }

            // ── Uptime ────────────────────────────────────────────────────────
            "uptime" => {
                let teal = Color::from_rgba(0.58, 0.89, 0.84, prog);
                let icon = if nerd { "\u{f150e}" } else { "UP" };
                let val  = fmt_uptime(self.sys.uptime_secs);
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(teal),
                        text(val).size(fsize).color(fg),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(teal),
                        text("Uptime").size(fsize - 2.5).color(Color { a: 0.45, ..fg }),
                        text(val).size(fsize).color(fg),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, teal)
            }

            // ── Temperature ───────────────────────────────────────────────────
            "temperature" => {
                let temp = self.sys.temp_celsius?;
                let heat = ((temp - 40.0) / 50.0).clamp(0.0, 1.0);
                let temp_col = lerp_color(
                    Color::from_rgba(0.67, 0.88, 0.63, prog),
                    Color::from_rgba(0.96, 0.54, 0.67, prog),
                    heat,
                );
                let icon = if nerd { "\u{f050f}" } else { "TMP" };
                let val  = format!("{temp:.0}°C");
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(temp_col),
                        text(val).size(fsize).color(fg),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(temp_col),
                        text("Temp").size(fsize - 2.5).color(Color { a: 0.45, ..fg }),
                        text(val).size(fsize).color(fg),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, temp_col)
            }

            // ── Updates ───────────────────────────────────────────────────────
            "updates" => {
                let yellow = Color::from_rgba(0.98, 0.70, 0.53, prog);
                let icon = if nerd { "\u{f0954}" } else { "UPD" };
                let val = match self.sys.update_count {
                    Some(0) => "Up to date".to_string(),
                    Some(n) => format!("{n} updates"),
                    None    => "Checking…".to_string(),
                };
                let content: Element<'_, Message> = if theme == "minimal" {
                    row![
                        text(icon).size(fsize).color(yellow),
                        text(val).size(fsize - 1.0).color(fg),
                    ].spacing(6.0).align_y(Alignment::Center).into()
                } else {
                    column![
                        text(icon).size(fsize + 10.0).color(yellow),
                        text("Updates").size(fsize - 2.5).color(Color { a: 0.45, ..fg }),
                        text(val).size(fsize - 1.0).color(fg),
                    ].spacing(4.0).align_x(Alignment::Center).into()
                };
                (content, yellow)
            }

            _ => return None,
        };

        // Card border style based on theme
        let (border_col, border_w) = match theme {
            "minimal" => (Color::TRANSPARENT, 0.0f32),
            "vivid"   => (Color { a: 0.65 * prog, ..card_color }, 1.5),
            "full"    => (Color { a: 0.35 * prog, ..card_color }, 1.0),
            _         => (Color { a: 0.18 * prog, ..fg }, 1.0),
        };

        Some(
            container(inner)
                .width(Length::Fixed(card_w))
                .height(Length::Fixed(card_h))
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(card_bg)),
                    border: Border { radius: 16.0.into(), color: border_col, width: border_w },
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
        let track_col  = Color { a: 0.15, ..fg };

        iced::widget::Row::from_vec(vec![
            container(iced::widget::Space::new())
                .width(Length::Fixed(fill_w))
                .height(Length::Fixed(4.0))
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(fill_solid)),
                    border: Border { radius: 99.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .into(),
            container(iced::widget::Space::new())
                .width(Length::Fixed(empty_w))
                .height(Length::Fixed(4.0))
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(track_col)),
                    border: Border { radius: 99.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .into(),
        ])
        .spacing(0.0)
        .into()
    }

    // ── Subscriptions ─────────────────────────────────────────────────────────

    fn subscription(&self) -> Subscription<Message> {
        let now = Instant::now();
        let animate = self.enter_anim.is_animating(now) || self.sys.media_playing;

        let mut subs = vec![
            iced::keyboard::listen().map(Message::KeyEvent),
            Subscription::run(sys_stream),
        ];
        if animate {
            subs.push(
                iced::time::every(Duration::from_millis(16))
                    .map(|_| Message::AnimFrame),
            );
        }
        Subscription::batch(subs)
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
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let snap = read_sys_snapshot().await;
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
    } else {
        format!("{:.0}K", bytes as f64 / 1024.0)
    }
}

fn fmt_uptime(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 { format!("{h}h {m:02}m") } else { format!("{m}m") }
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}
