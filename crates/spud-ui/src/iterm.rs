//! iTerm2 inline image protocol renderer.
//!
//! Renders full-resolution RGBA images using iTerm2's OSC 1337 escape sequence.
//! This works in iTerm2 and WezTerm terminals, providing much higher fidelity
//! than Unicode half-block rendering.
//!
//! The escape sequence format is:
//! ```text
//! \x1b]1337;File=inline=1;width={cells};height={cells};preserveAspectRatio=0:{base64}\x07
//! ```
//!
//! Since ratatui's cell-based buffer can't represent inline images, the caller
//! must write this escape sequence directly to the terminal backend after the
//! `terminal.draw()` call completes.

use std::io::{self, Write};

use image::ImageFormat;
use ratatui::layout::Rect;

/// Cache for encoded image data to avoid re-encoding identical frames.
///
/// The face animator updates frames every 300ms, but the render loop runs at
/// ~60fps. We cache the encoded PNG+base64 to avoid redundant work.
struct ImageCache {
    /// Pointer to the source RGBA data (used as cache key).
    last_data_ptr: usize,
    /// Cached image width (part of cache key).
    last_width: u32,
    /// Cached image height (part of cache key).
    last_height: u32,
    /// Pre-encoded base64 PNG string.
    last_encoded: String,
}

impl ImageCache {
    fn new() -> Self {
        Self {
            last_data_ptr: 0,
            last_width: 0,
            last_height: 0,
            last_encoded: String::new(),
        }
    }

    /// Get cached encoding or compute a new one if the data changed.
    fn get_or_encode(&mut self, data: &[u8], width: u32, height: u32) -> io::Result<String> {
        let data_ptr = data.as_ptr() as usize;

        // Cache hit: same data pointer AND dimensions means same frame
        if data_ptr == self.last_data_ptr
            && width == self.last_width
            && height == self.last_height
            && !self.last_encoded.is_empty()
        {
            return Ok(self.last_encoded.clone());
        }

        // Cache miss: encode RGBA → PNG → base64
        let rgba_image = image::RgbaImage::from_raw(width, height, data.to_vec())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid RGBA dimensions"))?;

        let mut png_bytes = Vec::new();
        rgba_image
            .write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
            .map_err(io::Error::other)?;

        self.last_encoded = base64_encode(&png_bytes);
        self.last_data_ptr = data_ptr;
        self.last_width = width;
        self.last_height = height;

        Ok(self.last_encoded.clone())
    }
}

/// Render an RGBA image at a specific terminal position using iTerm2 inline images.
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
pub fn render_iterm_face(
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

    // Thread-local cache to persist across calls
    thread_local! {
        static CACHE: std::cell::RefCell<ImageCache> = std::cell::RefCell::new(ImageCache::new());
    }

    let encoded = CACHE.with(|cache| {
        cache
            .borrow_mut()
            .get_or_encode(data, src_width, src_height)
    })?;

    // Position cursor at top-left of area (1-indexed for ANSI escape codes)
    write!(writer, "\x1b[{};{}H", area.y + 1, area.x + 1)?;

    // Write iTerm2 inline image escape sequence
    write!(
        writer,
        "\x1b]1337;File=inline=1;width={};height={};preserveAspectRatio=0:{}\x07",
        area.width, area.height, encoded
    )?;

    Ok(())
}

/// Base64-encode bytes using the standard alphabet.
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        buf[..chunk.len()].copy_from_slice(chunk);

        result.push(ALPHABET[(buf[0] >> 2) as usize] as char);
        result.push(ALPHABET[(((buf[0] & 0x03) << 4) | (buf[1] >> 4)) as usize] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[(((buf[1] & 0x0f) << 2) | (buf[2] >> 6)) as usize] as char);
            if chunk.len() > 2 {
                result.push(ALPHABET[(buf[2] & 0x3f) as usize] as char);
            } else {
                result.push('=');
            }
        } else {
            result.push('=');
            result.push('=');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encode_empty() {
        assert_eq!(base64_encode(&[]), "");
    }

    #[test]
    fn base64_encode_short() {
        // "Man" in ASCII
        assert_eq!(base64_encode(b"Man"), "TWFu");
    }

    #[test]
    fn base64_encode_with_padding() {
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(base64_encode(b"M"), "TQ==");
    }

    #[test]
    fn render_iterm_face_zero_area_is_noop() {
        let mut output = Vec::new();
        let area = Rect::new(0, 0, 0, 0);
        let data = vec![0u8; 16]; // 2×2 RGBA

        let result = render_iterm_face(&mut output, area, &data, 2, 2);

        assert!(result.is_ok());
        assert!(output.is_empty());
    }

    #[test]
    fn render_iterm_face_writes_escape_sequence() {
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

        let result = render_iterm_face(&mut output, area, &data, 4, 4);
        assert!(result.is_ok());

        let output_str = String::from_utf8_lossy(&output);

        // Check for expected components
        assert!(output_str.contains("\x1b[11;6H")); // cursor position (1-indexed)
        assert!(output_str.contains("\x1b]1337")); // OSC 1337 start
        assert!(output_str.contains("File=inline=1"));
        assert!(output_str.contains("width=8"));
        assert!(output_str.contains("height=8"));
        assert!(output_str.contains("preserveAspectRatio=0"));
        assert!(output_str.contains("\x07")); // BEL terminator

        // Should contain base64-encoded PNG data (non-empty)
        let parts: Vec<&str> = output_str.split(':').collect();
        assert!(parts.len() >= 2);
        let base64_part = parts.last().unwrap();
        assert!(base64_part.len() > 10); // Should have substantial data
    }

    #[test]
    fn render_iterm_face_caches_identical_data() {
        let mut output1 = Vec::new();
        let mut output2 = Vec::new();
        let area = Rect::new(0, 0, 4, 4);

        // 2×2 solid blue image
        let data = vec![
            0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255,
        ];

        // First render
        render_iterm_face(&mut output1, area, &data, 2, 2).unwrap();

        // Second render with same data
        render_iterm_face(&mut output2, area, &data, 2, 2).unwrap();

        // Both should produce identical output (cache working)
        assert_eq!(output1, output2);
    }
}
