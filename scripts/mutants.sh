#!/usr/bin/env bash
# Mutation-test only the src/ Rust code changed since <base> (default: main).
# Usage: scripts/mutants.sh [base]      e.g. scripts/mutants.sh main
# Needs cargo-mutants (`cargo install cargo-mutants --locked`). Builds in its own
# target dir so it never shares artifacts with the gate. Output: mutants.out/.
set -euo pipefail
cd "$(dirname "$0")/.."
base=${1:-main}
diff=$(mktemp)
trap 'rm -f "$diff"' EXIT
git diff "$base...HEAD" -- 'src/**/*.rs' 'src/*.rs' > "$diff"
if [[ ! -s $diff ]]; then
  echo "mutants: no src/**/*.rs changes in $base...HEAD; nothing to test."
  exit 0
fi
export CARGO_TARGET_DIR="${MUTANTS_TARGET_DIR:-$PWD/target/mutants}"
# `-- --no-fail-fast` goes to `cargo test` (ADR 0008).
cargo mutants --in-diff "$diff" -- --no-fail-fast
