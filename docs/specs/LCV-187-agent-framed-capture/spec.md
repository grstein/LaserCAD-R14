# LCV-187 — Framed canvas capture

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

`capture_canvas` takes no arguments and rasterizes the operator's current viewport
(`src/app/agent_capture.rs::capture`), at most 1024 px on the long edge. In an agent session
(2026-09-30) the viewport was much larger than the drawing, so contours and details were too small
to judge. The image travels in a later `user` message (`src/agent/loop_.rs::send_images`) and the
only trace is a transcript note written before sending. Neither the operator nor a test can tell a
requested capture from one that was sent, withheld or lost to a failed request. Calling the tool
proves a request, not a useful visual review.

## Stories

- As an operator, I want the agent to capture exactly the drawing or a region it names, at a size
  that shows the details, so that its self-review catches real defects.
- As an operator, I want the transcript to say what happened to each capture so that I can trust
  the agent's claim that it looked.

## Acceptance criteria

1. WHEN `capture_canvas` is called without arguments or with `frame: "view"`, THE SYSTEM SHALL
   behave exactly as today (LCV-145).
2. WHEN it is called with `frame: "drawing"`, THE SYSTEM SHALL frame the extents of all entities
   plus a 5 % margin on each side, and render the longest edge at exactly 1024 px.
3. WHEN it is called with `frame: "region"` and `x0, y0, x1, y1` in mm, THE SYSTEM SHALL frame that
   world rectangle and render the longest edge at exactly 1024 px.
4. IF the frame has no area (empty drawing, degenerate region, non-finite value) THEN THE SYSTEM
   SHALL refuse with a message naming the cause and capture nothing.
5. WHEN any frame is rendered, THE SYSTEM SHALL keep the aspect ratio, the grayscale content, the
   2 MiB cap and the outcome text of LCV-145 AC 5, with the actual frame in mm.
6. WHEN a request carrying images returns, THE SYSTEM SHALL add one transcript `note` per image:
   `Canvas image for call <id> sent.`, `… withheld (permission changed).` or `… not delivered
   (request failed).`, after LCV-145's pre-send note, which stays.
7. WHEN the two opt-ins are off, THE SYSTEM SHALL keep `capture_canvas` unadvertised and refused
   as today.

## Out of scope

- Framing the operator's selection (the agent never selects; selection trips the fence).
- Grid, selection highlight or colour in the raster (ADR 0011).
- Judging visual quality automatically.

## Open questions

- None.
