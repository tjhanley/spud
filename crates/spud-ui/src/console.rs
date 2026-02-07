use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use spud_core::console::Console;
use spud_core::logging::LogLevel;

/// Render the drop-down console overlay covering the top half of the screen.
///
/// The overlay consists of three bands:
/// 1. **Title bar** — shows `CONSOLE` label, current TPS, and close hint.
/// 2. **Log area** — colour-coded log entries with scroll support.
/// 3. **Input line** — single-line command input with cursor.
pub fn render_console(
    f: &mut Frame,
    area: Rect,
    console: &Console,
    tps: f64,
    fraction: f64,
    show_cursor: bool,
) {
    let max_height = area.height / 2;
    let mut overlay_height = ((max_height as f64) * fraction).round() as u16;
    // Need at least 3 rows for title + log + input; clamp during animation,
    // skip entirely when fully hidden.
    if overlay_height < 3 {
        if fraction > 0.0 {
            overlay_height = 3;
        } else {
            return;
        }
    }
    let overlay = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: overlay_height,
    };

    // Clear the area behind the overlay
    f.render_widget(Clear, overlay);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(1),    // log area
            Constraint::Length(1), // input line
        ])
        .split(overlay);

    // Title bar with TPS
    let title = Line::from(vec![
        Span::styled(
            " CONSOLE ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  TPS: {:.1}  ", tps)),
        Span::styled("~ to close", Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(
        Paragraph::new(title).style(Style::default().bg(Color::DarkGray).fg(Color::White)),
        chunks[0],
    );

    // Log lines with color-coded levels
    let log_lines = console.log_lines();
    let visible_height = chunks[1].height as usize;
    let total = log_lines.len();
    let scroll_offset = console.scroll_offset();

    let start = if total > visible_height + scroll_offset {
        total - visible_height - scroll_offset
    } else {
        0
    };
    let end = total.saturating_sub(scroll_offset);

    let lines: Vec<Line> = log_lines
        .iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .map(|entry| {
            let level_color = match entry.level {
                LogLevel::Error => Color::Red,
                LogLevel::Warn => Color::Yellow,
                LogLevel::Info => Color::Green,
                LogLevel::Debug => Color::Cyan,
                LogLevel::Trace => Color::DarkGray,
            };
            Line::from(vec![
                Span::styled(
                    format!(" {:5} ", entry.level),
                    Style::default()
                        .fg(level_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("[{}] ", entry.target),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(&entry.message),
            ])
        })
        .collect();

    let log_block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT)
        .style(Style::default().bg(Color::Black));

    f.render_widget(
        Paragraph::new(lines)
            .block(log_block)
            .wrap(Wrap { trim: false }),
        chunks[1],
    );

    // Input line
    let input_line = Line::from(vec![
        Span::styled(
            "> ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(&console.input_buffer),
    ]);
    f.render_widget(
        Paragraph::new(input_line).style(Style::default().bg(Color::Black).fg(Color::White)),
        chunks[2],
    );

    // Position cursor in the input field only when fully open
    if show_cursor {
        let display_col = console.input_buffer[..console.cursor_pos].width() as u16;
        f.set_cursor_position((chunks[2].x + 2 + display_col, chunks[2].y));
    }
}
