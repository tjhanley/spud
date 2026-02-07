use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Counts ticks-per-second over a sliding window.
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
    pub fn new(window: Duration) -> Self {
        Self {
            timestamps: VecDeque::new(),
            window,
        }
    }

    pub fn tick(&mut self, now: Instant) {
        self.timestamps.push_back(now);
        self.prune(now);
    }

    pub fn tps(&self) -> f64 {
        if self.timestamps.len() < 2 {
            return 0.0;
        }
        let now = self.timestamps.back().copied().unwrap();
        let window_start = now - self.window;
        let count = self.timestamps.iter().filter(|&&t| t >= window_start).count();
        count as f64 / self.window.as_secs_f64()
    }

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

        // Add 5 ticks in the first second
        for i in 0..5 {
            counter.tick(base + Duration::from_millis(i * 200));
        }

        // Add 3 ticks in the second second (should prune old ones)
        for i in 0..3 {
            counter.tick(base + Duration::from_millis(1000 + i * 300));
        }

        // Only recent timestamps should remain
        assert!(counter.timestamps.len() <= 5, "timestamps: {}", counter.timestamps.len());
    }
}
