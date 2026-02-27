pub mod schema;
pub mod watcher;

pub use schema::{BarConfig, GlobalConfig, MonitorConfig, Position, ThemeConfig, WidgetConfig};
pub use watcher::ConfigWatcher;

use bar_core::{BarError, Result};
use std::path::{Path, PathBuf};

/// Load configuration from a TOML file.  Returns `BarConfig::default()` if
/// the file doesn't exist so the bar always has sensible defaults.
pub fn load(path: impl AsRef<Path>) -> Result<BarConfig> {
    let path = path.as_ref();
    if !path.exists() {
        tracing::warn!(
            "Config file not found at '{}'; using defaults.",
            path.display()
        );
        return Ok(BarConfig::default());
    }

    let raw = std::fs::read_to_string(path)
        .map_err(|e| BarError::Config(format!("cannot read '{}': {e}", path.display())))?;

    toml::from_str(&raw).map_err(|e| BarError::Config(format!("TOML parse error: {e}")))
}

/// Return the default config path, honouring `$XDG_CONFIG_HOME`.
pub fn default_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        });
    base.join("bar").join("bar.toml")
}
