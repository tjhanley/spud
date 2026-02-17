# Phase 6: Agentic Module Design

This document captures the target architecture for `v0.6 — Agentic Workflows`.

Tracking issue: [#52](https://github.com/tjhanley/spud/issues/52)

## Goals
- Add a first-party agentic workflow to SPUD without hardcoding a model vendor.
- Keep provider/model selection fully configuration-driven.
- Let agents call SPUD capabilities through a permissioned module bridge.
- Expose clear agent state and output in existing TUI layout.

## Non-Goals
- No provider lock-in in `spud-core`.
- No async runtime migration in this phase.
- No UI redesign that discards current HUD shell structure.

## UX Layout Mapping
- Bottom-left tray: persistent prompt composer.
- Right panel: spawned agents with lifecycle state, elapsed time, and mood badge.
- Hero pane: output stream for currently selected agent.

## Runtime Architecture
- `spud-core`:
  - agent lifecycle/state machine types
  - orchestration queue (enqueue, spawn, cancel, retry)
  - event contracts for state/output/mood
- `spud-config`:
  - provider/model config schema
  - limits and permission allowlists
- `spud-ui`:
  - prompt composer widget
  - agent list/state panel
  - hero output stream view
- Provider adapters:
  - implemented behind shared traits
  - local + cloud paths selectable by config

## Agent Lifecycle
- `queued`
- `planning`
- `running`
- `waiting_input`
- `completed`
- `failed`
- `cancelled`

State transitions are explicit and validated by tests.

## Tool/Module Bridge
- Agents can invoke allowlisted module actions only.
- Unauthorized actions return structured errors.
- Mood hints from agents are normalized by host and emitted as UI-consumable mood events.

## Config Shape (Proposed)
```toml
[agent]
provider = "ollama"
model = "qwen2.5-coder:14b"
temperature = 0.2
max_tokens = 2048
max_concurrent_agents = 2
request_timeout_ms = 30000
max_tool_calls_per_task = 12

[agent.permissions]
module_actions = ["stats.snapshot", "hello.greet", "core.command.run"]
```

## Delivery Slices
- #47: runtime contract (provider-agnostic)
- #50: orchestration runtime
- #51: tool bridge + module permissions
- #46: config-driven provider adapters
- #49: UI surfaces (tray/panel/hero)
- #48: safety limits + observability
