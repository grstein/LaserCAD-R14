# LCV-114 — Configurable bed size (1..2000 mm), stored in the document and read back on import

- **Status**: Done
- **Phase**: 11
- **Depends on**: LCV-100 (Done), LCV-057 (Done), LCV-102 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (commit bd49c11; reviewed, APPROVED on first pass)

## Problem

v2 hard-codes the machine bed at 400 × 400 mm in two `const`s
(`src/util/units.rs::BED_WIDTH_MM` / `BED_HEIGHT_MM`) and reads them from four
places: the grid/bed renderer, `do_fit_to_bed`, the SVG canvas header, and
`flip_y`. An operator with a 3018 (300 × 180), a K40 (300 × 200) or a 1390
(1300 × 900) cannot tell the app what machine they own. That is not a cosmetic
problem: `flip_y` mirrors world Y around the **bed height**, so on any machine
that is not 400 mm tall, every exported job comes out vertically offset by
`400 − actual_height` millimetres. The drawing looks right on screen and cuts in
the wrong place. v1 had this configurable from day one
(`../LaserCAD-R14/src/ui/document-size-dialog.ts`, 1..2000 mm, default
128 × 128) and wrote the chosen size into the SVG header, so LaserGRBL and a
re-import both agreed on the work area.

This demand makes the bed a property of the **document**, exports it, reads it
back, and gives it a dialog — without breaking the LCV-100 Y-axis contract.

## Scope

- `Document::bed_mm: [f64; 2]` as the single owner of the current bed size.
- `Settings::default_bed_mm` as the seed for *new* documents only.
- `BED_WIDTH_MM` / `BED_HEIGHT_MM` renamed to `DEFAULT_BED_WIDTH_MM` /
  `DEFAULT_BED_HEIGHT_MM`, value **unchanged at 400 × 400**, plus `BED_MIN_MM`
  / `BED_MAX_MM` and a pure `clamp_bed_mm` helper.
- `flip_y(y_mm, bed_height_mm)` — the mirror axis becomes a parameter.
- `App::bed` (the `render::Bed` field) removed; the renderer is fed from
  `document.bed_mm` per frame.
- SVG export writes the document's bed as `width` / `height` / `viewBox` and
  mirrors around its height.
- SVG import reads the bed back and the opened document adopts it.
- A `SetBedSize` command on the undo stack, driven by a `File > Bed size…`
  dialog.
- Autosave envelope carries `bed_mm`.

## Out of scope

- **Non-square pixel units, `pt`/`in`/`px` suffixes, `%` and unitless-with-scale
  SVG headers.** Millimetres are canonical (AGENTS.md). The importer accepts a
  bare number and a number with a `mm` suffix; anything else is a malformed
  header (AC 9).
- **`preserveAspectRatio`, non-zero `viewBox` origins, transforms on the root
  `<svg>`.** The exporter has always written `viewBox="0 0 W H"`; the importer
  reads that shape and rejects others rather than implementing a transform
  stack.
- **Per-document origin, machine-origin corner selection, or a "home is
  bottom-left/top-left" toggle.** World is Y-up, SVG is Y-down, the mirror is
  `bed_height − y`. LCV-100 settled that; this demand only parameterises the
  constant.
- **Clipping, warning or refusing geometry outside the bed.** Drawing outside
  the work area stays legal and exports as-is; a bounds warning is a separate
  demand if anyone ever asks for it.
- **Rescaling existing geometry when the bed changes.** Changing the bed moves
  the frame, never the entities.
- **A machine-profile library** (named presets with bed size + speeds + power).
  That is Marco 3+ territory at best; this demand ships one dialog with two
  numbers.
- **Changing `SCHEMA_VERSION`.** See AC 13.

## Product decisions (do not re-open these)

1. **Owner: the document.** `Document::bed_mm` — the bed travels with the
   drawing, round-trips through SVG and autosave, and is undoable. `App` keeps
   no copy: a second copy is a second source of truth, and the current
   `App::bed` field is exactly that.
