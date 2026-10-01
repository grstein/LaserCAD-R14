# ADR 0012 — Layers live in the `Document`; the mother SVG has one `<g>` per layer; one export file per layer

- **Status**: Accepted
- **Date**: 2026-09-29
- **Deciders**: architect (LCV-156; LightBurn-style layers chosen by the user on 2026-09-29)

## Context

LaserGRBL applies one speed/power setting per imported file. LCV-115 made the machining profile a
whole-document `Preset` on `App::export_preset`, written as three fixed `<g id="cut|mark|engrave">`
groups. LCV-156 replaces it with document-owned layers (name, color, Output), per-entity membership,
a "mother" SVG with one `<g>` per layer and `File > Export layers` writing one file per layer.
Binding constraints: kernel purity; every mutation is a `Command` via `App::commit`; `Document` is
`!Clone`; entity indices are the agent's handles (ADR 0007, 0010); the tree has 373 `.entities`
reads in 61 files; `app/mod.rs` (295) and `ui/menubar.rs` (288) sit at the cap (ADR 0004).

## Decision

**1. Types — kernel, new file `src/document/layer.rs`.** `LayerId(u32)` (stable, never reused in a
document); `Layer { id, name: String, color: [u8; 3], output: bool }`; `LayerError` (duplicate name,
duplicate color, empty name, layer not empty, last layer, unknown layer) with operator-facing
`Display`; `file_key(name) -> String`: keep `char::is_alphanumeric`, `-`, `_`; every other run of
characters becomes one `_`; trim `_`/`-` at both ends; ≤64 chars; empty result = `EmptyName`. Name
equality everywhere (uniqueness, agent lookup) is `file_key(a).to_lowercase() == file_key(b)…`.
Color hex read/write (`#rrggbb`, lowercase out, either case in) lives here. All serde-derivable.

**2. Membership is a parallel vector, private, kept in lockstep by `Document` methods.**
`Document` gains private `layers: Vec<Layer>` (order = display and file order), `current_layer:
LayerId`, `entity_layers: Vec<LayerId>` (invariant: `len == entities.len()`, every id exists, ≥1
layer, keys and colors unique). `entities` stays `pub` for reads, so the 373 readers do not change.
Mutation of the vector goes only through `push_entity(e, id)`, `insert_entity(i, e, id)`,
`remove_entity(i) -> (Entity, LayerId)`, `truncate_entities(n)`; each `debug_assert`s lockstep.
Construction: `Default` (one `Cut`, `#ff0000`, Output on, current — AC 1), `with_bed(bed)`, and the
validated `from_parts(bed, layers, current, entities, entity_layers) -> Result<Document, LayerError>`
used by open, import and autosave. Private fields make every outside `Document { .. }` literal a
compile error, which is the point: each construction site is forced onto a constructor. Tests that
`doc.entities.push(e)` are rewritten mechanically to `doc.push_entity(e, doc.current_layer())`
(or a `push_current(e)` shorthand). Rejected: a field on each `Entity` variant (touches geometry and
every match) and a `Placed { entity, layer }` wrapper (rewrites all 373 reads).

**3. What each existing command does.** `CreateLine/Circle/Arc`, `CreateEntities` gain
`layer: Option<LayerId>` and a `.on_layer(id)` builder; `do_` resolves `*self.layer.get_or_insert(
doc.current_layer())` and uses `push_entity`; undo uses `remove_entity` / `truncate_entities`.
Tool call sites are unchanged. `DeleteEntities` captures `(i, Entity, LayerId)` and restores with
`insert_entity`. `MoveEntities`, `TrimEntity`, `ExtendEntity` replace in place: membership untouched.
`CompositeCommand`, `SetBedSize`, `SelectionCommand`: untouched. New file
`src/document/commands/layer.rs`: `AddLayer` (allocates `max id + 1` on first `do_`, keeps it for
redo), `EditLayer` (name/color/output, captures the old `Layer`), `DeleteLayer` (captures layer +
position + prior current; a deleted current layer hands "current" to the first remaining layer),
`SetCurrentLayer`, `SetEntityLayers { indices, layer }` (captures old ids). Each is one undo step
(AC 8). Current-layer changes are commands too: the current layer is saved in the file, so a change
must mark the document dirty. Validation (AC 6/7) is `Document::check_*` returning `LayerError`,
called by the app before commit; commands `debug_assert` it and never refuse.

