# ADR 0011 — A canvas observation is an offscreen raster of the live document, never a framebuffer read

- **Status**: Accepted
- **Date**: 2026-09-27
- **Deciders**: architect (LCV-145; in the 1.0 scope by the 2026-09-27 scope
  decision recorded in `PLAN.md`)

## Context

LCV-145 lets an opted-in, vision-capable model ask to *see* the drawing. Its
draft proposes egui 0.29.1's `ViewportCommand::Screenshot` / `Event::Screenshot`
and then spends seven acceptance criteria containing what that route drags in:

- the event is a **full-window framebuffer** — menus, toolbar, transcript, and
  potentially the settings window with the API key field — so isolation
  depends on a crop that must never be wrong, plus a modal/overlay detector;
- the event carries **no request cookie**, so correlating it to a turn, a
  request, a revision and a camera needs proof, a deadline and late-event
  discard: a second multi-frame lifecycle beside ADR 0007 §D11's;
- it exists only under a live eframe backend, so it needs a disarmed native
  adapter (ADR 0005 style) and cannot be exercised by `cargo test`;
- even when perfectly cropped, it shows **UI state** — grid, snap markers,
  cursor, hover, preview, selection tint — not the drawing.

Constraints: ADR 0007 §D1 (worker holds no document state; `Document` stays
`!Clone`), §D8 (only `panel.rs` / `settings_ui.rs` in `src/agent/` import
`egui`; only `transport.rs` imports `reqwest`), §D10 (key never leaves its
lane), AGENTS.md's network-isolation rule for tests.

## Decision

**The image is rendered, not captured.** On the UI thread, inside the `Act`,
the live `Document` is rasterized in software into a small grayscale PNG. No
framebuffer is read, so no chrome, transcript, settings or desktop pixel can be
in it — by construction, not by a crop.

