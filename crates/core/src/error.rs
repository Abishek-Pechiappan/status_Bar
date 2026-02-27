use thiserror::Error;

/// Top-level error type used across the entire application.
#[derive(Debug, Error)]
pub enum BarError {
    #[error("config error: {0}")]
    Config(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("system error: {0}")]
    System(String),

    #[error("wayland error: {0}")]
    Wayland(String),

    #[error("widget error: {0}")]
    Widget(String),

    #[error("I/O error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
}

pub type Result<T, E = BarError> = std::result::Result<T, E>;
