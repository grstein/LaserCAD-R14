# LCV-156 — Layers with one export file per layer

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

A laser job mixes operations (cut the outline, engrave the text), and LaserGRBL applies one
speed and power per imported file. Today the export preset (`File > Export preset`) is global to
the document, so every entity lands in the same cut/mark/engrave group and a mixed job needs the
drawing split by hand. Direction set by the user on 2026-09-29: LightBurn-style layers.

## Stories

- As an operator, I want to create layers with a name and a color and put each entity on one of
  them so that I can see which parts are cut and which are engraved.
- As an operator, I want export to write one LaserGRBL-ready file per layer so that each file
  gets its own speed and power in LaserGRBL.
- As an operator, I want my saved drawing to keep all its layers so that it reopens as I left it.

## Direction (user decisions 2026-09-29)

- A layer is a name and a color, owned by the document; the color identifies the layer.
- A new document starts with one default layer; the user adds, renames, recolors and deletes
  layers.
- Every entity belongs to exactly one layer and is drawn in its color. New entities go to the
  current layer; moving the selection to another layer is an undoable `Command`.
- Save writes the "mother" SVG with every layer (one `<g>` per layer, carrying name and color)
  and Open reads it back.
- Export writes one file per layer that has entities, named `<mother>-<layer>.svg`, each keeping
  the LaserGRBL rules (header, Y flip, `fill="none"`, arcs as `A`).
- Replaces `File > Export preset` and the fixed cut/mark/engrave colors of the SVG export
  contract in `AGENTS.md`; needs an ADR at /design. Lifts the "no layers" non-goal in
  `docs/product/README.md`.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Speed, power or passes per layer (LaserGRBL owns those).
- Blocks, xref, nested layers.

## Open questions

- Per-layer Output and Show toggles, as in LightBurn?
- Sanitising layer names for file names; export of empty layers.
- Reading v0.2 files: fixed-color groups become layers?
- Which agent tools choose or create a layer?
