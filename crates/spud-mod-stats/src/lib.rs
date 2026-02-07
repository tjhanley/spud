mod format;
mod render;
mod telemetry;

use std::any::Any;

use ratatui::{layout::Rect, Frame};

use spud_core::{
    event::Event,
    module::{HudContribution, Module},
};
use spud_ui::renderer::HeroRenderer;

pub use telemetry::MetricsSnapshot;

use crate::format::{format_bytes, format_percent};
use crate::telemetry::TelemetryCollector;

/// System-stats module providing live CPU, memory, swap, and process telemetry.
///
/// Refreshes metrics at a 1-second interval via [`TelemetryCollector`] and renders
/// them as gauges in the hero pane and live numbers in the HUD panels.
pub struct StatsModule {
    collector: TelemetryCollector,
}

impl Default for StatsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsModule {
    /// Create a new `StatsModule` with default refresh interval.
    pub fn new() -> Self {
        Self {
            collector: TelemetryCollector::new(),
        }
    }
}

impl Module for StatsModule {
    fn id(&self) -> &'static str {
        "stats"
    }

    fn title(&self) -> &'static str {
        "System Stats"
    }

    fn handle_event(&mut self, ev: &Event) {
        if let Event::Tick { now } = ev {
            self.collector.maybe_refresh(*now);
        }
    }

    fn hud(&self) -> HudContribution {
        let snap = self.collector.snapshot();

        let cpu_text = format!("CPU: {}", format_percent(snap.cpu_global));
        let mem_text = if snap.mem_total > 0 {
            let pct = snap.mem_used as f32 / snap.mem_total as f32 * 100.0;
            format!(
                "MEM: {} ({})",
                format_percent(pct),
                format_bytes(snap.mem_used)
            )
        } else {
            "MEM: --".into()
        };
        let rss_text = match snap.self_rss {
            Some(rss) => format!("RSS: {}", format_bytes(rss)),
            None => "RSS: --".into(),
        };

        HudContribution {
            left_lines: vec!["Tab: next module".into(), "`: console".into()],
            right_lines: vec![cpu_text, mem_text, rss_text],
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl HeroRenderer for StatsModule {
    fn render_hero(&self, f: &mut Frame, area: Rect) {
        render::render_hero_content(f, area, self.collector.snapshot());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn id_and_title() {
        let m = StatsModule::new();
        assert_eq!(m.id(), "stats");
        assert_eq!(m.title(), "System Stats");
    }

    #[test]
    fn tick_triggers_refresh() {
        let mut m = StatsModule::new();
        m.handle_event(&Event::Tick {
            now: Instant::now(),
        });
        assert!(m.collector.snapshot().mem_total > 0);
    }

    #[test]
    fn hud_contains_expected_labels() {
        let mut m = StatsModule::new();
        m.handle_event(&Event::Tick {
            now: Instant::now(),
        });
        let hud = m.hud();
        assert!(hud.right_lines.iter().any(|l| l.contains("CPU:")));
        assert!(hud.right_lines.iter().any(|l| l.contains("MEM:")));
        assert!(hud.right_lines.iter().any(|l| l.contains("RSS:")));
    }
}
