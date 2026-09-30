# LCV-185 — Batch drawing schema matches its validator

- **Status**: Specified
- **Depends on**: none
- **Implementation**: -

## Problem

`create_drawing` publishes one flat item schema (`src/agent/drawing.rs::schema`): every line,
circle and arc key sits in the same `items.properties`, with only `type` required. The validator
(`drawing.rs::item`) allowlists keys per `type` and refuses any other key as "unknown key". In an
agent session (three views of a Beetle, 2026-09-30) the model filled the keys of other types, got
the same refusal again and again, and fell back to hundreds of scalar calls. No test proves that a
payload shaped by the published schema is accepted. ADR 0010 §2 keeps the schema provider-safe
(no `oneOf`/`additionalProperties`), so the fix must keep that rule.

## Stories

- As an operator, I want the agent's batch drawing calls to succeed on the first well-formed try
  so that a drawing costs few calls and little time.
- As a developer, I want a test binding the published schema to the validator so that they cannot
  drift apart again.

## Acceptance criteria

A *foreign key* is a key the schema publishes for another entity type (e.g. `r` on a line).

1. WHEN an item carries a foreign key whose value is `null`, THE SYSTEM SHALL ignore that key and
   validate the item by its own `type`.
2. IF an item carries a foreign key with a non-null value THEN THE SYSTEM SHALL refuse the batch
   with `create_drawing entities[i].<key>: not a <type> key; a <type> takes <its keys>`.
3. IF an item carries a key the schema does not publish THEN THE SYSTEM SHALL refuse the batch
   with the current `unknown key` message, unchanged.
4. WHEN a test builds, for each `type`, an item that sets every published property (the item's own
   keys valid, all foreign keys `null`), THE SYSTEM SHALL accept it and draw that one entity.
5. WHEN a test compares `drawing.rs::schema` with the per-type key lists, THE SYSTEM SHALL show
   that every published item property belongs to at least one type and every type key is published.
6. WHEN the schema is published, THE SYSTEM SHALL still contain no `oneOf`, `anyOf`, `allOf`,
   `const` or `additionalProperties` (ADR 0010 §2).
7. WHEN the built-in prompt describes `create_drawing`, THE SYSTEM SHALL state that keys of other
   types may be omitted or `null`, and SHALL match the schema (`tests/it/agent/default_prompt.rs`).

## Out of scope

- A per-type union schema (breaks the provider-safety rule of ADR 0010 §2).
- New entity types in the batch (ellipses are LCV-176's open question).
- Changing the scalar `create_line`/`create_circle`/`create_arc` tools.

## Open questions

- None. Only `null` is tolerated (proposed option; self-approved per user goal 2026-09-30).
