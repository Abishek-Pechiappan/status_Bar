pub mod battery;
pub mod cpu;
pub mod memory;

use bar_core::state::SystemSnapshot;
use sysinfo::{Components, Disks, Networks, System};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;

/// Spawn a background Tokio task that polls system stats every `interval_ms`
/// milliseconds and forwards [`SystemSnapshot`]s through the returned channel.
///
/// `custom_cmd` is a shell command string whose stdout is exposed as
/// `snapshot.custom_output`.  Pass an empty string to disable.
///
/// The task stops automatically when the receiver is dropped.
pub fn spawn_monitor(interval_ms: u64, custom_cmd: String) -> mpsc::Receiver<SystemSnapshot> {
    let (tx, rx) = mpsc::channel(4);
    let interval = Duration::from_millis(interval_ms);
    let interval_secs = interval_ms as f64 / 1000.0;

    tokio::spawn(async move {
        let mut sys      = System::new_all();
        let mut networks = Networks::new_with_refreshed_list();
        let mut ticker   = time::interval(interval);

        loop {
            ticker.tick().await;
            sys.refresh_all();
            networks.refresh(false);

            let snapshot = take_snapshot(&sys, &networks, interval_secs, &custom_cmd).await;

            if tx.send(snapshot).await.is_err() {
                break;
            }
        }
    });

    rx
}

async fn take_snapshot(
    sys: &System,
    networks: &Networks,
    interval_secs: f64,
    custom_cmd: &str,
) -> SystemSnapshot {
    // ── CPU ──────────────────────────────────────────────────────────────────
    let cpu_per_core: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
    let cpu_average = if cpu_per_core.is_empty() {
        0.0
    } else {
        cpu_per_core.iter().sum::<f32>() / cpu_per_core.len() as f32
    };

    // ── CPU temperature ───────────────────────────────────────────────────────
    let cpu_temp = read_cpu_temp();

    // ── Memory + Swap ────────────────────────────────────────────────────────
    let ram_used  = sys.used_memory();
    let ram_total = sys.total_memory();
    let swap_used  = sys.used_swap();
    let swap_total = sys.total_swap();

    // ── Disk ─────────────────────────────────────────────────────────────────
    let disks = Disks::new_with_refreshed_list();
    let (disk_used, disk_total) = disks
        .iter()
        .find(|d| d.mount_point() == std::path::Path::new("/"))
        .map(|d| (d.total_space() - d.available_space(), d.total_space()))
        .unwrap_or((0, 0));

    // ── Network ──────────────────────────────────────────────────────────────
    let raw_rx: u64 = networks.iter().map(|(_, d)| d.received()).sum();
    let raw_tx: u64 = networks.iter().map(|(_, d)| d.transmitted()).sum();
    let net_rx = (raw_rx as f64 / interval_secs) as u64;
    let net_tx = (raw_tx as f64 / interval_secs) as u64;

    // Primary interface: highest combined traffic, excluding loopback.
    let net_interface = networks
        .iter()
        .filter(|(name, _)| *name != "lo")
        .max_by_key(|(_, d)| d.received().saturating_add(d.transmitted()))
        .map(|(name, _)| name.clone())
        .unwrap_or_default();

    let net_signal = if net_interface.is_empty() {
        None
    } else {
        read_wifi_signal(&net_interface)
    };

    // ── Battery ──────────────────────────────────────────────────────────────
    let (battery_percent, battery_charging) = match battery::read_battery() {
        Some((pct, chg)) => (Some(pct), Some(chg)),
        None             => (None, None),
    };
    let battery_time_min = battery::read_battery_time();

    // ── Volume ───────────────────────────────────────────────────────────────
    let (volume, volume_muted) = read_volume().await;

    // ── Brightness ───────────────────────────────────────────────────────────
    let brightness = read_brightness();

    // ── Uptime ───────────────────────────────────────────────────────────────
    let uptime_secs = System::uptime();

    // ── Load averages ────────────────────────────────────────────────────────
    let (load_1, load_5, load_15) = read_loadavg();

    // ── Media player (playerctl) ─────────────────────────────────────────────
    let (media_title, media_artist, media_playing) = read_media().await;

    // ── Custom command ───────────────────────────────────────────────────────
    let custom_output = if custom_cmd.is_empty() {
        String::new()
    } else {
        run_custom(custom_cmd).await
    };

    SystemSnapshot {
        cpu_per_core,
        cpu_average,
        ram_used,
        ram_total,
        swap_used,
        swap_total,
        disk_used,
        disk_total,
        net_rx,
        net_tx,
        net_interface,
        net_signal,
        battery_percent,
        battery_charging,
        battery_time_min,
        cpu_temp,
        volume,
        volume_muted,
        brightness,
        uptime_secs,
        load_1,
        load_5,
        load_15,
        media_title,
        media_artist,
        media_playing,
        custom_output,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_cpu_temp() -> Option<f32> {
    let components = Components::new_with_refreshed_list();
    components
        .iter()
        .find(|c| {
            let lbl = c.label().to_lowercase();
            lbl.contains("package") || lbl.contains("tdie") || lbl.contains("tctl")
        })
        .or_else(|| {
            components.iter().find(|c| {
                let lbl = c.label().to_lowercase();
                lbl.contains("cpu") || lbl.contains("core 0")
            })
        })
        .and_then(|c| c.temperature())
}

async fn read_volume() -> (Option<f32>, bool) {
    let result = tokio::process::Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .await;

    match result {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let muted = text.contains("[MUTED]");
            let vol = text
                .trim_start_matches("Volume:")
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<f32>().ok());
            (vol, muted)
        }
        _ => (None, false),
    }
}

