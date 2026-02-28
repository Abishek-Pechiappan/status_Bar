/// Read battery state from the Linux sysfs power-supply interface.
///
/// Returns `(percent, charging)` for the first battery found, or `None`
/// if the system has no battery (desktop, VM).
pub fn read_battery() -> Option<(u8, bool)> {
    for name in ["BAT0", "BAT1", "BAT2"] {
        let base = std::path::Path::new("/sys/class/power_supply").join(name);
        if !base.exists() {
            continue;
        }

        let capacity = std::fs::read_to_string(base.join("capacity")).ok()?;
        let status   = std::fs::read_to_string(base.join("status")).ok()?;

        let percent  = capacity.trim().parse::<u8>().ok()?;
        let charging = matches!(status.trim(), "Charging" | "Full");

        return Some((percent, charging));
    }
    None
}

/// Estimate minutes of battery life remaining (or until full when charging).
///
/// Uses sysfs energy/power readings for accuracy.  Returns `None` if the
/// battery driver doesn't expose the required counters or if a division by
/// zero would occur.
pub fn read_battery_time() -> Option<u32> {
    for name in ["BAT0", "BAT1", "BAT2"] {
        let base = std::path::Path::new("/sys/class/power_supply").join(name);
        if !base.exists() {
            continue;
        }

        // Energy-based: µWh / µW → hours → minutes
        if let (Some(e_now), Some(e_full), Some(power)) = (
            read_u64(&base.join("energy_now")),
            read_u64(&base.join("energy_full")),
            read_u64(&base.join("power_now")),
        ) {
            if power > 0 {
                let status = std::fs::read_to_string(base.join("status"))
                    .unwrap_or_default();
                let mins = if matches!(status.trim(), "Charging") {
                    (e_full.saturating_sub(e_now) * 60 / power) as u32
                } else {
                    (e_now * 60 / power) as u32
                };
                return Some(mins);
            }
        }

        // Charge-based: µAh / µA → hours → minutes
        if let (Some(c_now), Some(c_full), Some(current)) = (
            read_u64(&base.join("charge_now")),
            read_u64(&base.join("charge_full")),
            read_u64(&base.join("current_now")),
        ) {
            if current > 0 {
                let status = std::fs::read_to_string(base.join("status"))
                    .unwrap_or_default();
                let mins = if matches!(status.trim(), "Charging") {
                    (c_full.saturating_sub(c_now) * 60 / current) as u32
                } else {
                    (c_now * 60 / current) as u32
                };
                return Some(mins);
            }
        }
    }
    None
}

fn read_u64(path: &std::path::Path) -> Option<u64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}
