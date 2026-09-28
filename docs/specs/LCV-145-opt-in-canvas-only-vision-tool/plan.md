# LCV-145 — Plan

## Approach

ADR 0011 as written. A kernel-pure `src/render/raster.rs` draws the live document into an
8-bit grayscale buffer and encodes it with `png` (already compiled). On the `Act`,
`src/app/agent_capture.rs` checks both live settings, frames the visible viewport from
`App::camera`, rasterizes, encodes and answers `Observed { text, png }` in the same frame.
There is no pending state. The worker (`loop_.rs`) collects the PNGs of a batch. After the
batch's tool results it appends one `user` message of text+image parts (`wire.rs`,
base64). Before sending an image it asks the UI once through a non-step `AuthorizeUpload`.
After that send, whether it succeeded or failed, it elides every image. Text-only requests
serialize exactly as today, and fixtures captured *before* the type change pin that. No
thread, channel or module boundary is added beyond ADR 0011.

## Touches

- `Cargo.toml` — `png = "0.18"`, `base64 = "0.22"`. `Cargo.lock` gains no `[[package]]`:
  there is one `png 0.18.1` and one `base64 0.22.1` today.
- `src/render/raster.rs` (new), `src/render/mod.rs` — `rasterize(entities, bed_mm,
  world: [f64; 4], w, h) -> Vec<u8>`, `encode_png_gray(&[u8], w, h)`. Liang–Barsky clip
  before the line walk (−10 000 mm costs nothing); arcs split into
  `⌈sweep / (2·acos(1 − 0.25/r_px))⌉` chords. Imports `document`, `geometry`, `png` only.
- `src/app/agent_capture.rs` (new), `src/app/mod.rs` (`mod` line, 292 → ~294 with LCV-144).
  - `capture(app) -> AgentOutcome`: live permission check, zero-area refusal, frame from
    `camera.screen_to_world` at `(0, h)`/`(w, 0)`, `pixel_size(vw, vh)` (≤ 1024, never
    upscaled), `check_size(png, limit)` (2 MiB), pinned outcome text.
  - `authorize(app, endpoint, model) -> AgentOutcome`: `Ok` = yes (and the `note` row),
    `Refused` = no.
- `src/app/agent_apply.rs::apply` — routes `CaptureCanvas` to `agent_capture::capture`,
  which is fenced and counted like every `Act`, then transcribes.
- `src/app/agent_poll.rs::poll_agent_rx` — routes `AuthorizeUpload` to
  `agent_capture::authorize` **before** `apply_fenced`: not a step, bypasses the fence, not
  transcribed.
- `src/agent/bridge.rs` — `AgentAction::{CaptureCanvas, AuthorizeUpload { endpoint, model
  }}` and `AgentOutcome::Observed { text, png }`. `text()`, `into_text()` and
  `is_refused()` cover it.
- `src/agent/tools.rs` — `tool_definitions(vision: bool)`, unless LCV-151 already added the
  flag. `capture_canvas` (no arguments, the empty-properties form) is inserted
  **before** `create_drawing`, so LCV-144 AC 10's "last" holds either way. Adds a
  `"capture_canvas"` parse arm.
- `src/agent/wire.rs` — `Content { Text, Parts }` (untagged), `ContentPart { Text, ImageUrl }`
  (tagged `type`); `ChatMessage.content: Option<Content>`, constructors keep their
  signatures; new `user_parts`, `text_content()`, `has_image()`, `replace_images(&mut
  [ChatMessage], &str)`, and `ContentPart::png(&[u8])` (base64 data URL).
- `src/agent/loop_.rs::agent_loop` — `dispatch_fn` takes `Dispatch::{Tool { name, args },
  AuthorizeUpload}` (one closure owns `ask`). `Observed` PNGs are collected per batch and
  one `user` parts message follows all its tool results. A `send_images` helper wraps every
  send (`last_word`'s too): authorize if `has_image`, withhold on no, elide after the send.
- `src/app/agent_worker.rs` — `TurnConfig.vision: bool` (in `Debug`). `drive_turn` maps
  `Dispatch::AuthorizeUpload` to the `Act` using `config.endpoint`/`model`, never the key.
  Tools come from `tool_definitions(config.vision)`.
- `src/app/agent_turn.rs::turn_config` — `vision = allow && supports` at `start_turn`.
- `src/io/settings.rs` — two `#[serde(default)] bool` fields (LCV-143 already did the
  `settings_store.rs` seam).
- `src/agent/settings_ui.rs` — two checkboxes and the disclosure sentence, placed
  **above** LCV-143's prompt editor so they paint at 800×600 without scrolling.
- `src/agent/prompt.rs`, `tests/it/lcv151_default_prompt.rs` — a `capture_canvas`
  paragraph; LCV-151's coverage loop iterates `[false, true]` (its plan's seam).
- `AGENTS.md` §Purity rule — adds `src/render/raster.rs`, and says `base64` is `wire.rs`
  only.
- `tests/it/lcv145_canvas_capture.rs` (new, `mod` in `tests/it/main.rs`).
- ADRs: 0011 items 1–12; 0007 §D1/§D8 (amendment 7), §D13, §D14. No amendment.

## Risks

- LOC cap: `app/mod.rs` 294 (one `mod` line only). `io/settings.rs` < 270 after LCV-143's
  seam (else T5 performs it). `loop_.rs` 190 → ~250 (−17 when LCV-143 moves the prompt);
  if it passes 270, image bookkeeping moves into `wire.rs` helpers. `wire.rs` ~230,
  `agent_poll.rs` ~240.
- Mutation testing: **yes** — `src/agent/` (`loop_.rs` authorize/elide ordering, `wire.rs`)
  is high risk.
- Existing tests read `content.as_deref()` (`loop_.rs`, `wire.rs`, `agent_worker.rs`). T4
  moves them to `text_content()` without changing any assertion; byte-identity fixtures
  pass *before* the type change.
- The paint test depends on LCV-141/143's scroll layout. Placing the checkboxes above the
  editor keeps them in the first 426pt.
- `App::default()` has a 0×0 viewport; harness tests set `camera.viewport_size_px`.
