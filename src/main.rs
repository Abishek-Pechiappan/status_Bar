//! bar — a production-grade, Wayland-native status bar for Hyprland.
//!
//! Run with:  `RUST_LOG=info bar`

use anyhow::Result;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Structured logging — RUST_LOG controls verbosity (default: info).
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("bar v{} starting", env!("CARGO_PKG_VERSION"));

    bar_wayland::run().map_err(Into::into)
}
