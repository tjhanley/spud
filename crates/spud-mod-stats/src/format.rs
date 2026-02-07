//! Human-readable formatting utilities for system metrics.
//!
//! All functions are pure and easy to test in isolation.

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;
const GIB: u64 = 1024 * MIB;
const TIB: u64 = 1024 * GIB;

/// Format a byte count as a human-readable string using binary units.
///
/// Returns values like `"1.2 GiB"`, `"384 MiB"`, `"0 B"`.
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= TIB {
        format!("{:.1} TiB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format a percentage value for display.
///
/// Returns `"--%"` for NaN values, otherwise formats as `"45.2%"`.
pub fn format_percent(value: f32) -> String {
    if value.is_nan() {
        "--%".into()
    } else {
        format!("{:.1}%", value)
    }
}

/// Format a duration in seconds as a human-readable uptime string.
///
/// Returns values like `"2h 15m 30s"`, `"3d 1h 45m"`, `"0s"`.
#[allow(dead_code)]
pub fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;

    if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h {mins}m {s}s")
    } else if mins > 0 {
        format!("{mins}m {s}s")
    } else {
        format!("{s}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn format_bytes_below_kib() {
        assert_eq!(format_bytes(512), "512 B");
    }

    #[test]
    fn format_bytes_exact_kib() {
        assert_eq!(format_bytes(1024), "1.0 KiB");
    }

    #[test]
    fn format_bytes_mib_range() {
        assert_eq!(format_bytes(384 * MIB), "384.0 MiB");
    }

    #[test]
    fn format_bytes_gib_range() {
        assert_eq!(format_bytes(GIB + GIB / 5), "1.2 GiB");
    }

    #[test]
    fn format_bytes_tib_range() {
        assert_eq!(format_bytes(2 * TIB), "2.0 TiB");
    }

    #[test]
    fn format_percent_normal() {
        assert_eq!(format_percent(45.2), "45.2%");
    }

    #[test]
    fn format_percent_zero() {
        assert_eq!(format_percent(0.0), "0.0%");
    }

    #[test]
    fn format_percent_nan() {
        assert_eq!(format_percent(f32::NAN), "--%");
    }

    #[test]
    fn format_percent_hundred() {
        assert_eq!(format_percent(100.0), "100.0%");
    }

    #[test]
    fn format_uptime_zero() {
        assert_eq!(format_uptime(0), "0s");
    }

    #[test]
    fn format_uptime_seconds_only() {
        assert_eq!(format_uptime(45), "45s");
    }

    #[test]
    fn format_uptime_minutes_seconds() {
        assert_eq!(format_uptime(125), "2m 5s");
    }

    #[test]
    fn format_uptime_hours() {
        assert_eq!(format_uptime(2 * 3600 + 15 * 60 + 30), "2h 15m 30s");
    }

    #[test]
    fn format_uptime_days() {
        assert_eq!(format_uptime(3 * 86400 + 3600 + 45 * 60), "3d 1h 45m");
    }
}
