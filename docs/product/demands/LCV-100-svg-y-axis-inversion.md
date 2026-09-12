# LCV-100 — SVG export/import Y-axis inversion (world Y-up ↔ SVG Y-down)

- **Status**: Ready
- **Phase**: 10
- **Depends on**: LCV-056 (Done), LCV-057 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

LaserCAD's world is **Y-up** (`src/render/camera.rs` — "world Y grows up
(mathematical / AutoCAD convention)"). SVG is **Y-down**. `src/io/svg/export.rs`
writes world coordinates straight into the SVG (`y1="{line.p1.y:.4}"`), and
`src/io/svg/import.rs` reads them back the same way. The two bugs cancel each
other, so the in-repo round-trip test (`import_svg(&export_svg(&doc))`) passes —
and every file the product actually ships is **mirrored vertically** the moment
it is opened in LaserGRBL or Inkscape. Engraved Hershey text comes out
unreadable; an "L" bracket comes out as an "Γ"; a part cut from the exported
file is flipped about the bed's horizontal mid-line. Producing a file that a
laser cuts correctly is the entire reason this product exists, so this is the
single most important defect in the backlog.

The convention is fixed and not open for discussion: **what the operator sees on
screen is what LaserGRBL must show.** Export maps `y_svg = BED_HEIGHT_MM -
y_world`; import is the same map applied in reverse (it is its own inverse).
Because the map is a mirror, it also reverses arc handedness — which is the part
that will silently stay broken if the implementer only fixes the coordinates.

## Scope

- **New kernel module `src/util/units.rs`** carrying the bed constants and the
  single flip helper used by both directions of the SVG pipeline:
  - `pub const BED_WIDTH_MM: f64 = 400.0;`
  - `pub const BED_HEIGHT_MM: f64 = 400.0;`
  - `pub fn flip_y(y_mm: f64) -> f64 { BED_HEIGHT_MM - y_mm }` — documented as
    an involution (`flip_y(flip_y(y)) == y`), used for both world→SVG and
    SVG→world.
  - `src/util/mod.rs` declares `pub mod units;` and re-exports the three items.
- **`src/render/bed.rs`**: `Bed::default()` reads `BED_WIDTH_MM` /
  `BED_HEIGHT_MM` instead of its own `400.0` literals, so the bed the operator
  sees and the constant the exporter flips around are the same number.
- **`src/io/svg/export.rs`**: apply `flip_y` to every emitted Y value —
  `<line y1 y2>`, `<circle cy>`, and both arc endpoints — and invert the arc
  sweep flag.
- **`src/io/svg/export.rs` canvas**: the exported canvas becomes the bed
  (`width="400mm" height="400mm" viewBox="0 0 400 400"`), replacing the
  bounds-derived canvas and the `100mm`/`0 0 100 100` empty-document fallback.
- **`src/io/svg/import.rs`**: apply `flip_y` to every parsed Y value and derive
  `Arc::ccw` from the inverted sweep flag, reconstructing the arc center in
  world space.
- **Documentation**: the `AGENTS.md` §"SVG export" bullets and the module-level
  doc comments of `export.rs` / `import.rs` are corrected — they currently state
  "`viewBox` in world coordinates" and "no Y-axis flip", which becomes false.
- **Golden-fixture tests** that pin absolute, hand-checked SVG output for an
  asymmetric figure — see Acceptance criteria. A round-trip test alone cannot
  catch this class of bug and is not sufficient evidence.

### The arc rule (derived and verified — do not re-derive)

A vertical mirror preserves the point set but reverses handedness. With
`s' = (s.x, flip_y(s.y))` and `e' = (e.x, flip_y(e.y))`:

| element | rule |
|---|---|
| `M` point | mirrored `arc.start_point()` — start and end keep their roles, they do **not** swap |
| `A` target point | mirrored `arc.end_point()` |
| `rx`, `ry`, x-axis-rotation | unchanged (`r r 0`) |
| `large-arc-flag` | **unchanged**: `if arc.sweep_angle() > PI { 1 } else { 0 }` |
| `sweep-flag` | **inverted**: `if arc.ccw { 0 } else { 1 }` (today's code emits `if arc.ccw { 1 } else { 0 }`) |

Import is the mirror image of that rule: mirror the two endpoints into world
space **first**, then reconstruct the center with the perpendicular-offset sign
negated relative to today's code (`sign = if large_arc == sweep_flag { 1.0 }
else { -1.0 }`, i.e. the current expression with its two branches swapped), take
`start_angle` / `end_angle` with `atan2` against the world-space center, and set
`ccw = !sweep_flag`.

Both directions were verified numerically against a 10-point sampling of six
arcs (CCW/CW × quarter / semicircle / three-quarter), max error 1.8e-15.

## Out of scope

- **Configurable bed size.** `BED_HEIGHT_MM` stays a compile-time constant;
  `export_svg(&Document)` keeps its current signature and does not gain a bed
  parameter. LCV-114 (Marco 1) makes the bed configurable and will thread the
  value through then.
- **Non-zero bed origin.** `Bed::origin_world` stays `(0, 0)` and plays no part
  in the flip.
- **Autosave format.** `src/io/autosave.rs` serialises the `Document` as JSON in
  world millimetres. It is not SVG and must not be flipped.
- **Screen rendering.** `src/render/camera.rs` already flips correctly for the
  viewport; no render code changes beyond `Bed::default()`.
- **X-axis changes, scaling, rotation, `transform` attributes.** The map is a Y
  mirror and nothing else.
- **Migrating SVG files written by earlier builds.** No compatibility shim, no
  version sniffing — see Risks.
- **New SVG features** (layers, `<text>`, groups per preset beyond the existing
  three, bézier paths).
- **Clipping or rejecting out-of-bed geometry.** See AC 12.

## Acceptance criteria

1. `src/util/units.rs` exists and declares `BED_WIDTH_MM: f64 = 400.0`,
   `BED_HEIGHT_MM: f64 = 400.0`, and `pub fn flip_y(y_mm: f64) -> f64`.
   `src/util/mod.rs` declares `pub mod units;` and re-exports all three.
   `grep -nE '^use (egui|eframe|rfd)' src/util/units.rs` returns no matches.

2. `flip_y` is an involution and matches the fixed bed: `flip_y(0.0) == 400.0`,
   `flip_y(400.0) == 0.0`, `flip_y(10.0) == 390.0`, and
   `flip_y(flip_y(y)) == y` within `EPSILON` for `y ∈ {0.0, 10.0, 250.0, 400.0,
   -5.0, 512.5}`.

3. Single source of truth: `Bed::default().size_mm == [BED_WIDTH_MM,
   BED_HEIGHT_MM]`, and no bed-size numeric literal remains in
   `src/render/bed.rs` outside its `#[cfg(test)] mod tests` block.

4. **Export — lines.** A `Line` from `(10, 10)` to `(10, 60)` emits exactly
   `<line x1="10.0000" y1="390.0000" x2="10.0000" y2="340.0000"/>`. X values are
   untouched.

5. **Export — circles.** `Circle::new(Vec2::new(100.0, 250.0), 5.0)` emits
   exactly `<circle cx="100.0000" cy="150.0000" r="5.0000"/>`. `cx` and `r` are
   untouched.

6. **Export — arc encoding rule.** The emitted `A` command follows the table in
   §Scope: mirrored start as the `M` point, mirrored end as the `A` target,
   `large` computed exactly as today, `sweep` inverted with respect to
   `arc.ccw`.

7. **Export — CCW quarter arc (golden).** `Arc::new(Vec2::new(0.0, 0.0), 10.0,
   0.0, FRAC_PI_2, true)` emits exactly
   `<path d="M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 0.0000 390.0000"/>`.

8. **Export — CW quarter arc (golden).** `Arc::new(Vec2::new(0.0, 0.0), 10.0,
   FRAC_PI_2, 0.0, false)` emits exactly
   `<path d="M 0.0000 390.0000 A 10.0000 10.0000 0 0 1 10.0000 400.0000"/>`.

9. **Export — semicircle (golden, the case where `sweep` alone decides).** With
   a chord equal to the diameter the center is unique and `large` is irrelevant;
   only the sweep flag selects which half is drawn.
   - `Arc::new(Vec2::new(0.0, 0.0), 10.0, 0.0, PI, true)` (world upper half,
     through `(0, 10)`) emits exactly
     `<path d="M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 -10.0000 400.0000"/>`,
     which traces through SVG `(0, 390)`.
   - `Arc::new(Vec2::new(0.0, 0.0), 10.0, 0.0, PI, false)` (world lower half,
     through `(0, -10)`) emits exactly
     `<path d="M 10.0000 400.0000 A 10.0000 10.0000 0 0 1 -10.0000 400.0000"/>`,
     which traces through SVG `(0, 410)`.

10. **Export — large arc (golden, `large` must not flip).**
    `Arc::new(Vec2::new(100.0, 100.0), 10.0, 0.0, 3.0 * FRAC_PI_2, true)` emits
    exactly `<path d="M 110.0000 300.0000 A 10.0000 10.0000 0 1 0 100.0000 310.0000"/>`
    — `large == 1`, `sweep == 0`.

11. **Export — canvas.** For every document, empty or not, the root element
    carries `width="400mm"`, `height="400mm"` and `viewBox="0 0 400 400"`,
    rendered from the constants (`format!("{BED_WIDTH_MM}mm")` yields `400mm`;
    `format!("0 0 {BED_WIDTH_MM} {BED_HEIGHT_MM}")` yields `0 0 400 400`). The
    bounds-derived canvas and the `100mm` / `0 0 100 100` empty-document
    fallback are gone; `xmlns`, `fill="none"`, the three preset groups, their
    colours and `stroke-width="0.1"` are unchanged.

12. **Export — out-of-bed geometry is never clipped or dropped.** A document
    containing `Line::new(Vec2::new(0.0, 500.0), Vec2::new(10.0, 500.0))` emits
    `<line x1="0.0000" y1="-100.0000" x2="10.0000" y2="-100.0000"/>`; the number
    of `<line` elements in the output equals the number of line entities in the
    document.

13. **Import — lines and circles.** `<line x1="1" y1="2" x2="11" y2="7"/>`
    imports as `Line { p1: (1, 398), p2: (11, 393) }`;
    `<circle cx="5" cy="5" r="3"/>` imports as
    `Circle { center: (5, 395), r: 3 }`. All within `EPSILON`.

14. **Import — arcs.** Each of the four golden path strings in AC 7-10 imports
    back to exactly the source `Arc` (center, `r`, `start_point()`,
    `end_point()`, `ccw`, and `sweep_angle()` all within `EPSILON`). Note that
    raw `start_angle`/`end_angle` may differ by a multiple of 2π because the
    importer normalises through `atan2` — compare endpoints and sweep, not raw
    angles.

15. **Golden orientation fixture — the asymmetry gate.** A document holding an
    "L" — vertical leg `(10, 10) → (10, 60)`, short horizontal leg at the
    **bottom** `(10, 10) → (40, 10)` — exports such that:
    - the horizontal (bottom) leg carries `y1 == y2 == 390.0000`;
    - the free end of the vertical leg carries `340.0000`;
    - therefore the leg that is lowest in the world has the **largest** SVG `y`.

    The test asserts the two exact `<line .../>` strings, and its doc comment
    states in one sentence why this test exists: a round-trip test cannot detect
    a symmetric transform bug, because export and import cancel each other's
    error.

16. **Hershey text follows the same path with no special case.**
    `layout_text` returns `Entity::Line` values, so text is covered by AC 4. A
    test exports a document built from `layout_text("F", Vec2::new(10.0, 10.0),
    10.0, 1.0)` and asserts that (a) `import_svg(&export_svg(&doc))` returns the
    same line endpoints within `EPSILON`, and (b) the emitted SVG contains
    `format!("{:.4}", flip_y(min_world_y))` where `min_world_y` is the smallest
    Y over all layout endpoints, and does **not** contain
    `format!("\"{:.4}\"", min_world_y)` as a `y1`/`y2` value.

17. **Round-trip still holds.** `import_svg(&export_svg(&doc))` reproduces a
    document containing one line, one circle, and all four arcs of AC 7-10
    within `EPSILON` (compare center, `r`, endpoints, `ccw`, `sweep_angle()`).

18. **Existing tests are updated, not deleted.** The expectations in the
    `#[cfg(test)]` modules of `src/io/svg/export.rs` and `src/io/svg/import.rs`
    and in `tests/lcv057_svg_import.rs` are rewritten to the flipped convention.
    No test is removed except where it asserted the retired bounds-derived
    canvas (`non_empty_document_viewbox_matches_bounds`,
    `empty_document_uses_fallback_canvas`), which are replaced by AC 11's test.

19. **`AGENTS.md` §"SVG export" is corrected.** The bullet reading
    "`xmlns` on the root `<svg>`; `width`/`height` in mm; `viewBox` in world
    coordinates" states instead that `width`/`height` are the bed size in mm,
    that `viewBox` is `0 0 <BED_WIDTH_MM> <BED_HEIGHT_MM>`, and that **Y is
    mirrored** on export (`y_svg = BED_HEIGHT_MM - y_world`) and un-mirrored on
    import. The arc bullet additionally states that the sweep flag is inverted
    relative to `Arc::ccw` because the mirror reverses handedness. This demand
    authorises edits to that section of `AGENTS.md` and to nothing else in it.

20. **Module docs match the code.** The `//!` header and the `export_svg` doc
    comment in `src/io/svg/export.rs` no longer claim "`viewBox` in world
    coordinates (no Y-axis flip…)"; `src/io/svg/import.rs`'s header states the
    inverse map. Both name `crate::util::flip_y`.

21. **Kernel purity and size.**
    `grep -nE '^use (egui|eframe|rfd)' src/io/svg/export.rs src/io/svg/import.rs src/util/units.rs`
    returns no matches; implementation LOC (excluding `#[cfg(test)]` blocks, per
    ADR 0002 §"The 300-LOC cap") stays ≤ 300 in every file touched.

22. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`
    and `cargo test --all` all exit 0.

## Expected tests

All unit tests live in the `#[cfg(test)] mod tests` of the file under test;
cross-module golden tests live in `tests/lcv100_svg_orientation.rs`.

- **Unit (AC 1)**: `units_module_exports_constants_and_flip` — the three items
  resolve through `crate::util::{BED_WIDTH_MM, BED_HEIGHT_MM, flip_y}`.
  Static check: the purity grep.
- **Unit (AC 2)**: `flip_y_is_an_involution` — the six sample values plus the
  three fixed points.
- **Unit (AC 3)**: `bed_default_uses_shared_constants` in `src/render/bed.rs`.
- **Unit (AC 4)**: `line_y_is_flipped_on_export` — exact string assert.
- **Unit (AC 5)**: `circle_cy_is_flipped_on_export` — exact string assert.
- **Unit (AC 6, 7)**: `arc_ccw_quarter_golden` — exact `d` string.
- **Unit (AC 8)**: `arc_cw_quarter_golden` — exact `d` string.
- **Unit (AC 9)**: `arc_semicircle_sweep_selects_correct_half` — both halves,
  exact `d` strings; the two assertions differ only in the sweep flag.
- **Unit (AC 10)**: `arc_large_flag_survives_the_mirror` — exact `d` string,
  asserting `0 1 0` in the flag triple.
- **Unit (AC 11)**: `canvas_is_always_the_bed` — empty document and non-empty
  document both yield `width="400mm"`, `height="400mm"`,
  `viewBox="0 0 400 400"`.
- **Unit (AC 12)**: `out_of_bed_geometry_is_emitted_verbatim` — negative emitted
  Y, element count preserved.
- **Unit (AC 13)**: `import_line_and_circle_unflip_y`.
- **Unit (AC 14)**: `import_arc_golden_paths_reconstruct_source_arcs` — loops
  the four golden `d` strings.
- **Integration (AC 15)**: `tests/lcv100_svg_orientation.rs::
  l_shape_bottom_leg_has_larger_svg_y` — the golden fixture; carries the
  "round-trip cannot catch this" doc comment.
- **Integration (AC 16)**: `tests/lcv100_svg_orientation.rs::
  hershey_text_is_not_mirrored`.
- **Integration (AC 17)**: `tests/lcv100_svg_orientation.rs::
  round_trip_preserves_all_entity_kinds`.
- **Static check (AC 18)**: `cargo test --all` green with the updated
  expectations; reviewer diffs the test files for deletions.
- **Static check (AC 19, 20)**: `grep -n "viewBox" AGENTS.md src/io/svg/export.rs`
  shows no remaining "world coordinates" claim; `grep -n "flip_y"
  src/io/svg/export.rs src/io/svg/import.rs` matches in both.
- **Static check (AC 21)**: purity greps; `wc -l` minus the test module.
- **Build gate (AC 22)**: the three cargo commands.
- **Manual smoke (the reason the demand exists)**: draw the AC 15 "L" plus
  `TEXT "LASER"` in the running app; File → Save As `/tmp/lcv100.svg`; open the
  file in Inkscape — the L reads as an L and the text reads left-to-right, right
  way up, positioned where the bed showed it; load the same file in LaserGRBL
  and confirm the preview matches the app's viewport, not its mirror image.
  Then File → Open the same file back into LaserCAD and confirm the geometry
  lands on the original coordinates.

## Risks

- **Files exported by earlier builds will import mirrored.** Accepted: v0.1.0
  has never been released (LCV-089 is Blocked, no tag exists), so there is no
  installed base to protect. No compatibility shim is added — one would
  reintroduce the ambiguity this demand removes. Any local `.svg` produced
  during development must be re-exported.
- **Geometry drawn outside the bed renders clipped** in viewers that honour the
  root `viewBox` (the data is still in the file — AC 12). The bed overlay in the
  viewport already discourages out-of-bed placement. LCV-114's configurable bed
  reduces the exposure.
- **The sweep inversion is the part that silently survives a lazy fix.** A
  coordinate-only fix still passes round-trip tests and still burns mirrored
  arcs. AC 9 is the guard: if the semicircle goldens are not asserted exactly,
  nothing else catches it.
- **`tests/lcv057_svg_import.rs` and two export unit tests will fail loudly**
  before being updated. That is expected — they encode the old convention. They
  must be corrected, not deleted (AC 18).
- **Line-number drift**: LCV-105 may split `src/app.rs` and touch `src/util/`'s
  `MODULE` placeholder concurrently. This demand only adds `src/util/units.rs`
  and one `pub mod` line, so the conflict surface is one line.

## Open questions

*(none — demand is Ready)*

## Notes

- Verification method for the arc table: both directions were sampled at 10
  points per arc for CCW/CW × {quarter, semicircle, three-quarter} and compared
  against the mirrored world arc; max deviation 1.8e-15. The implementer does
  not need to re-derive anything — encode the table.
- Today's exporter emits `sweep = if arc.ccw { 1 } else { 0 }`
  (`src/io/svg/export.rs`, `encode_entity`) and today's importer reads
  `ccw = sweep_flag` (`src/io/svg/import.rs`, `parse_path`). Both flip in this
  demand. They must flip **together** or the round-trip breaks.
- The importer's existing center-selection line is
  `let sign: f64 = if large_arc == sweep_flag { -1.0 } else { 1.0 };`. After the
  endpoints are mirrored into world space, the two branches swap. Equivalent
  implementations (reconstruct in SVG space, then mirror the center and recompute
  the angles) are acceptable as long as AC 14 and AC 17 hold.
- `Arc::sweep_angle()` returns the magnitude of the sweep in `[0, 2π]` and
  already accounts for `ccw`, so the `large` expression is unchanged by the
  mirror.
- Preset groups: the mirror is applied per entity inside `encode_entity`; group
  structure, ordering and colours are untouched.
- Related: ADR 0002 (`docs/adr/0002-headless-input-tests-and-dirty-tracking.md`)
  for the 300-LOC rule reading (implementation LOC only) and the "never call
  `App::new()` from a test" rule — this demand needs no `App` at all; every test
  here is pure-kernel.
- LCV-114 (Marco 1) makes the bed configurable; at that point `flip_y` gains a
  `bed_height_mm` parameter and `export_svg` a bed argument. Keeping the flip in
  one named function is what makes that change a one-file edit.
