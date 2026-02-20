use std::any::Any;
use std::collections::HashMap;
use std::env;
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Frame, Terminal};
use serde_json::{json, Value};

use spud_core::{
    bus::EventBus,
    command::{self, CommandContext, CommandOutput, CommandRegistry},
    console::Console,
    event::{Event, TelemetryValue},
    fps::TickCounter,
    logging::{self, LogBuffer, LogEntry, LogLevel},
    module::Module,
    registry::ModuleRegistry,
    state::AppState,
};
use spud_remote::{
    protocol::{
        ActiveModule, EventCategory, InvokeCommandParams, InvokeCommandResult, PublishEventParams,
        PublishEventResult, StateSnapshot,
    },
    runtime::{HostBridge, PluginRuntime, RuntimeError},
};
use spud_ui::{
    console::render_console,
    layout::doom_layout,
    renderer::HeroRenderer,
    shell::{render_shell, ShellView},
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
    plugin_runtime: Option<PluginRuntime>,
    log_buffer: LogBuffer,
    console: Console,
    tick_counter: TickCounter,
    commands: CommandRegistry,
    render_map: HashMap<String, RenderFn>,
    agent: spud_agent::Agent,
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
        let mut app = Self {
            state: AppState::new(),
            registry,
            bus: EventBus::new(),
            plugin_runtime: None,
            log_buffer,
            console: Console::default(),
            tick_counter: TickCounter::default(),
            commands: command::builtin_registry(),
            render_map,
            agent,
        };
        app.init_plugin_runtime();
        Ok(app)
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

    fn init_plugin_runtime(&mut self) {
        let roots = configured_plugin_roots();
        if roots.is_empty() {
            tracing::info!(
                "plugin runtime disabled (set SPUD_PLUGIN_DIRS to a path list to enable)"
            );
            return;
        }

        let mut runtime = match PluginRuntime::from_search_roots(&roots) {
            Ok(runtime) => runtime,
            Err(err) => {
                tracing::warn!(error = %err, "plugin runtime discovery failed");
                return;
            }
        };

        let plugin_ids = runtime
            .plugin_ids()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();

        if plugin_ids.is_empty() {
            tracing::info!("plugin runtime enabled but no plugin manifests were discovered");
            self.plugin_runtime = Some(runtime);
            return;
        }

        tracing::info!(
            plugin_count = plugin_ids.len(),
            "starting discovered plugin runtime sessions"
        );

        for plugin_id in plugin_ids {
            match runtime.start(&plugin_id, Duration::from_secs(2)) {
                Ok(handshake) => {
                    tracing::info!(
                        plugin_id = %plugin_id,
                        selected_api_version = %handshake.selected_api_version,
                        methods = handshake.host_capabilities.methods.len(),
                        event_categories = handshake.host_capabilities.event_categories.len(),
                        "plugin handshake completed"
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        plugin_id = %plugin_id,
                        error = %err,
                        "failed to start plugin runtime session"
                    );
                }
            }
        }

        self.plugin_runtime = Some(runtime);
    }

    fn pump_plugin_runtime(&mut self, timeout: Duration) {
        let Some(mut runtime) = self.plugin_runtime.take() else {
            return;
        };

        let plugin_ids = runtime
            .plugin_ids()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();

        if plugin_ids.is_empty() {
            self.plugin_runtime = Some(runtime);
            return;
        }

        let mut host = AppHost {
            state: &self.state,
            registry: &mut self.registry,
            bus: &mut self.bus,
            console: &mut self.console,
            tick_counter: &self.tick_counter,
            commands: &self.commands,
        };
        let pump_started_at = Instant::now();

        for plugin_id in plugin_ids {
            if pump_started_at.elapsed() >= timeout {
                tracing::debug!(
                    budget_ms = timeout.as_millis(),
                    "plugin pump budget exhausted for this frame"
                );
                break;
            }

            match runtime.pump_next(&plugin_id, &mut host, Duration::ZERO) {
                Ok(handled) => {
                    tracing::debug!(
                        plugin_id = %handled.plugin_id,
                        method = %handled.method,
                        responded_with_error = handled.responded_with_error,
                        "handled plugin request"
                    );
                }
                Err(RuntimeError::Timeout { .. } | RuntimeError::NotRunning(_)) => {}
                Err(RuntimeError::ProcessExited { .. }) => {
                    tracing::warn!(
                        plugin_id = %plugin_id,
                        "plugin process exited; runtime session detached"
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        plugin_id = %plugin_id,
                        error = %err,
                        "plugin runtime pump error"
                    );
                }
            }
        }

        self.plugin_runtime = Some(runtime);
    }

    fn forward_event_to_plugins(&mut self, event: &Event) {
        let Some(mut runtime) = self.plugin_runtime.take() else {
            return;
        };

        if let Some((category, tag, payload)) = map_event_for_plugins(event, self.state.started_at)
        {
            if let Err(err) = runtime.broadcast_event(category, tag.as_deref(), payload) {
                tracing::warn!(error = %err, "failed to broadcast host event to plugin runtime");
            }
        }

        self.plugin_runtime = Some(runtime);
    }
}

