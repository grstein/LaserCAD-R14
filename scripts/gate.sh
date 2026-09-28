#!/usr/bin/env bash
# The one gate before any "done": same checks as CI plus repo rules.
set -euo pipefail
cd "$(dirname "$0")/.."
step() { printf '\n== %s\n' "$*"; }
step fmt;      cargo fmt --all -- --check
step clippy;   cargo clippy --all-targets -- -D warnings
step test;     cargo test --all --no-fail-fast
step loc-cap;  scripts/loc-cap.sh
step backlog;  scripts/backlog.sh --check
printf '\nGATE GREEN\n'
