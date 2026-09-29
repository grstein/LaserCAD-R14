# LCV-155 — Tasks

- [x] T1 [AC1] Fast lane in AGENTS.md §Workflow; implementer accepts a fast-lane brief (files: AGENTS.md, .claude/agents/implementer-rust.md)
- [x] T2 [AC2] `scripts/check.sh [filter...]`; document in AGENTS.md §Commands and tests rules (files: scripts/check.sh, AGENTS.md, .claude/rules/tests.md)
- [x] T3 [AC3, AC5] Move `tests/it/*.rs` into area modules; recursive meta-tests; `CARGO_MANIFEST_DIR` include paths; update path mentions (files: tests/it/**, .claude/rules/*.md, docs/specs/_templates/tasks.md)
- [x] T4 [AC4, AC5] `is_test_file` helper and scan updates; move inline test modules >300 lines to sibling `tests.rs`; ADR 0004 amendment (files: tests/harness/scan.rs, scans, src/** large files, docs/adr/0004-measuring-the-300-loc-cap.md)