struct AppHost<'a> {
    state: &'a AppState,
    registry: &'a mut ModuleRegistry,
    bus: &'a mut EventBus,
    console: &'a mut Console,
    tick_counter: &'a TickCounter,
    commands: &'a CommandRegistry,
}

impl HostBridge for AppHost<'_> {
    fn state_snapshot(&mut self) -> Result<StateSnapshot> {
        let active_module = self.registry.active().map(|module| ActiveModule {
            id: module.id().to_string(),
            title: module.title().to_string(),
        });

        let uptime_seconds = Instant::now()
            .checked_duration_since(self.state.started_at)
            .unwrap_or(Duration::ZERO)
            .as_secs();

        Ok(StateSnapshot {
            active_module,
            status_line: self.state.status_line.clone(),
            uptime_seconds,
            tps: self.tick_counter.tps(),
            telemetry: Vec::new(),
        })
    }

    fn invoke_command(&mut self, params: InvokeCommandParams) -> Result<InvokeCommandResult> {
        let mut input = params.command.clone();
        if !params.args.is_empty() {
            input.push(' ');
            input.push_str(&params.args.join(" "));
        }

        let output = {
            let mut ctx = CommandContext {
                registry: self.registry,
                console: self.console,
                bus: self.bus,
                tick_counter: self.tick_counter,
                started_at: self.state.started_at,
            };
            self.commands.execute(&input, &mut ctx)
        };

        let lines = match output {
            CommandOutput::Lines(lines) => lines,
            CommandOutput::Quit => {
                self.bus.publish(Event::Quit);
                vec!["quit requested".to_string()]
            }
        };

        Ok(InvokeCommandResult { lines })
    }

    fn publish_event(&mut self, params: PublishEventParams) -> Result<PublishEventResult> {
        self.bus.publish(Event::Custom {
            tag: params.tag,
            payload: params.payload,
        });
        Ok(PublishEventResult { accepted: true })
    }
}

fn configured_plugin_roots() -> Vec<PathBuf> {
    let Some(raw) = env::var_os("SPUD_PLUGIN_DIRS") else {
        return Vec::new();
    };

    env::split_paths(&raw)
        .filter(|path| !path.as_os_str().is_empty())
        .collect()
}

fn map_event_for_plugins(
    event: &Event,
    started_at: Instant,
) -> Option<(EventCategory, Option<String>, Value)> {
    match event {
        Event::Tick { now } => Some((
            EventCategory::Tick,
            None,
            json!({
                "uptime_seconds": now
                    .checked_duration_since(started_at)
                    .unwrap_or(Duration::ZERO)
                    .as_secs_f64()
            }),
        )),
        Event::Resize { cols, rows } => Some((
            EventCategory::Resize,
            None,
            json!({
                "cols": cols,
                "rows": rows
            }),
        )),
        Event::ModuleActivated { id } => Some((
            EventCategory::ModuleLifecycle,
            Some("module.activated".to_string()),
            json!({ "id": id }),
        )),
        Event::ModuleDeactivated { id } => Some((
            EventCategory::ModuleLifecycle,
            Some("module.deactivated".to_string()),
            json!({ "id": id }),
        )),
        Event::Telemetry { source, key, value } => Some((
            EventCategory::Telemetry,
            None,
            json!({
                "source": source,
                "key": key,
                "value": telemetry_value_json(value)
            }),
        )),
        Event::Custom { tag, payload } => Some((
            EventCategory::Custom,
            Some(tag.clone()),
            parse_custom_payload(payload),
        )),
        Event::Key(_) | Event::Quit => None,
    }
}

fn telemetry_value_json(value: &TelemetryValue) -> Value {
    match value {
        TelemetryValue::Float(value) => json!(value),
        TelemetryValue::Int(value) => json!(value),
        TelemetryValue::Text(value) => json!(value),
    }
}

fn parse_custom_payload(payload: &str) -> Value {
    serde_json::from_str(payload).unwrap_or_else(|_| Value::String(payload.to_string()))
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
    let plugin_pump_timeout = Duration::from_millis(1);
    let mut last_tick = Instant::now();

    loop {
        // ── Sync logs from tracing into console ──
        app.sync_logs();
        app.pump_plugin_runtime(plugin_pump_timeout);

        // ── Update animation state ──
        let now = Instant::now();
        app.console.update(now);
        app.agent.tick(now);

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
                    hud_face_lines: app.agent.current_frame_lines().to_vec(),
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
            app.forward_event_to_plugins(ev);
        }
    }
}
