/// Terminal graphics backend for face rendering.
///
/// Only [`UnicodeBlock`](GraphicsBackend::UnicodeBlock) is implemented.
/// Future work will add Kitty, iTerm2 and Sixel image protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsBackend {
    /// Kitty graphics protocol.
    Kitty,
    /// iTerm2 inline image protocol.
    ITerm2,
    /// Sixel graphics protocol.
    Sixel,
    /// Unicode half-block character fallback (works everywhere).
    UnicodeBlock,
}

/// Detect the best available graphics backend.
///
/// Currently always returns [`GraphicsBackend::UnicodeBlock`]. Future
/// versions will inspect `TERM`, `TERM_PROGRAM`, and capability queries.
pub fn detect_backend() -> GraphicsBackend {
    // Future: check TERM, TERM_PROGRAM, etc.
    GraphicsBackend::UnicodeBlock
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_backend_returns_unicode_block() {
        assert_eq!(detect_backend(), GraphicsBackend::UnicodeBlock);
    }
}
