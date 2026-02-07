use std::collections::HashMap;
use std::time::Instant;

use crate::console::Console;
use crate::fps::TickCounter;
use crate::registry::ModuleRegistry;

/// Output from a command execution.
pub enum CommandOutput {
    /// Lines to display in the console.
    Lines(Vec<String>),
    /// Signal that the app should quit.
    Quit,
}

/// Context available to commands during execution.
pub struct CommandContext<'a> {
    pub registry: &'a mut ModuleRegistry,
    pub console: &'a mut Console,
    pub tick_counter: &'a TickCounter,
    pub started_at: Instant,
}

/// A console command.
pub trait Command: Send + Sync {
    fn name(&self) -> &str;
    fn aliases(&self) -> &[&str] { &[] }
    fn description(&self) -> &str;
    fn usage(&self) -> &str { self.name() }
    fn execute(&self, args: &[&str], ctx: &mut CommandContext) -> CommandOutput;
}

/// Registry of console commands.
pub struct CommandRegistry {
    commands: Vec<Box<dyn Command>>,
    lookup: HashMap<String, usize>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    pub fn register(&mut self, cmd: Box<dyn Command>) {
        let idx = self.commands.len();
        self.lookup.insert(cmd.name().to_string(), idx);
        for alias in cmd.aliases() {
            self.lookup.insert(alias.to_string(), idx);
        }
        self.commands.push(cmd);
    }

    pub fn execute(&self, input: &str, ctx: &mut CommandContext) -> CommandOutput {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return CommandOutput::Lines(vec![]);
        }

        let name = parts[0];
        let args = &parts[1..];

        match self.lookup.get(name) {
            Some(&idx) => self.commands[idx].execute(args, ctx),
            None => CommandOutput::Lines(vec![
                format!("unknown command: '{}'. Type 'help' for available commands.", name),
            ]),
        }
    }

    pub fn commands(&self) -> &[Box<dyn Command>] {
        &self.commands
    }
}

// ── Built-in commands ──

pub struct HelpCommand;

impl Command for HelpCommand {
    fn name(&self) -> &str { "help" }
    fn aliases(&self) -> &[&str] { &["?"] }
    fn description(&self) -> &str { "List commands or show specific help" }
    fn usage(&self) -> &str { "help [command]" }

    fn execute(&self, args: &[&str], _ctx: &mut CommandContext) -> CommandOutput {
        // Note: we can't access the CommandRegistry from inside a command easily,
        // so help with args is handled specially by the caller. This returns generic help.
        if !args.is_empty() {
            return CommandOutput::Lines(vec![
                format!("help for '{}' — use 'help' to list all commands", args[0]),
            ]);
        }
        // Placeholder — the real help list is injected by the caller
        CommandOutput::Lines(vec!["Type 'help' to list all commands.".into()])
    }
}

pub struct ClearCommand;

impl Command for ClearCommand {
    fn name(&self) -> &str { "clear" }
    fn aliases(&self) -> &[&str] { &["cls"] }
    fn description(&self) -> &str { "Clear console log" }

    fn execute(&self, _args: &[&str], ctx: &mut CommandContext) -> CommandOutput {
        ctx.console.clear_logs();
        CommandOutput::Lines(vec![])
    }
}

pub struct ModulesCommand;

impl Command for ModulesCommand {
    fn name(&self) -> &str { "modules" }
    fn aliases(&self) -> &[&str] { &["mods"] }
    fn description(&self) -> &str { "List registered modules" }

    fn execute(&self, _args: &[&str], ctx: &mut CommandContext) -> CommandOutput {
        let active_id = ctx.registry.active_id().map(|s| s.to_string());
        let lines: Vec<String> = ctx.registry.list().iter().map(|(id, title)| {
            let marker = if Some(id.to_string()) == active_id { " *" } else { "" };
            format!("  {} — {}{}", id, title, marker)
        }).collect();
        CommandOutput::Lines(lines)
    }
}

pub struct SwitchCommand;

impl Command for SwitchCommand {
    fn name(&self) -> &str { "switch" }
    fn aliases(&self) -> &[&str] { &["sw"] }
    fn description(&self) -> &str { "Switch active module" }
    fn usage(&self) -> &str { "switch <module_id>" }

