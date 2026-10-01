# LCV-200 — Plan

## Approach

There is one library seam. `app::run_turn_inline(app, prompt, config, send_fn)` arms a turn and
runs the private `agent_worker::drive_turn` on the calling thread. Its `ask` is
`answer_act(app, …)`, so every action takes the real apply path, with no channel and no window
(AC2). The turn closes through `turn_end::end_turn`, and the call returns the result, the
LCV-193 `TurnMetrics` and the wall time. Everything else is test code in the one integration
binary (LCV-152):
- `bench.rs` loads the four fixture folders.
- In replay, `send_fn` hands back the recorded `AssistantMessage`s in order.
- In the live run (`#[ignore]`), `send_fn` is `chat_completion` built from a settings file.
- `bench_score.rs` scores the drawing.
The score rasterizes the drawing and the imported reference SVG through
`render/raster.rs::rasterize`: same bed, 2 px/mm, both inked masks dilated by 1 px. It takes the
IoU of the ink and evaluates the JSON assertions; `closed` uses LCV-190 `check_drawing`. The
gate runs the replay tests (AC5, AC6) as ordinary tests.

## Touches

- `src/app/agent_inline.rs` (new) — `run_turn_inline` → `InlineTurn { result, metrics, wall }`;
  `src/app/mod.rs` re-export.
- `src/app/agent_worker.rs::drive_turn` — `pub(crate)`.
- `src/app/agent_poll.rs::answer_act`, `agent_poll/turn_end.rs::end_turn` — visibility to
  `pub(crate)` / `pub(super)` → `pub(in crate::app)` only as needed.
- `tests/fixtures/agent-bench/{plate-holes,box-face-tabs,gear-outline,text-label}/` — each folder
  holds:
  - `prompt.txt`, `reference.svg`, `assertions.json`;
  - `replies.json` (recorded `AssistantMessage` list);
  - `expected.json` (recorded `{iou, assertions: [bool…]}`);
  - plus `plate-holes/replies-defect.json` and `expected-defect.json`.
- `tests/it/agent/bench.rs` (new) — fixture loader, replay test per task (AC2, AC5), suite listing
  (AC1), JSON line (AC4), defect (AC6), `#[ignore] agent_bench_live` (AC7), settings check (AC8).
- `tests/it/agent/bench_score.rs` (new) — `iou(doc, reference)`, assertion kinds
  `circles {r, count, tol}`, `bbox {w, h, tol}`, `closed`, `min_entities {n}`.
- `tests/it/repo/single_test_binary.rs::ac1_tests_holds_only_the_one_binary_and_the_harness` —
  allow a data-only `fixtures/` dir holding no `.rs` file at any depth (the LCV-170 allowance).
- `scripts/agent-bench.sh` (new) — the usage check, then
  `LASERCAD_BENCH_SETTINGS=<file> cargo test --test it agent_bench_live -- --ignored --nocapture`.
- ADRs: none.

## Decisions (self-approved per user goal)

- Settings (AC7, AC8). The live runner reads the settings file as JSON and requires a non-empty
  `agent_model` string; endpoint and key come from the same file. A missing file or model
  refuses with `usage: scripts/agent-bench.sh <settings.json>  (the file must name agent_model)`
  before any `TurnConfig` is built. The script checks `$1` first; the Rust check is what the AC8
  test pins (`bench::live_config(None)`).
- Output. The live run writes `target/agent-bench/<model>.jsonl`, one line per task with keys
  `task, iou, passed, total, steps, applied, refused, repeated, captures, replies, wall_ms`. It
  also writes `<model>-<task>-replies.json`, so a run can be promoted to a fixture. Replay
  writes its lines to stdout only.
- IoU. `expected.json` stores IoU to 6 decimals; replay compares within 1e-6. Wall time is never
  asserted.
- Fixtures. The three geometric references are hand-authored SVGs. The `text-label` reference is
  LaserCAD's own export of the Hershey text (hand-tracing strokes is not practical), so its IoU
  measures placement and height, not the font. Recordings use LCV-196's `create_drawing` items
  (`rect`, `polygon`, `text`), which lands before this in the v0.7 order.

## Risks

- LOC cap: `src/app/agent_inline.rs` ~60 (new). `agent_worker.rs` and `agent_poll.rs` see
  visibility edits only. Test modules under `tests/it/` are exempt.
- Mutation testing: no. The library change is a thin re-wiring of `drive_turn`/`answer_act`; the
  replay tests pin it.
- Merge with the svg branch: LCV-170 adds the same `tests/fixtures/` allowance there. When the
  branches merge, keep **one** copy of the allowance in `single_test_binary.rs` (resolve the
  conflict, do not stack two checks).
- Recorded replies drift when a tool's wire shape changes (e.g. LCV-188 ids in outcomes). Replay
  feeds only the model side, so outcome text never mismatches, but a renamed tool or argument
  fails replay loudly. That is intended: the fixture is updated in the same commit as the tool.
- Determinism: rasterize is pure and the app is `App::default()`, so replay IoU is
  bit-reproducible. AC5 compares within 1e-6 so a float-formatting change cannot flake it.
