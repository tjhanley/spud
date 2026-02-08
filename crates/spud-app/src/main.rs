use std::any::Any;
use std::collections::HashMap;
use std::io::{self, Write, Stdout};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Frame, Terminal};

use spud_core::{
    bus::EventBus,
    command::{self, CommandContext, CommandOutput, CommandRegistry},
    console::Console,
    event::Event,
    fps::TickCounter,
    logging::{self, LogBuffer, LogEntry, LogLevel},
    module::Module,
    registry::ModuleRegistry,
    state::AppState,
};
use spud_ui::{
    console::render_console,
    graphics::{detect_backend, GraphicsBackend},
    iterm,
    layout::doom_layout,
    renderer::HeroRenderer,
    shell::{render_shell, FaceFrame, ShellView},
};

use spud_mod_hello::HelloModule;
use spud_mod_stats::StatsModule;

/// A type-erased render function that downcasts a module via `Any` and draws
/// its hero area.
type RenderFn = Box<dyn Fn(&dyn Any, &mut Frame, Rect)>;

struct App {
    state: AppState,
    registry: ModuleRegistry,
    bus: EventBus,
    log_buffer: LogBuffer,
    console: Console,
    tick_counter: TickCounter,
    commands: CommandRegistry,
    render_map: HashMap<String, RenderFn>,
    agent: spud_agent::Agent,
    graphics_backend: GraphicsBackend,
}

/// Register a module that also implements `HeroRenderer`.
///
/// Inserts the module into the registry and captures a type-aware render
/// closure in `render_map` so the app can call `render_hero` without
/// knowing the concrete module type.
fn register_module<M: Module + HeroRenderer + 'static>(
    registry: &mut ModuleRegistry,
    render_map: &mut HashMap<String, RenderFn>,
    module: M,
) -> Result<()> {
    let id = module.id().to_string();
    render_map.insert(
        id,
        Box::new(|any, f, area| {
            if let Some(m) = any.downcast_ref::<M>() {
                m.render_hero(f, area);
            }
        }),
    );
    registry.register(Box::new(module))
}

impl App {
    fn new(log_buffer: LogBuffer) -> Result<Self> {
        let mut registry = ModuleRegistry::new();
        let mut render_map: HashMap<String, RenderFn> = HashMap::new();
        register_module(&mut registry, &mut render_map, HelloModule::new())?;
        register_module(&mut registry, &mut render_map, StatsModule::new())?;
        let agent = spud_agent::Agent::load_default(Instant::now())?;
        let graphics_backend = detect_backend();
        tracing::info!("Graphics backend: {:?}", graphics_backend);
        Ok(Self {
            state: AppState::new(),
            registry,
            bus: EventBus::new(),
            log_buffer,
            console: Console::default(),
            tick_counter: TickCounter::default(),
            commands: command::builtin_registry(),
            render_map,
            agent,
            graphics_backend,
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
            let lines: Vec<String> = self
                .commands
                .commands()
                .iter()
                .map(|cmd| {
                    let aliases = cmd.aliases();
                    if aliases.is_empty() {
                        format!("  {:12} {}", cmd.usage(), cmd.description())
                    } else {
                        format!(
                            "  {:12} {} (aliases: {})",
                            cmd.usage(),
                            cmd.description(),
                            aliases.join(", ")
                        )
                    }
                })
                .collect();
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
            bus: &mut self.bus,
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
    let tick_interval = Duration::from_millis(100);
    let poll_timeout = Duration::from_millis(16);
    let mut last_tick = Instant::now();

    loop {
        // ── Sync logs from tracing into console ──
        app.sync_logs();

        // ── Update animation state ──
        let now = Instant::now();
        app.console.update(now);
        app.agent.tick(now);

        // ── Compute layout before drawing (needed for post-draw iTerm2 rendering) ──
        let terminal_size = terminal.size()?;
        let rects = doom_layout(
            Rect::new(0, 0, terminal_size.width, terminal_size.height),
            9,
            18,
        );

        // Keep face data alive for both draw and post-draw phases
        // (avoids cloning 60 times/sec, which breaks the pointer-based cache)
        let face = app.agent.current_frame();

        // ── Render ──
        terminal.draw(|f| {
            if let Some(m) = app.registry.active() {
                let hud = m.hud();
                let view = ShellView {
                    module_title: m.title(),
                    status_line: &app.state.status_line,
                    hud_left: hud.left_lines,
                    hud_right: hud.right_lines,
                    face: Some(FaceFrame {
                        data: &face.data,
                        width: face.width,
                        height: face.height,
                    }),
                    graphics_backend: app.graphics_backend,
                };

                let render_map = &app.render_map;
                render_shell(f, rects, view, |f, hero_area| {
                    if let Some(m) = app.registry.active() {
                        if let Some(render_fn) = render_map.get(m.id()) {
                            render_fn(m.as_any(), f, hero_area);
                        }
                    }
                });
            }

            // Console overlay on top
            if app.console.is_visible() {
                let fraction = app.console.overlay_fraction(now);
                let show_cursor = app.console.is_open();
                render_console(
                    f,
                    f.area(),
                    &app.console,
                    app.tick_counter.tps(),
                    fraction,
                    show_cursor,
                );
            }
        })?;

        // ── Post-draw: iTerm2 inline image rendering ──
        if app.graphics_backend == GraphicsBackend::ITerm2 {
            use ratatui::widgets::Block;
            let inner = Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .inner(rects.hud_face);
            // Pass reference to face.data - pointer stays stable until frame actually changes
            iterm::render_iterm_face(terminal.backend_mut(), inner, &face.data, face.width, face.height)?;
            terminal.backend_mut().flush()?;
        }

        // ── Poll → Publish ──
        if event::poll(poll_timeout)? {
            match event::read()? {
                CEvent::Key(key) => {
                    // Tilde always toggles the console
                    if key.code == KeyCode::Char('`') || key.code == KeyCode::Char('~') {
                        app.console.toggle(Instant::now());
                    } else if app.console.is_open() {
                        // Console captures all keys when fully open
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
                            KeyCode::Esc => app.console.toggle(Instant::now()),
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

        if last_tick.elapsed() >= tick_interval {
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