    fn execute(&self, args: &[&str], ctx: &mut CommandContext) -> CommandOutput {
        if args.is_empty() {
            return CommandOutput::Lines(vec!["usage: switch <module_id>".into()]);
        }
        match ctx.registry.activate(args[0]) {
            Ok(_events) => {
                let title = ctx.registry.active().map(|m| m.title()).unwrap_or("?");
                CommandOutput::Lines(vec![format!("Switched to: {}", title)])
            }
            Err(e) => CommandOutput::Lines(vec![format!("error: {}", e)]),
        }
    }
}

pub struct QuitCommand;

impl Command for QuitCommand {
    fn name(&self) -> &str { "quit" }
    fn aliases(&self) -> &[&str] { &["exit", "q"] }
    fn description(&self) -> &str { "Exit SPUD" }

    fn execute(&self, _args: &[&str], _ctx: &mut CommandContext) -> CommandOutput {
        CommandOutput::Quit
    }
}

pub struct UptimeCommand;

impl Command for UptimeCommand {
    fn name(&self) -> &str { "uptime" }
    fn aliases(&self) -> &[&str] { &[] }
    fn description(&self) -> &str { "Show runtime uptime" }

    fn execute(&self, _args: &[&str], ctx: &mut CommandContext) -> CommandOutput {
        let elapsed = ctx.started_at.elapsed();
        let secs = elapsed.as_secs();
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        let s = secs % 60;
        CommandOutput::Lines(vec![format!("Uptime: {:02}:{:02}:{:02}", hours, mins, s)])
    }
}

pub struct TpsCommand;

impl Command for TpsCommand {
    fn name(&self) -> &str { "tps" }
    fn aliases(&self) -> &[&str] { &["fps"] }
    fn description(&self) -> &str { "Show ticks-per-second" }

    fn execute(&self, _args: &[&str], ctx: &mut CommandContext) -> CommandOutput {
        CommandOutput::Lines(vec![format!("TPS: {:.1}", ctx.tick_counter.tps())])
    }
}

pub struct EchoCommand;

impl Command for EchoCommand {
    fn name(&self) -> &str { "echo" }
    fn aliases(&self) -> &[&str] { &[] }
    fn description(&self) -> &str { "Print message to console" }
    fn usage(&self) -> &str { "echo <message>" }

    fn execute(&self, args: &[&str], _ctx: &mut CommandContext) -> CommandOutput {
        CommandOutput::Lines(vec![args.join(" ")])
    }
}

