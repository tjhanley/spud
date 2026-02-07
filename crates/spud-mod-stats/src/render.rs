/// Hero-pane rendering for the stats module.
///
/// Draws CPU, memory, and swap gauges plus per-core breakdown and SPUD
/// process info. Adapts layout based on available terminal height.
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, LineGauge, Paragraph, Wrap},
    Frame,
};

use crate::format::{format_bytes, format_percent};
use crate::telemetry::MetricsSnapshot;

/// Clamp a percentage (0–100) to a ratio (0.0–1.0) safe for [`LineGauge::ratio`].
fn clamp_ratio(pct: f32) -> f64 {
    (pct as f64 / 100.0).clamp(0.0, 1.0)
}

/// Choose a color based on the metric value and thresholds.
fn threshold_color(pct: f32, warn: f32, crit: f32, normal: Color) -> Color {
    if pct >= crit {
        Color::Red
    } else if pct >= warn {
        Color::Yellow
    } else {
        normal
    }
}

/// Render the full stats hero pane into the given area.
///
/// Layout adapts based on available height:
/// - **6+ rows**: CPU gauge, MEM gauge, SWP gauge, per-core grid, SPUD process
/// - **< 6 rows**: CPU, MEM, SWP gauges only (compact mode)
pub fn render_hero_content(f: &mut Frame, area: Rect, snap: &MetricsSnapshot) {
    let block = Block::default().borders(Borders::ALL).title("SYSTEM STATS");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let compact = inner.height < 6;

    if compact {
        render_compact(f, inner, snap);
    } else {
        render_full(f, inner, snap);
    }
}

/// Compact layout: just the three gauges stacked vertically.
fn render_compact(f: &mut Frame, area: Rect, snap: &MetricsSnapshot) {
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);

    render_cpu_gauge(f, rows[0], snap);
    render_mem_gauge(f, rows[1], snap);
    render_swap_gauge(f, rows[2], snap);
}

/// Full layout with gauges, per-core grid, and process info.
fn render_full(f: &mut Frame, area: Rect, snap: &MetricsSnapshot) {
    // Calculate how many rows the per-core section needs.
    let cores_per_row = if area.width >= 60 { 5 } else { 4 };
    let core_rows = if snap.cpu_per_core.is_empty() {
        1
    } else {
        snap.cpu_per_core.len().div_ceil(cores_per_row)
    };

    let rows = Layout::vertical([
        Constraint::Length(1),                // CPU gauge
        Constraint::Length(1),                // blank
        Constraint::Length(1),                // MEM gauge
        Constraint::Length(1),                // SWP gauge
        Constraint::Length(1),                // blank
        Constraint::Length(1),                // "CORES" header
        Constraint::Length(core_rows as u16), // per-core grid
        Constraint::Length(1),                // blank
        Constraint::Length(1),                // "SPUD" header
        Constraint::Length(1),                // process line
        Constraint::Min(0),                   // remaining space
    ])
    .split(area);

    render_cpu_gauge(f, rows[0], snap);
    render_mem_gauge(f, rows[2], snap);
    render_swap_gauge(f, rows[3], snap);
    render_cores(f, rows[5], rows[6], snap, cores_per_row);
    render_process(f, rows[8], rows[9], snap);
}

/// Render the global CPU gauge.
fn render_cpu_gauge(f: &mut Frame, area: Rect, snap: &MetricsSnapshot) {
    let pct = if snap.cpu_global.is_nan() {
        0.0
    } else {
        snap.cpu_global
    };
    let color = threshold_color(pct, 70.0, 90.0, Color::Green);
    let label = format!("CPU  {}", format_percent(snap.cpu_global));

    let gauge = LineGauge::default()
        .ratio(clamp_ratio(pct))
        .label(label)
        .filled_style(Style::default().fg(color))
        .unfilled_style(Style::default().fg(Color::DarkGray));
    f.render_widget(gauge, area);
}

/// Render the memory gauge with byte counts.
fn render_mem_gauge(f: &mut Frame, area: Rect, snap: &MetricsSnapshot) {
    let pct = if snap.mem_total == 0 {
        0.0
    } else {
        snap.mem_used as f32 / snap.mem_total as f32 * 100.0
    };
    let color = threshold_color(pct, 80.0, 95.0, Color::Yellow);
    let label = format!(
        "MEM  {}  ({} / {})",
        format_percent(pct),
        format_bytes(snap.mem_used),
        format_bytes(snap.mem_total)
    );

    let gauge = LineGauge::default()
        .ratio(clamp_ratio(pct))
        .label(label)
        .filled_style(Style::default().fg(color))
        .unfilled_style(Style::default().fg(Color::DarkGray));
    f.render_widget(gauge, area);
}

