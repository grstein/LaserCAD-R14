# LCV-175 — Plan

## Approach

Three stages, all kernel-pure under `src/io/svg/`:

1. **Sheet** (`css.rs`, new): `parse_sheet(text) -> Sheet` strips `/* */`, skips at-rules
   (`@x …;` or `@x … { balanced }`, noted `style @x`), and splits `selectors { decls }`. A
   selector list is kept only if every item is a compound of `type|*`, `.class`, `#id`;
   whitespace inside an item, `> + ~ : [` drop the whole rule (`style rule (unsupported
   selector)`). `declarations(text)` returns `Decl { name, value, important }`; the `style`
   attribute and LCV-171's `report::style_decls` reuse it. Specificity is `(ids, classes, types)`.
2. **Cascade** (`import/style.rs`, new): the walk collects every SVG-namespace `<style>` in
   document order (one pre-pass over `root.descendants()`, CDATA via text children) into one
   `Sheet`. `Style::child(&self, node, &Sheet, &mut Report) -> Style` computes `stroke`, `fill`,
   `color`, `visibility` (inherited) and `display` (not inherited). Candidates for a property are
   ordered: important `style` attr > important rules (specificity, then order) > `style` attr >
   rules > presentation attribute > inherited. The first *valid* candidate wins; an invalid color
   is dropped and noted `<prop> (invalid color)` (AC 11). `inherit` takes the parent value;
   `currentColor` resolves to `color` (AC 6).
3. **Color → layer** (`layers.rs`): the walk pushes a `Slot` per entity instead of a `LayerId`:
   `Layer(id)` inside a `<g data-layer>`, else `Color(rgb)` (stroke, else fill when stroke is
   `none`, AC 9) or `First`. `LayerReader::finish(slots)` makes the default `Cut` only when there
   is no declared layer and (a `First` slot exists or no slot has a color), then resolves each
   `Color` to the first layer with that exact color, else appends `#rrggbb` (Output on) in order
   of first appearance, and returns `(layers, current, entity_layers)`.

`display:none` stops the descent and notes `hidden (display:none)` once for that element;
`visibility: hidden|collapse` skips an imported element and notes `hidden (visibility)`, while
descent continues so a `visible` child still imports (AC 7). `<g data-layer>` color reading
(`layer_stroke`/`own_stroke`) is untouched, so mother files reopen exactly (AC 12).

## Touches

- `src/io/svg/css.rs` (new, ~170; `css/selector.rs` if >200), `mod.rs` (private `mod css`).
- `src/io/svg/import/style.rs` (new, ~150): `Style`, `Style::root`, `Style::child`, `collect_sheet`.
- `src/io/svg/import/walk.rs`: carries `Style` beside LCV-173's `Ctx`; display/visibility
  checks; pushes `Slot`; a `<style>` element becomes silent (its problems are reported by the
  sheet), and a `<defs>` whose element children are all `<style>` is not reported.
- `src/io/svg/import/report.rs`: `display`, `visibility` leave `REPORTED_PROPERTIES` (applied now;
  amends LCV-171 AC 7); `style_decls` delegates to `css::declarations`.
- `src/io/svg/layers.rs`: `Slot`, `finish(slots)`; `STRAY_LAYER` goes.
- `src/io/svg/import.rs`: `import_svg` builds the sheet, passes `Style::root()`, uses `finish`.
- Tests: `tests/it/io_svg/{styling.rs, color_layers.rs, mod.rs}`; corpus pairs
  `illustrator-classes`, `inkscape-hidden-layer`; `inkscape-mm.expected` and
  `v02-presets.expected` now expect color layers; `layers_roundtrip.rs::
  stray_geometry_goes_to_the_first_layer` and `preset_roundtrip.rs` follow ADR 0012's amendment.
- Docs: `svg-spec-coverage.md` §5, `CHANGELOG.md`. ADRs: ADR 0012 §4 **amended** (stray geometry
  with a color goes to a color layer); no new ADR.

## Export contract changes

None. `export.rs` and `layers.rs::open_group` are untouched; AC 12 is pinned by
`layers_roundtrip.rs` and the `v03-mother-three-layers` corpus seed, unchanged.

## Decisions (self-approved per user goal)

- SVG's initial `fill: black` is **not** applied: only a declared (or inherited declared) color
  counts, so an unstyled line still lands on the first layer (rule 10).
- Layer groups keep their own strict stroke reading (ADR 0012 §4); the cascade applies only to
  geometry. A sheet rule never recolors a `<g data-layer>`.
- A `fill` color is still reported as `fill` (outline only, LCV-171); `display`/`visibility`
  are no longer reported as properties, only as `hidden (…)` counts.
- A new layer whose `#rrggbb` name key is already taken by a declared layer of another color is
  named `#rrggbb 2` (then `3`, …).
- `transparent`, `url(…)` and other non-colors are unsupported colors (dropped and reported).
- Type selectors match the local name case-sensitively; `@namespace` is an ignored at-rule; an
  unterminated block ends the sheet (`style rule (malformed)`).
- v0.2 preset files (`<g id="cut" stroke="#ff0000">`) now open as one layer per stroke color
  named by hex, not on `Cut`.

## Risks

- LOC cap: `walk.rs` ~170 after LCV-173 → ~200; `layers.rs` 141 → ~200; `import.rs` untouched in
  size. If `walk.rs` passes 250, the display/visibility gate moves into `style.rs`.
- Mutation testing: no (not `export.rs`/`agent/`/`History`); one test per precedence step/selector.
- Fixtures outside `io_svg` with a stray `stroke="#ff0000"` line (`agent/memory.rs`,
  `agent/turn_group.rs`, `app/document_title_and_file_feedback.rs`,
  `ui/discard_dialog_pointer_click.rs`) now open on a `#ff0000` layer; they count entities only,
  but T16 runs them.