**4. Mother SVG (`export_svg(doc) -> String`, no preset argument).** Header unchanged. One `<g>` per
layer, in layer order, **including empty layers**:
`<g data-layer="<name>" stroke="#rrggbb" stroke-width="0.1" data-output="1|0"[ data-current="1"]>`.
`data-*` is plain SVG 2; LaserGRBL and Inkscape ignore it. No `id` (a layer name is not a valid XML
ID). The name is XML-escaped (`& < > "`). Entities are written inside their layer's group in index
order. Import (`src/io/svg/layers.rs` reads/writes these attributes; `import.rs` keeps geometry): a
`<g>` with `data-layer` defines a layer in document order; the innermost one encloses its geometry;
the layer color is read from any CSS `<color>` (named keywords, `#rgb`, `#rrggbb` in any case,
`rgb()`, `hsl()`), given as the `stroke` attribute or inside `style="stroke:…"` (style wins), and
inherited from an ancestor `<g>`/`<svg>` when absent (LCV-156 AC 16; export stays lowercase
`#rrggbb`); `data-output` must be `0`/`1` (absent = `1`); an invalid or unsupported color or
output is a new `SvgImportError::MalformedLayer`, document untouched; duplicate keys or colors are errors too. The
first `data-current="1"` layer is current, else the first layer. **Geometry outside any layer group
goes to the first layer of the file; a file with no layer group gets the default `Cut` layer.**
v0.2 files (`<g id="cut|mark|engrave">`) therefore load entirely onto `Cut` (migration out of scope).
*Amended by LCV-175:* geometry outside any layer group that resolves a stroke color (else, with
`stroke` none, a fill color) through the style cascade goes to the first layer of that exact color,
else to a new layer named `#rrggbb` (Output on, appended in order of first appearance; `#rrggbb 2`, …
when the name key is taken); only uncolored stray geometry goes to the first layer, and the default
`Cut` layer is made only when the file declares no layer group and some geometry is uncolored (or
there is none). v0.2 files now open with one `#rrggbb` layer per stroke color they use. Layer groups
keep their own stroke reading; a `<style>` rule never recolors one.
`Preset`, `from_group_id`, `App::export_preset`, the preset menu and badge are deleted.

**5. Per-layer files.** Kernel, pure: `export_layer_svg(doc, id) -> String` = the same header and
only that layer's `<g>` from the same writer (no `data-current`), and
`layer_exports(doc, mother: &Path) -> Vec<(PathBuf, String)>` = one `<stem>-<file_key>.svg` in the
mother's folder per layer with Output on and ≥1 entity. `file_key` uniqueness (AC 6) makes the names
collision-free. App side (`src/io/export_layers.rs`, new): no path → "save first", nothing written
(AC 11); empty plan → says so (AC 12); else writes (overwrite) and reports the file names. It
exports the live document and does not save the mother.

**6. Agent.** Only `String` names cross into `src/agent/` (ADR 0007 §D1). `create_line`,
`create_circle`, `create_arc` gain an optional `layer` string property; `create_drawing` gains an
optional root `layer` (one layer per batch; amends ADR 0010 §2/§3). Worker shape check: string,
1..=64 chars. The apply site resolves it by key; unknown → `Refused` naming the existing layers,
nothing committed. `query_entities` adds a `Layers: Cut (current), …` line and a `layer <name>`
suffix per entity. No layer-editing tools.

**7. Command line.** A new fieldless `CommandInput::Layers` with word rows `layer`, `la` (R14's
alias; not a letter-axis entry). This is the variant change ADR 0003 §A2a reserves for an ADR.

**8. Autosave.** `SCHEMA_VERSION` 1 → 2. Envelope `{schema_version, bed_mm, layers, current_layer,
entities, entity_layers}`, loaded through `from_parts`; a v1 file or any failed validation → `None`
(discarded, as today for a mismatch).

**9. Seams.** `app/mod.rs` swaps `export_preset` for one `layers_dialog` state field; logic in new
`src/app/layers.rs`. Dialog UI in new `src/ui/layers_dialog.rs` (ADR 0009 cap); status-bar dropdown
in new `src/ui/layer_combo.rs`. `menubar.rs` loses `preset_submenu`, gains `Export layers` and a
`Format > Layers…` menu; over 285 → move `recent_*` helpers out. Render reads `doc.layer_color(i)`.

## Consequences

- Membership desync is possible only through `pub entities` direct mutation; library code has
  none after this change, and the lockstep `debug_assert`s catch tests that forget.
- Every Ctrl+Z step is a layer or membership change as the operator sees it, current layer included.
- The SVG export contract in AGENTS.md changes; v0.2 files lose their preset on open.
- Pending v1 autosaves are discarded once on upgrade.
