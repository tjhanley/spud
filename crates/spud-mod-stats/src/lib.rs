use std::any::Any;

use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use spud_core::{
    event::Event,
    module::{HudContribution, Module},
};
use spud_ui::renderer::HeroRenderer;

/// System-stats module (stub).
///
/// Will eventually display CPU, memory, and SPUD telemetry metrics. Currently
/// renders placeholder text in the hero area.
pub struct StatsModule;

impl Default for StatsModule {
    fn default() -> Self {
        Self
    }
}

impl StatsModule {
    /// Create a new `StatsModule`.
    pub fn new() -> Self {
        Self
    }
}

impl Module for StatsModule {
    fn id(&self) -> &'static str {
        "stats"
    }
    fn title(&self) -> &'static str {
        "Stats (stub)"
    }

    fn handle_event(&mut self, _ev: &Event) {}

    fn hud(&self) -> HudContribution {
        HudContribution {
            left_lines: vec!["Stats: stubbed".into()],
            right_lines: vec!["CPU: --%".into(), "RSS: --".into()],
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl HeroRenderer for StatsModule {
    fn render_hero(&self, f: &mut Frame, area: Rect) {
        let p = Paragraph::new(vec![
            Line::from("Stats module (stub)"),
            Line::from("Next: sysinfo + SPUD telemetry + gauges/tables"),
        ])
        .block(Block::default().borders(Borders::ALL).title("HERO"));

        f.render_widget(p, area);
    }
}
