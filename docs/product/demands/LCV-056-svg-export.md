# LCV-056 — SVG export (cut/mark/engrave presets, LaserGRBL)

- **Status**: Done
- **Phase**: 5
- **Depends on**: LCV-021 (Done), LCV-013 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: f6a402e — feat(LCV-056): SVG export (cut/mark/engrave, LaserGRBL-compatible)

## Problem

A laser-cutter operator finishes drawing in LaserCAD, opens LaserGRBL, and imports the job. Without a clean SVG file there is no way to bridge those two tools: the drawing stays trapped inside LaserCAD. The export must be a **pure function** — it takes a `Document` in memory and returns an SVG string — so the UI layer, autosave, and CLI callers can all reuse it. The output must survive LaserGRBL's strict SVG parser: no bézier arcs (LaserGRBL ignores `C`/`Q` commands for true-arc toolpaths), no filled shapes, no filter graph, and the stroke colour must match one of the three built-in LaserGRBL presets so the operator can assign power/speed without manual colour-picker work.

## Scope

- New file `src/io/svg/export.rs` declaring:
  - `pub fn export_svg(doc: &Document) -> String` — the only public surface; a pure function with no side effects and no file I/O.
- The returned string is a well-formed UTF-8 XML fragment rooted at `<svg>`.
- Root `<svg>` element attributes (non-negotiable; see AGENTS.md §"SVG export"):
  - `xmlns="http://www.w3.org/2000/svg"`
  - `width="{W}mm"` and `height="{H}mm"` where W and H are the bounding-box width and height derived from `doc.bounds()`.
  - `viewBox="{min_x} {min_y} {W} {H}"` using the same world-coordinate values (no unit suffix, no pixel conversion, no Y-axis flip).
  - `fill="none"` forced on the root element so that no child element ever gets an accidental fill.
  - When `doc.bounds()` returns `None` (empty document): W = 100, H = 100, viewBox = `"0 0 100 100"`.
- Three `<g>` child elements, always emitted in this order:
  1. `<g id="cut"   stroke="#ff0000" stroke-width="0.1">` — all current entities land here.
  2. `<g id="mark"  stroke="#0000ff" stroke-width="0.1">` — empty for now (layer assignment is a future demand).
  3. `<g id="engrave" stroke="#00aa00" stroke-width="0.1">` — empty for now.
- Entity encoding rules (all coordinates in mm, formatted to 4 decimal places, e.g., `format!("{:.4}", x)`):
  - `Entity::Line(line)` → `<line x1="{p1.x}" y1="{p1.y}" x2="{p2.x}" y2="{p2.y}"/>`.
  - `Entity::Circle(circle)` → `<circle cx="{center.x}" cy="{center.y}" r="{r}"/>`.
  - `Entity::Arc(arc)` → `<path d="M {sx:.4} {sy:.4} A {r:.4} {r:.4} 0 {large} {sweep} {ex:.4} {ey:.4}"/>` where:
    - `sx, sy` = `arc.start_point()`.
    - `ex, ey` = `arc.end_point()`.
    - `large` = `1` if `arc.sweep_angle() > std::f64::consts::PI`, else `0`.
    - `sweep` = `1` if `arc.ccw` is `true`, else `0`. (SVG `sweep-flag=1` advances angles in the positive direction, which in an un-flipped coordinate frame is the same winding as CAD CCW — see Notes.)
- All entities in `doc.entities` are written into the `cut` group in iteration order.
- Forbidden output: no `<filter`, `<mask`, `<clipPath`, or `<text` elements anywhere in the string.
- `src/io/svg/mod.rs` updated with `pub mod export;` and `pub use export::export_svg;`.
- File size: `src/io/svg/export.rs` ≤ 300 LOC.
- Kernel-purity: `src/io/svg/export.rs` MUST NOT import `egui`, `eframe`, or `rfd`.
- No `unwrap()` or `expect()` in the library function body; no panics on any valid `Document`.
- Doc comments (`///`) on the public function; module header (`//!`) on the file.
- Unit tests in `#[cfg(test)] mod tests` inside `export.rs` — no new file under `tests/`.

## Out of scope