2. **Settings holds a default, not the truth.** `Settings::default_bed_mm` seeds
   `File > New` and a cold boot, so the operator sets their machine once. It is
   written **only** by the Bed size… dialog; opening a 300 × 200 file must not
   silently re-home the operator's default.
3. **The default stays 400 × 400, not v1's 128 × 128.** v1's 128 was a
   compromise for an app that could not be configured per machine; once the
   dialog exists the number only decides what a blank document starts at, and
   400 is the better blank start for the class of machine this targets.
   Re-baselining to 128 would rewrite every LCV-100 / LCV-057 golden fixture and
   every `flip_y` doctest for zero user-visible gain. **Note:** the roadmap's
   Marco 1 verification line says *"Bed size 128×128 reflete no export"* — that
   check becomes *"a bed size set in the dialog (use 128 × 128) is reflected in
   the export header and in the mirrored Y"*, which is the property that
   actually matters.
4. **Opening a file adopts that file's bed size.** A 300 × 200 SVG opened,
   edited and re-saved must come back out at 300 × 200; keeping the app's bed
   would silently re-mirror every Y coordinate on save. This matches v1
   (`import-svg.ts` returns `bounds`, `replaceDocument` adopts it).
5. **A file with neither `width`/`height` nor `viewBox` falls back to
   400 × 400** (the `DEFAULT_*` constants), not to the current document's bed.
   Fallback-to-current would make import results depend on hidden state; the
   constant keeps every existing fixture in `src/io/svg/import.rs` and
   `tests/lcv057_svg_import.rs` valid unchanged.
6. **Bed changes go through `Command` + `History`.** AGENTS.md requires every
   document mutation to be a command, and it gives the operator Ctrl+Z after a
   fat-fingered `1300`. It also means the change dirties the document through
   the existing path, adding no second writer to `dirty_since` (LCV-102 AC 13).

## Acceptance criteria

1. **Constants.** `src/util/units.rs` exposes
   `DEFAULT_BED_WIDTH_MM: f64 = 400.0`, `DEFAULT_BED_HEIGHT_MM: f64 = 400.0`,
   `BED_MIN_MM: f64 = 1.0`, `BED_MAX_MM: f64 = 2000.0` and
   `pub fn clamp_bed_mm(v: f64) -> f64` (NaN → the matching default; otherwise
   `v.clamp(BED_MIN_MM, BED_MAX_MM)`). `grep -rn "BED_WIDTH_MM\|BED_HEIGHT_MM" src/`
   shows no occurrence of the old names. `src/util/mod.rs` re-exports the new
   items.

2. **`flip_y` takes the bed height.** `pub fn flip_y(y_mm: f64, bed_height_mm: f64) -> f64`
   returning `bed_height_mm - y_mm`. It stays an involution for any bed height,
   and its doctest is updated accordingly. Every caller passes the height of the
   document being exported/imported, never a constant.

3. **`Document::bed_mm`.** `pub bed_mm: [f64; 2]` on `Document`, `[width, height]`
   in millimetres. `Document` loses `#[derive(Default)]` in favour of a manual
   `impl Default` returning `[DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]` with
   the existing empty `entities` / `selection`. `document/` stays free of
   `egui`, `eframe` and `rfd`.

