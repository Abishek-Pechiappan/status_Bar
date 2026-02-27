use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Watches a config file for changes and sends a notification on every write.
///
/// # Example
/// ```no_run
/// let (_, mut rx) = ConfigWatcher::spawn("/home/user/.config/bar/bar.toml");
/// while rx.recv().await.is_some() {
///     println!("config changed — reloading");
/// }
/// ```
pub struct ConfigWatcher {
    path: PathBuf,
}

impl ConfigWatcher {
    /// Spawn a filesystem watcher for `path`.
    /// Returns the watcher handle and a receiver that fires on every detected change.
    pub fn spawn(path: impl AsRef<Path>) -> (Self, mpsc::Receiver<()>) {
        let (tx, rx) = mpsc::channel(1);
        let path = path.as_ref().to_path_buf();
        let watcher = Self { path: path.clone() };

        tokio::spawn(watch_loop(path, tx));

        (watcher, rx)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

async fn watch_loop(path: PathBuf, tx: mpsc::Sender<()>) {
    use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
    use std::time::Duration;
    use tokio::sync::mpsc as async_mpsc;

    let (sync_tx, mut sync_rx) = async_mpsc::channel::<notify::Result<Event>>(16);

    let mut watcher = match RecommendedWatcher::new(
        move |res| {
            let _ = sync_tx.blocking_send(res);
        },
        Config::default().with_poll_interval(Duration::from_secs(2)),
    ) {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to create filesystem watcher: {e}");
            return;
        }
    };

    if let Err(e) = watcher.watch(&path, RecursiveMode::NonRecursive) {
        error!("Failed to watch '{}': {e}", path.display());
        return;
    }

    info!("Watching config file: {}", path.display());

    while let Some(event) = sync_rx.recv().await {
        match event {
            Ok(e) => {
                use notify::EventKind::*;
                if matches!(e.kind, Modify(_) | Create(_)) {
                    if tx.send(()).await.is_err() {
                        break; // receiver dropped
                    }
                }
            }
            Err(e) => warn!("Watcher error: {e}"),
        }
    }
}
