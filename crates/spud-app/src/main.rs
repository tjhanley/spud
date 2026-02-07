use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{self, Event as CEvent, KeyCode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use spud_core::{
    bus::EventBus,
    command::{self, CommandContext, CommandOutput, CommandRegistry},
    console::Console,
    event::Event,
    fps::TickCounter,
    logging::{self, LogBuffer, LogEntry, LogLevel},
    state::AppState,
    registry::ModuleRegistry,
};
use spud_ui::{
    console::render_console,
    layout::doom_layout,
    shell::{render_shell, ShellView},
};

use spud_mod_hello::HelloModule;
use spud_mod_stats::StatsModule;

struct App {
    state: AppState,
    registry: ModuleRegistry,
    bus: EventBus,
    log_buffer: LogBuffer,
    console: Console,
    tick_counter: TickCounter,
    commands: CommandRegistry,
}

impl App {
    fn new(log_buffer: LogBuffer) -> Result<Self> {
        let mut registry = ModuleRegistry::new();
        registry.register(Box::new(HelloModule::new()))?;
        registry.register(Box::new(StatsModule::new()))?;
        Ok(Self {
            state: AppState::new(),
            registry,
            bus: EventBus::new(),
            log_buffer,
            console: Console::default(),
            tick_counter: TickCounter::default(),
            commands: command::builtin_registry(),
        })
    }

    /// Drain new entries from the shared log buffer into the console.
    fn sync_logs(&mut self) {
        if let Ok(mut buf) = self.log_buffer.lock() {
            for entry in buf.drain(..) {
                self.console.push_log(entry);
            }
        }
    }

    /// Execute a console command and handle the output.
    fn dispatch_command(&mut self, input: &str) -> bool {
        if input.trim().is_empty() {
            return false;
        }

        // Echo the command itself
        self.console.push_log(LogEntry {
            level: LogLevel::Info,
            target: "console".into(),
            message: format!("> {}", input),
        });

        let trimmed = input.trim();

        // Special-case "help" with no args to list all commands from the registry
        if trimmed == "help" || trimmed == "?" {
            let lines: Vec<String> = self.commands.commands().iter().map(|cmd| {
                let aliases = cmd.aliases();
                if aliases.is_empty() {
                    format!("  {:12} {}", cmd.usage(), cmd.description())
                } else {
                    format!("  {:12} {} (aliases: {})", cmd.usage(), cmd.description(), aliases.join(", "))
                }
            }).collect();
            for line in lines {
                self.console.push_log(LogEntry {
                    level: LogLevel::Info,
                    target: "help".into(),
                    message: line,
                });
            }
            return false;
        }

        let mut ctx = CommandContext {
            registry: &mut self.registry,
            console: &mut self.console,
            tick_counter: &self.tick_counter,
            started_at: self.state.started_at,
        };

        match self.commands.execute(trimmed, &mut ctx) {
            CommandOutput::Lines(lines) => {
                for line in lines {
                    self.console.push_log(LogEntry {
                        level: LogLevel::Info,
                        target: "console".into(),
                        message: line,
                    });
                }
                false
            }
            CommandOutput::Quit => true,
        }
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
    let log_buffer = logging::init();
    tracing::info!("SPUD starting up");

    let mut terminal = setup_terminal()?;
    let res = run(&mut terminal, log_buffer);
    restore_terminal(terminal)?;
    res
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, log_buffer: LogBuffer) -> Result<()> {
    let mut app = App::new(log_buffer)?;
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        // ── Sync logs from tracing into console ──
        app.sync_logs();

        // ── Render ──
        terminal.draw(|f| {
            let rects = doom_layout(f.area(), 9, 18);

            if let Some(m) = app.registry.active() {
                let hud = m.hud();
                let view = ShellView {
                    module_title: m.title(),
                    status_line: &app.state.status_line,
                    hud_left: hud.left_lines,
                    hud_right: hud.right_lines,
                };

                render_shell(f, rects, view, |f, hero_area| {
                    if let Some(m) = app.registry.active() {
                        m.render_hero(f, hero_area);
                    }
                });
            }

            // Console overlay on top
            if app.console.visible {
                render_console(f, f.area(), &app.console, app.tick_counter.tps());
            }
        })?;

        // ── Poll → Publish ──
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            match event::read()? {
                CEvent::Key(key) => {
                    // Tilde always toggles the console
                    if key.code == KeyCode::Char('`') || key.code == KeyCode::Char('~') {
                        app.console.toggle();
                    } else if app.console.visible {
                        // Console captures all keys when open
                        match key.code {
                            KeyCode::Enter => {
                                let input = app.console.submit_input();
                                if app.dispatch_command(&input) {
                                    return Ok(());
                                }
                            }
                            KeyCode::Backspace => app.console.backspace(),
                            KeyCode::Left => app.console.cursor_left(),
                            KeyCode::Right => app.console.cursor_right(),
                            KeyCode::PageUp => app.console.scroll_up(10),
                            KeyCode::PageDown => app.console.scroll_down(10),
                            KeyCode::Esc => app.console.toggle(),
                            KeyCode::Char(c) => app.console.insert_char(c),
                            _ => {}
                        }
                    } else {
                        // Normal mode
                        match key.code {
                            KeyCode::Char('q') => {
                                app.bus.publish(Event::Quit);
                            }
                            KeyCode::Tab => {
                                let lifecycle = app.registry.cycle_next();
                                for ev in lifecycle {
                                    app.bus.publish(ev);
                                }
                                if let Some(m) = app.registry.active() {
                                    app.state.status_line = format!("MODULE: {}", m.title());
                                }
                            }
                            _ => {
                                app.bus.publish(Event::Key(key));
                            }
                        }
                    }
                }
                CEvent::Resize(cols, rows) => {
                    app.bus.publish(Event::Resize { cols, rows });
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            app.tick_counter.tick(last_tick);
            app.bus.publish(Event::Tick { now: last_tick });
        }

        // ── Drain → Broadcast ──
        let events = app.bus.drain();
        for ev in &events {
            if matches!(ev, Event::Quit) {
                return Ok(());
            }
            app.registry.broadcast(ev);
        }
    }
}
