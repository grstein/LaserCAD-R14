# LCV-144 - Create a drawing from declarative JSON

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-122, LCV-123, LCV-142, LCV-143
- **Suggested agent**: architect
- **Suggested model**: opus
- **Implementation**: -

## Problem

One bounded declarative tool can construct a laser drawing more efficiently
than many primitive calls. Invalid batches must not leave partial geometry.
The user chose JSON entities, not command-line scripts or a new language VM.

## Scope

An append-only `create_drawing` agent tool accepting version-1 JSON lines,
circles and arcs; full validation followed by one atomic live command.

## Out of scope

Executable strings, loops, expressions, human command-line JSON routing,
replacement/deletion, implicit clearing, selection changes, file I/O, per-entity
presets, new geometry families or worker document snapshots.

## Acceptance criteria

1. Architect approves the pure bridge DTO, provider-compatible effective schema, shared scalar validation and atomic command ownership before implementation.
2. Require root `version: 1` and `entities`. Proposed delivery bounds are 1..1000 entities and at most 1 MiB UTF-8 arguments, checked before parsing. Reject unknown fields, missing fields, invalid types and unsupported versions.
3. All numeric fields must be finite; radii positive; reuse existing primitive validity rules. Validate the complete payload before any mutation and report the failing field/entity without echoing the whole payload.
4. Coordinates/radii are mm. Arc `start_deg`/`end_deg` are degrees, converted once to kernel radians; `ccw` uses existing Y-up semantics.
5. A valid, authorized, unfenced request appends all entities in order through one command/history commit. Existing entities, selection, bed size and export preset are unchanged.
6. Invalid/oversized/cancelled-before-apply/fenced requests append nothing and do not advance history revision.
7. One batch is one dispatch unit and one revision-changing command, independently bounded by entity/byte limits. Join the flat turn group without nesting a composite for each entity.
8. Return actual added count, inclusive zero-based added index range, resulting entity count and revision from the live apply site. Query tools immediately see the result.
9. Undo/Redo and SVG roundtrip preserve the exact expected geometry and existing LaserGRBL export contract.

## Expected tests

- AC 1-4: schema/validation review; 0/1/1000/1001 entities, exact byte limit/+1, unknown fields, malformed/non-finite values and both arc directions.
- AC 5-6: mixed batch on a nonempty drawing; invalid first/middle/last entity, cancellation and fence prove no partial append.
- AC 7: 1000 entities consume one dispatch/commit; mixed scalar/batch turns remain correctly grouped.
- AC 8-9: actual outcome/query agreement, whole-turn Undo/Redo and SVG import/export geometry.

## Open questions

Architect must approve DTO/schema, validation reuse and atomic append, and
confirm the proposed 1000-entity/1-MiB delivery limits before Ready.

## Notes

Primary files: `src/agent/tools.rs`, `src/agent/bridge.rs`,
`src/app/agent_apply.rs`, a focused pure drawing-payload module and document
command. This tool never calls human `submit`, `classify` or synthetic keys.
Each variant accepts only the fields shown below; all are required.

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
LCV-142. `product-owner` reconciles the ACs against ADR 0010 §Decision 1–9.
