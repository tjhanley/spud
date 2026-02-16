#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if command -v cargo-deny >/dev/null 2>&1; then
  exec cargo-deny check advisories "$@"
fi

if [ -x "$ROOT_DIR/.tools/cargo/bin/cargo-deny" ]; then
  exec "$ROOT_DIR/.tools/cargo/bin/cargo-deny" check advisories "$@"
fi

if cargo deny --version >/dev/null 2>&1; then
  exec cargo deny check advisories "$@"
fi

cat >&2 <<'EOF'
cargo-deny is not installed.
Install with one of:
  cargo install cargo-deny
  CARGO_HOME=.tools/cargo-home cargo install --root .tools/cargo cargo-deny
EOF
exit 127
