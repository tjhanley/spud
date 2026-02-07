use std::time::{Duration, Instant};

pub struct AppState {
    pub started_at: Instant,
    pub status_line: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            status_line: "DE-EVOLUTION IN PROGRESS.".to_string(),
        }
    }

    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}
