use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

/// Build display text for the agent face.
///
/// Supports two input modes:
/// - Plain text lines (fallback behavior).
/// - Pixel-art rows encoded with palette keys (Claude-style sprite rendering).
pub fn build_face_text(lines: &[String]) -> Text<'static> {
    if let Some(pixel_rows) = parse_pixel_rows(lines) {
        render_pixel_rows(&pixel_rows)
    } else {
        Text::from(
            lines
                .iter()
                .cloned()
                .map(Line::from)
                .collect::<Vec<Line<'static>>>(),
        )
    }
}

fn parse_pixel_rows(lines: &[String]) -> Option<Vec<Vec<char>>> {
    if lines.is_empty() {
        return None;
    }

    let mut rows = Vec::with_capacity(lines.len());
    for line in lines {
        let mut row = Vec::with_capacity(line.chars().count());
        for ch in line.chars() {
            if palette_color(ch).is_none() && ch != '.' {
                return None;
            }
            row.push(ch);
        }
        rows.push(row);
    }

    Some(rows)
}

fn render_pixel_rows(rows: &[Vec<char>]) -> Text<'static> {
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut rendered = Vec::new();
    let mut y = 0usize;

    while y < rows.len() {
        let top = &rows[y];
        let bottom = rows.get(y + 1);
        let mut spans = Vec::with_capacity(width);

        for x in 0..width {
            let top_color = top.get(x).copied().and_then(palette_color);
            let bottom_color = bottom
                .and_then(|row| row.get(x).copied())
                .and_then(palette_color);

            let (glyph, style) = match (top_color, bottom_color) {
                (Some(top), Some(bottom)) if top == bottom => ('█', Style::default().fg(top)),
                (Some(top), Some(bottom)) => ('▀', Style::default().fg(top).bg(bottom)),
                (Some(top), None) => ('▀', Style::default().fg(top)),
                (None, Some(bottom)) => ('▄', Style::default().fg(bottom)),
                (None, None) => (' ', Style::default()),
            };

            spans.push(Span::styled(glyph.to_string(), style));
        }

        rendered.push(Line::from(spans));
        y += 2;
    }

    Text::from(rendered)
}

fn palette_color(ch: char) -> Option<Color> {
    match ch {
        '.' => None,
        'O' => Some(Color::Rgb(255, 141, 92)),
        'o' => Some(Color::Rgb(242, 111, 72)),
        'd' => Some(Color::Rgb(217, 81, 56)),
        'k' => Some(Color::Rgb(10, 14, 30)),
        'h' => Some(Color::Rgb(255, 184, 132)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_fallback_for_non_palette_lines() {
        let lines = vec!["hello".to_string(), "world".to_string()];
        let text = build_face_text(&lines);
        assert_eq!(text.lines.len(), 2);
    }

    #[test]
    fn pixel_rows_are_packed_to_half_height() {
        let lines = vec![
            "OO".to_string(),
            "OO".to_string(),
            "kk".to_string(),
            "..".to_string(),
        ];
        let text = build_face_text(&lines);
        assert_eq!(text.lines.len(), 2);
    }
}
