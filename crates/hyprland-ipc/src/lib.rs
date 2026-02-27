pub mod client;
pub mod events;

pub use client::{fetch_workspaces, HyprlandIpc};
pub use events::{ActiveWindowEvent, HyprlandEvent, WorkspaceEvent, WorkspaceInfo};