4. **`App::bed` is gone.** `render::Bed` gains
   `pub fn from_size_mm(size_mm: [f64; 2]) -> Self` (origin unchanged from
   today's `Default`), and the two consumers —
   `src/app/viewport.rs` (the `draw_bed` call) and
   `src/ui/menubar.rs::do_fit_to_bed` — build it from `app.document.bed_mm` at
   the point of use. `grep -rn "app\.bed\b\|\.bed\b" src/` shows no `App` field
   named `bed`.

5. **Export header.** `export_svg(doc)` keeps its signature and writes
   `width="{W}mm" height="{H}mm"` and `viewBox="0 0 {W} {H}"` from
   `doc.bed_mm`, formatted with the same numeric formatting helper used for
   coordinates today (no trailing `.0` where the existing code would not emit
   one). All other header attributes, the three preset groups, their ids,
   colours, `fill="none"` and `stroke-width="0.1"` are byte-identical to today.

6. **Export mirror.** Every Y coordinate — line endpoints, circle/arc centres,
   polyline vertices, path nodes, arc `A` parameters — is mirrored with
   `flip_y(y, doc.bed_mm[1])`. With the default bed the exporter's output is
   **byte-identical to the pre-change output** for the same document: the
   existing LCV-100 export tests pass unmodified except for the constant rename.

7. **Import returns the bed.** `pub struct ImportedSvg { pub entities: Vec<Entity>, pub bed_mm: [f64; 2] }`
   and `import_svg(src) -> Result<ImportedSvg, SvgImportError>`. Y is un-mirrored
   with the *imported* bed height, so a file authored at 300 × 200 lands at the
   same world coordinates it was exported from.

8. **Import precedence.** In order: (a) root `width` **and** `height` both
   present and parseable → that is the bed; (b) otherwise a root `viewBox`
   `"0 0 W H"` → `W`/`H` are the bed; (c) otherwise
   `[DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]`. Accepted numeric forms:
   `"400"`, `"400mm"`, `"400.5 mm"` (leading/trailing whitespace tolerated, `mm`
   suffix optional, case-insensitive).

9. **Import rejection.** A `width`/`height`/`viewBox` that is present but
   unparseable, non-finite, ≤ 0, or outside `1.0..=2000.0` returns
   `Err(SvgImportError::…)` — a new variant carrying the offending attribute
   name and raw value — and the caller leaves the open document **untouched**
   (no entities, no bed change, no history reset). Out-of-range values are
   rejected, not clamped: clamping a 5000 mm file to 2000 mm would silently
   halve nothing and place geometry wrongly, whereas the error tells the truth.

10. **Open adopts the bed.** `action_open` / `action_open_path` set
    `document.bed_mm` from `ImportedSvg::bed_mm` before entities are installed,
    and do **not** write `Settings::default_bed_mm`. A file opened and
    immediately re-saved produces a header with the same `width`/`height` it
    came in with.

11. **New seeds from settings.** `action_new` sets
    `document.bed_mm = settings.default_bed_mm` (each axis through
    `clamp_bed_mm`). At boot, `App::new()` applies the same seed when no autosave
    was recovered; when one was recovered, the envelope's `bed_mm` wins.

12. **The `SetBedSize` command.** `src/document/commands.rs` gains a variant
    holding the new `[f64; 2]` and, for undo, the previous one; `apply` /
    `undo` swap `Document::bed_mm` and touch nothing else (entities and
    selection are untouched in both directions). It goes through
    `App::commit`, so it dirties the document and is autosaved like any other
    edit.

13. **Autosave envelope.** `DocumentEnvelope` gains
    `#[serde(default = "…")] bed_mm: [f64; 2]` defaulting to the `DEFAULT_*`
    pair, so an envelope written before this demand still loads (a v2-schema
    file without `bed_mm` recovers at 400 × 400). `SCHEMA_VERSION` is **not**
    bumped: the documented policy bumps only on a breaking change, and this
    field is additive and back-compatible.

14. **The dialog.** `File > Bed size…` (placed directly above `Exit`, separated)
    opens a modal owned by `App::bed_dialog: Option<[f64; 2]>` holding the draft
    values. It shows two labelled `DragValue`s ("Width (mm)", "Height (mm)")
    with `.range(BED_MIN_MM..=BED_MAX_MM)` and `.speed(1.0)`, and **OK** /
    **Cancel** buttons. OK runs the draft through `clamp_bed_mm` per axis, and
    — only if it differs from the current bed — commits a `SetBedSize` command
    **and** writes `Settings::default_bed_mm` (saved through the existing
    `Settings::save`, whose failure is swallowed as elsewhere). Cancel changes
    nothing. The dialog reads no keyboard events (ADR 0002 §A6).

15. **The viewport follows immediately.** After OK, the grid/bed rectangle drawn
    by `render::bed::draw_bed` matches the new size on the next frame with no
    other interaction, and `View > Fit to Bed` frames the new rectangle.

16. **AGENTS.md stays true.** The implementer updates the §"SVG export"
    checklist so the `width`/`height`/`viewBox` line reads "= the document's bed
    size" instead of a fixed 400, and the mirror line references
    `flip_y(y, bed_height)`. No other section changes.

17. **Purity, caps, gates.** No `egui` / `eframe` / `rfd` import appears in
    `geometry/`, `document/`, `io/svg/` or `text/`. Every touched file stays
    ≤ 300 implementation lines (measured to the first `#[cfg(test)]`) — note
    `src/io/svg/import.rs` is at 149 and `export.rs` at 161, so the header
    parsing belongs in its own helper module under `io/svg/` if it does not fit.
    `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`
    and `cargo test --all` exit 0.

## Expected tests

**Quality bar (Marco 1).** Every criterion is covered by an automated test or
tagged **[manual]**. Anything touching `App::update_ui` carries a headless
regression test driving the real frame body through `tests/harness/mod.rs`; the
egui 0.29.1 traps are in ADR 0002 §A4 — follow it, do not restate it. egui is
**pinned at 0.29.1**; `DragValue::range(RangeInclusive<f64>)` exists there and is
the API to use (`clamp_range` is the older name and is not in 0.29.1).

- **Unit (AC 1)**: `clamp_bed_mm_clamps_and_rejects_nan` — `0.0 → 1.0`,
  `5000.0 → 2000.0`, `f64::NAN → default`, `300.0 → 300.0`.
- **Unit (AC 2)**: `flip_y_is_an_involution_at_any_bed_height` for 400, 128,
  1.0 and 2000.0; updated doctest in `src/util/units.rs`.
- **Unit (AC 3, 12)**: `document_default_bed_is_400_square`;
  `set_bed_size_round_trips_through_undo` — commit, assert the new bed, undo,
  assert the old bed, assert `entity_count()` unchanged in both directions.
- **Unit (AC 5, 6)** in `src/io/svg/export.rs`:
  `export_header_uses_document_bed` (set `bed_mm = [300.0, 200.0]`, assert the
  string contains `width="300mm"`, `height="200mm"`, `viewBox="0 0 300 200"`);
  `export_mirrors_around_document_bed_height` (a line at `y = 50` on a 200 mm
  bed emits `150`); and the existing LCV-100 export assertions kept unmodified as
  the byte-identity guard for the default bed.
- **Unit (AC 7, 8, 9)** in `src/io/svg/import.rs`:
  `import_reads_bed_from_width_height`, `..._from_viewbox_when_dimensions_absent`,
  `..._falls_back_to_default_when_both_absent` (reuses the existing `LINE_SVG`
  fixture, which has neither — this is the back-compat guard),
  `import_accepts_mm_suffix_and_whitespace`,
  `import_rejects_zero_negative_oversized_and_garbage_dimensions` (one assertion
  per case, each asserting the specific error variant).
- **Integration (AC 6, 7, 10)** in `tests/lcv114_bed_roundtrip.rs`:
  `roundtrip_at_a_non_default_bed_preserves_world_coordinates` — build a document
  with `bed_mm = [300.0, 180.0]` and a line at a known Y, `export_svg`,
  `import_svg`, assert `bed_mm` came back as `[300.0, 180.0]` and the endpoint Y
  matches the original within `1e-9`; and `roundtrip_at_the_default_bed` as the
  regression twin. Both assert millimetre values explicitly.
- **Unit (AC 11, 13)**: `new_document_seeds_bed_from_settings`;
  `envelope_without_bed_mm_loads_at_the_default` (deserialise a literal JSON
  string with `schema_version` and `entities` only) and
  `envelope_round_trips_bed_mm`; plus a static check that `SCHEMA_VERSION` is
  unchanged.
- **Integration (AC 14, 15)** in `tests/lcv114_bed_dialog.rs` with
  `mod harness;`: `bed_dialog_opens_and_commits_a_command` — set
  `app.bed_dialog = Some([128.0, 128.0])` directly (the menu click itself is
  manual), call the OK path helper, drive one `frame(&mut app, …)` through the
  real `App::update_ui`, then assert `document.bed_mm == [128.0, 128.0]`,
  `history.revision()` advanced by exactly 1, `settings.default_bed_mm ==
  [128.0, 128.0]`, and that a second frame renders without panic;
  `bed_dialog_cancel_changes_nothing`. The OK/Cancel decision must therefore
  live in a function callable without a pointer click (same split as LCV-113
  AC 11).
- **Static checks (AC 4, 16, 17)**: the `app.bed` grep; a grep that
  `AGENTS.md`'s SVG-export section no longer claims a fixed 400; the purity
  greps; `wc -l` to the first `#[cfg(test)]` for every touched file.
- **[manual] smoke**: `cargo run`; `File > Bed size…`; set 300 × 180; OK → the
  bed rectangle visibly changes and `View > Fit to Bed` frames it. Draw a line
  across the bed, `Ctrl+Shift+S` to a file, open the file in a text editor →
  `width="300mm" height="180mm" viewBox="0 0 300 180"`. Open the same file in
  LaserGRBL → the job lands in the same corner it appears in on screen (this is
  the criterion no unit test can cover). Ctrl+Z after the bed change → the bed
  returns to its previous size. Restart the app → the blank document starts at
  300 × 180 (the settings seed). Open the 300 × 180 file, then `File > New` →
  the new document is 300 × 180, not 400 × 400.

## Risks

- **Silent mis-mirroring is the whole point of this demand.** If any exporter
  call site keeps a constant height while the document uses another, the file
  looks plausible and cuts in the wrong place. AC 6's byte-identity test only
  guards the default bed; the non-default round-trip test is the one that
  matters.
- **LCV-115 changes `export_svg`'s signature too** (adding the preset) and
  extends `ImportedSvg` with a `preset` field. LCV-115 declares a dependency on
  this demand; land this one first and LCV-115 is additive.
- **LCV-113 edits the same File menu and the same `file_actions.rs`.** Expect a
  small textual merge; neither demand changes the other's semantics.
- **`Document` loses its derived `Default`.** Anything constructing it with
  `..Default::default()` still compiles, but a `Document { entities, selection }`
  struct literal will not. Fix the call sites; do not re-derive.
- **Old autosave envelopes.** Covered by AC 13's `serde(default)`; without it, a
  single stale `autosave.json` from a previous build makes the app start empty.

## Notes

- v1 reference: `../LaserCAD-R14/src/app/config.ts` (`documentBounds: { w: 128, h: 128 }`),
  `src/ui/document-size-dialog.ts` (min 1, max 2000, clamped on commit),
  `src/io/export-svg.ts` (header from `state.documentBounds`),
  `src/io/import-svg.ts` (returns `bounds`; `replaceDocument` adopts it),
  `src/io/autosave.ts` (persists `documentBounds`).
- LCV-100 established the Y-axis contract (world Y-up, SVG Y-down, mirror about
  the bed height, arc sweep inverted relative to `Arc::ccw`). This demand
  changes *what* the mirror axis is, never the contract.
- LaserGRBL reads `width`/`height` in mm together with the `viewBox` to scale the
  job; emitting the bed in both places is what keeps 1 mm on screen equal to
  1 mm on the machine.
- `Settings` is `#[serde(default)]` at struct level with an infallible `load()`,
  so adding `default_bed_mm` needs a field-level default only if the field's own
  `Default` (`[0.0, 0.0]`) would be wrong — it would be, so give it one.
