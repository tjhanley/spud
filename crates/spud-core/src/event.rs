use std::time::Instant;

/// A typed value attached to a [`Event::Telemetry`] event.
#[derive(Debug, Clone)]
pub enum TelemetryValue {
    /// A floating-point metric (e.g. CPU percentage).
    Float(f64),
    /// An integer metric (e.g. byte count).
    Int(i64),
    /// A textual metric (e.g. version string).
    Text(String),
}

/// Events flowing through the SPUD event bus.
///
/// The app loop publishes these into the [`crate::bus::EventBus`], then drains
/// and broadcasts them to modules via [`crate::registry::ModuleRegistry::broadcast`].
#[derive(Debug, Clone)]
pub enum Event {
    /// Periodic tick with the current timestamp. Sent to all modules.
    Tick { now: Instant },
    /// Keyboard input. Sent to the active module only.
    Key(crossterm::event::KeyEvent),
    /// Terminal resize. Sent to all modules.
    Resize { cols: u16, rows: u16 },
    /// Request to shut down the application.
    Quit,
    /// A module has become the active (foreground) module.
    ModuleActivated { id: String },
    /// A module has been moved to the background.
    ModuleDeactivated { id: String },
    /// A telemetry data point emitted by a module or subsystem.
    Telemetry { source: String, key: String, value: TelemetryValue },
    /// An application-defined event for extension points.
    Custom { tag: String, payload: String },
}