fn read_brightness() -> Option<u8> {
    let dir = std::fs::read_dir("/sys/class/backlight").ok()?;
    for entry in dir.flatten() {
        let path = entry.path();
        let current: u64 = std::fs::read_to_string(path.join("brightness"))
            .ok()?.trim().parse().ok()?;
        let max: u64 = std::fs::read_to_string(path.join("max_brightness"))
            .ok()?.trim().parse().ok()?;
        if max > 0 {
            return Some(((current * 100) / max).min(100) as u8);
        }
    }
    None
}

fn read_loadavg() -> (f32, f32, f32) {
    let content = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let mut parts = content.split_whitespace();
    let l1  = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let l5  = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let l15 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    (l1, l5, l15)
}

/// Query playerctl for the current media player state.
async fn read_media() -> (Option<String>, Option<String>, bool) {
    let status = tokio::process::Command::new("playerctl")
        .arg("status")
        .output()
        .await;

    let status_str = match status {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => return (None, None, false),
    };

    if status_str == "Stopped" || status_str.is_empty() {
        return (None, None, false);
    }

    let playing = status_str == "Playing";
    let title  = playerctl_get(&["metadata", "title"]).await;
    let artist = playerctl_get(&["metadata", "artist"]).await;
    (title, artist, playing)
}

async fn playerctl_get(args: &[&str]) -> Option<String> {
    let out = tokio::process::Command::new("playerctl")
        .args(args)
        .output()
        .await
        .ok()?;
    if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() { None } else { Some(s) }
    } else {
        None
    }
}

/// Read WiFi signal level in dBm for `iface` from `/proc/net/wireless`.
///
/// Returns `None` for wired interfaces (not listed in that file) or if the
/// file cannot be read.  The level column value is typically a negative dBm
/// integer followed by a period (e.g. `-43.`).
fn read_wifi_signal(iface: &str) -> Option<i32> {
    let content = std::fs::read_to_string("/proc/net/wireless").ok()?;
    for line in content.lines().skip(2) {
        let line = line.trim();
        // Lines look like:  wlan0: 0000   67.  -43.  -256.  ...
        let (name, rest) = line.split_once(':')?;
        if name.trim() != iface { continue; }
        // Fields after the colon: status  link  level  noise …
        let mut parts = rest.split_whitespace();
        let _ = parts.next(); // status
        let _ = parts.next(); // link quality
        let level = parts.next()?;
        return level.trim_end_matches('.').parse::<i32>().ok();
    }
    None
}

/// Run an arbitrary shell command and return its trimmed stdout.
async fn run_custom(cmd: &str) -> String {
    match tokio::process::Command::new("sh").args(["-c", cmd]).output().await {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        }
        _ => String::new(),
    }
}
