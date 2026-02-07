use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::logging::LogEntry;

/// Animation state for the drop-down console slide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlideState {
    /// Console is fully hidden.
    Hidden,
    /// Console is sliding open.
    Opening { started_at: Instant },
    /// Console is fully open.
    Open,
    /// Console is sliding closed.
    Closing { started_at: Instant },
}

/// Drop-down console state.
///
/// Manages visibility, a ring buffer of log lines, a single-line input buffer
/// with cursor, and scroll position. The console does **not** own rendering —
/// see [`spud_ui::console::render_console`] for the TUI layer.
pub struct Console {
    /// Current slide animation state.
    pub slide: SlideState,
    /// Duration of the slide animation (must be > 0).
    slide_duration: Duration,
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
    ///
    pub fn new(max_lines: usize) -> Self {
        let slide_duration = Duration::from_millis(250);
        debug_assert!(!slide_duration.is_zero(), "slide_duration must be > 0");
        Self {
            slide: SlideState::Hidden,
            slide_duration,
            log_lines: VecDeque::with_capacity(max_lines),
            input_buffer: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            max_lines,
        }
    }

    /// Toggle the console open/closed. Handles mid-animation reversals
    /// by preserving the current visual position.
    pub fn toggle(&mut self, now: Instant) {
        self.slide = match self.slide {
            SlideState::Hidden => SlideState::Opening { started_at: now },
            SlideState::Open => SlideState::Closing { started_at: now },
            SlideState::Opening { started_at } => {
                // Reverse: compute current open fraction, then backdate the
                // Closing started_at so its fraction equals the same position.
                // Closing fraction = 1.0 - close_progress, so we need
                // close_progress = 1.0 - open_fraction.
                let elapsed = now
                    .checked_duration_since(started_at)
                    .unwrap_or(Duration::ZERO);
                let open_fraction =
                    (elapsed.as_secs_f64() / self.slide_duration.as_secs_f64()).min(1.0);
                let elapsed_closing = self.slide_duration.mul_f64(1.0 - open_fraction);
                SlideState::Closing {
                    started_at: now - elapsed_closing,
                }
            }
            SlideState::Closing { started_at } => {
                // Reverse: compute current visible fraction, then backdate the
                // Opening started_at so its fraction equals the same position.
                // Opening fraction = open_progress, and current fraction =
                // 1.0 - close_progress, so open_progress = 1.0 - close_progress.
                let elapsed = now
                    .checked_duration_since(started_at)
                    .unwrap_or(Duration::ZERO);
                let close_progress =
                    (elapsed.as_secs_f64() / self.slide_duration.as_secs_f64()).min(1.0);
                let elapsed_opening = self.slide_duration.mul_f64(1.0 - close_progress);
                SlideState::Opening {
                    started_at: now - elapsed_opening,
                }
            }
        };
    }

    /// Advance animation state. Call each loop iteration before rendering.
    pub fn update(&mut self, now: Instant) {
        self.slide = match self.slide {
            SlideState::Opening { started_at } => {
                if now
                    .checked_duration_since(started_at)
                    .unwrap_or(Duration::ZERO)
                    >= self.slide_duration
                {
                    SlideState::Open
                } else {
                    self.slide
                }
            }
            SlideState::Closing { started_at } => {
                if now
                    .checked_duration_since(started_at)
                    .unwrap_or(Duration::ZERO)
                    >= self.slide_duration
                {
                    SlideState::Hidden
                } else {
                    self.slide
                }
            }
            other => other,
        };
    }

    /// Returns 0.0 (hidden) to 1.0 (fully open), linearly interpolated.
    pub fn overlay_fraction(&self, now: Instant) -> f64 {
        match self.slide {
            SlideState::Hidden => 0.0,
            SlideState::Open => 1.0,
            SlideState::Opening { started_at } => {
                let elapsed = now
                    .checked_duration_since(started_at)
                    .unwrap_or(Duration::ZERO);
                (elapsed.as_secs_f64() / self.slide_duration.as_secs_f64()).min(1.0)
            }
            SlideState::Closing { started_at } => {
                let elapsed = now
                    .checked_duration_since(started_at)
                    .unwrap_or(Duration::ZERO);
                let progress = (elapsed.as_secs_f64() / self.slide_duration.as_secs_f64()).min(1.0);
                1.0 - progress
            }
        }
    }

    /// Returns true if the console needs rendering (any state except Hidden).
    pub fn is_visible(&self) -> bool {
        !matches!(self.slide, SlideState::Hidden)
    }

    /// Returns true only when fully open (accepts keyboard input).
    pub fn is_open(&self) -> bool {
        matches!(self.slide, SlideState::Open)
    }

