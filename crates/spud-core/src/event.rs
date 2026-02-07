use std::time::Instant;

#[derive(Debug, Clone)]
pub enum TelemetryValue {
    Float(f64),
    Int(i64),
    Text(String),
}

#[derive(Debug, Clone)]
pub enum Event {
    Tick { now: Instant },
    Key(crossterm::event::KeyEvent),
    Resize { cols: u16, rows: u16 },
    Quit,
    ModuleActivated { id: String },
    ModuleDeactivated { id: String },
    Telemetry { source: String, key: String, value: TelemetryValue },
    Custom { tag: String, payload: String },
}
