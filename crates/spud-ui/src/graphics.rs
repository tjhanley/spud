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
/// Checks `SPUD_GRAPHICS` env var for explicit override (`iterm2` or `unicode`),
/// then falls back to `TERM_PROGRAM` detection for iTerm.app and WezTerm.
/// Returns [`GraphicsBackend::UnicodeBlock`] as the safe default.
pub fn detect_backend() -> GraphicsBackend {
    // Env override: SPUD_GRAPHICS=iterm2|unicode
    if let Ok(val) = std::env::var("SPUD_GRAPHICS") {
        match val.to_lowercase().as_str() {
            "iterm2" => return GraphicsBackend::ITerm2,
            "unicode" => return GraphicsBackend::UnicodeBlock,
            _ => {} // Invalid value, fall through to auto-detection
        }
    }

    // Auto-detect from TERM_PROGRAM
    match std::env::var("TERM_PROGRAM").as_deref() {
        Ok("iTerm.app") => GraphicsBackend::ITerm2,
        Ok("WezTerm") => GraphicsBackend::ITerm2, // WezTerm supports iTerm2 protocol
        _ => GraphicsBackend::UnicodeBlock,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize access to process-global env vars to prevent test races
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn detect_backend_default_returns_unicode_block() {
        let _guard = ENV_LOCK.lock().unwrap();

        // Save original values
        let original_graphics = std::env::var("SPUD_GRAPHICS").ok();
        let original_term_program = std::env::var("TERM_PROGRAM").ok();

        // Clear both env vars to test default
        std::env::remove_var("SPUD_GRAPHICS");
        std::env::remove_var("TERM_PROGRAM");

        assert_eq!(detect_backend(), GraphicsBackend::UnicodeBlock);

        // Restore original values
        if let Some(val) = original_graphics {
            std::env::set_var("SPUD_GRAPHICS", val);
        }
        if let Some(val) = original_term_program {
            std::env::set_var("TERM_PROGRAM", val);
        }
    }

    #[test]
    fn detect_backend_respects_spud_graphics_iterm2_override() {
        let _guard = ENV_LOCK.lock().unwrap();

        let original = std::env::var("SPUD_GRAPHICS").ok();

        std::env::set_var("SPUD_GRAPHICS", "iterm2");
        assert_eq!(detect_backend(), GraphicsBackend::ITerm2);

        // Restore original value
        match original {
            Some(val) => std::env::set_var("SPUD_GRAPHICS", val),
            None => std::env::remove_var("SPUD_GRAPHICS"),
        }
    }

    #[test]
    fn detect_backend_respects_spud_graphics_unicode_override() {
        let _guard = ENV_LOCK.lock().unwrap();

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
    fn detect_backend_detects_iterm_app() {
        let _guard = ENV_LOCK.lock().unwrap();

        let original_graphics = std::env::var("SPUD_GRAPHICS").ok();
        let original_term_program = std::env::var("TERM_PROGRAM").ok();

        std::env::remove_var("SPUD_GRAPHICS"); // Ensure override doesn't interfere
        std::env::set_var("TERM_PROGRAM", "iTerm.app");
        assert_eq!(detect_backend(), GraphicsBackend::ITerm2);

        // Restore original values
        match original_graphics {
            Some(val) => std::env::set_var("SPUD_GRAPHICS", val),
            None => std::env::remove_var("SPUD_GRAPHICS"),
        }
        match original_term_program {
            Some(val) => std::env::set_var("TERM_PROGRAM", val),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
    }

    #[test]
    fn detect_backend_detects_wezterm() {
        let _guard = ENV_LOCK.lock().unwrap();

        let original_graphics = std::env::var("SPUD_GRAPHICS").ok();
        let original_term_program = std::env::var("TERM_PROGRAM").ok();

        std::env::remove_var("SPUD_GRAPHICS");
        std::env::set_var("TERM_PROGRAM", "WezTerm");
        assert_eq!(detect_backend(), GraphicsBackend::ITerm2);

        // Restore original values
        match original_graphics {
            Some(val) => std::env::set_var("SPUD_GRAPHICS", val),
            None => std::env::remove_var("SPUD_GRAPHICS"),
        }
        match original_term_program {
            Some(val) => std::env::set_var("TERM_PROGRAM", val),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
    }
}
