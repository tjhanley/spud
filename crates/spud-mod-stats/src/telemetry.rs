/// Telemetry collector wrapping `sysinfo` with interval-gated refresh.
///
/// Gathers CPU, memory, swap, and SPUD process metrics. Designed to be
/// called every tick (~100ms) but only performs expensive sysinfo refreshes
/// at the configured interval (default 1s).
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessesToUpdate, System};

/// A point-in-time snapshot of system and process metrics.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    /// Global CPU usage as a percentage (0.0–100.0).
    pub cpu_global: f32,
    /// Per-core CPU usage percentages.
    pub cpu_per_core: Vec<f32>,
    /// Total physical memory in bytes.
    pub mem_total: u64,
    /// Used physical memory in bytes.
    pub mem_used: u64,
    /// Total swap space in bytes.
    pub swap_total: u64,
    /// Used swap space in bytes.
    pub swap_used: u64,
    /// Resident set size of the SPUD process in bytes.
    pub self_rss: Option<u64>,
    /// CPU usage of the SPUD process as a percentage.
    pub self_cpu: Option<f32>,
}

impl Default for MetricsSnapshot {
    fn default() -> Self {
        Self {
            cpu_global: f32::NAN,
            cpu_per_core: Vec::new(),
            mem_total: 0,
            mem_used: 0,
            swap_total: 0,
            swap_used: 0,
            self_rss: None,
            self_cpu: None,
        }
    }
}

/// Interval-gated system telemetry collector.
///
/// Wraps [`sysinfo::System`] and only performs expensive refresh calls when
/// the configured interval has elapsed. Call [`maybe_refresh`](Self::maybe_refresh)
/// on every tick; it returns `true` when a refresh actually occurred.
pub struct TelemetryCollector {
    sys: System,
    self_pid: Pid,
    interval: Duration,
    last_refresh: Option<Instant>,
    snapshot: MetricsSnapshot,
}

impl TelemetryCollector {
    /// Create a new collector with a 1-second refresh interval.
    ///
    /// Performs an initial CPU refresh to establish a baseline (the first
    /// sysinfo CPU reading always returns 0%).
    pub fn new() -> Self {
        Self::with_interval(Duration::from_secs(1))
    }

    /// Create a new collector with the specified refresh interval.
    pub fn with_interval(interval: Duration) -> Self {
        let mut sys = System::new();
        // Baseline refresh so the next call gets a real CPU delta.
        sys.refresh_cpu_usage();

        let self_pid = Pid::from_u32(std::process::id());

        Self {
            sys,
            self_pid,
            interval,
            last_refresh: None,
            snapshot: MetricsSnapshot::default(),
        }
    }

    /// Refresh system metrics if the interval has elapsed.
    ///
    /// Returns `true` if a refresh was performed.
    pub fn maybe_refresh(&mut self, now: Instant) -> bool {
        let should_refresh = match self.last_refresh {
            None => true,
            Some(last) => now
                .checked_duration_since(last)
                .is_some_and(|elapsed| elapsed >= self.interval),
        };

        if !should_refresh {
            return false;
        }

        self.last_refresh = Some(now);

        // Targeted refreshes — much cheaper than refresh_all().
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys
            .refresh_processes(ProcessesToUpdate::Some(&[self.self_pid]), false);

        // Build snapshot.
        self.snapshot.cpu_global = self.sys.global_cpu_usage();
        self.snapshot.cpu_per_core = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        self.snapshot.mem_total = self.sys.total_memory();
        self.snapshot.mem_used = self.sys.used_memory();
        self.snapshot.swap_total = self.sys.total_swap();
        self.snapshot.swap_used = self.sys.used_swap();

        match self.sys.process(self.self_pid) {
            Some(proc) => {
                self.snapshot.self_rss = Some(proc.memory());
                self.snapshot.self_cpu = Some(proc.cpu_usage());
            }
            None => {
                self.snapshot.self_rss = None;
                self.snapshot.self_cpu = None;
            }
        }

        true
    }

    /// Return the most recent metrics snapshot.
    pub fn snapshot(&self) -> &MetricsSnapshot {
        &self.snapshot
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_call_always_refreshes() {
        let mut c = TelemetryCollector::with_interval(Duration::from_secs(60));
        assert!(c.maybe_refresh(Instant::now()));
    }

    #[test]
    fn second_call_within_interval_skips() {
        let mut c = TelemetryCollector::with_interval(Duration::from_secs(60));
        let now = Instant::now();
        assert!(c.maybe_refresh(now));
        assert!(!c.maybe_refresh(now));
    }

    #[test]
    fn refresh_after_interval_fires() {
        let mut c = TelemetryCollector::with_interval(Duration::from_millis(10));
        let t0 = Instant::now();
        assert!(c.maybe_refresh(t0));
        // Simulate time passing beyond the interval without sleeping.
        let t1 = t0 + Duration::from_millis(11);
        assert!(c.maybe_refresh(t1));
    }

    #[test]
    fn memory_total_nonzero_after_refresh() {
        let mut c = TelemetryCollector::new();
        c.maybe_refresh(Instant::now());
        assert!(c.snapshot().mem_total > 0);
    }

    #[test]
    fn cores_nonempty_after_refresh() {
        let mut c = TelemetryCollector::new();
        c.maybe_refresh(Instant::now());
        assert!(!c.snapshot().cpu_per_core.is_empty());
    }

    #[test]
    fn self_process_found() {
        let mut c = TelemetryCollector::new();
        c.maybe_refresh(Instant::now());
        assert!(c.snapshot().self_rss.is_some());
    }

    #[test]
    fn default_snapshot_shows_nan_cpu() {
        let c = TelemetryCollector::new();
        assert!(c.snapshot().cpu_global.is_nan());
    }
}
