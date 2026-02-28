pub mod client;
pub mod events;

pub use client::{fetch_active_window, fetch_workspaces, HyprlandIpc};
pub use events::{ActiveWindowEvent, HyprlandEvent, WorkspaceEvent, WorkspaceInfo};
