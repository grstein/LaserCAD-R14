# LCV-152 — Tasks

- [x] T1 [AC3] Record baseline timings and `cargo test --all -- --list` output (files: docs/specs/LCV-152-faster-build-and-test-loop/plan.md)
- [x] T2 [AC1,AC2] Create `tests/it/main.rs`, move integration files and harness in as modules; fix relative paths (files: tests/**)
- [x] T3 [AC2] Diff the test list against the baseline; zero missing names (files: plan.md)
- [x] T4 [AC3] Dev profile tuning (files: Cargo.toml)
- [ ] T5 [AC3,AC4] Optional mold with clean fallback; document (files: .cargo/config.toml, docs/build-local.md)
- [ ] T6 Update harness path in LOC-cap script and test rule globs (files: scripts/loc-cap.sh, .claude/rules/tests.md)
- [ ] T7 [AC3,AC5] Record after timings; gate green (files: plan.md)
