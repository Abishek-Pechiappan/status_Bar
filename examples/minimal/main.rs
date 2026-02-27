//! Minimal example — launches the bar with all defaults.
//!
//! ```
//! cargo run --example minimal
//! ```

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    bar_wayland::run().map_err(Into::into)
}
