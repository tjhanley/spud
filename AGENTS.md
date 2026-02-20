# Repository Guidelines

## Project Structure & Module Organization
SPUD is a Rust workspace (`Cargo.toml` at repo root) with crates under `crates/`:
- `spud-app`: binary entrypoint and main loop (`cargo run -p spud-app`).
- `spud-core`: shared state, events, commands, and module traits.
- `spud-ui`: terminal rendering and layout.
- `spud-mod-*`: first-party modules (`spud-mod-hello`, `spud-mod-stats`).
- `spud-agent`, `spud-config`, `spud-remote`: supporting/stub crates.

Non-code assets live in `assets/` (for example `assets/faces/default/`), and helper scripts live in `scripts/`.
If `assets/` or `scripts/` are absent in the current branch, treat this as optional structure rather than required layout.

## Build, Test, and Development Commands
- `cargo run -p spud-app`: run the TUI app locally.
- `cargo build --workspace`: compile all crates.
- `cargo test --workspace`: run all tests.
- `cargo test -p spud-core console`: run targeted tests by crate/name filter.
- `cargo fmt --all -- --check`: verify formatting.
- `cargo clippy --workspace -- -D warnings`: lint with warnings treated as errors (CI behavior).

## Coding Style & Naming Conventions
- Rust 2021 edition, formatted with `rustfmt` (default 4-space indentation).
- Keep public APIs documented with `///` doc comments.
- Prefer `anyhow::Result<T>` and `anyhow::bail!()` for internal error flow.
- Use typed errors for public control-flow contracts when callers must branch on variants (for example `spud-remote::runtime::RuntimeError`).
- Keep crate names consistent: `spud-{component}` and `spud-mod-{name}`.
- Keep the codebase synchronous (no async runtime patterns).

## Testing Guidelines
- Write unit tests inline using `#[cfg(test)] mod tests` at the end of source files.
- Add or update tests with behavior changes, especially around event flow, commands, and rendering logic.
- Run `cargo test --workspace` before opening a PR; use targeted `cargo test -p <crate>` while iterating.

## Commit & Pull Request Guidelines
- Create feature branches from `main` using `tjh/<issue-number>-<feature-name>`.
- Use short, imperative commit subjects (for example, `Fix console cursor position (#28)`).
- PRs should link the issue, pass fmt/clippy/test checks, and include screenshots/GIFs for UI-visible changes.
- Match issue metadata on the PR: labels, milestone, and project (`SPUD Roadmap`).

## Roadmap Hygiene Protocol
- Before implementation: create or update an issue with clear scope, acceptance criteria, and dependencies.
- On issue creation: set labels, milestone, assignee, project (`SPUD Roadmap`), `Phase`, and initial `Status`.
- During implementation: keep issue and tracking tickets updated when scope changes or follow-up issues are split out.
- On PR open: link the issue, confirm project field alignment, and keep PR scope to one issue whenever possible.
- On merge: close/retarget related issues, update tracking issue checklists, and set project status fields to reflect reality.
- Weekly cleanup: resolve stale `In Progress`/`Todo` mismatches, close completed tracking tickets, and ensure no board items are missing `Phase`.

## Security & Configuration Tips
- Follow `SECURITY.md` for responsible disclosure.
- Use `SPUD_LOG_DIR` to override local log output location when needed.
