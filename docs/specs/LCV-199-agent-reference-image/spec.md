# LCV-199 — Agent reference image

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

Operators often start from a sketch on paper or a photo of a part with its dimensions. Today they
must describe it in words. Image-to-CAD is exactly what current frontier models score highest on
(BenchCAD Vision2Code, <https://benchcad.com>); LaserCAD already sends images to vision models
(ADR 0011) but only its own canvas.

## Stories

- As an operator, I want to attach a sketch or photo to my agent request so that the agent draws
  the part from it, using the dimensions written on it.

## Acceptance criteria

1. WHEN the operator attaches a PNG or JPEG to the agent prompt (agent panel button), THE SYSTEM
   SHALL send it with the next request, once, and show its file name in the transcript.
2. WHILE the model is not marked as supporting vision, THE SYSTEM SHALL disable attachment and
   say why.
3. IF the file is larger than 2 MB after downscaling to 1024 px on the long edge, or not a PNG or
   JPEG, THEN THE SYSTEM SHALL refuse it before sending.
4. WHEN the image is sent, THE SYSTEM SHALL pass the same upload authorization as canvas capture
   (ADR 0011) and elide it from later requests, as canvas captures are.
5. WHEN no image is attached, THE SYSTEM SHALL behave as today.

## Out of scope

- Bitmap tracing or vectorizing (the model interprets; no raster-to-vector code).
- Keeping the image as a canvas underlay.

## Open questions

- Is the file picker allowed outside `src/io/dialogs.rs`? (No: route through it, ADR 0005.)
