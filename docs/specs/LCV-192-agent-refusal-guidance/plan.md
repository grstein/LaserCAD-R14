# LCV-192 — Plan

## Approach

One refusal shape, `<tool> <path>: <reason>; expected <form>`, built by one formatter,
`tools/args.rs::refusal`, and used by all three refusal sources:

- **Parse side (AC1, AC3).** `ToolCallError` becomes `UnknownTool` plus one variant
  `Arg { tool, path, reason, expected }`, whose `Display` is the shape. `MissingField`,
  `InvalidArg`, `DrawingRoot` and `DrawingItem` go. `expected_form(field)` is a static table
  (`r` → `a positive finite number in mm`, `index` → `a non-negative integer (an index from
  query_entities)`, `ccw` → `true or false`, …). "missing" (absent) and "not a number" /
  "not a boolean" (wrong type) become separate reasons. `create_drawing` builds `Arg` with
  paths `version`, `entities`, `entities[3]`, `entities[3].r`. Its old `arguments` path becomes
  `(root)`. `agent_worker.rs::to_action` answers invalid JSON and oversize arguments with
  `Arg { path: "(root)" }` (AC3).
- **Document side (AC2).** `agent_apply.rs::target_layer`, `agent_apply/edit.rs::in_range`,
  the mirror identity refusal and LCV-186's `agent_apply/set.rs` call `refusal(..)`. They get the
  tool name from a new `AgentAction::tool_name`. Out of range becomes `delete_entity index: 7 is
  out of range; expected 0..=2 (the drawing has 3 entities)`, and an unknown layer becomes
  `create_line layer: unknown layer "Foo"; expected one of "0", "Cut"`.
- **Repeat guard (AC4).** A new kernel-pure `src/agent/repeat.rs::RefusedCalls` maps `(name,
  args)` byte for byte to the first refusal text. The one place that sees both the raw call and
  its outcome is the closure `agent_worker.rs::drive_turn::dispatch_fn`, which is created once per
  turn. For a repeat it sends `AgentAction::Malformed { reason: "repeated call, refused before:
  <first>; change the arguments" }` in place of the call. So the UI answers it without reading
  the document (§D15), the transcript gets a `refused` row, and `agent_loop` counts the step as
  it counts every dispatch. `loop_.rs` (257 LOC) is not touched.
- **AC5.** Payload is never echoed: numbers keep today's echo, string values are never quoted
  (except layer names, which are ≤64 chars by `layer_arg`), and unknown keys stay cut to 64 by
  `drawing::cut`.

## Touches

- `src/agent/tools/args.rs` (new, kernel-pure): `ToolCallError`, the getters and validators
  moved from `tools.rs`, `expected_form`, `refusal`. `tools.rs` re-exports them, so
  `agent::ToolCallError` and the `mod.rs` re-export are unchanged.
- `src/agent/drawing/schema.rs` (new, kernel-pure): `schema` and `layer_schema` moved out.
- `src/agent/drawing.rs`: errors go through `Arg`.
- `src/agent/tools/transform.rs` (LCV-186): set refusals in the shape.
- `src/agent/repeat.rs` (new, kernel-pure) and `src/agent/mod.rs` (`mod repeat`).
- `src/agent/bridge/action.rs::AgentAction::tool_name`.
- `src/app/agent_worker.rs`: `to_action` and `drive_turn`.
- `src/app/agent_apply.rs`, `agent_apply/edit.rs`, `agent_apply/set.rs`.
- `AGENTS.md` purity list: `tools/args.rs`, `drawing/schema.rs`, `repeat.rs`.
- ADRs: amend ADR 0007 §D15 (the refusal shape and the repeat rule: a repeated refused call is
  answered Malformed and is still a step) and ADR 0010 §3 (the `create_drawing` error gains
  `; expected <form>`, and `arguments` becomes `(root)`). No new ADR and no architect.
- `docs/specs/LCV-185-*/spec.md`: a note that LCV-192 rewords AC2 and AC3.

## Decisions (self-approved per user goal)

- The foreign-key refusal becomes `…entities[0].x2: not a circle key; expected null or a circle
  key (cx, cy, r)`, and an unknown key becomes `…: unknown key; expected a circle key (cx, cy, r)`
  or, at the root, `expected version, entities or layer`. This rewords LCV-185 AC2 and AC3,
  since AC1 here covers every argument refusal.
- `unknown tool` keeps today's text. It is not an argument refusal, so AC1 does not cover it.
- Capture, upload and fence refusals keep today's text. They are neither argument nor document
  refusals.
- The literal "in this turn" rule: a repeat is refused even if the document changed after the
  first refusal. The call can still pass with any other argument bytes.
- An empty drawing's index refusal reads `expected an index once the drawing has entities (it
  has 0)`.
- The prompt is not changed: the refusal text carries its own guidance.

## Risks

- LOC cap: `tools.rs` is at 263 and `drawing.rs` at 275, both over 270. The two moves (T1, T2)
  come first and need no behaviour change. `agent_worker.rs` at 198 gains about 12 lines.
- Mutation testing: yes, for `src/agent/`: the `expected_form` table rows, the repeat map
  (key equality, only `Refused` recorded, `Fenced` not recorded) and the range bounds in
  `in_range`.
- Test churn: about 47 references to the old variants, plus exact strings in
  `tests/it/agent/{drawing_batch,layers,transform_tools,turn_group,progress_row}.rs` and
  `agent_apply/tests.rs`. They are updated in the task that changes each string.
- Overlap with LCV-186, which is being implemented now: `tools.rs`, `tools/transform.rs`,
  `bridge/action.rs` and `agent_apply/set.rs`. LCV-192 starts only after LCV-186 is Done, and T8
  rewords the refusals it added. LCV-187 edits `tools.rs` and `bridge/action.rs` again.
