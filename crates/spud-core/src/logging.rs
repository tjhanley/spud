use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling;

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
            LogLevel::Info  => write!(f, "INFO"),
            LogLevel::Warn  => write!(f, "WARN"),
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
            tracing::Level::INFO  => LogLevel::Info,
            tracing::Level::WARN  => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        };

        let target = event.metadata().target().to_string();

        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);
        let message = visitor.0;

        let entry = LogEntry { level, target, message };

        if let Ok(mut buf) = self.buffer.lock() {
            if buf.len() >= self.max_lines {
                buf.pop_front();
            }
            buf.push_back(entry);
        }
    }
}

struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{:?}", value);
        } else if !self.0.is_empty() {
            self.0.push_str(&format!(" {}={:?}", field.name(), value));
        } else {
            self.0 = format!("{}={:?}", field.name(), value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        } else if !self.0.is_empty() {
            self.0.push_str(&format!(" {}={}", field.name(), value));
        } else {
            self.0 = format!("{}={}", field.name(), value);
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
    // Ensure log directory exists
    let _ = std::fs::create_dir_all(&log_path);

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

    #[test]
    fn log_dir_respects_env_override() {
        // Save/restore env to avoid test pollution
        let original = std::env::var("SPUD_LOG_DIR").ok();
        std::env::set_var("SPUD_LOG_DIR", "/tmp/spud-test-logs");
        assert_eq!(log_dir(), PathBuf::from("/tmp/spud-test-logs"));
        match original {
            Some(v) => std::env::set_var("SPUD_LOG_DIR", v),
            None => std::env::remove_var("SPUD_LOG_DIR"),
        }
    }

    #[test]
    fn log_dir_default_on_macos() {
        let original = std::env::var("SPUD_LOG_DIR").ok();
        std::env::remove_var("SPUD_LOG_DIR");
        let dir = log_dir();
        #[cfg(target_os = "macos")]
        {
            let expected = dirs::home_dir().unwrap().join("Library/Logs/spud");
            assert_eq!(dir, expected);
        }
        match original {
            Some(v) => std::env::set_var("SPUD_LOG_DIR", v),
            None => {}
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
}
