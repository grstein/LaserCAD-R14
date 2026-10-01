# LCV-200 — Tasks

- [x] T1 [AC1] Allow a data-only `tests/fixtures/` (no `.rs` at any depth) in `single_test_binary`,
  unless already present; note in the commit that the svg-branch merge keeps one copy
  (files: tests/it/repo/single_test_binary.rs)
- [ ] T2 [AC2] `run_turn_inline` + `InlineTurn`; `drive_turn`/`answer_act`/`end_turn` visibility;
  unit test with a two-reply scripted `send_fn` that draws a line
  (files: src/app/agent_inline.rs, src/app/agent_worker.rs, src/app/mod.rs)
- [ ] T3 [AC3] Scorer: rasterize + 1 px dilation + IoU, and the four assertion kinds, with
  self-tests (identical → 1.0, disjoint → 0.0) (files: tests/it/agent/bench_score.rs,
  tests/it/agent/mod.rs)
- [ ] T4 [AC1] Fixtures `plate-holes`, `box-face-tabs`: prompt, reference, assertions, replies
  (files: tests/fixtures/agent-bench/plate-holes/*, tests/fixtures/agent-bench/box-face-tabs/*)
- [ ] T5 [AC1] Fixtures `gear-outline`, `text-label` (files: tests/fixtures/agent-bench/gear-outline/*,
  tests/fixtures/agent-bench/text-label/*)
- [ ] T6 [AC1] [AC2] [AC4] [AC5] Bench loader, the suite-has-four test, and the replay test per task.
  Replay prints the JSON line and asserts `expected.json`, which is recorded once from this run
  and committed (files: tests/it/agent/bench.rs, tests/it/agent/mod.rs, tests/fixtures/agent-bench/*/expected.json)
- [ ] T7 [AC6] Defect recording (one hole missing) and the test: IoU and passed count are both
  below the clean run (files: tests/fixtures/agent-bench/plate-holes/replies-defect.json,
  tests/fixtures/agent-bench/plate-holes/expected-defect.json, tests/it/agent/bench.rs)
- [ ] T8 [AC7] [AC8] `live_config` + its usage refusal test; `#[ignore] agent_bench_live` writing
  `target/agent-bench/<model>.jsonl` and the replies (files: tests/it/agent/bench.rs)
- [ ] T9 [AC7] [AC8] `scripts/agent-bench.sh` (usage on no arg, then the ignored test)
  (files: scripts/agent-bench.sh)
- [ ] T10 CHANGELOG line (maintainer tooling; user-visible only through the script) and the
  AGENTS.md Commands line (files: CHANGELOG.md, AGENTS.md)
