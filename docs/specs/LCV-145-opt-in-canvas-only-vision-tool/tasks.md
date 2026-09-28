# LCV-145 — Tasks

- [x] T1 [P] [AC7] Add `png = "0.18"` and `base64 = "0.22"` to `[dependencies]`, then `cargo build`. Check that the `Cargo.lock` diff adds no `[[package]]`. (files: Cargo.toml, Cargo.lock)
- [x] T2 [AC1, AC5] Test first: create a `src/render/raster.rs` stub with failing unit tests, decoding with `png`:
  - A corner-to-corner line is black on the diagonal and white elsewhere.
  - Pixel values are 255 / 128 / 0 at known points.
  - Every rasterized circle pixel lies within 0.25 + 0.5 px of the true circle.
  - An entity at (−10000, −10000) is clipped cheaply.
  
  Register it in `render/mod.rs` and add the file to AGENTS.md's purity list. (files: src/render/raster.rs, src/render/mod.rs, AGENTS.md)
- [x] T3 [AC1, AC5] Implement `rasterize` (clip, line walk, chord tessellation) and `encode_png_gray`. T2 goes green. (files: src/render/raster.rs)
- [ ] T4 [AC9] Test first: in `wire.rs`, pin today's text-only system / user / assistant-with-tool-calls / tool JSON as literal fixtures, and they pass. Then add a failing `Parts` → pinned OpenAI shape test. Then add `Content`, `ContentPart`, the base64 data URL, `user_parts`, `text_content`, `has_image` and `replace_images`, and move existing test accesses to `text_content()`. (files: src/agent/wire.rs, src/agent/loop_.rs, src/app/agent_worker.rs)
- [ ] T5 [P] [AC2] Test first: an old settings file loads both new flags as `false`. Add `agent_allow_canvas_capture` and `agent_model_supports_vision`. (LCV-143 already moved persistence into `settings_store.rs`.) (files: src/io/settings.rs)
- [ ] T6 [AC2, AC3, AC8] Test first in `tools.rs`: `tool_definitions(false)` omits `capture_canvas`, `(true)` includes it before `create_drawing`, and `capture_canvas` parses to `CaptureCanvas`. Then add the bridge variants `CaptureCanvas`, `AuthorizeUpload` and `Observed`, with `text`/`into_text`/`is_refused` covering them, the flag, the registration and the arm. (files: src/agent/tools.rs, src/agent/bridge.rs)
- [ ] T7 [AC2] Test first: the `turn_config` snapshot's `vision` is true only for (on, on), and flipping a setting afterwards leaves it unchanged. Then add `TurnConfig.vision` and `tool_definitions(config.vision)`. (files: src/app/agent_turn.rs, src/app/agent_worker.rs)
- [ ] T8 [AC3, AC9, AC10, AC11] Test first in `loop_.rs`, with a fake `send_fn` and dispatch:
  - No `capture_canvas` call means no `image_url` is sent. Each capture counts as one step.
  - A batch [query, capture, capture] gives three `tool` results, then one `user` message with two text+image pairs naming the right IDs.
  - The next request carries the elided placeholder and no `image_url`, including after a send error.
  - Authorize is asked exactly once before an image send and never for text-only sends, and its fields carry no key. A "no" answer gives the withheld placeholder and a text-only request. Cancel means `send_fn` is not called again.
  
  Then implement `Dispatch`, batch capture collection, `send_images` and the `drive_turn` mapping. (files: src/agent/loop_.rs, src/app/agent_worker.rs)
- [ ] T9 [AC2, AC4, AC5, AC6] Test first in `agent_capture` unit tests:
  - `pixel_size`: 1600×900 → 1024×576, and 800×600 → 800×600.
  - A 0×600 viewport gets the pinned refusal.
  - `check_size` with an injected limit.
  - The outcome string is pinned character for character, and a live setting off gives the pinned refusal.
  - `authorize` answers yes only when both flags are on and endpoint and model match, and a yes writes the `note` row.
  
  Then implement `capture` and `authorize`. (files: src/app/agent_capture.rs, src/app/mod.rs)
- [ ] T10 [AC2, AC6, AC11] Integration tests (network-isolated):
  - Of the four live setting combinations, only (on, on) gives `Observed`; the other three get the pinned `Refused`, and a fenced capture is `Fenced`.
  - The `Act` is answered within the same `update_ui`.
  - The PNG bytes are identical with the agent panel, the settings window or a tooltip open vs. closed, and with a selection or hover preview present vs. absent.
  - `AuthorizeUpload` adds no step, is answered after the fence tripped, and answers "no" after `agent_model` changes.
  
  (files: tests/it/lcv145_canvas_capture.rs, tests/it/main.rs)
- [ ] T11 [AC2, AC6, AC11] Route `CaptureCanvas` in `agent_apply::apply` (fenced, counted, transcribed), and route `AuthorizeUpload` in `poll_agent_rx` before `apply_fenced` (no step, no fence). T10 goes green. (files: src/app/agent_apply.rs, src/app/agent_poll.rs)
- [ ] T12 [AC2] Harness paint test at 800×600: both checkbox labels and the exact disclosure sentence are painted in Agent Settings. Then add the checkboxes and sentence above the prompt editor. (files: src/agent/settings_ui.rs, tests/it/lcv145_canvas_capture.rs)
- [ ] T13 [AC1, AC7, AC8, AC12] Scans, each with a positive control:
  - `raster.rs` imports no `egui`/`eframe`/`rfd`.
  - `src/` has no `ViewportCommand::Screenshot` or `Event::Screenshot`.
  - `base64` appears in no `src/` file except `wire.rs`.
  - After a capture, no `agent.chat` row contains `iVBOR` or `data:image`.
  - No logging call carries `png`.
  
  Every transport test added uses the unparseable or owned-socket pattern. (files: tests/it/lcv145_canvas_capture.rs)
- [ ] T14 [AC2] Name `capture_canvas` in the built-in system prompt so LCV-151's tool-enumeration test (over `tool_definitions(true)`) stays green. Change LCV-151's coverage loop to iterate `[false, true]` and update the pinned-text test. (files: src/agent/prompt.rs, tests/it/lcv151_default_prompt.rs)
- [ ] T15 CHANGELOG line: opt-in canvas pictures for vision models. Two Agent Settings checkboxes, off by default; the drawing only, never the window. (files: CHANGELOG.md)
