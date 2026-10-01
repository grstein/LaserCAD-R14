# LCV-191 — Tasks

Starts after LCV-186 and LCV-192 are Done (shared: `tools.rs`, `tools/transform.rs`,
`bridge/action.rs`, `agent_apply/set.rs`). Mutation testing: yes.

- [x] T1 [AC1] [AC4] Test: `set_layer` parses `{indices, layer}` into `Set { op: SetOp::Layer }`; missing/non-string/65-char `layer`, missing `indices`, `index` given, and each LCV-186 AC5 bad list are refused naming the field (files: tests/it/agent/layers.rs)
- [x] T2 [AC1] [AC4] `SetOp::Layer` + `tool_name` arm; `parse_set_layer` reusing `indices(..)` and `layer_arg`; `"set_layer"` arm in `parse_tool_call` (files: src/agent/bridge/action.rs, src/agent/tools/transform.rs, src/agent/tools.rs)
- [ ] T3 [AC1] Schema: `set_layer` after `scale_entity`, `indices` and `layer` required; tool count 13; order doc updated (files: src/agent/tools/schema.rs, src/agent/tools.rs, src/agent/tools/tests.rs)
- [ ] T4 [AC1] [AC2] [AC3] [AC4] Test: applying moves every listed entity to the layer as one command; indices given descending still work; some already there → moved count + `(K already there)`; all already there → success, history unchanged; unknown layer → refused in `target_layer`'s text, nothing changed; out-of-range index → refused, nothing changed (files: tests/it/agent/layers.rs)
- [ ] T5 [AC1] [AC2] [AC3] [AC4] `Layer` arm in `set.rs`: `target_layer`, drop already-there indices, `SetEntityLayers`, narration (files: src/app/agent_apply/set.rs)
- [ ] T6 [AC5] Test: a turn of create + `set_layer` + move undoes in one step, every entity back on its original layer, redo restores the move (files: tests/it/agent/turn_group.rs)
- [ ] T7 [AC1] Test: one `set_layer` call over 5 entities costs one step (steps-left line) (files: src/app/agent_worker/tests.rs)
- [ ] T8 [AC6] Prompt: `set_layer {indices, layer}` paragraph; LAYERS section says `set_layer` moves entities and still no tool creates, renames or deletes layers; `default_prompt.rs` stays green (files: src/agent/prompt.rs, tests/it/agent/default_prompt.rs, tests/it/agent/system_prompt.rs)
- [ ] T9 Mutation testing: `MUTANTS_TARGET_DIR=/tmp/mutants-agent scripts/mutants.sh <base>`; one test per survivor (files: tests as needed)
- [ ] T10 ADR 0012 §6 amendment: `set_layer` moves entities onto an existing layer (files: docs/adr/0012-*.md)
- [ ] T11 CHANGELOG line (files: CHANGELOG.md)
