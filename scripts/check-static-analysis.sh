#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Running cargo-deny advisories check..."
"$ROOT_DIR/scripts/check-cargo-deny.sh"

echo "Running Semgrep Rust scan..."
"$ROOT_DIR/scripts/check-semgrep.sh"
