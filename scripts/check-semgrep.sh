#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

DEFAULT_ARGS=(
  scan
  --config
  p/rust
  --error
  --metrics=off
  --exclude-rule
  rust.lang.security.unsafe-usage.unsafe-usage
  --exclude-rule
  rust.lang.security.temp-dir.temp-dir
)

if [ -n "${SPUD_SEMGREP_BIN:-}" ]; then
  SEMGREP_BIN="$SPUD_SEMGREP_BIN"
elif [ -x "$ROOT_DIR/.tools/semgrep-venv/bin/pysemgrep" ]; then
  SEMGREP_BIN="$ROOT_DIR/.tools/semgrep-venv/bin/pysemgrep"
elif [ -x "$ROOT_DIR/.tools/semgrep-venv/bin/semgrep" ]; then
  SEMGREP_BIN="$ROOT_DIR/.tools/semgrep-venv/bin/semgrep"
elif command -v semgrep >/dev/null 2>&1; then
  SEMGREP_BIN="semgrep"
else
  cat >&2 <<'EOF'
semgrep is not installed.
Install with one of:
  python3 -m pip install semgrep
  python3 -m venv .tools/semgrep-venv && .tools/semgrep-venv/bin/pip install semgrep
EOF
  exit 127
fi

if [[ "$SEMGREP_BIN" == "$ROOT_DIR/.tools/semgrep-venv/bin/"* ]]; then
  SPUD_SEMGREP_HOME="${SPUD_SEMGREP_HOME:-$ROOT_DIR/.tools/home}"
  mkdir -p "$SPUD_SEMGREP_HOME"
  # Keep Semgrep cache/log writes inside repo-local tooling paths.
  export HOME="$SPUD_SEMGREP_HOME"
fi

export SEMGREP_ENABLE_VERSION_CHECK="${SEMGREP_ENABLE_VERSION_CHECK:-0}"
export SEMGREP_SEND_METRICS="${SEMGREP_SEND_METRICS:-off}"

exec "$SEMGREP_BIN" "${DEFAULT_ARGS[@]}" "$@"
