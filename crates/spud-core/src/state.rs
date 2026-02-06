use std::time::{Duration, Instant};

pub struct AppState {
    pub started_at: Instant,
    pub active_module_idx: usize,
    pub status_line: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            active_module_idx: 0,
            status_line: "DE-EVOLUTION IN PROGRESS.".to_string(),
        }
    }

    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}
