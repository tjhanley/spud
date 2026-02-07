use ratatui::{Frame, layout::Rect};

/// Trait for modules that render content in the hero (main) area.
///
/// Implement this alongside [`spud_core::module::Module`] to provide
/// hero-area rendering. The app wires renderers to modules at registration
/// time via [`std::any::Any`] downcasting — no rendering types leak into
/// spud-core.
pub trait HeroRenderer {
    /// Render the hero (main content) area of the screen.
    ///
    /// Called each frame when this module is active.
    fn render_hero(&self, f: &mut Frame, area: Rect);
}
