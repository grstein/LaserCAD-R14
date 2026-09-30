# LCV-185 — Batch drawing schema matches its validator

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

`create_drawing` publishes one flat item schema (`src/agent/drawing.rs::schema`): every line,
circle and arc key sits in the same `items.properties`, with only `type` required. The validator
(`drawing.rs::item`) allowlists keys per `type` and refuses any foreign key as "unknown key". In an
agent session (three views of a Beetle, 2026-09-30) the model filled keys of other types, got the
same refusal repeatedly and abandoned the batch tool for hundreds of scalar calls. No test proves
that a payload shaped by the published schema is accepted by the validator. ADR 0010 §2 keeps the
schema provider-safe (no `oneOf`/`additionalProperties`), so the fix must respect that rule.

## Stories

- As an operator, I want the agent's batch drawing calls to succeed on the first well-formed try
  so that a drawing costs few calls and little time.
- As a developer, I want a test binding the published schema to the validator so that they cannot
  drift apart again.

## Acceptance criteria

To be written by /specify.

## Out of scope

- New entity types in the batch (ellipses are LCV-176's open question).
- Changing the scalar `create_line`/`create_circle`/`create_arc` tools.

## Open questions

- Per-type discriminated schema (if all configured providers accept it) or a validator that
  tolerates null/absent foreign keys while still refusing unknown names?
- Should the refusal list the keys expected for the item's `type`? (overlaps LCV-192)
