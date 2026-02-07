use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    text::Line,
};

use spud_core::{event::Event, module::{HudContribution, Module}};

pub struct StatsModule;

impl Default for StatsModule {
    fn default() -> Self { Self }
}

impl StatsModule {
    pub fn new() -> Self { Self }
}

impl Module for StatsModule {
    fn id(&self) -> &'static str { "stats" }
    fn title(&self) -> &'static str { "Stats (stub)" }

    fn handle_event(&mut self, _ev: &Event) {}

    fn hud(&self) -> HudContribution {
        HudContribution {
            left_lines: vec!["Stats: stubbed".into()],
            right_lines: vec!["CPU: --%".into(), "RSS: --".into()],
        }
    }

    fn render_hero(&self, f: &mut Frame, area: Rect) {
        let p = Paragraph::new(vec![
            Line::from("Stats module (stub)"),
            Line::from("Next: sysinfo + SPUD telemetry + gauges/tables"),
        ])
        .block(Block::default().borders(Borders::ALL).title("HERO"));

        f.render_widget(p, area);
    }
}
