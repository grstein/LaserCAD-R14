# LCV-144 - Create a drawing from declarative JSON

- **Status**: Ready
- **Phase**: 12
- **Depends on**: LCV-122, LCV-123, LCV-142, LCV-143
- **Suggested agent**: implementer-rust
- **Suggested model**: opus
- **Implementation**: -

## Problem

Laying out a cutting job through the scalar tools costs one step, one round
trip and one frame per line, circle or arc — a 200-hole grid burns 200 steps
and minutes of latency for one intention, and a model that stops halfway
leaves partial geometry on the bed. One bounded, declarative tool that appends
many lines, circles and arcs at once, validated completely before anything is
drawn, makes the agent useful for real layouts while keeping the drawing
either fully updated or untouched. The user chose JSON entities, not
command-line scripts or a new language.

## Scope

- An always-advertised, append-only `create_drawing` agent tool per ADR 0010:
  version-1 JSON of lines, circles and arcs, fully validated in the worker,
  applied as one command on the UI thread.
- The universal 1 MiB tool-argument byte cap (ADR 0010 §3.1), for every tool.
- The `src/app/agent_apply.rs` → `src/app/agent_narrate.rs` split that must
  precede the new apply arm (ADR 0010 §9).

## Out of scope

- Executable strings, loops, expressions, variables, a script language, human
  command-line JSON input.
- Replacing, deleting, clearing, selecting, file I/O, per-entity presets, new
  geometry families (polylines, text, rectangles as a type).
- Any rule the scalar tools do not have: **no bed-bounds check**, no
  zero-length-line rejection, no magnitude cap (ADR 0010 §3).
- A setting to disable the tool; worker-held document state.

## Acceptance criteria

1. **Architecture gate — satisfied.** ADR 0010 is the contract (DTO, schema,
   validation, command, atomicity); it confirms 1..=1000 entities and a
   1 MiB argument cap.
2. **Limits and root shape.** The raw argument string of **every** tool call
   is checked against `MAX_TOOL_ARGUMENT_BYTES = 1_048_576` before
   `serde_json` parses it; over the cap the call is refused as
   `tool \`{tool}\` arguments exceed 1048576 bytes`. For `create_drawing` the
   root is an object whose key set is exactly `{version, entities}`,
   `version` is the integer `1`, `entities` is an array of 1..=1000 items.
   Anything else — missing key, extra key, `version: 2`, `version: "1"`,
   empty array, 1001 items — is refused.
3. **Per-entity validation and error reporting.** Each item is an object with
   `type` ∈ {`line`, `circle`, `arc`} and **exactly** that type's keys:
   `line {x1,y1,x2,y2}`, `circle {cx,cy,r}`, `arc {cx,cy,r,start_deg,end_deg,ccw}`
   (all required). Numbers are finite; `ccw` is a boolean; `r` passes the
   **same** positive-finite helper `create_circle`/`create_arc` use (shared,
   not copied). Validation stops at the first failure and is complete before
   any `Act` is sent. **Errors are reported under ADR 0007 §D15** (LCV-142 AC
   10): a `Malformed` `Act`, answered `Refused`, one `refused` transcript row,
   one step, nothing applied, revision unchanged. The reason names the path:
   `create_drawing entities[{i}].{field}: {reason}` for items and
   `create_drawing {field}: {reason}` for the root, e.g.
   `create_drawing entities[17].r: -3 is not a positive finite number`. The
   argument payload is never echoed; an unknown key's name is echoed truncated
   to 64 characters. **There is no bed-bounds rule**: an entity wholly
   outside the bed is accepted, exactly as `create_line` accepts it.
4. **Units.** Coordinates and radii are millimetres in and out.
   `start_deg`/`end_deg` are converted to radians once, in
   `src/agent/drawing.rs`; `ccw` has `create_arc`'s Y-up meaning. A batch of
   one item produces the same entity the matching scalar tool produces from
   the same numbers.
5. **Apply.** A valid, unfenced `create_drawing` appends all entities in
   order through one command (`CreateEntities` or equivalent in
   `document/commands/`) committed with `History::commit_grouped`. Existing
   entities, selection, bed size and export preset are unchanged; undo
   truncates back to the pre-append length.
6. **Atomicity.** An invalid, oversized, fenced, or cancelled-before-apply
   batch appends nothing and does not advance the revision.
7. **Accounting.** One batch is one step, one `Act`, one command and one
   revision bump regardless of entity count, and joins the turn's flat group
   (ADR 0007 §D12) as a single command — no per-entity composite.
8. **Outcome wording.** Computed at the live apply site after the commit,
   with the document's numbers:
   `Created {n} entities (indices {a}..={b}). The drawing now has {m} entities. Revision {r}.`
   and for one item
   `Created 1 entity (index {a}). The drawing now has {m} entities. Revision {r}.`
   Indices are zero-based. The same text is the tool result and the `tool`
   transcript row. A `query_entities` in the same turn lists the new entities.