    /// Returns the slide animation duration.
    pub fn slide_duration(&self) -> Duration {
        self.slide_duration
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
    fn toggle_from_hidden_starts_opening() {
        let mut c = Console::default();
        let now = Instant::now();
        assert_eq!(c.slide, SlideState::Hidden);
        c.toggle(now);
        assert!(matches!(c.slide, SlideState::Opening { .. }));
    }

    #[test]
    fn toggle_from_open_starts_closing() {
        let mut c = Console::default();
        let now = Instant::now();
        c.slide = SlideState::Open;
        c.toggle(now);
        assert!(matches!(c.slide, SlideState::Closing { .. }));
    }

    #[test]
    fn toggle_mid_opening_reverses_to_closing() {
        let mut c = Console::default();
        let start = Instant::now();
        c.toggle(start); // Hidden -> Opening

        // Advance halfway through the animation
        let mid = start + c.slide_duration() / 2;
        c.toggle(mid); // Opening -> Closing (preserving position)

        assert!(matches!(c.slide, SlideState::Closing { .. }));
        // Fraction at the reversal point should be ~0.5
        let frac = c.overlay_fraction(mid);
        assert!((frac - 0.5).abs() < 0.01);
    }

    #[test]
    fn toggle_mid_closing_reverses_to_opening() {
        let mut c = Console::default();
        let start = Instant::now();
        c.slide = SlideState::Open;
        c.toggle(start); // Open -> Closing

        let mid = start + c.slide_duration() / 2;
        c.toggle(mid); // Closing -> Opening (preserving position)

        assert!(matches!(c.slide, SlideState::Opening { .. }));
        let frac = c.overlay_fraction(mid);
        assert!((frac - 0.5).abs() < 0.01);
    }

    #[test]
    fn toggle_mid_opening_reverses_at_quarter() {
        let mut c = Console::default();
        let start = Instant::now();
        c.toggle(start); // Hidden -> Opening

        // Advance 25% through the animation
        let quarter = start + c.slide_duration() / 4;
        let frac_before = c.overlay_fraction(quarter);
        assert!(
            (frac_before - 0.25).abs() < 0.01,
            "expected ~0.25, got {frac_before}"
        );

        c.toggle(quarter); // Opening -> Closing (preserving position)
        assert!(matches!(c.slide, SlideState::Closing { .. }));

        let frac_after = c.overlay_fraction(quarter);
        assert!(
            (frac_after - 0.25).abs() < 0.01,
            "expected ~0.25 after reversal, got {frac_after}"
        );
    }

    #[test]
    fn toggle_mid_closing_reverses_at_quarter() {
        let mut c = Console::default();
        let start = Instant::now();
        c.slide = SlideState::Open;
        c.toggle(start); // Open -> Closing

        // Advance 25% through closing (fraction should be 0.75)
        let quarter = start + c.slide_duration() / 4;
        let frac_before = c.overlay_fraction(quarter);
        assert!(
            (frac_before - 0.75).abs() < 0.01,
            "expected ~0.75, got {frac_before}"
        );

        c.toggle(quarter); // Closing -> Opening (preserving position)
        assert!(matches!(c.slide, SlideState::Opening { .. }));

        let frac_after = c.overlay_fraction(quarter);
        assert!(
            (frac_after - 0.75).abs() < 0.01,
            "expected ~0.75 after reversal, got {frac_after}"
        );
    }

    #[test]
    fn update_transitions_opening_to_open() {
        let mut c = Console::default();
        let start = Instant::now();
        c.toggle(start);

        let after = start + c.slide_duration();
        c.update(after);
        assert_eq!(c.slide, SlideState::Open);
    }

    #[test]
    fn update_transitions_closing_to_hidden() {
        let mut c = Console::default();
        let start = Instant::now();
        c.slide = SlideState::Open;
        c.toggle(start);

        let after = start + c.slide_duration();
        c.update(after);
        assert_eq!(c.slide, SlideState::Hidden);
    }

    #[test]
    fn overlay_fraction_hidden_is_zero() {
        let c = Console::default();
        assert_eq!(c.overlay_fraction(Instant::now()), 0.0);
    }

    #[test]
    fn overlay_fraction_open_is_one() {
        let mut c = Console::default();
        c.slide = SlideState::Open;
        assert_eq!(c.overlay_fraction(Instant::now()), 1.0);
    }

    #[test]
    fn overlay_fraction_mid_animation() {
        let mut c = Console::default();
        let start = Instant::now();
        c.toggle(start);

        let mid = start + c.slide_duration() / 2;
        let frac = c.overlay_fraction(mid);
        assert!(frac > 0.4 && frac < 0.6, "expected ~0.5, got {frac}");
    }

    #[test]
    fn is_visible_false_only_for_hidden() {
        let mut c = Console::default();
        assert!(!c.is_visible());

        let now = Instant::now();
        c.toggle(now); // Opening
        assert!(c.is_visible());

        c.slide = SlideState::Open;
        assert!(c.is_visible());

        c.toggle(now); // Closing
        assert!(c.is_visible());
    }

    #[test]
    fn is_open_only_for_open_state() {
        let mut c = Console::default();
        assert!(!c.is_open());

        let now = Instant::now();
        c.toggle(now); // Opening
        assert!(!c.is_open());

        c.slide = SlideState::Open;
        assert!(c.is_open());

        c.toggle(now); // Closing
        assert!(!c.is_open());
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
