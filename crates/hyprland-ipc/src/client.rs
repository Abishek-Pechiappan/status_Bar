use crate::events::{parse_event, HyprlandEvent, WorkspaceInfo};
use bar_core::{BarError, Result};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Hyprland IPC client.
///
/// Connects to the Hyprland event socket and streams typed [`HyprlandEvent`]s.
/// Automatically reconnects if the socket connection drops.
pub struct HyprlandIpc {
    /// Path to `socket2.sock` (the event socket).
    event_socket: PathBuf,
    /// Path to `socket.sock` (the command socket).
    cmd_socket: PathBuf,
}

impl HyprlandIpc {
    /// Create a new client, discovering sockets from `$HYPRLAND_INSTANCE_SIGNATURE`.
    pub fn new() -> Result<Self> {
        let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").map_err(|_| {
            BarError::Ipc(
                "HYPRLAND_INSTANCE_SIGNATURE not set — is Hyprland running?".into(),
            )
        })?;

        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| "/run/user/1000".to_string());

        let base = PathBuf::from(format!("{runtime_dir}/hypr/{sig}"));

        Ok(Self {
            event_socket: base.join(".socket2.sock"),
            cmd_socket:   base.join(".socket.sock"),
        })
    }

    /// Path to the event socket (`socket2.sock`).
    pub fn event_socket(&self) -> &std::path::Path {
        &self.event_socket
    }

    /// Spawn a background task that reads from the Hyprland event socket and
    /// forwards typed [`HyprlandEvent`]s on the returned channel.
    ///
    /// The task reconnects automatically on socket errors.
    pub fn spawn_listener(self) -> mpsc::Receiver<HyprlandEvent> {
        let (tx, rx) = mpsc::channel(32);
        let path = self.event_socket;

        tokio::spawn(async move {
            loop {
                match UnixStream::connect(&path).await {
                    Ok(stream) => {
                        info!("Connected to Hyprland event socket");
                        let mut lines = BufReader::new(stream).lines();

                        while let Ok(Some(line)) = lines.next_line().await {
                            let event = parse_event(&line);
                            if tx.send(event).await.is_err() {
                                return; // all receivers dropped
                            }
                        }

                        warn!("Hyprland IPC connection lost; reconnecting in 2s…");
                    }
                    Err(e) => {
                        error!("Cannot connect to Hyprland IPC: {e}; retrying in 2s…");
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        });

        rx
    }

    /// Send a one-shot command to Hyprland and return the raw response.
    pub async fn command(&self, cmd: &str) -> Result<String> {
        let mut stream = UnixStream::connect(&self.cmd_socket)
            .await
            .map_err(|e| BarError::Ipc(format!("connect: {e}")))?;

        stream
            .write_all(cmd.as_bytes())
            .await
            .map_err(|e| BarError::Ipc(format!("write: {e}")))?;

        let mut buf = String::new();
        let mut lines = BufReader::new(&mut stream).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            buf.push_str(&line);
            buf.push('\n');
        }

        Ok(buf)
    }
}

/// Fetch the current workspace list from Hyprland via `hyprctl workspaces -j`.
pub async fn fetch_workspaces(ipc: &HyprlandIpc) -> Result<Vec<WorkspaceInfo>> {
    let raw = ipc.command("j/workspaces").await?;
    serde_json::from_str(&raw)
        .map_err(|e| BarError::Ipc(format!("parse workspaces: {e}")))
}
