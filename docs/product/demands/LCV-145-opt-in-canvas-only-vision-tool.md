# LCV-145 - Opt-in canvas observations for vision models

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-123, LCV-129, LCV-137, LCV-141, LCV-143
- **Suggested agent**: architect
- **Suggested model**: opus
- **Implementation**: -

## Problem

A vision-capable model should be able to request a view of the CAD canvas when
explicitly enabled. Current transport content is text-only and the application
has no capture bridge. Chat, settings and desktop pixels must never be sent.

## Scope

Explicit opt-in plus explicit model-vision configuration, canvas-only
observations, a bounded app-owned capture lifecycle and typed multimodal wire
content. Permissions remain independent of editable prompt instructions.

## Out of scope

Desktop/OS capture tools, chat/settings uploads, automatic/periodic capture,
image persistence/logging, provider auto-switching, model-name capability
guessing or worker-owned Document/entity snapshots.

## Acceptance criteria

1. Architect verifies the pinned egui/eframe capture path and records the native boundary before implementation. Prefer `ViewportCommand::Screenshot` and `Event::Screenshot`, already present in egui 0.29.1; their existence does not prove application integration or safe correlation.
2. Advertise and execute the tool only when both Allow canvas capture and configured model supports vision are true. Both default false for old/new settings. Disclose that canvas pixels go to the configured provider/model.
3. A tool call is required for each capture. Crop to the visible CAD viewport before encoding or crossing into the agent worker; no chrome, transcript, settings, tooltips or desktop pixels may appear in the image. Refuse while a modal or ambiguous overlay makes isolation unsafe.
4. Keep any integration-provided full-window framebuffer transient inside the capture adapter, crop immediately and release it. Never serialize, log or retain the uncropped frame for the agent.
5. Pure bridge DTOs carry only request/outcome data. `src/app/agent_capture.rs` owns nonblocking request/completion, with a disarmed-by-default native adapter outside `agent/` and deterministic test fakes. Only real boot arms native capture.
6. Correlate turn, request, rendered revision and viewport/camera geometry. Cancel, foreign changes, mismatches and late completions discard the image. Since egui 0.29 Screenshot events lack a request cookie, the adapter must prove late-event isolation rather than assume the event carries its own ID.
7. Proposed bounds: one pending capture, 5-second deadline, PNG at most 1024 physical pixels on its longest edge and at most 2 MiB before base64. Preserve aspect ratio without upscaling; refuse if bounds cannot be met.
8. Timeout, cancellation, permission revocation and adapter failure release pending work. Keep existing repaint discipline and terminal cleanup; no unconditional polling or new perpetual repaint.
9. Typed text/image message content preserves current text-only JSON representation and omitted fields. Pair every assistant tool-call ID with its result; if an image-bearing user message is needed, append it after all results in that batch.
10. Recheck authorization, configured destination and cancellation immediately before an image-bearing HTTP send. Revocation prevents not-yet-started upload; already-sent bytes cannot be recalled.
11. Images stay in request/turn-owned ephemeral memory, never logs/settings/persisted transcript/disk. A capture is one tool dispatch. Native capture and provider uploads are not performed in automated tests.

## Expected tests

- AC 1: native feasibility and manual isolated-data smoke at the pinned runtime.
- AC 2-4: full capability matrix, explicit-call-only behavior, modal refusal and colored/textual chrome witnesses excluded from encoded canvas output.
- AC 5-6: disarmed default, fake request/completion, late event after timeout/cancel followed by a new turn, revision/viewport/DPI mismatch and no stale delivery.
- AC 7-8: deadline, oversize/high-DPI image, second pending request, failure/cancel/revocation and buffer cleanup.
- AC 9: old text JSON fixtures, image serialization and multi-tool-result pairing/order.
- AC 10: pause fake transport before send, revoke/cancel/change destination, resume and assert no image request begins.
- AC 11: payload absence from persisted/logged content and one-dispatch accounting.

## Open questions

Architect must approve safe readback/crop correlation with the pinned event
shape, transport-time permission/cancel checking, image message pairing and
proposed resource bounds. Amend ADR 0007 D1/D8 to allow explicitly requested
visual observations without permitting worker Document clones. Keep Draft
until these decisions are recorded; no desktop fallback.

## Notes

Relevant seams: `src/agent/wire.rs`, `src/agent/transport.rs`,
`src/agent/bridge.rs`, `src/app/agent_poll.rs`, `src/app/agent_capture.rs`,
`src/app/viewport.rs` and the native adapter. Use small codec dependencies only
if the verified path needs them; do not add another GUI/runtime.

Pinned API references:
[ViewportCommand::Screenshot](https://docs.rs/egui/0.29.1/egui/viewport/enum.ViewportCommand.html#variant.Screenshot)
and [Event::Screenshot](https://docs.rs/egui/0.29.1/egui/enum.Event.html#variant.Screenshot).

## Architecture decision

Recorded 2026-09-27 in
[ADR 0011](../../adr/0011-canvas-observation-is-an-offscreen-raster.md): the
image is a software raster of the live document rendered on the UI thread
inside the `Act`, **not** a `ViewportCommand::Screenshot` framebuffer read.
That replaces draft AC 1 and AC 3–8 (no crop, modal check, pending capture,
deadline, late-event correlation or native adapter exist to specify) and
answers AC 9–11 in ADR 0011 §Decision 8–10. ADR 0007 §D1/§D8 are clarified
there and in amendment (7). `product-owner` rewrites the ACs against ADR 0011.
