use std::any::Any;

use crate::event::Event;

/// Lines contributed by a module to the Doom-style HUD panel.
#[derive(Default)]
pub struct HudContribution {
    /// Lines rendered in the left HUD column (e.g. keybindings, status).
    pub left_lines: Vec<String>,
    /// Lines rendered in the right HUD column (e.g. metrics, indicators).
    pub right_lines: Vec<String>,
}

/// A pluggable SPUD module.
///
/// Modules are the primary extension point for SPUD. Each module provides a
/// unique [`id`](Module::id), a human-readable [`title`](Module::title), and
/// optional implementations for event handling and HUD contributions.
///
/// Rendering is handled separately via `spud_ui::renderer::HeroRenderer`.
/// Modules that render hero content should implement that trait in addition
/// to `Module`.
///
/// Modules are registered with [`crate::registry::ModuleRegistry`] and receive
/// events via [`handle_event`](Module::handle_event).
pub trait Module {
    /// Unique identifier for this module (e.g. `"hello"`, `"stats"`).
    fn id(&self) -> &'static str;

    /// Human-readable display name shown in the top bar and module list.
    fn title(&self) -> &'static str;

    /// Handle an incoming event. Called by the registry during broadcast.
    ///
    /// The default implementation is a no-op.
    fn handle_event(&mut self, _ev: &Event) {}

    /// Return lines to display in the HUD panel while this module is active.
    ///
    /// The default implementation returns empty contributions.
    fn hud(&self) -> HudContribution {
        HudContribution::default()
    }

    /// Return `self` as `&dyn Any` to enable downcasting for type-aware
    /// rendering in spud-ui.
    fn as_any(&self) -> &dyn Any;
}
