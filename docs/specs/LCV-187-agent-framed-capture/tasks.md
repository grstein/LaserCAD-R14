# LCV-187 — Tasks

- [x] T1 [AC1] [AC4] [AC7] Test: no args / `"view"` parse to `View`; bad frame name, missing or non-finite region corner refused; opt-ins off still unadvertised and refused (files: tests/it/agent/canvas_capture.rs)
- [ ] T2 [AC1] [AC4] `CaptureFrame` in the action, `frame`/corner schema and parser (files: src/agent/bridge/action.rs, src/agent/tools.rs)
- [ ] T3 [AC2] [AC3] [AC4] [AC5] Test: drawing frame = bounds + 5 % with longest edge 1024 px; region framed exactly; empty drawing / zero-area region refused; outcome text reports the actual frame; PNG under 2 MiB (files: tests/it/agent/canvas_capture.rs)
- [ ] T4 [AC2] [AC3] [AC4] [AC5] Frame resolution and exact-1024 sizing in `capture` (files: src/app/agent_capture.rs, src/app/agent_apply.rs)
- [ ] T5 [AC6] Test: sent / withheld / not-delivered note per image call id, after the pre-send note; a `Note` is not a step (files: src/agent/loop_/tests.rs, tests/it/agent/canvas_capture.rs)
- [ ] T6 [AC6] `Dispatch::Note` from `send_images`; worker + poll push the row (files: src/agent/loop_.rs, src/app/agent_worker.rs, src/app/agent_poll.rs)
- [ ] T7 Prompt: capture_canvas paragraph names `frame`, `x0`, `y0`, `x1`, `y1` (files: src/agent/prompt.rs, tests/it/agent/default_prompt.rs)
- [ ] T8 Amend ADR 0011 items 1–2 and 10 (files: docs/adr/0011-canvas-observation-is-an-offscreen-raster.md)
- [ ] T9 CHANGELOG line (files: CHANGELOG.md)
