use std::any::Any;

use ratatui::{
    layout::{Alignment, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use spud_core::{
    event::Event,
    module::{HudContribution, Module},
};
use spud_ui::renderer::HeroRenderer;

/// A minimal welcome-screen module.
///
/// Displays the SPUD title and a "Hello World" message in the hero area.
/// Serves as the default landing module and a reference implementation for
/// the [`Module`] trait.
pub struct HelloModule;

impl Default for HelloModule {
    fn default() -> Self {
        Self
    }
}

impl HelloModule {
    /// Create a new `HelloModule`.
    pub fn new() -> Self {
        Self
    }
}

impl Module for HelloModule {
    fn id(&self) -> &'static str {
        "hello"
    }
    fn title(&self) -> &'static str {
        "Hello"
    }

    fn handle_event(&mut self, _ev: &Event) {}

    fn hud(&self) -> HudContribution {
        HudContribution {
            left_lines: vec!["Tab: next module".into(), "q: quit".into()],
            right_lines: vec!["HMR: (planned)".into(), "IMG: (planned)".into()],
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl HeroRenderer for HelloModule {
    fn render_hero(&self, f: &mut Frame, area: Rect) {
        let p = Paragraph::new(vec![
            Line::from("SPUD"),
            Line::from("Suspiciously Powerful Utility of De-evolution"),
            Line::from(""),
            Line::from("Hello World"),
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("HERO"));

        f.render_widget(p, area);
    }
}