9. **Round trip.** Whole-turn Undo removes the batch; Redo restores it with
   identical geometry; SVG export of the result follows the existing
   LaserGRBL contract (AGENTS.md §SVG export: bed-mm `viewBox`, Y mirror,
   `fill="none"`, arcs as `A` paths with inverted sweep) and re-imports to
   the same entities.
10. **Registration.** `tool_definitions()` appends `create_drawing` last; its
    schema uses only `type`/`properties`/`required`/`description`/`enum`/
    `minItems`/`maxItems`/`items` — no `oneOf`, `anyOf`, `const` or
    `additionalProperties` (ADR 0010 §2). The schema fragment, parser and
    `DrawingItem` live in `src/agent/drawing.rs`, which names no document
    type; `tools.rs` gains only the registration and one `parse_tool_call` arm.
11. **Seam first.** Before the new apply arm lands, `pt`, `sweep`, `kind`,
    `geometry`, `describe`, `bed_line`, `list_entities` and `list_selection`
    move from `src/app/agent_apply.rs` to a new `src/app/agent_narrate.rs`
    with no behaviour change. Every touched file stays ≤ 300 implementation
    LOC by the ADR 0004 recipe.

## Expected tests

All network-isolated: parser tests are pure; apply tests drive `arm_turn` and
hand-pushed `Act`s; loop tests use the fake `send_fn`.

- AC 1: review.
- AC 2: `drawing.rs` unit — 0, 1, 1000, 1001 entities; missing/extra root
  key; `version` 2 and `"1"`. Dispatch-closure unit — an argument string of
  exactly 1 048 576 bytes (valid JSON padded with spaces) passes the cap, one
  byte more is refused with the cap message, for `create_drawing` and for
  `create_line`.
- AC 3: `drawing.rs` unit — unknown key (and a 200-char key name, asserting
  truncation to 64), missing key, wrong JSON type, `ccw` as string, `r` of 0
  and −3, invalid first/middle/last item each report the right index; the
  error string never contains a sentinel value placed elsewhere in the
  payload. A table test feeds the same radius values through `create_circle`
  and a one-item batch and asserts identical accept/reject (breaks if the
  helper is copied and drifts). Integration — an invalid batch yields one
  `refused` row, one step, unchanged revision; an entity at (−10000, −10000)
  is accepted.
- AC 4: `drawing.rs` unit — degree→radian conversion and both `ccw` values;
  a one-item batch equals the scalar `parse_tool_call` result for line,
  circle and arc.
- AC 5: integration — mixed batch on a drawing with 2 entities, a selection
  and a non-default bed/preset: entities 2.. are appended in order, the
  first two, selection, bed and preset unchanged; `CreateEntities` unit —
  undo truncates to the pre-append length.
- AC 6: integration — fenced batch (foreign commit before the `Act`) and a
  cancel with the `Act` still queued: entity count and revision unchanged.
- AC 7: integration — a 1000-item batch moves the revision by exactly 1 and
  the step count by exactly 1; a turn of scalar + batch + scalar undoes in one
  `Ctrl+Z`.
- AC 8: `agent_apply` unit — both outcome strings pinned character for
  character against a 3-entity drawing; `query_entities` after the batch
  lists them.
- AC 9: integration — undo/redo geometry equality, SVG export → import
  equality within the existing roundtrip tolerance.
- AC 10: `tools.rs` unit — `create_drawing` is last; a recursive walk of its
  schema finds none of the forbidden keywords; `maxItems == 1000`, `version`
  enum `[1]`. `tests/lcv122_source_scans.rs` covers `src/agent/drawing.rs`
  (the scan enumerates the directory; verify it is not a fixed file list).
- AC 11: source scan — the eight helper `fn`s are defined in
  `agent_narrate.rs` and not in `agent_apply.rs`; existing `agent_apply`
  narration tests stay green unchanged. LOC: review.

## Open questions

None. ADR 0010 closes the architecture gates and the limits.

## Notes

Primary files: new `src/agent/drawing.rs`, `src/agent/tools.rs`,
`src/agent/bridge.rs`, `src/agent/mod.rs`, new `src/app/agent_narrate.rs`,
`src/app/agent_apply.rs`, `src/document/commands/create.rs`. Implemented
after LCV-142 (§D12 group, §D15 `Malformed`) and LCV-143 (its built-in prompt
already names `create_drawing`). This tool never calls the human `submit`,
`classify` or synthetic keys.

```json
{
  "version": 1,
  "entities": [
    {"type": "line", "x1": 10, "y1": 10, "x2": 40, "y2": 10},
    {"type": "circle", "cx": 25, "cy": 25, "r": 5},
    {"type": "arc", "cx": 40, "cy": 25, "r": 10,
     "start_deg": 0, "end_deg": 90, "ccw": true}
  ]
}
```

## Architecture decision

Recorded 2026-09-27 in
[ADR 0010](../../adr/0010-declarative-drawing-batch-tool.md) (satisfies AC 1's
gate; confirms the 1000-entity / 1 MiB limits). Builds on ADR 0007 §D12
(flat group), §D13 (step = `Act`) and §D15 (malformed → `Refused` `Act`) from
LCV-142.
