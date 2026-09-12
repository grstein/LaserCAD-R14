# LCV-057 — SVG import (roxmltree, strict subset)

- **Status**: Done
- **Implementation**: d5b2edb — feat(LCV-057): SVG import — parse lines, circles, arcs from LaserGRBL SVG files
- **Phase**: 5
- **Depends on**: LCV-056
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

A laser-cutter operator exports a job from LaserCAD, makes a cut, then needs to reopen the SVG file to add or edit geometry. Without an import path, the SVG file is write-only: the operator must redraw from scratch instead of resuming from the saved state. The importer must be a **pure function** — it takes an SVG string and returns a `Vec<Entity>` — so it is reusable by the file-open dialog (LCV-062), autosave restore (LCV-059), and future CLI callers without any UI coupling. The supported input is the strict subset that LCV-056's exporter emits, with tolerant whitespace; nothing more is promised or tested.

## Scope

- New file `src/io/svg/import.rs` declaring:
  - `pub fn import_svg(src: &str) -> Result<Vec<Entity>, SvgImportError>` — the only public surface; a pure function with no file I/O and no side effects.
  - `pub enum SvgImportError` — see Acceptance criteria AC 3.
- `roxmltree = "0.20"` added to `[dependencies]` in `Cargo.toml`. (Implementer: verify latest compatible version on crates.io before committing; the chosen version must be semver-compatible with `"0.20"`.)
- Recognised elements (all coordinates treated as bare millimetre values — no unit-suffix parsing required, since the exporter emits no unit suffix on coordinates):
  - `<line x1 y1 x2 y2/>` → `Entity::Line(Line::new(Vec2::new(x1, y1), Vec2::new(x2, y2)))`.
  - `<circle cx cy r/>` → `Entity::Circle(Circle::new(Vec2::new(cx, cy), r))`. Return `Err(MalformedAttribute{…})` if `r ≤ 0`.
  - `<path d="M sx sy A r r 0 large sweep ex ey …"/>` → `Entity::Arc` reconstructed from start/end points, radius, and flags (algorithm in Notes). Return `Err(MalformedPath(…))` if the `d` value starts with `M … A …` but a numeric token cannot be parsed. Silently skip any `<path>` whose `d` does not match the `M … A …` shape (e.g., a Bézier path from a foreign tool).
- Traversal: walk the entire XML tree depth-first. Entities are collected from every depth (not only direct `<svg>` children), which handles the three-`<g>`-group structure that LCV-056 emits. Order of traversal is document order.
- Unknown elements (anything that is not `<line>`, `<circle>`, or `<path>`) are silently skipped; no error is returned.
- Unknown attributes on recognised elements are silently ignored.
- The function must not panic on any valid or invalid `&str` input.
- `src/io/svg/mod.rs` updated:
  - Add `pub mod import;`.
  - Add `pub use import::{import_svg, SvgImportError};`.
- `src/io/mod.rs` updated:
  - Add `pub use svg::{import_svg, SvgImportError};` alongside the existing `svg` module declaration.
- Unit tests in `#[cfg(test)] mod tests` inside `import.rs` — no new file under `tests/`.
- `src/io/svg/import.rs` ≤ 300 LOC.
- Kernel-purity: `src/io/svg/import.rs` MUST NOT import `egui`, `eframe`, or `rfd`.
- No `unwrap()` or `expect()` in the library function body.
- Doc comments (`///`) on the public function and `SvgImportError`; module header (`//!`) on the file.

## Out of scope

- File I/O — the caller (LCV-062 or a test) does `std::fs::read_to_string`.
- Y-axis transformation / mirroring — the exporter emits coordinates verbatim in world space; the importer reads them verbatim. No coordinate transformation is applied.
- Layer or preset-colour round-trip — the returned `Vec<Entity>` carries no colour/layer metadata; all entities are returned in a flat list regardless of which `<g>` group contained them.
- Bézier paths, elliptic arcs, `<polyline>`, `<polygon>`, `<rect>`, `<text>`, `<use>` — silently skipped.
- SVG `transform` attributes — silently ignored; the strict-subset exporter emits no transforms.
- `width` / `height` / `viewBox` parsing — out of scope; no `Document` canvas size is reconstructed. The caller receives only entities.
- Hershey/text entities — `Entity` has no `Text` variant yet (LCV-048). `<text>` and `<path>` elements produced by Hershey text (which are ordinary line/arc entities) are imported via the standard `<line>`/`<path>` rules.
- Streaming or large-file handling — `roxmltree` builds a full in-memory tree; file size is the caller's concern.
- Error recovery / partial results on malformed input — the function returns the first error encountered; no partial `Vec<Entity>` is returned alongside an error.
- Writing back to SVG — owned by LCV-056.

