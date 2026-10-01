# LCV-199 — Agent reference image

- **Status**: Done
- **Depends on**: none
- **Implementation**: 545b6ba..a9a7961

## Problem

Operators often start from a sketch on paper or a photo of a part with its dimensions. Today they
must describe it in words. Image-to-CAD is what current frontier models score highest on
(BenchCAD Vision2Code, <https://benchcad.com>); LaserCAD already sends images to vision models
(ADR 0011) but only its own canvas.

## Stories

- As an operator, I want to attach a sketch or photo to my agent request so that the agent draws
  the part from it, using the dimensions written on it.

## Acceptance criteria

1. WHEN the operator presses the agent panel's `Attach image…` button, THE SYSTEM SHALL open a
   PNG/JPEG file picker through `src/io/dialogs.rs` (ADR 0005) and show the chosen file name as a
   removable chip above the prompt.
2. WHEN the operator sends the prompt with an attached image, THE SYSTEM SHALL put the image in
   that user message (`image/png` or `image/jpeg`), add the transcript row
   `Image: <file name>`, and clear the attachment.
3. WHILE `Model supports images` is off, THE SYSTEM SHALL disable the button with the tooltip
   `Turn on "Model supports images" in agent settings.`
4. IF the file is not a PNG or JPEG by its first bytes, or is larger than 2 MB, THEN THE SYSTEM
   SHALL refuse it with a status message naming the reason and attach nothing.
5. IF the file cannot be read when the prompt is sent, THEN THE SYSTEM SHALL not send the request
   and SHALL say so in the transcript, keeping the prompt text.
6. WHEN a later request of the conversation is built, THE SYSTEM SHALL replace the attached image
   with the text `image elided`, as canvas captures are (ADR 0011, §D16 memory).
7. WHEN no image is attached, THE SYSTEM SHALL send byte-identical requests to today's.

## Out of scope

- Bitmap tracing or vectorizing (the model interprets; no raster-to-vector code).
- Keeping the image as a canvas underlay; drag-and-drop; more than one image per prompt.
- Decoding, resizing or re-encoding the image.

## Decisions (self-approved per user goal, 2026-09-30)

- The operator's explicit attach is the upload consent; `Allow canvas capture` governs automatic
  captures only and is not required here. `Model supports images` is.
- No downscaling: it would need a JPEG decoder (new dependency) for little gain. The file is sent
  as is; 2 MB raw is the cap; the operator resizes larger photos.
- The picker goes through `src/io/dialogs.rs` (disarmed in tests); tests attach through the same
  seam the dialog result feeds. `panel.rs` only raises the request.

## Open questions

- None.
