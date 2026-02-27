use crate::{event::Message, state::AppState};

/// Every built-in (and future plugin) widget must implement this trait.
///
/// Widgets are purely reactive: they receive a read-only view of `AppState`
/// and return optional `Message`s to drive further state updates.
/// All rendering is handled by the `bar-renderer` / `bar-wayland` crates.
pub trait BarWidget: Send + Sync + std::fmt::Debug {
    /// Unique string identifier, e.g. `"clock"` or `"workspaces"`.
    fn id(&self) -> &str;

    /// Called once after the application launches.
    /// Can return an initial message to seed state (e.g. fetch workspace list).
    fn init(&mut self) -> Option<Message> {
        None
    }

    /// Called whenever `AppState` is updated.
    /// The widget may emit a follow-up message if it holds derived state.
    fn on_state_change(&mut self, _state: &AppState) -> Option<Message> {
        None
    }
}