## Acceptance criteria

1. `src/io/svg/import.rs` exists and declares `pub fn import_svg(src: &str) -> Result<Vec<Entity>, SvgImportError>` with no file I/O and no `egui`/`eframe`/`rfd` imports; `grep -nE '^use (egui|eframe|rfd)' src/io/svg/import.rs` returns no matches.

2. `Cargo.toml` contains a `roxmltree` dependency entry under `[dependencies]`.

3. `SvgImportError` is defined in `import.rs` with `#[derive(Debug, thiserror::Error)]` and **at least** these variants:
   - `XmlParse(#[from] roxmltree::Error)` — wraps a `roxmltree` parse failure; display message includes the underlying error.
   - `NoSvgRoot` — the document contains no `<svg>` root element.
   - `MalformedAttribute { element: &'static str, attr: &'static str, value: String }` — a required numeric attribute on a recognised element could not be parsed; display message names the element, attribute, and offending value.
   - `MalformedPath(String)` — a `<path>` element whose `d` attribute began with `M … A …` but contained a non-numeric token; display message includes the raw `d` value.

4. A valid but empty SVG string (`"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"`) returns `Ok(vec![])`.

5. Invalid XML (e.g., `"<svg><unclosed"`) returns `Err(SvgImportError::XmlParse(_))`.

6. A well-formed XML document with no `<svg>` root element (e.g., `"<root/>"`) returns `Err(SvgImportError::NoSvgRoot)`.

7. An SVG containing `<line x1="1.0000" y1="2.0000" x2="11.0000" y2="7.0000"/>` returns `Ok` with one `Entity::Line` whose `p1 = Vec2::new(1.0, 2.0)` and `p2 = Vec2::new(11.0, 7.0)` (each component within `EPSILON` of the expected value).

8. An SVG containing `<circle cx="5.0000" cy="5.0000" r="3.0000"/>` returns `Ok` with one `Entity::Circle` whose `center = Vec2::new(5.0, 5.0)` and `r = 3.0` (within `EPSILON`).

9. An SVG containing `<path d="M 10.0000 0.0000 A 10.0000 10.0000 0 0 1 0.0000 10.0000"/>` (CCW quarter-arc of radius 10, exported representation of `Arc::new(Vec2::new(0.0,0.0), 10.0, 0.0, FRAC_PI_2, true)`) returns `Ok` with one `Entity::Arc` satisfying all of the following (each within `EPSILON`):
   - `center = Vec2::new(0.0, 0.0)`.
   - `r = 10.0`.
   - `start_angle = 0.0` (i.e., `atan2(0.0 − 0.0, 10.0 − 0.0)`).
   - `end_angle = FRAC_PI_2`.
   - `ccw = true` (sweep-flag was 1).

10. A large-arc CCW half-circle `<path d="M 10.0000 0.0000 A 10.0000 10.0000 0 1 1 -10.0000 0.0000"/>` (large=1, sweep=1, exported from `Arc::new(Vec2::new(0.0,0.0), 10.0, 0.0, PI, true)`) returns `Ok` with one `Entity::Arc` whose `center = Vec2::new(0.0, 0.0)`, `r = 10.0`, `start_angle ≈ 0.0`, `end_angle ≈ PI`, `ccw = true`.

11. A CW arc `<path d="M 10.0000 0.0000 A 10.0000 10.0000 0 0 0 0.0000 -10.0000"/>` (sweep=0, large=0, a CW quarter-arc) returns `Ok` with one `Entity::Arc` with `ccw = false`.

12. An SVG containing a `<circle>` with `r="-1.0000"` returns `Err(SvgImportError::MalformedAttribute { element: "circle", attr: "r", … })` (i.e., the `r ≤ 0` guard fires).

13. An SVG containing `<line x1="abc" y1="0" x2="0" y2="0"/>` returns `Err(SvgImportError::MalformedAttribute { element: "line", attr: "x1", value: "abc" })`.

14. A `<path>` with `d="M 0 0 A notanumber 10 0 0 1 5 5"` returns `Err(SvgImportError::MalformedPath(_))`.

15. A `<path>` with `d="M 0 0 L 10 10"` (Bézier/line-to path, not an arc) is silently skipped and the result is `Ok(vec![])`.

