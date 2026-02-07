use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Measures ticks-per-second over a sliding time window.
///
/// Call [`tick`](TickCounter::tick) once per frame/tick, then
/// [`tps`](TickCounter::tps) to read the current rate. Old timestamps outside
/// the window are automatically pruned.
pub struct TickCounter {
    timestamps: VecDeque<Instant>,
    window: Duration,
}

impl Default for TickCounter {
    fn default() -> Self {
        Self::new(Duration::from_secs(1))
    }
}

impl TickCounter {
    /// Create a counter with the given measurement window.
    pub fn new(window: Duration) -> Self {
        Self {
            timestamps: VecDeque::new(),
            window,
        }
    }

    /// Record a tick at the given instant and prune expired timestamps.
    pub fn tick(&mut self, now: Instant) {
        self.timestamps.push_back(now);
        self.prune(now);
    }

    /// Return the current ticks-per-second based on timestamps in the window.
    ///
    /// Returns `0.0` if fewer than two ticks have been recorded.
    pub fn tps(&self) -> f64 {
        if self.timestamps.len() < 2 {
            return 0.0;
        }
        let now = self.timestamps.back().copied().unwrap();
        let window_start = now - self.window;
        let count = self
            .timestamps
            .iter()
            .filter(|&&t| t >= window_start)
            .count();
        count as f64 / self.window.as_secs_f64()
    }

    /// Remove timestamps older than `now - window`.
    fn prune(&mut self, now: Instant) {
        let cutoff = now - self.window;
        while let Some(&front) = self.timestamps.front() {
            if front < cutoff {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_counter_returns_zero() {
        let counter = TickCounter::default();
        assert_eq!(counter.tps(), 0.0);
    }

    #[test]
    fn single_tick_returns_zero() {
        let mut counter = TickCounter::default();
        counter.tick(Instant::now());
        assert_eq!(counter.tps(), 0.0);
    }

    #[test]
    fn tick_records_timestamps() {
        let mut counter = TickCounter::new(Duration::from_secs(1));
        let base = Instant::now();
        for i in 0..10 {
            counter.tick(base + Duration::from_millis(i * 100));
        }
        let tps = counter.tps();
        assert!(tps > 9.0 && tps < 11.0, "tps was {}", tps);
    }

    #[test]
    fn old_timestamps_pruned() {
        let mut counter = TickCounter::new(Duration::from_secs(1));
        let base = Instant::now();

        for i in 0..5 {
            counter.tick(base + Duration::from_millis(i * 200));
        }

        for i in 0..3 {
            counter.tick(base + Duration::from_millis(1000 + i * 300));
        }

        assert!(
            counter.timestamps.len() <= 5,
            "timestamps: {}",
            counter.timestamps.len()
        );
    }
}
