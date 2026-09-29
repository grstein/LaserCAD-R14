#!/usr/bin/env bash
# Inner loop: clippy, then only lib + integration tests matching the filter (no doctests).
# Usage: scripts/check.sh [filter...]   e.g. scripts/check.sh agent
set -euo pipefail
cd "$(dirname "$0")/.."
step() { printf '\n== %s\n' "$*"; }
step clippy;   cargo clippy --all-targets -- -D warnings
step test;     cargo test --lib --tests --no-fail-fast -q -- "$@"
printf '\nCHECK GREEN (not the gate)\n'
