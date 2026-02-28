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
/// The task stops automatically when the receiver is dropped.
pub fn spawn_monitor(interval_ms: u64) -> mpsc::Receiver<SystemSnapshot> {
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
            networks.refresh(false); // false = keep existing interfaces list

            let snapshot = take_snapshot(&sys, &networks, interval_secs).await;

            if tx.send(snapshot).await.is_err() {
                break; // all receivers dropped
            }
        }
    });

    rx
}

async fn take_snapshot(sys: &System, networks: &Networks, interval_secs: f64) -> SystemSnapshot {
    // ── CPU ──────────────────────────────────────────────────────────────────
    let cpu_per_core: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
    let cpu_average = if cpu_per_core.is_empty() {
        0.0
    } else {
        cpu_per_core.iter().sum::<f32>() / cpu_per_core.len() as f32
    };

    // ── CPU temperature ───────────────────────────────────────────────────────
    let cpu_temp = read_cpu_temp();

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

    // ── Battery ──────────────────────────────────────────────────────────────
    let (battery_percent, battery_charging) = match battery::read_battery() {
        Some((pct, chg)) => (Some(pct), Some(chg)),
        None             => (None, None),
    };

    // ── Volume ───────────────────────────────────────────────────────────────
    let (volume, volume_muted) = read_volume().await;

    // ── Brightness ───────────────────────────────────────────────────────────
    let brightness = read_brightness();

    SystemSnapshot {
        cpu_per_core,
        cpu_average,
        ram_used:   sys.used_memory(),
        ram_total:  sys.total_memory(),
        disk_used,
        disk_total,
        net_rx,
        net_tx,
        battery_percent,
        battery_charging,
        cpu_temp,
        volume,
        volume_muted,
        brightness,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Read the CPU package/die temperature from sysinfo Components.
fn read_cpu_temp() -> Option<f32> {
    let components = Components::new_with_refreshed_list();
    // Prefer "Package" label (Intel/AMD die temp), fall back to first CPU core.
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

/// Query the default audio sink volume via `wpctl`.
/// Returns `(Some(volume), is_muted)` or `(None, false)` if wpctl is absent.
async fn read_volume() -> (Option<f32>, bool) {
    let result = tokio::process::Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .await;

    match result {
        Ok(out) if out.status.success() => {
            // Output: "Volume: 0.50\n" or "Volume: 0.50 [MUTED]\n"
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

/// Read screen brightness as a percentage from `/sys/class/backlight`.
fn read_brightness() -> Option<u8> {
    let dir = std::fs::read_dir("/sys/class/backlight").ok()?;
    for entry in dir.flatten() {
        let path = entry.path();
        let current: u64 = std::fs::read_to_string(path.join("brightness"))
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let max: u64 = std::fs::read_to_string(path.join("max_brightness"))
            .ok()?
            .trim()
            .parse()
            .ok()?;
        if max > 0 {
            return Some(((current * 100) / max).min(100) as u8);
        }
    }
    None
}
