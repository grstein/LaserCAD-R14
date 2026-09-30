//! LCV-170 AC 1..4 — the SVG conformance corpus in `tests/fixtures/svg/`.
//!
//! Each `<name>.svg` is paired with a hand-written `<name>.expected` (format:
//! [`expected`]). The expectation is derived from the SVG text by hand, never
//! by running `import_svg`, so the corpus measures import against SVG 2 rather
//! than against LaserCAD's own exporter.

mod expected;
