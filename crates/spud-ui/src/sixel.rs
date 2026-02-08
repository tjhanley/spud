//! Sixel graphics protocol renderer.
//!
//! Renders full-resolution RGBA images using the Sixel graphics protocol,
//! which is more efficient than iTerm2 inline images for animation.
//! Sixel is supported by XTerm, WezTerm, mlterm, foot, and many other terminals.
//!
//! The Sixel protocol encodes pixels directly without PNG compression,
//! making it much faster for real-time rendering.

use std::io::{self, Write};

use ratatui::layout::Rect;

/// Render an RGBA image at a specific terminal position using Sixel graphics.
///
/// Writes directly to the provided writer (typically the crossterm backend).
/// The caller is responsible for cursor positioning before and after.
///
/// # Arguments
///
/// * `writer` - Output stream (usually `terminal.backend_mut()`)
/// * `area` - Screen coordinates in terminal cells (inner area, after border)
/// * `data` - Raw RGBA pixel data (4 bytes per pixel: R, G, B, A)
/// * `src_width` - Image width in pixels
/// * `src_height` - Image height in pixels
///
/// # Returns
///
/// `Ok(())` on success, or an I/O error if writing fails.
pub fn render_sixel_face(
    writer: &mut impl Write,
    area: Rect,
    data: &[u8],
    src_width: u32,
    src_height: u32,
) -> io::Result<()> {
    // No-op for zero-area
    if area.width == 0 || area.height == 0 {
        return Ok(());
    }

    // Position cursor at top-left of area (1-indexed for ANSI escape codes)
    write!(writer, "\x1b[{};{}H", area.y + 1, area.x + 1)?;

    // Start Sixel sequence: DCS (Device Control String) + 'q' for Sixel mode
    write!(writer, "\x1bPq")?;

    // Set aspect ratio (1:1 pixels)
    write!(writer, "\"1;1")?;

    // Encode the image data as Sixel
    encode_sixel_rgba(writer, data, src_width, src_height)?;

    // End Sixel sequence: ST (String Terminator)
    write!(writer, "\x1b\\")?;

    Ok(())
}

/// Encode RGBA image data as Sixel format.
///
/// This is a simplified encoder that uses direct color mode for better performance.
/// Each sixel character represents 6 vertical pixels.
fn encode_sixel_rgba(
    writer: &mut impl Write,
    data: &[u8],
    width: u32,
    height: u32,
) -> io::Result<()> {
    // Build a simple color palette (256 colors is enough for faces)
    let mut palette: Vec<(u8, u8, u8)> = Vec::new();
    let mut color_map: std::collections::HashMap<(u8, u8, u8), usize> =
        std::collections::HashMap::new();

    // Extract unique colors from the image (limit to 256 for Sixel)
    for chunk in data.chunks_exact(4) {
        let (r, g, b, a) = (chunk[0], chunk[1], chunk[2], chunk[3]);

        // Skip fully transparent pixels
        if a < 128 {
            continue;
        }

        let rgb = (r, g, b);
        if !color_map.contains_key(&rgb) && palette.len() < 256 {
            color_map.insert(rgb, palette.len());
            palette.push(rgb);
        }
    }

    // Define color palette
    for (idx, (r, g, b)) in palette.iter().enumerate() {
        // Sixel color format: #<idx>;2;<r>;<g>;<b>
        // RGB values are 0-100 in Sixel
        let r_pct = (*r as u16 * 100) / 255;
        let g_pct = (*g as u16 * 100) / 255;
        let b_pct = (*b as u16 * 100) / 255;
        write!(writer, "#{};2;{};{};{}", idx, r_pct, g_pct, b_pct)?;
    }

    // Encode pixel data row by row (6 pixels per sixel)
    for y in (0..height).step_by(6) {
        // For each color in the palette
        for (color_idx, &(r, g, b)) in palette.iter().enumerate() {
            write!(writer, "#{}", color_idx)?;

            // Encode this color's pixels for the next 6 rows
            for x in 0..width {
                let mut sixel: u8 = 0;

                // Check 6 vertical pixels
                for dy in 0..6 {
                    let py = y + dy;
                    if py >= height {
                        break;
                    }

                    let pixel_idx = ((py * width + x) * 4) as usize;
                    if pixel_idx + 3 < data.len() {
                        let (pr, pg, pb, pa) = (
                            data[pixel_idx],
                            data[pixel_idx + 1],
                            data[pixel_idx + 2],
                            data[pixel_idx + 3],
                        );

                        // If this pixel matches the current color and is opaque
                        if pa >= 128 && pr == r && pg == g && pb == b {
                            sixel |= 1 << dy;
                        }
                    }
                }

                // Write sixel character (offset by 63 to make it printable)
                if sixel != 0 {
                    write!(writer, "{}", (sixel + 63) as char)?;
                } else {
                    // Skip empty sixels for compression
                    write!(writer, "?")?; // Sixel "skip" character
                }
            }

            // Move to next line for next color
            write!(writer, "$")?; // Carriage return
        }

        // Move to next sixel row
        write!(writer, "-")?; // Line feed
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_sixel_face_zero_area_is_noop() {
        let mut output = Vec::new();
        let area = Rect::new(0, 0, 0, 0);
        let data = vec![0u8; 16]; // 2×2 RGBA

        let result = render_sixel_face(&mut output, area, &data, 2, 2);

        assert!(result.is_ok());
        assert!(output.is_empty());
    }

    #[test]
    fn render_sixel_face_writes_escape_sequence() {
        let mut output = Vec::new();
        let area = Rect::new(5, 10, 8, 8);

        // 4×4 solid red image (RGBA)
        let mut data = vec![0u8; 4 * 4 * 4];
        for pixel in data.chunks_exact_mut(4) {
            pixel[0] = 255; // R
            pixel[1] = 0; // G
            pixel[2] = 0; // B
            pixel[3] = 255; // A
        }

        let result = render_sixel_face(&mut output, area, &data, 4, 4);
        assert!(result.is_ok());

        let output_str = String::from_utf8_lossy(&output);

        // Check for expected Sixel components
        assert!(output_str.contains("\x1b[11;6H")); // cursor position
        assert!(output_str.contains("\x1bPq")); // DCS + Sixel mode
        assert!(output_str.contains("\x1b\\")); // ST terminator
        assert!(output_str.contains("#0")); // Color definition
    }
}