- File I/O — the caller (future LCV-062 or a test) does the `std::fs::write`.
- Y-axis transformation / mirroring — coordinates are emitted as-is from world space. A follow-on demand can add a `transform="scale(1,-1)"` wrapper once the laser-bed coordinate convention is settled.
- Per-entity layer or colour assignment — all entities go to the cut group. Layer assignment is a future demand.
- Import (`roxmltree`) — owned by LCV-057.
- SVG import round-trip — not tested here.
- Hershey text encoding — `Entity` has no `Text` variant yet; SVG text is blocked on LCV-048 landing a storable text entity.
- `<path>` encoding for circles — use the native `<circle>` element; bézier approximation of circles is never acceptable.
- Pretty-printing / indentation — whitespace is an implementer choice; no AC mandates it.
- `<?xml version="1.0" encoding="UTF-8"?>` declaration — not required; omitting keeps embedding simpler.
- `version="1.1"` attribute on `<svg>` — not required; `xmlns` alone is sufficient for LaserGRBL.
- Degenerate arcs (sweep_angle ≈ 0) — out of scope; the function must not panic on them but the visual output is undefined.
- Windows line endings — output uses `\n` or no explicit newlines; the caller normalises if needed.

## Acceptance criteria

1. `src/io/svg/export.rs` exists and declares `pub fn export_svg(doc: &Document) -> String` with no file I/O and no egui/eframe/rfd imports.
2. The returned string contains `xmlns="http://www.w3.org/2000/svg"` on the root `<svg>` element.
3. For a non-empty document whose `bounds()` returns `(Vec2::new(0.0, 0.0), Vec2::new(50.0, 30.0))`, the string contains `width="50.0000mm"` (or equivalent 4-decimal representation), `height="30.0000mm"`, and `viewBox="0.0000 0.0000 50.0000 30.0000"`.
4. For `Document::default()` (empty), the string contains `width="100mm"`, `height="100mm"`, and `viewBox="0 0 100 100"` (exact literal; no decimal padding required for the fallback values).
5. The root `<svg>` element carries `fill="none"`.
6. The output contains exactly three `<g` elements whose `stroke` attributes are (in order) `#ff0000`, `#0000ff`, and `#00aa00`; each carries `stroke-width="0.1"`.
7. `Entity::Line(Line::new(Vec2::new(1.0, 2.0), Vec2::new(11.0, 7.0)))` in the document produces a substring matching `<line x1="1.0000" y1="2.0000" x2="11.0000" y2="7.0000"/>` inside the `#ff0000` group.
8. `Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 3.0))` in the document produces a substring matching `<circle cx="5.0000" cy="5.0000" r="3.0000"/>` inside the `#ff0000` group.
9. `Entity::Arc(Arc::new(Vec2::new(0.0,0.0), 10.0, 0.0, FRAC_PI_2, true))` — a CCW quarter-arc of radius 10 from `(10,0)` to `(0,10)` — produces a `<path>` whose `d` attribute: (a) starts with `M 10.0000 0.0000`; (b) contains `A 10.0000 10.0000 0`; (c) has large-arc-flag `0` (sweep ≈ π/2 < π); (d) has sweep-flag `1` (ccw=true).
10. An arc with `sweep_angle() > π` (e.g., `Arc::new(Vec2::new(0.0,0.0), 10.0, 0.0, 3.0*FRAC_PI_2, true)`, sweep = 3π/2) produces a `<path>` with large-arc-flag `1`.
11. An arc with `ccw = false` produces a `<path>` with sweep-flag `0`.
12. The output contains no occurrence of `<filter`, `<mask`, `<clipPath`, or `<text`.
13. `src/io/svg/mod.rs` re-exports `export_svg` so `use lasercad::io::svg::export_svg;` compiles from outside the module.
14. `grep -nE '^use (egui|eframe|rfd)' src/io/svg/export.rs` returns no matches; `wc -l src/io/svg/export.rs` reports ≤ 300.
15. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1, 2, 5):** `svg_root_has_required_attributes` — calls `export_svg(&Document::default())`, asserts the string contains `xmlns="http://www.w3.org/2000/svg"` and `fill="none"`.
- **Unit (AC 4):** `empty_document_uses_fallback_canvas` — `export_svg(&Document::default())` contains `width="100mm"`, `height="100mm"`, `viewBox="0 0 100 100"`.
- **Unit (AC 3):** `non_empty_document_viewbox_matches_bounds` — document with `Line::new(Vec2::new(0.0,0.0), Vec2::new(50.0,30.0))`; assert `width="50.0000mm"`, `height="30.0000mm"`, `viewBox="0.0000 0.0000 50.0000 30.0000"` are present.
- **Unit (AC 6):** `three_groups_always_emitted` — `export_svg(&Document::default())` contains substrings `stroke="#ff0000"`, `stroke="#0000ff"`, `stroke="#00aa00"`, and all three contain `stroke-width="0.1"`.
- **Unit (AC 7):** `line_entity_encodes_to_svg_line` — document with one line; assert the output contains `<line x1="1.0000" y1="2.0000" x2="11.0000" y2="7.0000"/>` and that it appears before `</g>` of the `#ff0000` group.
- **Unit (AC 8):** `circle_entity_encodes_to_svg_circle` — document with one circle; assert `<circle cx="5.0000" cy="5.0000" r="3.0000"/>` in output.
- **Unit (AC 9):** `arc_ccw_quarter_encodes_correctly` — CCW quarter-arc (r=10, 0→π/2); assert M, A, large=0, sweep=1 as described.
- **Unit (AC 10):** `arc_large_flag_set_for_sweep_over_pi` — arc with sweep = 3π/2; assert large-arc-flag = 1 in the `d` string.
- **Unit (AC 11):** `arc_cw_sweep_flag_is_zero` — arc with ccw=false; assert sweep-flag = 0 in the `d` string.
- **Unit (AC 12):** `no_forbidden_svg_elements` — `export_svg` on a doc with one entity of each type; assert the string does not contain `<filter`, `<mask`, `<clipPath`, `<text`.
- **Unit (AC 13):** integration-style test inside `export.rs` that imports `use crate::io::svg::export_svg;` — proves the re-export path compiles (or use the full path; the test compilation is the assertion).
- **Static check (AC 14):** `grep -nE '^use (egui|eframe|rfd)' src/io/svg/export.rs` returns no matches; `wc -l src/io/svg/export.rs` ≤ 300.
- **Build gate (AC 15):** `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **Coordinate frame — no Y-flip.** The world coordinate system is mm, with the same axis orientation as the document. ViewBox and shape coordinates are emitted verbatim from `Entity` field values. A Y-axis transformation (`transform="scale(1,-1)"`) would be correct for browser-rendering parity but is not in scope; LaserGRBL drives the laser directly from SVG path coordinates and handles its own bed-origin mapping.
- **Sweep-flag derivation.** SVG `sweep-flag=1` advances the arc in the "positive-angle" direction as defined by the SVG spec (`x = cx + r·cos θ`, `y = cy + r·sin θ`, θ increasing). In an un-flipped coordinate frame (same as CAD), this is the same as `ccw=true` in the `Arc` type — both advance angles counterclockwise in the mathematical sense. Therefore `sweep = if arc.ccw { 1 } else { 0 }`. A concrete sanity check: Arc `center=(0,0), r=10, start=0, end=π/2, ccw=true` → `M 10.0000 0.0000 A 10.0000 10.0000 0 0 1 0.0000 10.0000`; the path traces from `(10,0)` to `(0,10)` through the first quadrant.
- **`<circle>` vs. two-arc path for circles.** The native `<circle>` element is fully supported by LaserGRBL and is unambiguous. Do not approximate circles with bézier curves or two-arc paths.
- **Number formatting.** Use `format!("{:.4}", value)` throughout. 4 decimal places = 0.1 μm resolution, adequate for any laser cutter. Consistent formatting makes the output diff-friendly for tests. The fallback canvas literals (`100mm`, `0 0 100 100`) are integer strings; AC 4 matches them exactly so the implementer must not add decimal padding to the fallback path.
- **mod.rs update.** `src/io/svg/mod.rs` currently has a stub constant `MODULE`; the implementer removes or keeps it, adds `pub mod export;` and `pub use export::export_svg;`.
- **300 LOC ceiling.** If the emitter logic alone would exceed 300 lines, extract a private `write_entity` helper and/or a `format_coord` helper into the same file (they count toward the ceiling), not into a separate file. If the file still won't fit, route to `architect` before writing code.
- **v1 reference.** The TypeScript v1 exporter lived in `src/io/svg/SvgExporter.ts`; it used identical group colours and the same per-entity dispatch pattern. The Rust version follows the same dispatch but as a free function rather than a class.
- **LaserGRBL checklist source:** AGENTS.md §"SVG export (LaserGRBL compatibility)" — any change to that checklist must be confirmed by `product-owner` before implementation.
