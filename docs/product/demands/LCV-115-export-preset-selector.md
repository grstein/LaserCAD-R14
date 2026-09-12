# LCV-115 — Export preset selector (cut / mark / engrave)

- **Status**: Draft
- **Phase**: 11
- **Depends on**: LCV-114, LCV-100 (Done), LCV-057 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

The exporter always writes every entity into the `cut` group.
`src/io/svg/export.rs` emits three `<g>` elements — `cut` (`#ff0000`), `mark`
(`#0000ff`), `engrave` (`#00aa00`) — and two of them are always empty. In
LaserGRBL the group colour is how the operator assigns speed and power: red
strokes get the cutting profile, blue the marking profile, green the raster
profile. An operator who wants to score a fold line or mark a part number has
one option today: export, open the SVG in a text editor, and retype the colour
by hand.

The three groups already exist, in the right order, with the right ids and
colours. What is missing is the one-bit decision of which one the geometry goes
into — and a way for the operator to make it without leaving the app.

## Scope

- A `Preset` enum (`Cut`, `Mark`, `Engrave`) in the SVG layer with its id and
  colour, replacing the three literals in the exporter.
- `export_svg(doc, preset)` — the selected group receives all geometry; the other
  two stay empty.
- `App::export_preset` — session state, `Cut` by default, set from
  `File > Export preset ▸`.
- A read-only preset label in the status bar, so the export colour is never
  invisible state.
- Import detects the preset of the group the geometry came from, so an
  open/edit/save round-trip does not silently downgrade a marking job to a
  cutting job.

## Out of scope

