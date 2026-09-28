# LCV-144 — Tasks

- [x] T1 [AC11] Test: source scan. The eight helper `fn`s (`pt`, `sweep`, `kind`, `geometry`, `describe`, `bed_line`, `list_entities`, `list_selection`) are defined in `src/app/agent_narrate.rs` and not in `src/app/agent_apply.rs`. Include a positive control. (files: tests/it/lcv144_drawing_batch.rs, tests/it/main.rs)
- [x] T2 [AC11] Move the eight helpers verbatim into new `src/app/agent_narrate.rs`, with no behaviour change. Existing `agent_apply` narration tests stay green and unchanged. Run `scripts/loc-cap.sh`. (files: src/app/agent_narrate.rs, src/app/agent_apply.rs, src/app/mod.rs)
- [x] T3 [AC2, AC3, AC4] Test first: create `src/agent/drawing.rs` with a `parse` stub and failing unit tests:
  - 0 / 1 / 1000 / 1001 entities; missing and extra root keys; `version` 2 and `"1"`.
  - An unknown key, including a 200-char name cut to 64. Missing key, wrong JSON type, `ccw` as a string, `r` of 0 and −3.
  - A bad first, middle and last item each report the right index. A sentinel value elsewhere in the payload never appears in the error.
  - Degrees become radians, for both `ccw` values.
  - A one-item batch equals the scalar `parse_tool_call` result for line, circle and arc.
  - A radius table where `create_circle` and a one-item batch accept and reject the same values.
  
  Register the module and name `drawing.rs` in AGENTS.md's kernel-pure agent bucket. (files: src/agent/drawing.rs, src/agent/mod.rs, AGENTS.md)
- [x] T4 [AC1, AC2, AC3, AC4] Implement `drawing::parse` and `DrawingItem`. Add the `ToolCallError::DrawingRoot` and `DrawingItem` variants with the pinned wording, and make `validate_r` `pub(crate)` and call it from `drawing.rs`. T3 goes green. (files: src/agent/drawing.rs, src/agent/tools.rs)
- [x] T5 [AC10] Test first in `tools.rs`: `create_drawing` is last in `tool_definitions()`. A recursive walk of its schema finds no `oneOf`/`anyOf`/`const`/`additionalProperties`, `maxItems == 1000`, and the `version` enum is `[1]`. Then add `drawing::schema()`, the registration, the one parse arm and `AgentAction::CreateDrawing`. (files: src/agent/tools.rs, src/agent/drawing.rs, src/agent/bridge.rs)
- [x] T6 [P] [AC2] Test first in `agent_worker` tests: valid JSON padded with spaces to exactly 1 048 576 bytes passes the cap, and one byte more is `Malformed` with the pinned cap message, for `create_drawing` and `create_line`. Then add `MAX_TOOL_ARGUMENT_BYTES` and the check in `to_action` before parsing. (files: src/app/agent_worker.rs)
- [x] T7 [AC5, AC7, AC8] Test first in `agent_apply` unit tests: on a 3-entity drawing, both outcome strings are pinned character for character (n = 1 and n > 1), and `QueryEntities` afterwards lists the new entities. Then add the `CreateDrawing` arm: convert `DrawingItem → Entity`, commit `CreateEntities` once through `commit_grouped`, and narrate after the commit via `agent_narrate::batch_created`. (files: src/app/agent_apply.rs, src/app/agent_narrate.rs)
- [x] T8 [AC3, AC5, AC6, AC7, AC9] Integration, network-isolated: drive `arm_turn` and push `Act`s by hand.
  - An invalid batch gives one `refused` row, one step and the same revision. An entity at (−10000, −10000) is accepted.
  - A mixed batch on 2 entities with a selection and a non-default bed and preset appends in order. The first two entities, the selection, the bed and the preset are unchanged.
  - A fenced batch, and a cancel with the `Act` still queued, leave the count and the revision unchanged.
  - A 1000-item batch moves the revision by 1 and the steps by 1.
  - A scalar + batch + scalar turn undoes with one `Ctrl+Z`.
  - Undo then redo gives identical geometry. SVG export then import gives equal entities within the existing tolerance.
  
  (files: tests/it/lcv144_drawing_batch.rs)
- [x] T9 [AC10] Describe `create_drawing` (name, `version`, `entities`, per-type keys) in the built-in system prompt so LCV-151's tool-enumeration test stays green. Update the golden text test wherever LCV-143/151 left it. (files: src/agent/prompt.rs, tests/it/lcv143_system_prompt.rs)
- [x] T10 CHANGELOG line: the agent can draw many lines, circles and arcs in one validated, atomic `create_drawing` call. (files: CHANGELOG.md)

## Notes

- Mutation testing (cargo-mutants 27.1.0, `src/agent/drawing.rs` + `agent_worker.rs::to_action`,
  own `CARGO_TARGET_DIR`): 27 mutants, 24 caught, 3 unviable (`Default::default()` on types
  without `Default`), 0 survivors.
- Deviation (T5): adding `AgentAction::CreateDrawing` needed a temporary refusing arm in
  `agent_apply.rs` until T7, and the tool-count fixtures in `transport.rs` / `agent_worker.rs`
  tests moved from 7 to 8.
- Deviation (T4, wording not pinned by the spec): root failures without a key use
  `create_drawing arguments: must be a JSON object`; a non-object item uses
  `create_drawing entities[{i}]: must be a JSON object` (the `DrawingRoot` form, since there is no
  field). Other reasons: `missing`, `unknown key`, `must be a finite number`, `must be a boolean`,
  `must be the integer 1`, `must hold 1..=1000 items, got {n}`.
- Deviation (T9): the UNITS paragraph now says angles appear in `create_arc` and `create_drawing`
  arcs; the placeholder "If create_drawing is advertised" paragraph became the tool's own paragraph.
