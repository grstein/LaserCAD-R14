# LCV-188 — Tasks

- [x] T1 Refactor (no behaviour change): move the layer-edit `impl Document` block
  (`check_*_layer`, `insert/remove/replace_layer`, `set_current_layer`, `set_entity_layer`) to
  `state/layers.rs` (files: src/document/state.rs, src/document/state/layers.rs)
- [x] T2 [AC2] Test: pushes get distinct ids `e1, e2, …`; after a delete, every other entity keeps
  its id; `from_parts` numbers `e1..=en`; `index_of`/`entity_id` round-trip; a new push after
  deletes and truncates never reuses an id (files: tests/it/document/entity_ids.rs, tests/it/document/mod.rs)
- [x] T3 [AC2] `EntityId` + `Display`, `entity_ids`/`next_id` in lockstep in the four mutators and
  `from_parts`, `entity_id`, `index_of` (files: src/document/state.rs, src/document/state/ids.rs, src/document/mod.rs)
- [x] T4 [AC3] Test: undo then redo restores ids for CreateLine/Circle/Arc, CreateEntities,
  CopyEntities, TransformEntities (in place and keep_source), DeleteEntities (non-contiguous
  set), Move, Trim, Extend; a mixed `History` sequence gives the same id list after undo-all/redo-all
  (files: tests/it/document/entity_ids.rs)
- [x] T5 [AC3] `remove_entity` returns the id, `insert_entity` takes it; `DeleteEntities` captures
  and restores it; update the other `remove_entity` callers (files: src/document/state.rs, src/document/commands/edit.rs, src/document/commands/create.rs)
- [x] T6 [AC3] `IdLedger` + `push_entity_as` (debug-assert: below `next_id`, not live); ledger in
  `CreateLine/Circle/Arc` and `CreateEntities` (files: src/document/state/ids.rs, src/document/commands/create.rs)
- [x] T7 [AC3] Ledger in `CopyEntities` and keep-source `TransformEntities`; add redo-id cases for
  any creating command brought in by the rebase (files: src/document/commands/edit.rs, src/document/commands/transform.rs)
- [x] T8 [AC2] Test: File > New and `action_open_path` after deletes continue the counter (no id of
  the old document is reused) (files: tests/it/app/entity_ids.rs, tests/it/app/mod.rs)
- [x] T9 [AC2] `Document::ids_after`; call it at the three `app.document = …` sites (files: src/document/state/ids.rs, src/io/file_actions.rs)
- [x] T10 [AC7] Test: two documents with the same geometry and different ids (one built by
  create/delete/undo) export byte-identical mother SVG, per-layer SVG and autosave envelope; no
  `id`/`e<N>` in the output (files: tests/it/io_svg/entity_ids.rs, tests/it/io_svg/mod.rs)
- [x] T11 [AC1] Test, then `list_entities` prints `<i> e<N>: …`; update the exact-string listings
  (files: tests/it/agent/entity_ids.rs, tests/it/agent/mod.rs, src/app/agent_narrate.rs)
- [x] T12 [AC4, AC5] Test (parser): `id`/`ids` on the six edit tools and `set_layer` build `ById`;
  refused: bare integer, `e0`, `x7`, empty, 1001 entries, duplicate, and any two of
  `index|indices|id|ids` (files: src/agent/tools/tests.rs)
- [x] T13 [AC4] `AgentAction::ById` + `tool_name` arm; `id`/`ids` parse; routing in
  `parse_tool_call` (files: src/agent/bridge/action.rs, src/agent/tools/transform.rs, src/agent/tools.rs)
- [x] T14 [AC4] Schema `id`/`ids` on the 7 tools; `expected_form` rows (files: src/agent/tools/schema.rs, src/agent/tools/args.rs)
- [x] T15 [AC4, AC5] Test (apply): for each op (delete, move, copy, rotate, mirror ±erase, scale,
  set_layer), `ids` gives the same document and outcome as the matching `indices`; the ids stay
  valid after an earlier delete in the same turn; an unknown id is refused and the revision does not
  move (files: tests/it/agent/entity_ids.rs)
- [x] T16 [AC4, AC5] `set.rs::resolve`; the `ById` arm in `plan` (files: src/app/agent_apply.rs, src/app/agent_apply/set.rs)
- [x] T17 [AC6] Test, then the ` New id(s): …` suffix after any commit that grew the document
  (create_*, create_drawing, copy, mirror/rotate/scale with keep); update the exact outcomes
  (files: tests/it/agent/entity_ids.rs, src/app/agent_apply.rs, src/app/agent_narrate.rs)
- [x] T18 [AC4] `DEFAULT_PROMPT`: ids paragraph (prefer ids, `"e<N>"` strings, unknown is refused);
  the default-prompt test covers `id`/`ids` (files: src/agent/prompt.rs, tests/it/agent/default_prompt.rs)
- [x] T19 Docs: header pointer amendments in ADR 0007 (§D5, §Deferred) and ADR 0010 (§7), with the
  next free numbers (files: docs/adr/0007-agent-turn-mutates-the-live-document.md, docs/adr/0010-declarative-drawing-batch-tool.md)
- [ ] T20 AGENTS.md ADR list `0014` (+ purity list if a seam fired); CHANGELOG line (files: AGENTS.md, CHANGELOG.md)
- [ ] T21 `scripts/mutants.sh` on the diff (src/agent/, document ids/ledger, file_actions); kill or
  justify every survivor; `scripts/gate.sh` green

- Review note (LCV-186): in the T19 ADR 0007 pointer, add one line that set outcomes (`agent_apply/set.rs::plan`) report ids/counts instead of the §D5 per-entity description.