- **A per-entity preset field.** The roadmap calls this the *ideal* shape, and
  it stays deferred to a future demand (provisionally "LCV-1xx — per-entity
  export preset"): it needs an `Entity`-level field, a schema bump, a
  `SetEntityPreset` command, selection-aware UI, per-preset stroke colours in
  the viewport renderer, and a migration path for existing autosaves. The
  whole-document selector delivers the actual workflow — *this job is a mark
  job* — at a fraction of the surface. Nothing in this demand may be designed
  in a way that blocks it: `Preset`, the group mapping and the importer's
  detection are exactly the pieces the per-entity demand will reuse.
- **Rendering entities in their preset colour in the viewport.** The viewport
  keeps its current stroke colours; the status-bar label is the feedback
  channel. Per-entity colour only makes sense once presets are per-entity.
- **Per-preset stroke width, power, speed or passes in the SVG.**
  `stroke-width="0.1"` stays fixed for all three groups (AGENTS.md §SVG export);
  speed and power live in LaserGRBL, not in the file.
- **Layers.** `docs/product/README.md` lists "no layers" as an explicit
  non-goal. Presets are not layers: there is exactly one active preset and no
  visibility, locking, ordering or per-layer selection.
- **Persisting the preset in Settings.** See the decisions below.
- **Storing the preset in the autosave envelope or in `Document`.** Same.
- **A fourth preset, custom colours, or a colour picker.** Three groups, three
  fixed colours, matching LaserGRBL convention and the existing exporter.

## Product decisions (do not re-open these)

1. **Scope of the selection: the whole document, at export time.** One preset
   per exported file.
2. **Home of the state: `App::export_preset`, session-only.** Not in `Settings`
   (a persisted preset means the operator who marked one part yesterday cuts
   their next job in the mark profile tomorrow without being told — a
   sticky-mode trap with a physical, destructive failure mode). Not in
   `Document` and not in the autosave envelope (it is an export-time choice, not
   drawing data, and putting it in the document forces the schema bump this
   demand is explicitly avoiding). It resets to `Cut` on every app start, and
   `File > New` leaves it alone — the operator doing three mark jobs in a row
   should not re-pick it each time within one session.
3. **Where the operator picks it: `File > Export preset ▸` with three radio
   items**, placed directly above `Save` / `Save As…` so it reads as a property
   of saving. Not a toolbar dropdown (the toolbar is tools), not a dialog on
   every save (an extra click on the most-used action).
4. **The choice is always visible.** A read-only `CUT` / `MARK` / `ENGRAVE`
   label in the status bar. Invisible export state that changes what the machine
   does is the failure this decision exists to prevent.
5. **Import adopts the file's preset.** Opening a file whose geometry is in the
   `mark` group sets `export_preset = Mark`, so open → edit → Ctrl+S returns it
   to the `mark` group. Without this, every existing mark/engrave file silently
   becomes a cut file on the first re-save. This is why the demand depends on
   LCV-114: both extend the importer's return type, and doing it once is
   cheaper than merging it twice.

## Acceptance criteria

1. **The enum.** `src/io/svg/export.rs` defines
   `pub enum Preset { Cut, Mark, Engrave }` (`Debug`, `Clone`, `Copy`,
   `PartialEq`, `Eq`, `Default` with `Cut` as default) with
   `pub const fn id(self) -> &'static str` → `"cut"` / `"mark"` / `"engrave"`,
   `pub const fn color(self) -> &'static str` → `"#ff0000"` / `"#0000ff"` /
   `"#00aa00"`, and `pub const ALL: [Preset; 3]` in cut, mark, engrave order.
   It is re-exported from `crate::io` alongside the existing exports. No
   `egui`/`eframe`/`rfd` import enters `io/svg/`.

2. **The three group literals are gone.** The exporter builds each `<g>` from
   `Preset::ALL` using `id()` and `color()`;
   `grep -n '#ff0000\|#0000ff\|#00aa00\|"cut"\|"mark"\|"engrave"' src/io/svg/export.rs`
   matches only inside the `Preset` impl (and in tests).

3. **The signature.** `pub fn export_svg(doc: &Document, preset: Preset) -> String`.
   All geometry goes into the group whose id equals `preset.id()`; the other two
   groups are still emitted, in the same order, with the same attributes, and
   empty. Header, `fill="none"`, `stroke-width="0.1"`, the Y mirror and the arc
   sweep rule are unchanged (AGENTS.md §SVG export, LCV-100).

4. **Byte identity for the default.** `export_svg(doc, Preset::Cut)` produces
   output byte-identical to the pre-change `export_svg(doc)` for the same
   document and bed. The existing LCV-100 export tests pass with only the extra
   argument added.

5. **The app field.** `App::export_preset: Preset`, `Preset::Cut` in
   `App::default()`, documented as session state that does not persist.
   `action_save` and `action_save_as` pass `app.export_preset` to `export_svg`.
   `action_new` does **not** reset it.

6. **The menu.** `File > Export preset ▸` submenu with three
   `ui.radio_value(&mut app.export_preset, …, "Cut")` / `"Mark"` / `"Engrave"`
   items, generated from `Preset::ALL` with a single `label()` helper — no
   second hand-written list of names. Selecting one closes the menu and changes
   nothing else (no document mutation, no history entry, no dirty flag).

7. **The status bar.** `draw_statusbar` renders the active preset as an
   uppercase label (`CUT` / `MARK` / `ENGRAVE`) in the existing separated-segment
   style, always visible, never interactive. A pure
   `pub(crate) fn format_preset(preset: Preset) -> &'static str` supplies the
   text and is unit-tested for all three variants.

8. **Import detection.** `ImportedSvg` (introduced by LCV-114) gains
   `pub preset: Preset`. During `collect()`, the importer tracks the `id`
   attribute of the enclosing `<g>`; the resulting preset is that of the **first**
   group whose subtree produced at least one entity, matching on the exact ids
   `"cut"` / `"mark"` / `"engrave"`. Geometry outside any recognised group, an
   empty file, or an unrecognised group id yields `Preset::Cut`.

9. **Open adopts it.** `action_open` / `action_open_path` set
   `app.export_preset` from `ImportedSvg::preset`. A file exported with `Mark`,
   re-opened and saved again lands back in the `mark` group with its geometry
   unchanged.

10. **No persistence.** `grep -rn "export_preset" src/io/settings.rs src/io/autosave.rs src/document/`
    returns nothing; restarting the app returns the selector to `Cut`.

11. **Purity, caps, gates.** `io/svg/` stays UI-free; every touched file stays
    ≤ 300 implementation lines (measured to the first `#[cfg(test)]`) — note
    `src/io/svg/export.rs` is at 161 and `src/ui/menubar.rs` at 189 before
    LCV-113/114 land, so re-measure after rebasing. `cargo fmt --all -- --check`,
    `cargo clippy --all-targets -- -D warnings` and `cargo test --all` exit 0.

## Expected tests

**Quality bar (Marco 1).** Every criterion is covered by an automated test or
tagged **[manual]**. This demand touches `App` state and the status bar, so the
menu and status bar are exercised through a headless frame driving the real
`App::update_ui` via `tests/harness/mod.rs`; the egui 0.29.1 traps are in
ADR 0002 §A4 — follow it, do not restate it. egui is pinned at 0.29.1;
`Ui::radio_value` and `Ui::menu_button` both exist there.

- **Unit (AC 1)** in `src/io/svg/export.rs`: `preset_ids_and_colors` — all three
  variants' `id()` and `color()`; `preset_default_is_cut`;
  `preset_all_is_in_export_order`.
- **Unit (AC 2, 3)**: `export_places_geometry_in_the_selected_group` — one line
  exported three times, asserting the `d`/coordinate text appears after
  `id="mark"` and before the next `</g>` for `Preset::Mark`, and that the other
  two groups are empty (`<g id="cut" …></g>` shape); plus the grep from AC 2 as
  a static check.
- **Unit (AC 4)**: the existing LCV-100 export assertions, updated only by
  passing `Preset::Cut`, kept as the byte-identity guard.
- **Unit (AC 8)** in `src/io/svg/import.rs`:
  `import_detects_preset_from_group_id` (one case per id, using the existing
  `G_GROUPS_SVG`-style fixtures extended with mark/engrave variants);
  `import_defaults_to_cut_for_bare_geometry`;
  `import_ignores_empty_groups_and_takes_the_first_group_with_geometry` — a file
  with an empty `cut` group followed by a populated `mark` group yields `Mark`
  (this is exactly the shape this exporter writes, so it is the real-world case);
  `import_defaults_to_cut_for_unknown_group_id`.
- **Integration (AC 9)** in `tests/lcv115_preset_roundtrip.rs`:
  `mark_export_reimports_as_mark_and_survives_a_resave` — export with `Mark`,
  import, assert `preset == Mark`, re-export with the imported preset, assert the
  geometry is in the `mark` group and the coordinates match within `1e-9`
  (millimetres, at the document's bed height).
- **Integration (AC 5, 6, 7)** in `tests/lcv115_preset_ui.rs` with `mod harness;`:
  `status_bar_shows_the_active_preset` — set `app.export_preset = Preset::Engrave`,
  drive one `frame(&mut app, …)` through the real `App::update_ui` and assert it
  does not panic, then assert `format_preset(Preset::Engrave) == "ENGRAVE"`
  (egui has no text-scraping API; the frame proves the render path, the pure
  function proves the text);
  `action_new_preserves_the_export_preset` — set `Mark`, call `action_new`,
  assert it is still `Mark` and the document is empty.
- **Static checks (AC 10, 11)**: the persistence greps, the purity grep, `wc -l`
  to the first `#[cfg(test)]` for every touched file.
- **[manual] smoke**: `cargo run`; the status bar reads `CUT`. Draw a rectangle;
  `File > Export preset ▸ Mark` → the status bar reads `MARK`; `Ctrl+Shift+S` →
  the file's geometry sits inside `<g id="mark" stroke="#0000ff" …>` and the
  `cut` and `engrave` groups are empty. Open that file in LaserGRBL → it is
  recognised as blue/mark. Back in the app, `File > New` → the status bar still
  reads `MARK`. Restart the app → it reads `CUT`. Open the saved mark file →
  the status bar reads `MARK`; Ctrl+S → the geometry is still in the `mark`
  group.

## Risks

- **Silent profile changes are destructive.** A mark job exported as cut burns
  through the workpiece. AC 9 (import adopts) and AC 7 (always visible) exist
  for that reason and must not be dropped as "polish".
- **Depends on LCV-114.** Both extend the importer's return value. If LCV-114
  has not landed, `ImportedSvg` does not exist yet; do not invent a parallel
  return type — wait, or land them in one branch.
- **`export_svg`'s signature change ripples.** `action_save`, `action_save_as`
  and every export test call it. That is the full call-site list today; verify
  with `grep -rn "export_svg(" src/ tests/` before starting, since LCV-114 also
  touches these files.
- **Scope creep toward layers.** Any request to hide, lock or per-entity-assign
  presets is the deferred demand, not this one.

## Notes

- v1 reference: `../LaserCAD-R14/src/io/export-svg.ts` takes
  `opts.preset: 'cut' | 'mark' | 'engrave'` and drives group id and colour from
  it; v1 never had a per-entity field either.
- AGENTS.md §"SVG export" fixes the group order, ids, colours, `fill="none"`
  and `stroke-width="0.1"`. This demand changes which group is populated and
  nothing else in that checklist; no AGENTS.md edit is expected (LCV-114 owns
  the bed-size line).
- `docs/product/README.md` non-goals: no layers, no G-code. Presets are a
  colour-group assignment for LaserGRBL, not a layer system.
- The deferred per-entity demand is the roadmap's "ideal"; when it is written,
  `App::export_preset` becomes the default for newly created entities and this
  selector keeps its meaning for files whose entities all share one preset.