/// Render the swap gauge (or N/A if swap is not available).
fn render_swap_gauge(f: &mut Frame, area: Rect, snap: &MetricsSnapshot) {
    if snap.swap_total == 0 {
        let text = Paragraph::new(Line::from("SWP  N/A".dark_gray()));
        f.render_widget(text, area);
        return;
    }

    let pct = snap.swap_used as f32 / snap.swap_total as f32 * 100.0;
    let color = threshold_color(pct, 50.0, 80.0, Color::Magenta);
    let label = format!(
        "SWP  {}  ({} / {})",
        format_percent(pct),
        format_bytes(snap.swap_used),
        format_bytes(snap.swap_total)
    );

    let gauge = LineGauge::default()
        .ratio(clamp_ratio(pct))
        .label(label)
        .filled_style(Style::default().fg(color))
        .unfilled_style(Style::default().fg(Color::DarkGray));
    f.render_widget(gauge, area);
}

/// Render the per-core CPU grid.
fn render_cores(
    f: &mut Frame,
    header_area: Rect,
    grid_area: Rect,
    snap: &MetricsSnapshot,
    cores_per_row: usize,
) {
    let header = Paragraph::new(Line::from("CORES".bold()));
    f.render_widget(header, header_area);

    if snap.cpu_per_core.is_empty() {
        return;
    }

    let mut spans = Vec::new();
    for (i, &pct) in snap.cpu_per_core.iter().enumerate() {
        let color = threshold_color(pct, 70.0, 90.0, Color::Green);
        spans.push(Span::styled(
            format!("{i:>2}: {:>3.0}%", pct),
            Style::default().fg(color),
        ));
        // Separator between columns (but not at end of row).
        if (i + 1) % cores_per_row != 0 {
            spans.push(Span::raw("   "));
        } else if i + 1 < snap.cpu_per_core.len() {
            spans.push(Span::raw("\n"));
        }
    }

    let text = Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false });
    f.render_widget(text, grid_area);
}

/// Render the SPUD process info section.
fn render_process(f: &mut Frame, header_area: Rect, data_area: Rect, snap: &MetricsSnapshot) {
    let header = Paragraph::new(Line::from("SPUD".bold()));
    f.render_widget(header, header_area);

    let rss = snap
        .self_rss
        .map(format_bytes)
        .unwrap_or_else(|| "--".into());
    let cpu = snap
        .self_cpu
        .map(format_percent)
        .unwrap_or_else(|| "--%".into());

    let line = Line::from(vec![
        Span::raw("RSS: "),
        Span::styled(&rss, Style::default().fg(Color::Cyan)),
        Span::raw("   CPU: "),
        Span::styled(&cpu, Style::default().fg(Color::Cyan)),
    ]);
    f.render_widget(Paragraph::new(line), data_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    /// Helper to render into a test terminal and return the buffer.
    fn render_to_buffer(
        width: u16,
        height: u16,
        snap: &MetricsSnapshot,
    ) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_hero_content(f, f.area(), snap);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    #[test]
    fn no_panic_with_default_snapshot() {
        let snap = MetricsSnapshot::default();
        let buf = render_to_buffer(60, 20, &snap);
        let text: String = buf
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(text.contains("CPU"));
        assert!(text.contains("MEM"));
    }

    #[test]
    fn no_panic_with_zero_area() {
        let snap = MetricsSnapshot::default();
        // 2x2 means inner area after borders is 0x0.
        let _buf = render_to_buffer(2, 2, &snap);
    }

    #[test]
    fn compact_mode_under_six_rows() {
        let snap = MetricsSnapshot {
            cpu_global: 50.0,
            mem_total: 16 * 1024 * 1024 * 1024,
            mem_used: 8 * 1024 * 1024 * 1024,
            ..MetricsSnapshot::default()
        };
        // height 7 => inner height 5 (< 6) => compact mode.
        let buf = render_to_buffer(60, 7, &snap);
        let text: String = buf
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(text.contains("CPU"));
        assert!(text.contains("MEM"));
        // No CORES header in compact mode.
        assert!(!text.contains("CORES"));
    }
}
