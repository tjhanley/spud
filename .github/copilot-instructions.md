# Copilot Instructions for SPUD

SPUD (Suspiciously Powerful Utility of De-evolution) is a modular Rust TUI dashboard inspired by classic Doom HUD layouts. It uses ratatui for rendering and crossterm for terminal I/O.

## Project Structure

This is a Rust workspace (`edition = "2021"`) with these crates:

| Crate | Role |
|-------|------|
| `spud-app` | Binary — main loop, input handling, terminal setup |
| `spud-core` | Runtime state, module trait, event bus, command registry, console |
| `spud-ui` | HUD layout and rendering (ratatui widgets) |
| `spud-agent` | Personality engine and animated face system |
| `spud-config` | Configuration loader (XDG-compatible) |
| `spud-remote` | TypeScript plugin runtime (JSON-RPC bridge) |
| `spud-mod-*` | First-party modules (e.g., `spud-mod-hello`, `spud-mod-stats`) |

## Architecture Patterns

### Module Trait

All modules implement `spud_core::module::Module`:

```rust
pub trait Module {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn handle_event(&mut self, _ev: &Event) {}
    fn hud(&self) -> HudContribution { HudContribution::default() }
    fn as_any(&self) -> &dyn Any;
}
```

- Use `&'static str` for id and title
- Provide default no-op implementations for optional methods
- `as_any()` has no default — the compiler enforces it on all implementors
- Modules register via `register_module(registry, render_map, MyModule::new())`

### HeroRenderer Trait

Modules that render hero content also implement `spud_ui::renderer::HeroRenderer`:

```rust
pub trait HeroRenderer {
    fn render_hero(&self, f: &mut Frame, area: Rect);
}
```

- `spud-core` has no `ratatui` dependency — rendering types live in `spud-ui`
- The app captures type-aware render closures at registration time via `as_any()` downcasting
- Modules that don't render can skip `spud-ui` entirely

### Event Bus

Central `EventBus` distributes events. Routing rules:
- `Tick` / `Resize` → broadcast to all modules
- `Key` → active module only
- `ModuleActivated` / `ModuleDeactivated` → named target module
- Commands return `CommandOutput` enum, not `Result`

### Console

The drop-down console uses a `SlideState` enum for time-based animation. Key methods:
- `toggle(now: Instant)` — state transitions with mid-animation reversal
- `update(now: Instant)` — advance animation each frame
- `overlay_fraction(now: Instant) -> f64` — linear interpolation for rendering
- `is_visible()` — render gate (any non-Hidden state)
- `is_open()` — input gate (only fully Open state)

Use `checked_duration_since` for all `Instant` arithmetic to avoid panics.

## Coding Conventions

### Style
- Rust 2021 edition, default `rustfmt` formatting
- Doc comments (`///`) on all public types and methods
- No redundant comments — code should be self-documenting
- Method chaining for ratatui widget builders
- Prefer `&'static str` over `String` for fixed identifiers

### Error Handling
- Use `anyhow::Result<T>` for fallible operations
- Use `anyhow::bail!()` for error returns
- No custom error types — rely on anyhow context
- Commands handle their own error display via `CommandOutput`

### Dependencies
- Pin to major version: `anyhow = "1"`, `ratatui = "0.30"`, `crossterm = "0.29"`
- Shared deps (`ratatui`, `crossterm`) are centralized in `[workspace.dependencies]`; crates reference them via `{ workspace = true }`
- Internal workspace crates use path dependencies: `{ path = "../crate-name" }`
- Logging via `tracing` crate (`tracing::info!`, `tracing::warn!`, etc.)

### Testing
- Inline test modules: `#[cfg(test)] mod tests { ... }` at end of file
- Test helpers are file-local, concrete types (no trait objects)
- Comprehensive assertions: `assert_eq!`, `matches!`, `.is_err()` checks
- No external test frameworks — standard `#[test]` only

### Naming
- Crate names: `spud-{component}` (kebab-case)
- Module crate names: `spud-mod-{name}`
- Struct names match their role: `Console`, `EventBus`, `ModuleRegistry`
- Methods use Rust conventions: `new()`, `default()`, `is_*()` for bool getters

## Review Guidelines

When reviewing PRs:
- Verify `cargo test --workspace` passes
- Verify `cargo clippy --workspace` is clean
- Check that new public APIs have doc comments
- Ensure time-based code uses `checked_duration_since`, not `duration_since`
- Confirm animation/UI code handles edge cases (zero height, minimum dimensions)
- New modules must implement the `Module` trait and register in `main.rs`
- Prefer early returns over deep nesting
- Only flag issues with high confidence — avoid nitpicks on style preferences that `rustfmt` handles

## What NOT to Suggest

- Don't suggest adding `unsafe` code
- Don't suggest custom error types — use `anyhow`
- Don't suggest `async` unless the crate already uses it (none currently do)
- Don't suggest adding dependencies without clear justification
- Don't suggest changes to `LICENSE` or `SECURITY.md`
