# LCV-187 — Framed canvas capture

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

`capture_canvas` takes no arguments and rasterizes the operator's current viewport
(`src/app/agent_capture.rs::capture`), at most 1024 px on the long edge. In an agent session
(2026-09-30) the viewport was much larger than the drawing, so contours and details were too small
to judge. The image travels as a later `user` message (`src/agent/loop_.rs::send_images`) and the
only trace is a transcript note, so neither the operator nor a test can tell a requested capture
from one actually sent, withheld or refused by the provider. Calling the tool proves a request, not
a useful visual review.

## Stories

- As an operator, I want the agent to capture exactly the drawing, the selection or a region it
  names, at a resolution that shows the details, so that its self-review catches real defects.
- As an operator, I want the transcript to say whether each capture was sent, withheld or failed
  so that I can trust the agent's claim that it looked.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Grid, selection highlight or colour in the raster (ADR 0011 keeps it grayscale geometry).
- Judging visual quality automatically.

## Open questions

- Framing modes: drawing extents, selection, world rectangle — all three or fewer?
- Maximum edge above 1024 px, within the 2 MiB PNG cap?
- How is the model told that its configured model cannot take images (refusal vs text-only note)?
