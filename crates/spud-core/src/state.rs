use std::time::{Duration, Instant};

/// Global application state shared across the app loop.
///
/// Holds the startup timestamp and the status line displayed in the top bar.
/// Module activation state has moved to [`crate::registry::ModuleRegistry`].
pub struct AppState {
    /// Timestamp when the application started.
    pub started_at: Instant,
    /// Text displayed in the top status bar (e.g. `"MODULE: Hello"`).
    pub status_line: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create a new `AppState` with the current time and the default status line.
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            status_line: "DE-EVOLUTION IN PROGRESS.".to_string(),
        }
    }

    /// Return the elapsed time since the application started.
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}
