use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tracing_appender::rolling;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Log severity level (mirrors tracing levels for UI use).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// A single log entry for display in the console overlay.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub target: String,
    pub message: String,
}

/// Shared ring buffer for log entries consumed by the console UI.
pub type LogBuffer = Arc<Mutex<VecDeque<LogEntry>>>;

/// Create a new shared log buffer with a given capacity.
pub fn new_log_buffer(capacity: usize) -> LogBuffer {
    Arc::new(Mutex::new(VecDeque::with_capacity(capacity)))
}

/// Return the log directory path.
///
/// Precedence: `SPUD_LOG_DIR` env var > platform default.
/// macOS: `~/Library/Logs/spud/`
/// Linux: `$XDG_DATA_HOME/spud/logs/` or `~/.local/share/spud/logs/`
pub fn log_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SPUD_LOG_DIR") {
        return PathBuf::from(dir);
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            return home.join("Library").join("Logs").join("spud");
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Some(data) = dirs::data_dir() {
            return data.join("spud").join("logs");
        }
    }

    PathBuf::from("logs")
}

const MAX_CONSOLE_LINES: usize = 1000;
const LOG_RETENTION_DAYS: u64 = 7;

/// Remove SPUD log files older than `max_age_days` from the given directory.
///
/// Only deletes files whose name starts with `spud.log` (the prefix used by
/// the daily rolling appender) to avoid accidentally removing unrelated files
/// if the log directory is shared.
fn cleanup_old_logs(log_path: &std::path::Path, max_age_days: u64) {
    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(max_age_days * 86400);
    if let Ok(entries) = std::fs::read_dir(log_path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("spud.log") {
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if modified < cutoff {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
}

/// A tracing layer that pushes log entries into a shared ring buffer.
struct ConsoleLayer {
    buffer: LogBuffer,
    max_lines: usize,
}

impl<S: tracing::Subscriber> Layer<S> for ConsoleLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let level = match *event.metadata().level() {
            tracing::Level::TRACE => LogLevel::Trace,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        };

        let target = event.metadata().target().to_string();

        let mut visitor = MessageVisitor {
            message: None,
            fields: Vec::new(),
        };
        event.record(&mut visitor);
        let message = visitor.finish();

        let entry = LogEntry {
            level,
            target,
            message,
        };

        if let Ok(mut buf) = self.buffer.lock() {
            if buf.len() >= self.max_lines {
                buf.pop_front();
            }
            buf.push_back(entry);
        }
    }
}

struct MessageVisitor {
    message: Option<String>,
    fields: Vec<String>,
}

impl MessageVisitor {
    fn finish(self) -> String {
        match self.message {
            Some(msg) if self.fields.is_empty() => msg,
            Some(msg) => format!("{} {}", msg, self.fields.join(" ")),
            None if self.fields.is_empty() => String::new(),
            None => self.fields.join(" "),
        }
    }
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        } else {
            self.fields.push(format!("{}={:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields.push(format!("{}={}", field.name(), value));
        }
    }
}

/// Initialize the logging subsystem. Returns the shared log buffer for the console.
///
/// Filter controlled by `SPUD_LOG` or `RUST_LOG` (default: `info`).
/// File output: daily rotation in `log_dir()`, 7-day retention.
/// Console buffer: ring buffer of `MAX_CONSOLE_LINES` entries.
pub fn init() -> LogBuffer {
    let buffer = new_log_buffer(MAX_CONSOLE_LINES);

    let filter = EnvFilter::try_from_env("SPUD_LOG")
        .or_else(|_| EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let log_path = log_dir();
    if let Err(e) = std::fs::create_dir_all(&log_path) {
        eprintln!(
            "warning: failed to create log directory {:?}: {}",
            log_path, e
        );
    }

    cleanup_old_logs(&log_path, LOG_RETENTION_DAYS);

    let file_appender = rolling::daily(&log_path, "spud.log");
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_target(true);

    let console_layer = ConsoleLayer {
        buffer: buffer.clone(),
        max_lines: MAX_CONSOLE_LINES,
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(console_layer)
        .init();

    buffer
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    // Serialize env-mutating tests to avoid data races.
    static ENV_LOCK: StdMutex<()> = StdMutex::new(());

    #[test]
    fn log_dir_respects_env_override() {
        let _guard = ENV_LOCK.lock().unwrap();
        let original = std::env::var("SPUD_LOG_DIR").ok();

        unsafe { std::env::set_var("SPUD_LOG_DIR", "/tmp/spud-test-logs") };
        assert_eq!(log_dir(), PathBuf::from("/tmp/spud-test-logs"));

        match original {
            Some(v) => unsafe { std::env::set_var("SPUD_LOG_DIR", v) },
            None => unsafe { std::env::remove_var("SPUD_LOG_DIR") },
        }
    }

    #[test]
    fn log_dir_default_on_macos() {
        let _guard = ENV_LOCK.lock().unwrap();
        let original = std::env::var("SPUD_LOG_DIR").ok();

        unsafe { std::env::remove_var("SPUD_LOG_DIR") };
        let dir = log_dir();

        #[cfg(target_os = "macos")]
        {
            let expected = dirs::home_dir().unwrap().join("Library/Logs/spud");
            assert_eq!(dir, expected);
        }

        if let Some(v) = original {
            unsafe { std::env::set_var("SPUD_LOG_DIR", v) };
        }
    }

    #[test]
    fn console_ring_buffer_caps_at_max() {
        let buf = new_log_buffer(3);
        {
            let mut b = buf.lock().unwrap();
            for i in 0..5 {
                if b.len() >= 3 {
                    b.pop_front();
                }
                b.push_back(LogEntry {
                    level: LogLevel::Info,
                    target: "test".into(),
                    message: format!("msg {}", i),
                });
            }
        }
        let b = buf.lock().unwrap();
        assert_eq!(b.len(), 3);
        assert_eq!(b[0].message, "msg 2");
        assert_eq!(b[1].message, "msg 3");
        assert_eq!(b[2].message, "msg 4");
    }

    #[test]
    fn log_entry_has_correct_fields() {
        let entry = LogEntry {
            level: LogLevel::Warn,
            target: "spud_core::foo".into(),
            message: "something happened".into(),
        };
        assert_eq!(entry.level, LogLevel::Warn);
        assert_eq!(entry.target, "spud_core::foo");
        assert_eq!(entry.message, "something happened");
    }

    #[test]
    fn log_level_display() {
        assert_eq!(format!("{}", LogLevel::Trace), "TRACE");
        assert_eq!(format!("{}", LogLevel::Debug), "DEBUG");
        assert_eq!(format!("{}", LogLevel::Info), "INFO");
        assert_eq!(format!("{}", LogLevel::Warn), "WARN");
        assert_eq!(format!("{}", LogLevel::Error), "ERROR");
    }

    #[test]
    fn message_visitor_message_only() {
        let v = MessageVisitor {
            message: Some("hello".into()),
            fields: Vec::new(),
        };
        assert_eq!(v.finish(), "hello");
    }

    #[test]
    fn message_visitor_fields_preserved() {
        let v = MessageVisitor {
            message: Some("hello".into()),
            fields: vec!["key=val".into()],
        };
        let result = v.finish();
        assert!(result.contains("hello"));
        assert!(result.contains("key=val"));
    }

    #[test]
    fn message_visitor_fields_without_message() {
        let v = MessageVisitor {
            message: None,
            fields: vec!["a=1".into(), "b=2".into()],
        };
        assert_eq!(v.finish(), "a=1 b=2");
    }

    #[test]
    fn message_visitor_empty() {
        let v = MessageVisitor {
            message: None,
            fields: Vec::new(),
        };
        assert_eq!(v.finish(), "");
    }

    #[test]
    fn cleanup_old_logs_removes_stale_files() {
        let tmp = std::env::temp_dir().join("spud-test-cleanup");
        let _ = std::fs::create_dir_all(&tmp);

        let spud_a = tmp.join("spud.log.2025-01-01");
        let spud_b = tmp.join("spud.log.2025-01-02");
        let other = tmp.join("other.txt");
        std::fs::write(&spud_a, "a").unwrap();
        std::fs::write(&spud_b, "b").unwrap();
        std::fs::write(&other, "c").unwrap();

        // max_age_days=0 means cutoff is "now", so all matching files get cleaned
        cleanup_old_logs(&tmp, 0);
        assert!(!spud_a.exists(), "spud log file should be deleted");
        assert!(!spud_b.exists(), "spud log file should be deleted");
        assert!(other.exists(), "non-spud file should be preserved");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
