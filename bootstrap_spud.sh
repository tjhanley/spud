#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "${ROOT}" ]]; then
  echo "ERROR: not inside a git repository."
  exit 1
fi
cd "$ROOT"

echo "==> Bootstrapping SPUD workspace in: $ROOT"

mkdir -p crates/{spud-app,spud-core,spud-ui,spud-agent,spud-config,spud-remote,spud-mod-hello,spud-mod-stats}
mkdir -p \
  crates/spud-app/src \
  crates/spud-core/src \
  crates/spud-ui/src \
  crates/spud-agent/src \
  crates/spud-config/src \
  crates/spud-remote/src \
  crates/spud-mod-hello/src \
  crates/spud-mod-stats/src

# -------------------------
# Root workspace
# -------------------------
cat > Cargo.toml <<'EOF'
[workspace]
resolver = "2"
members = [
  "crates/spud-app",
  "crates/spud-core",
  "crates/spud-ui",
  "crates/spud-agent",
  "crates/spud-config",
  "crates/spud-remote",
  "crates/spud-mod-hello",
  "crates/spud-mod-stats",
]
EOF

cat > README.md <<'EOF'
# SPUD
## Suspiciously Powerful Utility of De-evolution

A modular Rust TUI dashboard with a Doom-inspired HUD shell and an agent personality system.
Planned: native Rust modules + TypeScript plugin modules with hot reload.

### Run
```bash
cargo run -p spud-app
```

### License
Apache-2.0 OR MIT (planned)
EOF

# -------------------------
# crates/spud-core
# -------------------------
cat > crates/spud-core/Cargo.toml <<'EOF'
[package]
name = "spud-core"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
crossterm = "0.27"
EOF

cat > crates/spud-core/src/lib.rs <<'EOF'
pub mod event;
pub mod module;
pub mod state;
EOF

cat > crates/spud-core/src/event.rs <<'EOF'
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum Event {
    Tick { now: Instant },
    Key(crossterm::event::KeyEvent),
    Resize { cols: u16, rows: u16 },
    Quit,
}
EOF

cat > crates/spud-core/src/module.rs <<'EOF'
use crate::event::Event;

#[derive(Default)]
pub struct HudContribution {
    pub left_lines: Vec<String>,
    pub right_lines: Vec<String>,
}

pub trait Module {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;

    fn handle_event(&mut self, _ev: &Event) {}

    fn hud(&self) -> HudContribution {
        HudContribution::default()
    }
}
EOF

cat > crates/spud-core/src/state.rs <<'EOF'
use std::time::{Duration, Instant};

pub struct AppState {
    pub started_at: Instant,
    pub active_module_idx: usize,
    pub status_line: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            active_module_idx: 0,
            status_line: "DE-EVOLUTION IN PROGRESS.".to_string(),
        }
    }

    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}
EOF

# -------------------------
# crates/spud-ui
# -------------------------
cat > crates/spud-ui/Cargo.toml <<'EOF'
[package]
name = "spud-ui"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.28"
EOF

cat > crates/spud-ui/src/lib.rs <<'EOF'
pub mod layout;
pub mod shell;
EOF

cat > crates/spud-ui/src/layout.rs <<'EOF'
use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy)]
pub struct DoomRects {
    pub top: Rect,
    pub hero: Rect,
    pub hud: Rect,
    pub hud_left: Rect,
    pub hud_face: Rect,
    pub hud_right: Rect,
}

pub fn doom_layout(area: Rect, hud_height: u16, face_width: u16) -> DoomRects {
    let hud_height = hud_height.max(5).min(area.height.saturating_sub(2).max(5));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),          // top bar
            Constraint::Min(1),             // hero
            Constraint::Length(hud_height), // hud
        ])
        .split(area);

    let hud = chunks[2];
    let face_width = face_width.min(hud.width.saturating_sub(2).max(10));

    let hud_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(face_width),
            Constraint::Percentage(45),
        ])
        .split(hud);

    DoomRects {
        top: chunks[0],
        hero: chunks[1],
        hud,
        hud_left: hud_cols[0],
        hud_face: hud_cols[1],
        hud_right: hud_cols[2],
    }
}
EOF

