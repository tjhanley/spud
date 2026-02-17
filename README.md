# SPUD
## Suspiciously Powerful Utility of De-evolution

A modular Rust TUI dashboard with a Doom-inspired HUD shell, pluggable modules, and an animated ASCII agent face.

Current status:
- Native Rust module runtime (`spud-core`) with module registry + event bus.
- Doom-style shell/console UI (`spud-ui`) with HUD panels and animated overlay console.
- Built-in modules:
  - `spud-mod-hello`
  - `spud-mod-stats` (real telemetry via `sysinfo`)
- Phase 4 plugin runtime work is tracked in the GitHub roadmap.
- Phase 6 agentic module planning is tracked in [#52](https://github.com/tjhanley/spud/issues/52).

### Roadmap Snapshot
- `v0.4 — Plugin Runtime`:
  - [#13](https://github.com/tjhanley/spud/issues/13) tracking
- `v0.5 — Themes & Polish`:
  - [#14](https://github.com/tjhanley/spud/issues/14) tracking
- `v0.6 — Agentic Workflows`:
  - [#52](https://github.com/tjhanley/spud/issues/52) tracking
  - [`docs/phase-6-agentic-module.md`](docs/phase-6-agentic-module.md) design
  - [#47](https://github.com/tjhanley/spud/issues/47) runtime contract
  - [#50](https://github.com/tjhanley/spud/issues/50) orchestration runtime
  - [#51](https://github.com/tjhanley/spud/issues/51) tool bridge + permissions
  - [#46](https://github.com/tjhanley/spud/issues/46) provider adapters
  - [#49](https://github.com/tjhanley/spud/issues/49) UI surfaces
  - [#48](https://github.com/tjhanley/spud/issues/48) safety + observability

### Run
```bash
cargo run -p spud-app
```

### Controls
- `` ` `` or `~`: toggle console overlay
- `Tab`: cycle active module
- `q`: quit

### Dev Checks
```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
./scripts/check-cargo-deny.sh
./scripts/check-semgrep.sh
./scripts/check-static-analysis.sh
```

### License
Apache-2.0
