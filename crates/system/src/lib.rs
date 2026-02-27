pub mod battery;
pub mod cpu;
pub mod memory;

use bar_core::state::SystemSnapshot;
use sysinfo::{Disks, Networks, System};
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

            let snapshot = take_snapshot(&sys, &networks, interval_secs);

            if tx.send(snapshot).await.is_err() {
                break; // all receivers dropped
            }
        }
    });

    rx
}

fn take_snapshot(sys: &System, networks: &Networks, interval_secs: f64) -> SystemSnapshot {
    // ── CPU ──────────────────────────────────────────────────────────────────
    let cpu_per_core: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
    let cpu_average = if cpu_per_core.is_empty() {
        0.0
    } else {
        cpu_per_core.iter().sum::<f32>() / cpu_per_core.len() as f32
    };

    // ── Disk ─────────────────────────────────────────────────────────────────
    let disks = Disks::new_with_refreshed_list();
    let (disk_used, disk_total) = disks
        .iter()
        .find(|d| d.mount_point() == std::path::Path::new("/"))
        .map(|d| (d.total_space() - d.available_space(), d.total_space()))
        .unwrap_or((0, 0));

    // ── Network ──────────────────────────────────────────────────────────────
    // `received()` / `transmitted()` are deltas since the last refresh.
    // Dividing by the interval gives bytes/second.
    let raw_rx: u64 = networks.iter().map(|(_, d)| d.received()).sum();
    let raw_tx: u64 = networks.iter().map(|(_, d)| d.transmitted()).sum();
    let net_rx = (raw_rx as f64 / interval_secs) as u64;
    let net_tx = (raw_tx as f64 / interval_secs) as u64;

    // ── Battery ──────────────────────────────────────────────────────────────
    let (battery_percent, battery_charging) = match battery::read_battery() {
        Some((pct, chg)) => (Some(pct), Some(chg)),
        None             => (None, None),
    };

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
    }
}
