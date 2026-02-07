# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

SPUD (Suspiciously Powerful Utility of De-evolution) — a modular Rust TUI dashboard with a Doom-inspired HUD shell. Uses ratatui for rendering, crossterm for terminal I/O.

## Commands

```bash
cargo build --workspace          # build everything
cargo test --workspace           # run all tests
cargo test -p spud-core          # test a single crate
cargo test -p spud-core console  # test a single module (matches test names)
cargo clippy --workspace         # lint (CI doesn't run this yet, but PRs should be clean)
cargo run -p spud-app            # run the app
```

## Architecture

Rust workspace (`edition = "2021"`, `resolver = "2"`). Eight crates under `crates/`:

- **spud-app** — Binary. Owns the main loop: sync logs → update animation → render → poll input → publish events → drain → broadcast → check quit.
- **spud-core** — All shared state and traits. Modules depend on this.
- **spud-ui** — Rendering only. Doom-style layout (`doom_layout`), shell chrome, console overlay. No state ownership.
- **spud-mod-\*** — First-party modules (`hello`, `stats`). Each implements the `Module` trait.
- **spud-agent**, **spud-config**, **spud-remote** — Stubs for future phases.

### Module System

Modules implement `spud_core::module::Module` (id, title, handle_event, hud, as_any). Rendering lives in `spud_ui::renderer::HeroRenderer` — modules that render hero content implement both traits. Registered in `App::new()` via `register_module(registry, render_map, MyModule::new())`, which captures a type-aware render closure using `as_any()` downcasting. First registered module auto-activates.

### Event Flow

`EventBus` is a simple FIFO queue. The app loop publishes events, drains them, then `ModuleRegistry::broadcast()` routes them:
- `Tick` / `Resize` / `Telemetry` / `Custom` / `Quit` → all modules
- `Key` → active module only
- `ModuleActivated` / `ModuleDeactivated` → named target module

### Command System

Console commands implement the `Command` trait and register in `builtin_registry()`. Commands receive `CommandContext` (mutable access to registry, console, bus, tick counter) and return `CommandOutput::Lines(...)` or `CommandOutput::Quit`. Built-ins: help, clear, modules, switch, quit, uptime, tps, echo.

### Console Overlay

Drop-down console uses `SlideState` enum for time-based slide animation (250ms). `toggle(Instant)` handles mid-animation reversal. `is_visible()` gates rendering, `is_open()` gates input capture.

### Logging

`tracing` → shared `LogBuffer` (Arc<Mutex<VecDeque<LogEntry>>>) → drained into `Console` each frame. Daily rolling file appender with 7-day auto-cleanup. Log directory: `SPUD_LOG_DIR` env > `~/Library/Logs/spud` (macOS) > XDG data home.

## Conventions

- **Errors**: `anyhow::Result<T>` and `anyhow::bail!()`. No custom error types.
- **Tests**: Inline `#[cfg(test)] mod tests` at end of file. No external test frameworks.
- **Docs**: `///` doc comments on all public types and methods.
- **Time arithmetic**: Always use `checked_duration_since` on `Instant` to avoid panics.
- **Module identifiers**: `&'static str` for id/title, not `String`.
- **Dependencies**: Major-version pins (`anyhow = "1"`). Path deps for workspace crates. New third-party deps go in `[workspace.dependencies]` in the root `Cargo.toml`, then referenced with `{ workspace = true }` in crate `Cargo.toml` files.
- **Crate naming**: `spud-{component}` for infrastructure, `spud-mod-{name}` for modules.
- **No async** — the entire codebase is synchronous.

## Git Workflow

- Always create a feature branch before making changes — never commit directly to `main`.
- Branch naming: `tjh/<issue-number>-<feature-name>` (e.g. `tjh/19-remove-ratatui-from-core`).
- Push the feature branch and open a PR linked to the issue.
- PRs must match their linked issue's labels, milestone, and project (`SPUD Roadmap`).
  - Copy labels from the issue (e.g. `phase:2`, `crate:spud-agent`, `type:feature`).
  - Set the same milestone (e.g. `v0.2 — Agent & Rendering`).
  - After creating the PR, add it to the project board (see **Project Board Integration** below).

## Project Board Integration

`gh pr create --project` and `gh issue create --project` do **not** work with GitHub Projects v2. After creating an issue or PR, you must manually add it and set its fields.

**Step 1 — Add item to project:**
```bash
gh project item-add 2 --owner tjhanley --url <ISSUE_OR_PR_URL>
```

**Step 2 — Get the item ID:**
```bash
gh project item-list 2 --owner tjhanley --format json --jq '.items[] | select(.content.number == <NUMBER>) | .id'
```

**Step 3 — Set the Phase field** (based on the `phase:N` label):
```bash
gh project item-edit --project-id PVT_kwHNPBrOAToMfw --id <ITEM_ID> \
  --field-id PVTSSF_lAHNPBrOAToMf84PTGN5 --single-select-option-id <PHASE_OPTION_ID>
```

**Step 4 — Set the Status field** to "Todo":
```bash
gh project item-edit --project-id PVT_kwHNPBrOAToMfw --id <ITEM_ID> \
  --field-id PVTSSF_lAHNPBrOAToMf84PTF6l --single-select-option-id f75ad846
```

### Field IDs reference

| Field | Field ID |
|-------|----------|
| Phase | `PVTSSF_lAHNPBrOAToMf84PTGN5` |
| Status | `PVTSSF_lAHNPBrOAToMf84PTF6l` |

### Phase option IDs

| Phase | Option ID |
|-------|-----------|
| Phase 1: Foundation | `949bf2e5` |
| Phase 2: Agent & Rendering | `7180dd12` |
| Phase 3: Telemetry | `1d1da87b` |
| Phase 4: Plugins | `2063ea1b` |
| Phase 5: Themes | `000eeb20` |

### Status option IDs

| Status | Option ID |
|--------|-----------|
| Todo | `f75ad846` |
| In Progress | `47fc9ee4` |
| Done | `98236657` |

## PR Checklist

- `cargo fmt --all -- --check` passes
- `cargo test --workspace` passes
- `cargo clippy --workspace` is clean
- New public APIs have doc comments
- New modules registered in `spud-app/src/main.rs`
- Labels, milestone, and project match the linked issue
