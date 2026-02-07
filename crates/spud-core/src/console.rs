use std::collections::VecDeque;

use crate::logging::LogEntry;

/// Drop-down console state.
///
/// Manages visibility, a ring buffer of log lines, a single-line input buffer
/// with cursor, and scroll position. The console does **not** own rendering —
/// see [`spud_ui::console::render_console`] for the TUI layer.
pub struct Console {
    /// Whether the console overlay is currently visible.
    pub visible: bool,
    log_lines: VecDeque<LogEntry>,
    /// The current text in the input line.
    pub input_buffer: String,
    /// Byte offset of the cursor within `input_buffer`.
    pub cursor_pos: usize,
    scroll_offset: usize,
    max_lines: usize,
}

impl Default for Console {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl Console {
    /// Create a new console with the given maximum log line capacity.
    pub fn new(max_lines: usize) -> Self {
        Self {
            visible: false,
            log_lines: VecDeque::with_capacity(max_lines),
            input_buffer: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            max_lines,
        }
    }

    /// Toggle the console's visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Append a log entry. Drops the oldest entry if the buffer is full.
    pub fn push_log(&mut self, entry: LogEntry) {
        if self.log_lines.len() >= self.max_lines {
            self.log_lines.pop_front();
            if self.scroll_offset > 0 {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
        }
        self.log_lines.push_back(entry);
    }

    /// Return a reference to the log line buffer.
    pub fn log_lines(&self) -> &VecDeque<LogEntry> {
        &self.log_lines
    }

    /// Clear all log lines and reset the scroll position.
    pub fn clear_logs(&mut self) {
        self.log_lines.clear();
        self.scroll_offset = 0;
    }

    /// Return the current scroll offset (0 = bottom / most recent).
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Scroll up (toward older entries) by `amount` lines, clamped to bounds.
    pub fn scroll_up(&mut self, amount: usize) {
        let max_offset = self.log_lines.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
    }

    /// Scroll down (toward newer entries) by `amount` lines, clamped to 0.
    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Insert a character at the current cursor position.
    pub fn insert_char(&mut self, c: char) {
        self.input_buffer.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input_buffer[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input_buffer.remove(prev);
            self.cursor_pos = prev;
        }
    }

    /// Move the cursor one character to the left.
    pub fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.input_buffer[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    /// Move the cursor one character to the right.
    pub fn cursor_right(&mut self) {
        if self.cursor_pos < self.input_buffer.len() {
            self.cursor_pos = self.input_buffer[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.input_buffer.len());
        }
    }

    /// Submit the current input, returning the text and clearing the buffer.
    pub fn submit_input(&mut self) -> String {
        let input = self.input_buffer.clone();
        self.input_buffer.clear();
        self.cursor_pos = 0;
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::LogLevel;

    fn entry(msg: &str) -> LogEntry {
        LogEntry {
            level: LogLevel::Info,
            target: "test".into(),
            message: msg.into(),
        }
    }

    #[test]
    fn toggle_flips_visibility() {
        let mut c = Console::default();
        assert!(!c.visible);
        c.toggle();
        assert!(c.visible);
        c.toggle();
        assert!(!c.visible);
    }

    #[test]
    fn push_log_adds_entries() {
        let mut c = Console::new(10);
        c.push_log(entry("hello"));
        c.push_log(entry("world"));
        assert_eq!(c.log_lines().len(), 2);
    }

    #[test]
    fn ring_buffer_caps_at_max_lines() {
        let mut c = Console::new(3);
        for i in 0..5 {
            c.push_log(entry(&format!("msg {}", i)));
        }
        assert_eq!(c.log_lines().len(), 3);
        assert_eq!(c.log_lines()[0].message, "msg 2");
        assert_eq!(c.log_lines()[2].message, "msg 4");
    }

    #[test]
    fn scroll_up_and_down_clamp() {
        let mut c = Console::new(100);
        for i in 0..10 {
            c.push_log(entry(&format!("msg {}", i)));
        }
        c.scroll_up(5);
        assert_eq!(c.scroll_offset(), 5);
        c.scroll_up(100);
        assert_eq!(c.scroll_offset(), 9);
        c.scroll_down(3);
        assert_eq!(c.scroll_offset(), 6);
        c.scroll_down(100);
        assert_eq!(c.scroll_offset(), 0);
    }

    #[test]
    fn submit_input_returns_and_clears() {
        let mut c = Console::default();
        c.insert_char('h');
        c.insert_char('i');
        assert_eq!(c.input_buffer, "hi");
        let result = c.submit_input();
        assert_eq!(result, "hi");
        assert!(c.input_buffer.is_empty());
        assert_eq!(c.cursor_pos, 0);
    }

    #[test]
    fn input_buffer_editing() {
        let mut c = Console::default();
        c.insert_char('a');
        c.insert_char('b');
        c.insert_char('c');
        assert_eq!(c.input_buffer, "abc");
        assert_eq!(c.cursor_pos, 3);

        c.backspace();
        assert_eq!(c.input_buffer, "ab");
        assert_eq!(c.cursor_pos, 2);

        c.cursor_left();
        assert_eq!(c.cursor_pos, 1);
        c.insert_char('x');
        assert_eq!(c.input_buffer, "axb");
        assert_eq!(c.cursor_pos, 2);

        c.cursor_right();
        assert_eq!(c.cursor_pos, 3);
    }

    #[test]
    fn cursor_left_at_zero_is_noop() {
        let mut c = Console::default();
        c.cursor_left();
        assert_eq!(c.cursor_pos, 0);
    }

    #[test]
    fn cursor_right_at_end_is_noop() {
        let mut c = Console::default();
        c.insert_char('a');
        assert_eq!(c.cursor_pos, 1);
        c.cursor_right();
        assert_eq!(c.cursor_pos, 1);
    }

    #[test]
    fn backspace_at_zero_is_noop() {
        let mut c = Console::default();
        c.backspace();
        assert_eq!(c.input_buffer, "");
        assert_eq!(c.cursor_pos, 0);
    }

    #[test]
    fn clear_logs_empties_and_resets_scroll() {
        let mut c = Console::new(100);
        for i in 0..10 {
            c.push_log(entry(&format!("msg {}", i)));
        }
        c.scroll_up(5);
        c.clear_logs();
        assert!(c.log_lines().is_empty());
        assert_eq!(c.scroll_offset(), 0);
    }
}
