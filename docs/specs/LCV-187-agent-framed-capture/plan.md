# LCV-187 — Plan

## Approach

`capture_canvas` gains an optional `frame` (`"view"` | `"drawing"` | `"region"`) plus
`x0, y0, x1, y1` in mm used only by `"region"` (flat, provider-safe schema, ADR 0010 §2).
`tools.rs` parses it into `AgentAction::CaptureCanvas(CaptureFrame)`; no arguments or
`"view"` → `CaptureFrame::View`, today's path unchanged (AC1). `agent_capture::capture`
resolves the frame to a world rectangle: view = camera viewport as today; drawing =
`Document::bounds()` + 5 % margin per side; region = the given rectangle, corners normalised.
A frame with no area or a non-finite value is refused naming the cause (AC4). Drawing/region
render the longest edge at exactly 1024 px, aspect kept (may upscale); the grayscale raster,
2 MiB cap and outcome text are reused with the actual frame (AC2, AC3, AC5).
Post-send notes (AC6): `loop_.rs::send_images` records the call ids whose images it carries;
after `send_fn` returns it dispatches one non-step `Dispatch::Note(text)` per id — `sent`,
`withheld (permission changed)` when the upload verdict was not Ok, or `not delivered (request
failed)` when `send_fn` erred. The worker maps it to `AgentAction::Note`, which `agent_poll`
pushes as a `("note", text)` row after LCV-145's pre-send note. Opt-ins off: unadvertised and
refused as today (AC7).

## Touches

- `src/agent/bridge/action.rs` — `CaptureCanvas(CaptureFrame)`, `enum CaptureFrame`, `Note(String)`.
- `src/agent/tools.rs` — `capture_canvas_definition` gains `frame` + corners; parse arm
  (seam if >270: move both to `src/agent/tools/capture.rs`, kernel-pure, AGENTS.md list).
- `src/agent/loop_.rs::send_images` — image call ids, `Dispatch::Note` after the send.
- `src/app/agent_worker.rs::drive_turn` — map `Dispatch::Note`; `src/app/agent_poll.rs` — push note.
- `src/app/agent_capture.rs::capture` — frame resolution, exact-1024 sizing for drawing/region.
- `src/agent/prompt.rs::DEFAULT_PROMPT` — capture_canvas paragraph names `frame`, `x0..y1`.
- ADRs: ADR 0011 items 1–2 amended (argument form; frame no longer only the viewport; the
  deferred "framing argument" note is resolved) and item 10 (post-send notes).

## Decisions (self-approved per user goal)

- Region corners in any order are normalised; region args given with `view`/`drawing` are refused
  as unknown for that frame (same shape as LCV-185's foreign-key rule: `null` tolerated).
- Drawing extents include every entity on every layer (Output on or off).
- A `Note` is not a step and does not trip the fence; failure notes are sent before the error returns.

## Risks

- LOC cap: `tools.rs` 263 (+ 186's routing) — likely needs the `tools/capture.rs` seam;
  `loop_.rs` after 189 ~256 → watch 270, seam `loop_/images.rs`; `agent_poll.rs` 247 +3.
- Mutation testing: yes — `src/agent/`; margin, exact-1024 rounding and the three note branches.
- Upscaling a tiny region yields a 1024 px PNG of a few strokes — still under 2 MiB (test pins it).
- Cross-spec overlap: `tools.rs`, `bridge/action.rs` (186), `loop_.rs` (189), `prompt.rs` (all).
