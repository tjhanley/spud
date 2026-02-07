use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    widgets::{Block, Borders, Widget},
};

/// Minimum alpha value (0–255) for a pixel to be considered opaque.
///
/// Pixels below this threshold are skipped, leaving the cell empty.
const ALPHA_THRESHOLD: u8 = 128;

/// Render an RGBA image into a terminal rect using Unicode half-block characters.
///
/// Each terminal cell represents two vertically stacked pixels via the upper
/// half-block character (`▀`). The image is downsampled from `(src_width ×
/// src_height)` to fit `area` using nearest-neighbour scaling.
///
/// Draws a border titled "AGENT" and renders pixels into the inner area.
/// Fully transparent pixels (alpha < 128) are skipped.
pub fn render_face(buf: &mut Buffer, area: Rect, data: &[u8], src_width: u32, src_height: u32) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    // Draw the border block first.
    let block = Block::default().borders(Borders::ALL).title("AGENT");
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if src_width == 0 || src_height == 0 {
        return;
    }

    let expected_len = match (src_width as u64)
        .checked_mul(src_height as u64)
        .and_then(|v| v.checked_mul(4))
    {
        Some(v) => v,
        None => return,
    };

    if (data.len() as u64) < expected_len {
        return;
    }

    let cell_w = inner.width as u32;
    let cell_h = inner.height as u32;
    let pixel_h = cell_h * 2; // two vertical pixels per cell

    for cy in 0..cell_h {
        for cx in 0..cell_w {
            let top_py = (cy * 2 * src_height) / pixel_h;
            let bot_py = ((cy * 2 + 1) * src_height) / pixel_h;
            let px = (cx * src_width) / cell_w;

            let top = match sample_pixel(data, src_width, px, top_py) {
                Some(p) => p,
                None => continue,
            };
            let bot = match sample_pixel(data, src_width, px, bot_py) {
                Some(p) => p,
                None => continue,
            };

            let top_opaque = top.3 >= ALPHA_THRESHOLD;
            let bot_opaque = bot.3 >= ALPHA_THRESHOLD;

            let x = inner.x + cx as u16;
            let y = inner.y + cy as u16;

            if !top_opaque && !bot_opaque {
                continue;
            }

            if let Some(cell) = buf.cell_mut((x, y)) {
                if top_opaque && bot_opaque {
                    cell.set_char('▀');
                    cell.set_fg(Color::Rgb(top.0, top.1, top.2));
                    cell.set_bg(Color::Rgb(bot.0, bot.1, bot.2));
                } else if top_opaque {
                    cell.set_char('▀');
                    cell.set_fg(Color::Rgb(top.0, top.1, top.2));
                    cell.set_bg(Color::Reset);
                } else {
                    cell.set_char('▄');
                    cell.set_fg(Color::Rgb(bot.0, bot.1, bot.2));
                    cell.set_bg(Color::Reset);
                }
            }
        }
    }
}

/// Read an RGBA pixel from row-major data.
///
/// Returns `None` if the computed index overflows or falls outside `data`.
fn sample_pixel(data: &[u8], width: u32, x: u32, y: u32) -> Option<(u8, u8, u8, u8)> {
    let idx = (y as usize)
        .checked_mul(width as usize)?
        .checked_add(x as usize)?
        .checked_mul(4)?;
    let r = *data.get(idx)?;
    let g = *data.get(idx + 1)?;
    let b = *data.get(idx + 2)?;
    let a = *data.get(idx + 3)?;
    Some((r, g, b, a))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a small RGBA image filled with a single colour.
    fn solid_image(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> Vec<u8> {
        let mut data = vec![0u8; (w * h * 4) as usize];
        for pixel in data.chunks_exact_mut(4) {
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
            pixel[3] = a;
        }
        data
    }

    #[test]
    fn render_face_basic() {
        // 4×4 red image → render into a 6×4 area (inner = 4×2 cells = 4×4 pixels)
        let data = solid_image(4, 4, 255, 0, 0, 255);
        let area = Rect::new(0, 0, 6, 4);
        let mut buf = Buffer::empty(area);

        render_face(&mut buf, area, &data, 4, 4);

        // Inner area is (1,1)–(4,2), i.e. 4 cells wide, 2 cells tall.
        let cell = buf.cell((1, 1)).unwrap();
        assert_eq!(cell.symbol(), "▀");
        assert_eq!(cell.fg, Color::Rgb(255, 0, 0));
        assert_eq!(cell.bg, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn render_face_transparent_skips_cells() {
        let data = solid_image(4, 4, 0, 255, 0, 0); // fully transparent
        let area = Rect::new(0, 0, 6, 4);
        let mut buf = Buffer::empty(area);

        render_face(&mut buf, area, &data, 4, 4);

        // Inner cells should not have the half-block character.
        let cell = buf.cell((1, 1)).unwrap();
        assert_ne!(cell.symbol(), "▀");
        assert_ne!(cell.symbol(), "▄");
    }

    #[test]
    fn render_face_empty_area_no_panic() {
        let data = solid_image(4, 4, 255, 0, 0, 255);
        let area = Rect::new(0, 0, 0, 0);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        render_face(&mut buf, area, &data, 4, 4);
        // Just verify no panic.
    }

    #[test]
    fn render_face_short_data_no_panic() {
        let area = Rect::new(0, 0, 6, 4);
        let mut buf = Buffer::empty(area);
        // data too short for claimed dimensions
        render_face(&mut buf, area, &[0; 8], 4, 4);
    }

    #[test]
    fn render_face_half_transparent() {
        // Top two rows opaque red, bottom two rows transparent.
        let mut data = vec![0u8; 4 * 4 * 4];
        for y in 0..4u32 {
            for x in 0..4u32 {
                let idx = ((y * 4 + x) * 4) as usize;
                if y < 2 {
                    data[idx] = 255;
                    data[idx + 3] = 255;
                }
                // else stays 0,0,0,0 (transparent)
            }
        }

        let area = Rect::new(0, 0, 6, 4);
        let mut buf = Buffer::empty(area);
        render_face(&mut buf, area, &data, 4, 4);

        // First cell row: top pixel=red, bottom pixel=transparent → '▀' with fg red
        let cell = buf.cell((1, 1)).unwrap();
        assert_eq!(cell.symbol(), "▀");
        assert_eq!(cell.fg, Color::Rgb(255, 0, 0));
    }
}