cat > crates/spud-ui/src/shell.rs <<'EOF'
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    style::Style,
    text::{Line, Text},
};

use crate::layout::DoomRects;

pub struct ShellView<'a> {
    pub module_title: &'a str,
    pub status_line: &'a str,
    pub hud_left: Vec<String>,
    pub hud_right: Vec<String>,
}

pub fn render_shell(
    f: &mut Frame,
    rects: DoomRects,
    view: ShellView<'_>,
    hero: impl FnOnce(&mut Frame, Rect),
) {
    let top = Paragraph::new(Line::from(format!(
        "SPUD | {} | {}",
        view.module_title, view.status_line
    )))
    .style(Style::default())
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(top, rects.top);

    hero(f, rects.hero);

    f.render_widget(Block::default().borders(Borders::ALL).title("HUD"), rects.hud);

    let left_text = Text::from(view.hud_left.into_iter().map(Line::from).collect::<Vec<_>>());
    let left = Paragraph::new(left_text).block(Block::default().borders(Borders::ALL).title("LEFT"));
    f.render_widget(left, rects.hud_left);

    let face = Paragraph::new(Line::from("[ FACE ]"))
        .block(Block::default().borders(Borders::ALL).title("AGENT"));
    f.render_widget(face, rects.hud_face);

    let right_text = Text::from(view.hud_right.into_iter().map(Line::from).collect::<Vec<_>>());
    let right = Paragraph::new(right_text).block(Block::default().borders(Borders::ALL).title("RIGHT"));
    f.render_widget(right, rects.hud_right);
}
EOF

# -------------------------
# crates/spud-mod-hello
# -------------------------
cat > crates/spud-mod-hello/Cargo.toml <<'EOF'
[package]
name = "spud-mod-hello"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.28"
spud-core = { path = "../spud-core" }
EOF

cat > crates/spud-mod-hello/src/lib.rs <<'EOF'
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph},
    text::Line,
};

use spud_core::{event::Event, module::{HudContribution, Module}};

pub struct HelloModule;

impl HelloModule {
    pub fn new() -> Self { Self }
}

impl Module for HelloModule {
    fn id(&self) -> &'static str { "hello" }
    fn title(&self) -> &'static str { "Hello" }

    fn handle_event(&mut self, _ev: &Event) {}

    fn hud(&self) -> HudContribution {
        HudContribution {
            left_lines: vec![
                "Tab: next module".into(),
                "q: quit".into(),
            ],
            right_lines: vec![
                "HMR: (planned)".into(),
                "IMG: (planned)".into(),
            ],
        }
    }
}

pub fn render_hero(f: &mut Frame, area: Rect) {
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
EOF

# -------------------------
# crates/spud-mod-stats (stub)
# -------------------------
cat > crates/spud-mod-stats/Cargo.toml <<'EOF'
[package]
name = "spud-mod-stats"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.28"
spud-core = { path = "../spud-core" }
EOF

cat > crates/spud-mod-stats/src/lib.rs <<'EOF'
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    text::Line,
};

use spud_core::{event::Event, module::{HudContribution, Module}};

pub struct StatsModule;

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
}

pub fn render_hero(f: &mut Frame, area: Rect) {
    let p = Paragraph::new(vec![
        Line::from("Stats module (stub)"),
        Line::from("Next: sysinfo + SPUD telemetry + gauges/tables"),
    ])
    .block(Block::default().borders(Borders::ALL).title("HERO"));

    f.render_widget(p, area);
}
EOF

# -------------------------
# crates/spud-agent (stub)
# -------------------------
cat > crates/spud-agent/Cargo.toml <<'EOF'
[package]
name = "spud-agent"
version = "0.1.0"
edition = "2021"