1. **Tool** `capture_canvas`, no arguments, one step (ADR 0007 §D13). Advertised
   only if the turn-start snapshot (`TurnConfig`) says both settings are on;
   `tool_definitions` takes that flag. Executed only if both are **still** on
   live at the apply site; otherwise `Refused` ("canvas capture is disabled in
   Agent settings"). Behind the fence like every action.
2. **Frame** = the world rectangle currently visible in the CAD viewport,
   derived from `App::camera` (its `viewport_size_px` and `screen_to_world`) —
   the operator and the model look at the same region. Pixel size = the
   viewport's size, scaled down uniformly so the longest edge is ≤ 1024;
   never upscaled; aspect preserved. A zero-area viewport is `Refused`.
3. **Content**: 8-bit grayscale; white background; bed outline mid-grey; every
   entity black, 1 px; circles and arcs tessellated with chord error ≤ 0.25 px.
   No grid, snaps, cursor, preview or selection tint (`query_selection` answers
   selection). The tool result states the mapping so the model can read
   coordinates off the picture: `Canvas W×H px of X x0..x1 mm, Y y0..y1 mm
   (Y up; bed outline grey, entities black), revision R.`
4. **Rasterizer**: new file `src/render/raster.rs`, **kernel-pure by
   declaration** (no `egui`/`eframe`/`rfd`; the rest of `render/` may import
   egui, this file may not). A pure function of `(entities, bed, world rect,
   width, height) → Vec<u8>`, plus `encode_png_gray`. AGENTS.md's purity list
   gains this one file when it ships, with an import scan.
5. **PNG**: direct dependency `png = "0.18"`. It is **already compiled** — the
   normal tree has `eframe → image → png 0.18.1` — so `Cargo.lock` must gain no
   new package. The 2 MiB bound holds by construction (1024 × 1024 × 1 byte
   plus filter bytes ≈ 1 MiB raw; line art compresses to tens of KB); an
   explicit > 2 MiB check still refuses.
6. **Orchestration**: `src/app/agent_capture.rs` — live permission check,
   frame, raster, encode, outcome — synchronously, in the frame that received
   the `Act`. There is **no pending capture, no deadline, no late event, no
   native adapter and nothing to arm**: the pixels and the fence check happen
   in one call on one thread at one revision, so correlation is exact by
   construction.
7. **Bridge**: `AgentAction::CaptureCanvas`; `AgentOutcome::Observed { text,
   png: Vec<u8> }`. `text` is the tool result and the transcript row; `png` is
   never transcribed, logged, persisted or put in `agent_chat`. **§D1 is
   clarified, not relaxed**: a fresh rendered observation is output, like
   `QueryEntities`' text — it cannot be indexed, replayed or diffed into a
   document. Nothing under `src/agent/` names a document type.
8. **Wire**: `ChatMessage.content` becomes `Option<Content>`, `#[serde(untagged)]
   enum Content { Text(String), Parts(Vec<ContentPart>) }`, `ContentPart`
   tagged by `type`: `text {text}` | `image_url {image_url: {url}}` with a
   `data:image/png;base64,…` URL (OpenAI chat-completions form, which
   OpenRouter forwards to vision models). Text-only messages serialize
   **byte-identically** to today; `AssistantMessage` is unchanged. Tool results
   stay text: after **all** tool results of a batch, one `user` message is
   appended carrying, per capture, a text part naming the `tool_call_id` and
   the image part. Base64 via `base64 = "0.22"`, already compiled through
   `reqwest` (no new package), used in `wire.rs` only — whose "serde is the
   only import" note becomes "serde and base64".
9. **One-shot retention.** An image rides in exactly one request — the next
   one. After that send returns, the loop replaces each image part in the
   message list with a text placeholder ("canvas image elided after one use;
   call capture_canvas to look again"). Per-request payload is bounded to one
   batch's captures, and no image outlives one round trip in memory.
10. **Upload authorization.** Before a request that carries an image, the
    worker makes one **non-step** rendezvous, `AgentAction::AuthorizeUpload
    { endpoint, model }` (never the key). `agent_poll` routes it to
    `agent_capture`, bypassing the fence and the step counter; the answer is
    yes only if both settings are still on **and** endpoint and model equal the
    live settings. On yes, a `note` row discloses that a canvas image is being
    sent to that model. On no, the image parts become a "withheld" placeholder
    and the request goes out text-only. Cancel fails the rendezvous and nothing
    is sent (§D2). Bytes already handed to `reqwest` cannot be recalled; the
    settings disclosure says so.
11. **Settings**: `agent_allow_canvas_capture: bool` and
    `agent_model_supports_vision: bool`, both `serde` default `false`; two
    checkboxes in `settings_ui.rs` with a disclosure sentence that canvas
    images go to the configured provider and model.
12. **Tests stay network-isolated by construction.** Raster and PNG are pure
    unit tests (decode with `png` in the test; assert size and pixels). A
    headless-harness test proves the image is a function of the document and
    camera only: identical bytes with the agent panel, the settings window and
    a tooltip open or closed. Loop tests use the fake `send_fn` and inspect the
    request messages; wire tests are JSON fixtures. Any transport test uses
    AGENTS.md's unparseable-URL or owned-socket pattern. No test contacts a
    provider; there is no native capture to disarm.

**Size.** Roughly 350–450 implementation LOC across `raster.rs`,
`agent_capture.rs`, `wire.rs`, `loop_.rs`, `bridge.rs`, settings and
`settings_ui.rs`, plus two `Cargo.toml` lines naming crates already compiled.
It fits 1.0. The screenshot route does not, and is **not** this design's
fallback: no framebuffer or desktop path is to be built.

## Consequences

- The model sees the drawing, not the UI; isolation needs no test to discover a
  crop bug, because there is nothing to crop.
- The picture does not show the grid, the selection tint or the preview. That
  is accepted: the model has `query_entities` / `query_selection` for exact
  data; the image is for shape.
- Viewport framing means a model working off-screen sees an empty frame. The
  outcome's mapping text makes that legible. A framing argument
  (`"bed"` / `"drawing"`) is deferred until someone needs it.
- Draft LCV-145 AC 1 and AC 3–8 are replaced, not narrowed; `product-owner`
  rewrites them against this ADR.

## Alternatives considered

- **`ViewportCommand::Screenshot` + crop** — see §Context; too large and
  untestable for 1.0, and it shows UI state.
- **GPU offscreen render of the egui canvas** — needs backend-specific readback
  (wgpu/glow) and an async completion; the same lifecycle problem.
- **Hand-rolled stored-deflate PNG** — zero dependencies, but `png` is already
  compiled, so it buys nothing.
- **Images inside `tool` messages** — rejected by most providers.
- **Keep every image for the whole turn** — payload grows per capture and a
  revoked permission would keep re-uploading old pixels.
- **Recheck permission inside the transport** — the worker cannot see live
  settings (§D1); asking the UI thread is the rendezvous that already exists.
