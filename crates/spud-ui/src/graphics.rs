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
/// Checks `SPUD_GRAPHICS` env var for explicit override (`sixel` or `unicode`),
/// then falls back to detecting Sixel support via TERM or TERM_PROGRAM.
/// Returns [`GraphicsBackend::UnicodeBlock`] as the safe default.
pub fn detect_backend() -> GraphicsBackend {
    // Env override: SPUD_GRAPHICS=sixel|unicode
    if let Ok(val) = std::env::var("SPUD_GRAPHICS") {
        match val.to_lowercase().as_str() {
            "sixel" => return GraphicsBackend::Sixel,
            "unicode" => return GraphicsBackend::UnicodeBlock,
            _ => {} // Invalid value, fall through to auto-detection
        }
    }

    // Check for Sixel support via TERM_PROGRAM
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "WezTerm" => return GraphicsBackend::Sixel,
            "mlterm" => return GraphicsBackend::Sixel,
            _ => {}
        }
    }

    // Check for Sixel support via TERM variable
    if let Ok(term) = std::env::var("TERM") {
        // XTerm and terminals that advertise Sixel support
        if term.contains("xterm") || term.contains("mlterm") || term == "foot" {
            return GraphicsBackend::Sixel;
        }
    }

    // Safe default: Unicode blocks work everywhere
    GraphicsBackend::UnicodeBlock
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize access to process-global env vars to prevent test races
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn detect_backend_default_returns_unicode_block() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Save original values
        let original_graphics = std::env::var("SPUD_GRAPHICS").ok();
        let original_term_program = std::env::var("TERM_PROGRAM").ok();
        let original_term = std::env::var("TERM").ok();

        // Clear all env vars to test default
        std::env::remove_var("SPUD_GRAPHICS");
        std::env::remove_var("TERM_PROGRAM");
        std::env::remove_var("TERM");

        assert_eq!(detect_backend(), GraphicsBackend::UnicodeBlock);

        // Restore original values
        if let Some(val) = original_graphics {
            std::env::set_var("SPUD_GRAPHICS", val);
        }
        if let Some(val) = original_term_program {
            std::env::set_var("TERM_PROGRAM", val);
        }
        if let Some(val) = original_term {
            std::env::set_var("TERM", val);
        }
    }

    #[test]
    fn detect_backend_respects_spud_graphics_sixel_override() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let original = std::env::var("SPUD_GRAPHICS").ok();

        std::env::set_var("SPUD_GRAPHICS", "sixel");
        assert_eq!(detect_backend(), GraphicsBackend::Sixel);

        // Restore original value
        match original {
            Some(val) => std::env::set_var("SPUD_GRAPHICS", val),
            None => std::env::remove_var("SPUD_GRAPHICS"),
        }
    }

    #[test]
    fn detect_backend_respects_spud_graphics_unicode_override() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let original = std::env::var("SPUD_GRAPHICS").ok();

        std::env::set_var("SPUD_GRAPHICS", "unicode");
        assert_eq!(detect_backend(), GraphicsBackend::UnicodeBlock);

        // Restore original value
        match original {
            Some(val) => std::env::set_var("SPUD_GRAPHICS", val),
            None => std::env::remove_var("SPUD_GRAPHICS"),
        }
    }

    #[test]
    fn detect_backend_detects_wezterm() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let original_graphics = std::env::var("SPUD_GRAPHICS").ok();
        let original_term_program = std::env::var("TERM_PROGRAM").ok();
        let original_term = std::env::var("TERM").ok();

        std::env::remove_var("SPUD_GRAPHICS");
        std::env::remove_var("TERM");
        std::env::set_var("TERM_PROGRAM", "WezTerm");
        assert_eq!(detect_backend(), GraphicsBackend::Sixel);

        // Restore original values
        match original_graphics {
            Some(val) => std::env::set_var("SPUD_GRAPHICS", val),
            None => std::env::remove_var("SPUD_GRAPHICS"),
        }
        match original_term_program {
            Some(val) => std::env::set_var("TERM_PROGRAM", val),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
        match original_term {
            Some(val) => std::env::set_var("TERM", val),
            None => std::env::remove_var("TERM"),
        }
    }

    #[test]
    fn detect_backend_detects_xterm() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let original_graphics = std::env::var("SPUD_GRAPHICS").ok();
        let original_term = std::env::var("TERM").ok();
        let original_term_program = std::env::var("TERM_PROGRAM").ok();

        std::env::remove_var("SPUD_GRAPHICS");
        std::env::remove_var("TERM_PROGRAM");
        std::env::set_var("TERM", "xterm-256color");
        assert_eq!(detect_backend(), GraphicsBackend::Sixel);

        // Restore original values
        match original_graphics {
            Some(val) => std::env::set_var("SPUD_GRAPHICS", val),
            None => std::env::remove_var("SPUD_GRAPHICS"),
        }
        match original_term {
            Some(val) => std::env::set_var("TERM", val),
            None => std::env::remove_var("TERM"),
        }
        match original_term_program {
            Some(val) => std::env::set_var("TERM_PROGRAM", val),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
    }
}
