# LCV-145 - Opt-in canvas observations for vision models

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-123, LCV-129, LCV-137, LCV-141, LCV-143
- **Suggested agent**: architect
- **Suggested model**: opus
- **Implementation**: -

## Problem

The agent works blind: it reads the drawing only as a list of numbers from
`query_entities`, so it cannot notice that a layout looks wrong — parts
overlapping, a slot on the wrong side, a shape that is not what the operator
described — before the job goes to the laser. A vision-capable model, when the
operator explicitly allows it, should be able to ask for a picture of the
drawing. Nothing else on screen — chat, settings, the API key, the desktop —
may ever be sent, and nothing may be sent without both opt-ins.

## Scope

Per ADR 0011: an explicitly requested `capture_canvas` tool that renders the
live document (bed outline and entities only) in software into a small
grayscale PNG on the UI thread, two opt-in settings, typed multimodal wire
content, one-shot image retention, and a pre-upload permission re-check.
Permissions are enforced in code, independently of the (editable) prompt.

## Out of scope

- Any framebuffer, screenshot, GPU readback or desktop capture —
  `ViewportCommand::Screenshot` / `Event::Screenshot` included. No fallback to
  them is to be built.
- Showing grid, snap markers, cursor, hover, preview or selection tint in the
  image (`query_selection` answers selection).
- A framing argument (`"bed"` / `"drawing"`); the frame is the visible
  viewport.
- Automatic or periodic capture; images inside `tool` messages; image
  persistence, logging or transcript display; provider auto-switching;
  guessing vision support from the model name; worker-held document state.

## Acceptance criteria

1. **Rendered, not captured.** The image is an offscreen software raster of
   the live `Document` only, produced by a new kernel-pure
   `src/render/raster.rs` (no `egui`/`eframe`/`rfd` import; the rest of
   `render/` may) exposing a pure function of `(entities, bed, world rect,
   width, height) → Vec<u8>` plus `encode_png_gray`. AGENTS.md's purity list
   gains this file. No code path reads a framebuffer: `src/` contains no
   `ViewportCommand::Screenshot` and no `Event::Screenshot`.
2. **Two opt-ins, off by default.** `Settings` gains
   `agent_allow_canvas_capture: bool` and `agent_model_supports_vision: bool`,
   both `serde` default `false` (old files load `false`). Agent Settings shows
   two checkboxes, `Allow canvas capture` and `Model supports images`, and
   beneath them exactly: `When both are on, the agent may send a picture of
   the drawing (not the window) to the configured provider and model.`
   `capture_canvas` is advertised only when the turn-start `TurnConfig`
   vision flag is true (both on at `start_turn`), and executed only when both
   are **still** on live at the apply site; otherwise it is `Refused` with
   `canvas capture is disabled in Agent settings`. It passes the fence like
   every action.
3. **Explicit, counted.** Each image requires one `capture_canvas` call (no
   arguments); it is one step (ADR 0007 §D13). No image is ever produced or
   sent without that call.
4. **Frame and size.** The frame is the world rectangle visible in the CAD
   viewport, derived from `App::camera` (`viewport_size_px`,
   `screen_to_world`). Pixel size is the viewport size scaled down uniformly
   so the longest edge is ≤ 1024 px, never upscaled, aspect preserved to
   within one pixel. A zero-area viewport is `Refused` with
   `the canvas viewport has no area; nothing to capture`. A PNG over 2 MiB
   is `Refused` with `canvas image exceeds 2 MiB`.
5. **Content.** 8-bit grayscale; background 255; bed outline 128; every
   entity 0, 1 px wide; circles and arcs tessellated with chord error
   ≤ 0.25 px. Nothing else is drawn. The tool result and the `tool`
   transcript row are exactly
   `Canvas {w}×{h} px of X {x0}..{x1} mm, Y {y0}..{y1} mm (Y up; bed outline grey, entities black), revision {r}.`
   with millimetres to 3 decimals.
6. **Synchronous orchestration.** `src/app/agent_capture.rs` performs the
   live permission check, framing, raster, encode and outcome in the frame
   that received the `Act`. There is no pending capture, deadline, late-event
   path, native adapter or arming step.
7. **Dependencies.** `png = "0.18"` and `base64 = "0.22"` become direct
   dependencies; `Cargo.lock` gains **no new `[[package]]`** (both are already
   compiled via `eframe → image → png` and `reqwest`). `base64` is used only
   in `src/agent/wire.rs`.
8. **Bridge and privacy.** `AgentAction::CaptureCanvas`;
   `AgentOutcome::Observed { text, png: Vec<u8> }`. Only `text` is the tool
   result and the transcript row; the PNG bytes and their base64 are never
   transcribed, logged, written to `agent_chat`, settings, autosave or any
   file. Nothing under `src/agent/` names a document type.
9. **Wire (ADR 0011 item 8).** `ChatMessage.content` becomes
   `Option<Content>`, `#[serde(untagged)] enum Content { Text(String),
   Parts(Vec<ContentPart>) }`, `ContentPart` tagged `text {text}` |
   `image_url {image_url: {url}}` with a `data:image/png;base64,…` URL.
   **Every text-only request serializes byte-identically to today.** Tool
   results stay text. After **all** tool results of a batch, one `user`
   message is appended carrying, per capture in call order, a text part
   `canvas image for tool call {tool_call_id}` followed by its image part.
10. **One-shot retention (item 9).** An image rides in exactly the next
    request. After that send returns — success or error — each image part in
    the message list is replaced by the text part
    `canvas image elided after one use; call capture_canvas to look again`.
