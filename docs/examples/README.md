# LaserCAD example SVGs

Two checked-in SVG fixtures, meant to be opened by a human in **LaserGRBL**
(or Inkscape, or any SVG viewer) to eyeball what this app exports. Both are
produced by `src/io/svg/export.rs::export_svg` — they are not hand-authored
XML — and both are covered by an import round-trip test at
`tests/docs_examples_svg_roundtrip.rs`.

| file | bed size | what it holds |
|---|---|---|
| `bed-128mm.svg` | 128 mm × 128 mm | line, circle, arc, text |
| `bed-400mm.svg` | 400 mm × 400 mm (this app's default bed) | line, circle, arc, text |

## What you should see

Both files hold the same four kinds of geometry, laid out one per quadrant
so nothing overlaps, all in the red **cut** group:

- **bottom-left** — a straight line.
- **bottom-right** — a circle.
- **top-left** — a quarter-circle arc.
- **top-right** — the fixture's own bed size, spelled out as text
  (`"128"` or `"400"`), so you can tell the two files apart at a glance.

Everything should sit right-side-up and inside the bed rectangle when you
open the file: nothing mirrored, nothing off-canvas, nothing outside the red
`cut` color group. If LaserGRBL (or your viewer) shows the geometry upside
down relative to this description, or outside the bed rectangle, that is a
real defect in the export/import pipeline — report it, don't edit the
fixture.

## Why two bed sizes

LaserCAD's world is **Y-up**; SVG is **Y-down**; every Y coordinate this app
writes or reads is mirrored around the *document's own* bed height
(`y_svg = bed_height_mm - y_world`), never a constant (see AGENTS.md's SVG
export rules). A test that only ever used the 400 mm bed — this app's
default — could not tell that mirror apart from a bug that hard-codes
400 mm: at the default bed both are the same number. The 128 mm fixture is
the one that catches that bug, because 128 ≠ 400. See the module doc on
`tests/docs_examples_svg_roundtrip.rs` for the exact assertions and the
tolerance they use.

## What "round-tripping text" means here

There is no `<text>` element in either file, and there never will be:
`export_svg` refuses to emit one and `import_svg` does not recognise one.
`src/text/layout.rs::layout_text` turns a string into plain `Entity::Line`
Hershey strokes *before* export ever runs, so the "text" in these fixtures
is, by the time it reaches the SVG, a few dozen ordinary `<line>` elements —
indistinguishable from a line an operator drew by hand. Round-tripping text
therefore means the *stroke geometry* survives import intact, not that the
string `"128"` or `"400"` is recovered from the file (nothing in this crate
does OCR). If you open one of these files and cannot read the number in the
top-right quadrant, that is a Hershey-layout defect, not an SVG defect.
