# LCV-192 — Tasks

Starts after LCV-186 is Done (shared files: `tools.rs`, `tools/transform.rs`, `bridge/action.rs`,
`agent_apply/set.rs`). Mutation testing: yes.

- [x] T1 Refactor, no behaviour change: move `ToolCallError`, `get_f64`, `get_bool`, `get_index`, `validate_r`, `validate_positive` and `get_layer` into the new kernel-pure `tools/args.rs`, re-exported by `tools.rs`; add it to the AGENTS.md purity list (files: src/agent/tools.rs, src/agent/tools/args.rs, AGENTS.md)
- [x] T2 Refactor, no behaviour change: move `drawing::schema` and `layer_schema` into the new kernel-pure `drawing/schema.rs`, re-exported by `drawing.rs`; AGENTS.md purity list (files: src/agent/drawing.rs, src/agent/drawing/schema.rs, AGENTS.md)
- [x] T3 [AC1] Test: scalar refusals pinned exactly — missing `r`, `r` of the wrong type, `r = -3`, `factor = 0`, `index = -1` / `1.5`, `layer` too long and `layer` not a string, each `<tool> <field>: <reason>; expected <form>` (files: src/agent/tools/tests.rs)
- [x] T4 [AC1] `ToolCallError::Arg { tool, path, reason, expected }` replaces MissingField/InvalidArg/DrawingRoot/DrawingItem; `expected_form(field)` table; `refusal(tool, path, reason, expected)` formatter; old assertions updated (files: src/agent/tools/args.rs, src/agent/tools.rs, src/agent/tools/tests.rs)
- [x] T5 [AC1] Test: `create_drawing` refusals pinned — unknown root key, `version`, `entities` empty and 1001 items, `entities[i]` not an object, `type`, foreign key, unknown item key, `entities[17].r` (files: tests/it/agent/drawing_batch.rs)
- [x] T6 [AC1] `drawing.rs` builds `Arg` for every failure; path `arguments` → `(root)`; foreign/unknown key wording per plan; its unit tests updated (files: src/agent/drawing.rs, src/agent/tools/args.rs)
- [x] T7 [AC1] Note in LCV-185 spec that AC2/AC3 wording is superseded by LCV-192 AC1; update `tests/it/agent/layers.rs` parse-side strings (files: docs/specs/LCV-185-agent-drawing-contract/spec.md, tests/it/agent/layers.rs)
- [x] T8 [AC1] Test then code: LCV-186 set refusals in the shape — `delete_entity indices[3]: duplicate of indices[1]; expected distinct indices`, empty/oversize list, `index` with `indices` (files: src/agent/tools/transform.rs, tests/it/agent/transform_tools.rs)
- [x] T9 [AC3] Test: invalid JSON → `create_line (root): not valid JSON (<serde message>); expected a JSON object`; oversize → `(root): arguments exceed 1048576 bytes; expected at most 1048576 bytes` (files: src/app/agent_worker/tests.rs)
- [x] T10 [AC3] `to_action` builds `ToolCallError::Arg` with path `(root)` for both (files: src/app/agent_worker.rs)
- [x] T11 [AC2] Test: out-of-range index (scalar, set, empty drawing), unknown layer (scalar and `create_drawing`), mirror line with two equal points, each in the shape (files: src/app/agent_apply/tests.rs, tests/it/agent/layers.rs)
- [x] T12 [AC2] `AgentAction::tool_name`; `target_layer` names the tool and lists the layers as `expected one of …` (files: src/agent/bridge/action.rs, src/app/agent_apply.rs)
- [x] T13 [AC2] `in_range(tool, index, doc)` and the mirror refusal through `refusal(..)`; `set.rs` range refusal `indices[k]` (files: src/app/agent_apply/edit.rs, src/app/agent_apply/set.rs)
- [x] T14 [AC4] Test: `RefusedCalls` — a byte-identical refused call returns the repeat text quoting the first refusal; other args bytes, other tool, an `Ok` outcome and a `Fenced` outcome are never matched (files: src/agent/repeat.rs)
- [x] T15 [AC4] Implement `RefusedCalls { record, check }`; `mod repeat` in mod.rs; AGENTS.md purity list (files: src/agent/repeat.rs, src/agent/mod.rs, AGENTS.md)
- [ ] T16 [AC4] Test: in `drive_turn`, the second identical refused call reaches `ask` as `Malformed` with `repeated call, refused before: <first>; change the arguments`, its tool result says the same, and the steps-left line counts it as a step (files: src/app/agent_worker/tests.rs)
- [ ] T17 [AC4] Wire `RefusedCalls` into `drive_turn::dispatch_fn` (files: src/app/agent_worker.rs)
- [ ] T18 [AC5] Test: a 200-char unknown key is cut to 64 at the root and in an item; a string value (`"layer": 5`, `"r": "SECRET…"`, invalid JSON holding `SECRET`) never appears in the refusal (files: tests/it/agent/drawing_batch.rs, src/app/agent_worker/tests.rs)
- [ ] T19 Mutation testing: `scripts/mutants.sh <base>` with MUTANTS_TARGET_DIR; one test per survivor (files: tests as needed)
- [ ] T20 ADR 0007 §D15 amendment (shape, repeat rule, still a step) and ADR 0010 §3 amendment (`; expected <form>`, `(root)`) (files: docs/adr/0007-*.md, docs/adr/0010-*.md)
- [ ] T21 CHANGELOG line (files: CHANGELOG.md)
