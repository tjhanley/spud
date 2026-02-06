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
