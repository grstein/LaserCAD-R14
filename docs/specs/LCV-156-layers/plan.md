# LCV-156 — Plan

## Approach

Layers are document state in the kernel (ADR 0012). `Document` gains private `layers`,
`current_layer` and a parallel `entity_layers: Vec<LayerId>` kept in lock-step with the still-`pub`
`entities` through four mutators (`push_entity`, `insert_entity`, `remove_entity`,
`truncate_entities`). Create commands take the current layer by default; `DeleteEntities` restores
membership on undo; move/trim/extend edit in place and are untouched. Layer edits are new
`Command`s. Save writes the mother SVG (one `<g data-layer>` per layer); `File > Export layers`
writes one LaserGRBL file per layer with Output on and entities. The global `Preset` and its UI go away.

## Touches

- `src/document/layer.rs` (new) — `LayerId`, `Layer`, `LayerError`, `file_key`; `document/mod.rs` re-exports.
- `src/document/state.rs` — private layer fields, mutators, `from_parts`, `check_*` (AC6/AC7).
- `src/document/entity.rs::SCHEMA_VERSION` 1 → 2.
- `src/document/commands/create.rs`, `edit.rs` (`DeleteEntities`) — membership; new `commands/layer.rs`
  (`AddLayer`, `EditLayer`, `DeleteLayer`, `SetCurrentLayer`, `SetEntityLayers`).
- `src/io/svg/layers.rs` (new) — `<g>` attribute write/read; `export.rs::export_svg(doc)` mother
  and `export_layer_svg(doc, id)`; `import.rs` restores layers, `MalformedLayer` error; `Preset` removed.
- `src/io/export_layers.rs` (new) — pure `layer_exports(doc, mother)` + disk writes; `file_actions.rs` wiring.
- `src/io/autosave.rs` — envelope carries layers; v1 discarded.
- `src/render/entities.rs` — stroke in the layer color.
- `src/app/layers.rs` (new) — dialog state, validation, commits; `app/mod.rs` swaps `export_preset`.
- `src/ui/layers_dialog.rs`, `src/ui/layer_combo.rs` (new); `menubar.rs` (Export layers,
  Format > Layers…, preset submenu removed); `statusbar.rs` (preset badge → layer combo).
- `src/cmdline/` — `LAYER`/`LA` → `CommandInput::Layers`; `app/cmdline.rs` one arm.
- `src/agent/tools.rs`, `drawing.rs`, `app/agent_apply.rs`, `prompt.rs` — optional `layer`,
  `query_entities` reports layers.
- Docs: `docs/product/README.md` (drop "no layers" non-goal), `CHANGELOG.md`.
- ADRs: 0012 (new), 0010 and 0003 amended; AGENTS.md §SVG export rewritten (user approved the
  contract change on 2026-09-29).

## Risks

- LOC cap: `app/mod.rs` 295, `menubar.rs` 288 — new code in new files; if `menubar.rs` > 285
  move `recent_*` helpers out. Watch `agent/tools.rs` (~260), `agent_apply.rs` (~260),
  `app/cmdline.rs` (270).
- Membership drift: every mutator asserts equal lengths in debug; a property test runs random
  create/delete/undo/redo sequences and checks `entities.len() == entity_layers.len()` and memberships.
- Churn: private fields break every outside `Document {..}` literal and `doc.entities.push` in
  tests — a mechanical first task moves them onto constructors/mutators.
- LOC seams at close (≥270): `document/state.rs` 276, `io/svg/import.rs` 270, `app/cmdline.rs` 271 — next demand touching them splits first.
- Mutation testing: yes, on `src/io/svg/export.rs`, `src/io/svg/layers.rs` and `commands/layer.rs`.
- Interpretations (from ADR 0012): deleting the current layer makes the first remaining layer
  current; stray geometry goes to the first layer; Export layers does not save the mother;
  changing the current layer is an undo step; v0.2 files open all on `Cut`.
- Manual check: open a mother file and a per-layer file in LaserGRBL (user smoke).