[dependencies]
EOF

cat > crates/spud-agent/src/lib.rs <<'EOF'
// Placeholder crate. Next milestone: face-pack loader + mood state machine.
EOF

# -------------------------
# crates/spud-config (stub)
# -------------------------
cat > crates/spud-config/Cargo.toml <<'EOF'
[package]
name = "spud-config"
version = "0.1.0"
edition = "2021"

[dependencies]
EOF

cat > crates/spud-config/src/lib.rs <<'EOF'
// Placeholder crate. Next milestone: XDG config + theme selection.
EOF

# -------------------------
# crates/spud-remote (stub)
# -------------------------
cat > crates/spud-remote/Cargo.toml <<'EOF'
[package]
name = "spud-remote"
version = "0.1.0"
edition = "2021"

[dependencies]
EOF

cat > crates/spud-remote/src/lib.rs <<'EOF'
// Placeholder crate. Next milestone: TS module JSON-RPC + hot reload.
EOF

# -------------------------
# crates/spud-app (binary)
# -------------------------
cat > crates/spud-app/Cargo.toml <<'EOF'
[package]
name = "spud-app"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
crossterm = "0.27"
ratatui = "0.28"

spud-core = { path = "../spud-core" }
spud-ui = { path = "../spud-ui" }
spud-mod-hello = { path = "../spud-mod-hello" }
spud-mod-stats = { path = "../spud-mod-stats" }
EOF

cat > crates/spud-app/src/main.rs <<'EOF'
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{self, Event as CEvent, KeyCode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use spud_core::{event::Event, state::AppState, module::Module};
use spud_ui::{layout::doom_layout, shell::{render_shell, ShellView}};

use spud_mod_hello::HelloModule;
use spud_mod_stats::StatsModule;

struct App {
    state: AppState,
    modules: Vec<Box<dyn Module>>,
}

impl App {
    fn new() -> Self {
        Self {
            state: AppState::new(),
            modules: vec![
                Box::new(HelloModule::new()),
                Box::new(StatsModule::new()),
            ],
        }
    }

    fn active_module_mut(&mut self) -> &mut Box<dyn Module> {
        &mut self.modules[self.state.active_module_idx]
    }

    fn active_module(&self) -> &Box<dyn Module> {
        &self.modules[self.state.active_module_idx]
    }

    fn next_module(&mut self) {
        self.state.active_module_idx = (self.state.active_module_idx + 1) % self.modules.len();
        self.state.status_line = format!("MODULE: {}", self.active_module().title());
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn main() -> Result<()> {
    let mut terminal = setup_terminal()?;
    let res = run(&mut terminal);
    restore_terminal(terminal)?;
    res
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    let mut app = App::new();
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| {
            let rects = doom_layout(f.area(), 9, 18);
            let m = app.active_module();
            let hud = m.hud();

            let view = ShellView {
                module_title: m.title(),
                status_line: &app.state.status_line,
                hud_left: hud.left_lines,
                hud_right: hud.right_lines,
            };

            let mid = m.id();
            render_shell(f, rects, view, |f, hero_area| {
                match mid {
                    "hello" => spud_mod_hello::render_hero(f, hero_area),
                    "stats" => spud_mod_stats::render_hero(f, hero_area),
                    _ => {}
                }
            });
        })?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            match event::read()? {
                CEvent::Key(key) => {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Tab => app.next_module(),
                        _ => app.active_module_mut().handle_event(&Event::Key(key)),
                    }
                }
                CEvent::Resize(cols, rows) => {
                    app.active_module_mut().handle_event(&Event::Resize { cols, rows });
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            app.active_module_mut().handle_event(&Event::Tick { now: last_tick });
        }
    }
}
EOF

echo "==> Done."
echo "==> Run: cargo run -p spud-app"
echo "==> Then: git status && git add -A && git commit -m 'Stub SPUD workspace + shell'"