/// Create a CommandRegistry pre-loaded with all built-in commands.
pub fn builtin_registry() -> CommandRegistry {
    let mut reg = CommandRegistry::new();
    reg.register(Box::new(HelpCommand));
    reg.register(Box::new(ClearCommand));
    reg.register(Box::new(ModulesCommand));
    reg.register(Box::new(SwitchCommand));
    reg.register(Box::new(QuitCommand));
    reg.register(Box::new(UptimeCommand));
    reg.register(Box::new(TpsCommand));
    reg.register(Box::new(EchoCommand));
    reg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::Console;
    use crate::fps::TickCounter;
    use crate::module::Module;
    use crate::registry::ModuleRegistry;

    struct FakeModule {
        id: &'static str,
        title: &'static str,
    }
    impl Module for FakeModule {
        fn id(&self) -> &'static str { self.id }
        fn title(&self) -> &'static str { self.title }
    }

    fn make_ctx() -> (ModuleRegistry, Console, TickCounter, Instant) {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule { id: "hello", title: "Hello" })).unwrap();
        reg.register(Box::new(FakeModule { id: "stats", title: "Stats" })).unwrap();
        (reg, Console::default(), TickCounter::default(), Instant::now())
    }

    fn ctx_from(parts: &mut (ModuleRegistry, Console, TickCounter, Instant)) -> CommandContext<'_> {
        CommandContext {
            registry: &mut parts.0,
            console: &mut parts.1,
            tick_counter: &parts.2,
            started_at: parts.3,
        }
    }

    // ── Parsing tests ──

    #[test]
    fn empty_input_returns_empty() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("", &mut ctx) {
            CommandOutput::Lines(lines) => assert!(lines.is_empty()),
            _ => panic!("expected Lines"),
        }
    }

    #[test]
    fn unknown_command_returns_error() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("foobar", &mut ctx) {
            CommandOutput::Lines(lines) => {
                assert!(lines[0].contains("unknown command"));
            }
            _ => panic!("expected Lines"),
        }
    }

    #[test]
    fn command_name_extraction() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        // "echo hello world" should parse "echo" as command, ["hello", "world"] as args
        match reg.execute("echo hello world", &mut ctx) {
            CommandOutput::Lines(lines) => assert_eq!(lines[0], "hello world"),
            _ => panic!("expected Lines"),
        }
    }

    // ── Alias tests ──

    #[test]
    fn lookup_by_alias() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        // "?" is an alias for "help"
        match reg.execute("?", &mut ctx) {
            CommandOutput::Lines(_) => {} // just checking it resolves
            _ => panic!("expected Lines"),
        }
        // "cls" is an alias for "clear"
        match reg.execute("cls", &mut ctx) {
            CommandOutput::Lines(_) => {}
            _ => panic!("expected Lines"),
        }
    }

    // ── Built-in command tests ──

    #[test]
    fn help_command() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("help", &mut ctx) {
            CommandOutput::Lines(lines) => assert!(!lines.is_empty()),
            _ => panic!("expected Lines"),
        }
    }

    #[test]
    fn clear_command_clears_console() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        parts.1.push_log(crate::logging::LogEntry {
            level: crate::logging::LogLevel::Info,
            target: "test".into(),
            message: "hello".into(),
        });
        assert_eq!(parts.1.log_lines().len(), 1);
        let mut ctx = ctx_from(&mut parts);
        reg.execute("clear", &mut ctx);
        assert!(parts.1.log_lines().is_empty());
    }

    #[test]
    fn modules_command_lists_modules() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("modules", &mut ctx) {
            CommandOutput::Lines(lines) => {
                assert_eq!(lines.len(), 2);
                assert!(lines[0].contains("hello"));
                assert!(lines[1].contains("stats"));
                // Active module should be marked
                assert!(lines[0].contains("*"));
            }
            _ => panic!("expected Lines"),
        }
    }

    #[test]
    fn switch_command_changes_active() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("switch stats", &mut ctx) {
            CommandOutput::Lines(lines) => {
                assert!(lines[0].contains("Stats"));
            }
            _ => panic!("expected Lines"),
        }
        assert_eq!(parts.0.active_id(), Some("stats"));
    }

    #[test]
    fn switch_command_invalid_id() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("switch nope", &mut ctx) {
            CommandOutput::Lines(lines) => {
                assert!(lines[0].contains("error"));
            }
            _ => panic!("expected Lines"),
        }
    }

    #[test]
    fn switch_command_no_args() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("switch", &mut ctx) {
            CommandOutput::Lines(lines) => {
                assert!(lines[0].contains("usage"));
            }
            _ => panic!("expected Lines"),
        }
    }

    #[test]
    fn quit_command_signals_quit() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("quit", &mut ctx) {
            CommandOutput::Quit => {}
            _ => panic!("expected Quit"),
        }
    }

    #[test]
    fn quit_aliases() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        assert!(matches!(reg.execute("exit", &mut ctx), CommandOutput::Quit));
        let mut ctx = ctx_from(&mut parts);
        assert!(matches!(reg.execute("q", &mut ctx), CommandOutput::Quit));
    }

    #[test]
    fn uptime_command() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("uptime", &mut ctx) {
            CommandOutput::Lines(lines) => {
                assert!(lines[0].starts_with("Uptime:"));
            }
            _ => panic!("expected Lines"),
        }
    }

    #[test]
    fn tps_command() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("tps", &mut ctx) {
            CommandOutput::Lines(lines) => {
                assert!(lines[0].starts_with("TPS:"));
            }
            _ => panic!("expected Lines"),
        }
    }

    #[test]
    fn tps_alias_fps() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("fps", &mut ctx) {
            CommandOutput::Lines(lines) => {
                assert!(lines[0].starts_with("TPS:"));
            }
            _ => panic!("expected Lines"),
        }
    }

    #[test]
    fn echo_command() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("echo hello world", &mut ctx) {
            CommandOutput::Lines(lines) => {
                assert_eq!(lines[0], "hello world");
            }
            _ => panic!("expected Lines"),
        }
    }

    #[test]
    fn echo_empty() {
        let reg = builtin_registry();
        let mut parts = make_ctx();
        let mut ctx = ctx_from(&mut parts);
        match reg.execute("echo", &mut ctx) {
            CommandOutput::Lines(lines) => {
                assert_eq!(lines[0], "");
            }
            _ => panic!("expected Lines"),
        }
    }
}