16. An SVG with mixed elements — one `<line>`, one unknown `<rect>`, and one `<circle>` — returns `Ok` with exactly two entities, in document order (line before circle if that is the document order), and no error.

17. Entities nested inside `<g>` children are collected: an SVG identical to the three-group structure produced by `export_svg` (two empty `<g>` groups + one `<g>` containing a `<line>`) returns `Ok` with exactly one `Entity::Line`.

18. Round-trip: for a `Document` containing one `Entity::Line`, one `Entity::Circle`, and one `Entity::Arc` (use the quarter-arc from AC 9), calling `import_svg(&export_svg(&doc))` returns `Ok` with three entities whose field values each match the originals within `EPSILON`. (This test requires `export_svg` to be available; it should call `use crate::io::svg::export_svg;`.)

19. `src/io/svg/mod.rs` re-exports `import_svg` and `SvgImportError` so that `use lasercad::io::svg::import_svg;` and `use lasercad::io::SvgImportError;` both compile from outside the module.

20. `wc -l src/io/svg/import.rs` reports ≤ 300; `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 4):** `empty_svg_returns_no_entities` — calls `import_svg` on a minimal valid `<svg/>` string; asserts `Ok(vec![])`.
- **Unit (AC 5):** `invalid_xml_returns_xml_parse_error` — passes `"<svg><unclosed"` and asserts `Err(SvgImportError::XmlParse(_))`.
- **Unit (AC 6):** `no_svg_root_returns_error` — passes `"<root/>"` and asserts `Err(SvgImportError::NoSvgRoot)`.
- **Unit (AC 7):** `line_element_parsed_to_entity_line` — SVG string with one `<line x1="1.0000" y1="2.0000" x2="11.0000" y2="7.0000"/>`; asserts one `Entity::Line` with p1/p2 within `EPSILON`.
- **Unit (AC 8):** `circle_element_parsed_to_entity_circle` — SVG string with one `<circle cx="5.0000" cy="5.0000" r="3.0000"/>`; asserts one `Entity::Circle` within `EPSILON`.
- **Unit (AC 9):** `arc_ccw_quarter_reconstructed_correctly` — parses `M 10.0000 0.0000 A 10.0000 10.0000 0 0 1 0.0000 10.0000`; asserts center, r, start_angle, end_angle, ccw as specified.
- **Unit (AC 10):** `arc_large_flag_selects_correct_center` — parses the large-arc half-circle; asserts center `(0.0, 0.0)` within `EPSILON`.
- **Unit (AC 11):** `arc_cw_sweep_flag_zero_sets_ccw_false` — parses a CW arc; asserts `ccw = false`.
- **Unit (AC 12):** `circle_negative_radius_returns_malformed_attribute` — `r="-1.0000"`; asserts `Err(MalformedAttribute { element: "circle", attr: "r", … })`.
- **Unit (AC 13):** `line_bad_attribute_returns_malformed_attribute` — `x1="abc"`; asserts `Err(MalformedAttribute { element: "line", attr: "x1", value: "abc" })`.
- **Unit (AC 14):** `path_with_non_numeric_a_command_returns_malformed_path` — `d="M 0 0 A notanumber 10 0 0 1 5 5"`; asserts `Err(MalformedPath(_))`.
- **Unit (AC 15):** `non_arc_path_silently_skipped` — `d="M 0 0 L 10 10"`; asserts `Ok(vec![])`.
- **Unit (AC 16):** `unknown_elements_silently_skipped` — SVG with `<line …/>`, `<rect …/>`, `<circle …/>`; asserts two entities, no error.
- **Unit (AC 17):** `entities_inside_g_groups_collected` — SVG replicating the three-group export structure; asserts exactly one `Entity::Line`.
- **Integration (AC 18):** `round_trip_line_circle_arc` — builds a `Document` with one entity of each kind, calls `import_svg(&export_svg(&doc))`, asserts three entities match within `EPSILON`. Placed in `#[cfg(test)] mod tests` inside `import.rs`; uses `crate::io::svg::export_svg`.
- **Compile check (AC 19):** a test inside `import.rs` that writes `let _f: fn(&str) -> _ = import_svg;` — proves the re-export path compiles (compilation is the assertion).
- **Static check (AC 1):** `grep -nE '^use (egui|eframe|rfd)' src/io/svg/import.rs` returns no matches.
- **Build gate (AC 20):** `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

### Coordinate passthrough

The LCV-056 exporter emits all coordinates verbatim from world space (mm), with no Y-flip and no SVG `transform`. Consequently the importer reads them back verbatim; no coordinate conversion is needed. `<line x1="3.5000" …/>` means `x = 3.5 mm` in world space, directly.

### Arc center reconstruction algorithm

SVG encodes arcs by their endpoints, radius, and two binary flags. The `Arc` struct (LCV-013) stores center + angles. Reconstruction:

Given `sx, sy` (start), `ex, ey` (end), `r`, `large_arc: bool`, `sweep: bool` (true = CCW):

1. `dx = ex − sx`, `dy = ey − sy`, `d = hypot(dx, dy)` (chord length).
2. Guard: if `d < EPSILON`, return `Err(MalformedPath(…))` (start == end, degenerate; the exporter never emits this).
3. Guard: if `d > 2.0 * r + EPSILON`, return `Err(MalformedPath(…))` (chord longer than diameter; geometrically impossible).
4. Midpoint: `mx = (sx + ex) / 2.0`, `my = (sy + ey) / 2.0`.
5. Half-chord height: `h = sqrt(r² − (d / 2.0)²)`.
6. Perpendicular unit vector (rotated 90° CCW from the chord direction): `ux = −dy / d`, `uy = dx / d`.
7. Sign selection: `sign = if large_arc == sweep { −1.0 } else { 1.0 }`.
   - Derivation: in SVG, when `sweep=1` (CCW) and `large=0`, the center lies on the left side of the directed chord (start→end); when `large=1` it is on the right. The formula above encodes that rule.
   - Sanity check: quarter-arc `center=(0,0), r=10`, start `(10,0)` → end `(0,10)`, large=0, sweep=1 → `sign=1`, `cx = 5 + 1·7.071·(−0.707) ≈ 0`, `cy = 5 + 1·7.071·(−0.707) ≈ 0`. ✓
8. `cx = mx + sign * h * ux`, `cy = my + sign * h * uy`.
9. `start_angle = atan2(sy − cy, sx − cx)`.
10. `end_angle   = atan2(ey − cy, ex − cx)`.
11. `ccw = sweep`.

The resulting `Arc::new(Vec2::new(cx, cy), r, start_angle, end_angle, ccw)` is the reconstructed entity. No angle normalisation is applied; `Arc` stores angles as-supplied (per LCV-013).

### Path `d` attribute tokenisation

Split the `d` attribute value on ASCII whitespace (including tabs, newlines, `\r\n`). Tokens for the expected form:

```
index:  0   1   2   3   4   5   6   7       8      9   10
value: "M" sx  sy  "A"  rx  ry  "0" large  sweep  ex  ey
```

- If the token count is fewer than 11, or `tokens[0]` is not `"M"` / `"m"` (case-insensitive), or `tokens[3]` is not `"A"` / `"a"` (case-insensitive): **silently skip** the `<path>` — it is not an arc in the supported form.
- Otherwise parse tokens 1, 2, 4, 5, 9, 10 as `f64`; tokens 7 and 8 as `u8`. Any parse failure → `Err(MalformedPath(raw_d))`.
- Validate `rx == ry` (within `EPSILON`): if not, return `Err(MalformedPath(…))` — the exporter never emits elliptic arcs.
- Token 6 (`x-axis-rotation`) must be `"0"` (or parseable as `f64` equal to `0.0` within `EPSILON`); if not, return `Err(MalformedPath(…))`.
- Tokens beyond index 10 are ignored (tolerant of a trailing `Z` or further subpaths).

### Module wiring checklist

```
src/io/svg/mod.rs   — add: pub mod import;
                         pub use import::{import_svg, SvgImportError};
src/io/mod.rs       — add: pub use svg::{import_svg, SvgImportError};
Cargo.toml          — add: roxmltree = "0.20"
```

`src/io/mod.rs` already declares `pub mod svg;`; no further `mod` declaration is needed there.

### thiserror is already a dependency

`thiserror = "2"` is already in `Cargo.toml` (added by LCV-022 area); the implementer does not need to add it again.

### v1 reference

LaserCAD R14 v1 had no SVG importer; this is a new capability for v2. There is no TypeScript reference implementation to port from.

### LaserGRBL round-trip note

The SVG files produced by LCV-056 are the primary input for this importer. Files imported from LaserGRBL itself may contain Bézier curves, `<text>` elements, or other constructs outside the strict subset — those are silently skipped without error, which is the expected behaviour (operators are warned to use LaserCAD's own export for lossless round-trips).
