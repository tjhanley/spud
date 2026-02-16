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
