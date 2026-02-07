use std::collections::VecDeque;

use crate::logging::LogEntry;

pub struct Console {
    pub visible: bool,
    log_lines: VecDeque<LogEntry>,
    pub input_buffer: String,
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

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn push_log(&mut self, entry: LogEntry) {
        if self.log_lines.len() >= self.max_lines {
            self.log_lines.pop_front();
            // Adjust scroll offset if we're scrolled up
            if self.scroll_offset > 0 {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
        }
        self.log_lines.push_back(entry);
    }

    pub fn log_lines(&self) -> &VecDeque<LogEntry> {
        &self.log_lines
    }

    pub fn clear_logs(&mut self) {
        self.log_lines.clear();
        self.scroll_offset = 0;
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let max_offset = self.log_lines.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn insert_char(&mut self, c: char) {
        self.input_buffer.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            // Find the previous char boundary
            let prev = self.input_buffer[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input_buffer.remove(prev);
            self.cursor_pos = prev;
        }
    }

    pub fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.input_buffer[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn cursor_right(&mut self) {
        if self.cursor_pos < self.input_buffer.len() {
            self.cursor_pos = self.input_buffer[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.input_buffer.len());
        }
    }

    /// Submit the current input buffer. Returns the input and clears the buffer.
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
        c.scroll_up(100); // should clamp to max
        assert_eq!(c.scroll_offset(), 9); // 10 lines, max offset = 9
        c.scroll_down(3);
        assert_eq!(c.scroll_offset(), 6);
        c.scroll_down(100); // should clamp to 0
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