11. **Upload re-check (item 10).** Before any request carrying an image the
    worker makes one `AgentAction::AuthorizeUpload { endpoint, model }`
    rendezvous (never the key). It is **not a step**: it bypasses the fence
    and the step counter and does not appear in progress. The UI answers yes
    only if both settings are still on and `endpoint`/`model` equal the live
    settings. Yes → a `note` row `Sending a canvas image to {model}.` and the
    request goes out. No → every image part becomes
    `canvas image withheld: capture permission changed during the turn` and
    the request goes out text-only. A cancelled turn fails the rendezvous and
    sends nothing.
12. **Network-isolated tests.** No test contacts a provider; transport tests,
    if any, use AGENTS.md's unparseable-URL or owned-socket pattern.

## Expected tests

- AC 1: `raster.rs` unit — a line from bed corner to corner produces black
  pixels on the expected diagonal and white elsewhere (decode with `png`);
  purity scan (non-vacuous, positive control) that `raster.rs` imports no
  `egui`/`eframe`/`rfd`; scan that `src/` contains no `Screenshot` variant use.
- AC 2: `settings.rs` unit — old file → both `false`. `tools.rs` unit —
  `tool_definitions(false)` omits and `(true)` includes `capture_canvas`.
  Integration — with each of the four live setting combinations, a
  `CaptureCanvas` `Act` is `Observed` only for (on, on), `Refused` with the
  pinned text otherwise; a fenced one is `Fenced`. Harness paint test — both
  checkbox labels and the disclosure sentence are painted in Agent Settings
  at 800×600 (within LCV-141's scroll area).
- AC 3: `loop_.rs` unit with fake `send_fn` — a turn with no
  `capture_canvas` call sends no `image_url` part; each capture increments the
  step count by one.
- AC 4: `agent_capture` unit — viewport 1600×900 → 1024×576; 800×600 →
  800×600 (no upscale); a 0×600 viewport → pinned refusal; the >2 MiB check
  exercised through a helper with an injected limit.
- AC 5: `raster.rs` unit — pixel values 255/128/0 at known points; a circle's
  rasterized pixels all lie within 0.25 px + ½ px of the true circle; the
  outcome string pinned character for character.
- AC 6: harness integration — the PNG bytes are identical with the agent
  panel, the settings window and a tooltip open vs. closed, and with a
  selection / hover preview present vs. absent (breaks if any UI state leaks
  into the raster); the `Act` is answered within the same `update_ui` call.
- AC 7: review — `Cargo.lock` diff adds no `[[package]]`; scan that
  `base64` appears in no `src/` file but `wire.rs`.
- AC 8: integration — after a capture, no `agent_chat` row contains `iVBOR`
  (PNG base64 prefix) or `data:image`; source scan for logging calls carrying
  `png`.
- AC 9: `wire.rs` unit — JSON fixtures of today's text-only system/user/
  assistant/tool messages serialize byte-identically; a `Parts` message
  serializes to the pinned OpenAI shape. `loop_.rs` unit — a batch of
  [query, capture, capture] produces three `tool` results in order followed by
  one `user` message with two text+image pairs naming the right IDs.
- AC 10: `loop_.rs` unit — the request after the image-bearing one contains
  the placeholder and no `image_url`; also after a send error.
- AC 11: `loop_.rs` unit with fake `send_fn`/`ask` — `AuthorizeUpload` is
  asked exactly once before the image-bearing send and never for text-only
  sends, its fields contain no key; a "no" answer yields a text-only request
  with the withheld placeholder. Integration — `AuthorizeUpload` does not
  change the step count and is answered even after the fence tripped;
  changing `agent_model` live after capture → "no"; a yes writes the pinned
  `note` row. Cancel before answering → `send_fn` never called again.
- AC 12: review of every test added above; any transport-level test uses the
  unparseable-endpoint or owned-socket pattern.
- Manual smoke: with a vision model on OpenRouter and both boxes ticked, ask
  "look at the drawing and describe it"; confirm the note row, a sensible
  description, and that no image appears in the transcript.

## Open questions

- **Pending user confirmation before implementation:** is a drawing-only
  raster (bed outline and entities; no grid, selection, preview or UI) an
  acceptable canvas observation for 1.0? ADR 0011 is written on that basis.
  If the user says no, this demand returns to `architect`; the screenshot
  route is not a fallback.

## Notes

Primary files: new `src/render/raster.rs`, new `src/app/agent_capture.rs`,
`src/agent/wire.rs`, `src/agent/loop_.rs`, `src/agent/bridge.rs`,
`src/agent/tools.rs`, `src/app/agent_poll.rs`, `src/app/agent_worker.rs`
(`TurnConfig` vision flag), `src/io/settings.rs`, `src/agent/settings_ui.rs`,
`Cargo.toml`. Implemented after LCV-142 (`TurnConfig`, step counting) and
LCV-143. If the two settings fields take `src/io/settings.rs` above 270
implementation LOC, perform ADR 0007 §D8's `settings_store.rs` seam unless an
earlier demand already did.

## Architecture decision

Recorded 2026-09-27 in
[ADR 0011](../../adr/0011-canvas-observation-is-an-offscreen-raster.md): the
image is a software raster of the live document rendered on the UI thread
inside the `Act`, **not** a `ViewportCommand::Screenshot` framebuffer read.
Draft AC 1 and AC 3–8 were replaced accordingly; draft AC 9–11 map to ADR
0011 items 8–10 (AC 9–11 above). ADR 0007 §D1/§D8 are clarified there and in
amendment (7).
