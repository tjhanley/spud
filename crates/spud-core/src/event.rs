use std::time::Instant;

#[derive(Debug, Clone)]
pub enum Event {
    Tick { now: Instant },
    Key(crossterm::event::KeyEvent),
    Resize { cols: u16, rows: u16 },
    Quit,
}
