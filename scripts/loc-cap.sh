#!/usr/bin/env bash
# 300 implementation-LOC cap per .rs file (ADR 0004).
# Implementation LOC = lines before the first column-0 `#[cfg(test)]`.
# Exempt: data-only files with no `fn`, and test-only files (`tests.rs`,
# the integration-test modules under tests/it/). tests/harness/ is capped.
set -euo pipefail
cd "$(dirname "$0")/.."
cap=300; fail=0
while IFS= read -r f; do
  [[ $(basename "$f") == tests.rs ]] && continue
  grep -q '\bfn ' "$f" || continue
  n=$(awk '/^#\[cfg\(test\)\]/{print NR-1; f=1; exit} END{if(!f) print NR}' "$f")
  if (( n > cap )); then echo "LOC cap: $f has $n implementation lines (> $cap)"; fail=1; fi
done < <(find src tests/harness -name '*.rs' | sort)
exit $fail
