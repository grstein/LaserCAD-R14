---
paths:
  - "tests/**"
  - "**/tests.rs"
---
# Test rules

- Gate is `scripts/gate.sh`; `cargo test` always with `--no-fail-fast` (ADR 0008).
- Inner loop: `scripts/check.sh <filter>` (clippy + matching lib/integration tests, no doctests),
  e.g. `check.sh agent` runs `agent::*` and `it::agent::*`. It never replaces the gate.
- One test per acceptance criterion, named after what it proves.
- **Paths in scans**: rebuild from `components()` joined with `/`; never compare `display()` /
  `to_string_lossy()` of a whole path (breaks on Windows CI). Sort on the rendered string.
- **Network isolation is owned by the test**: an endpoint that must not be reached is *unparseable*;
  a loopback fixture binds and owns its socket (`.cargo/config.toml` sets `NO_PROXY`). Never rely on
  "unreachable" (LCV-130).
- **Rendering ACs** are proven on painted shapes via `tests/harness/paint.rs` (`FullOutput.shapes`,
  `Shape::Text`), not by source scans. A "shows all of table X" test derives its expected set from X.
- Widening `pub(crate)` → `pub` for an integration test is allowed for immutable data or pure
  derivations; the doc comment names the test. No test-only accessor functions.
- No test sends Ctrl+O / Ctrl+S / Ctrl+Shift+S or arms dialogs. Persistence tests inject tempdir
  paths into `settings_path` / `autosave_path` (ADR 0006).
- Integration tests are modules of the one binary `tests/it/main.rs` (LCV-152); a new file gets a
  `mod` line there and `use crate::harness;` if it needs the harness. Never add `tests/<name>.rs`.
- `tests/harness/` is shared implementation and obeys the 300-LOC cap; `tests/it/` modules are exempt.
