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
