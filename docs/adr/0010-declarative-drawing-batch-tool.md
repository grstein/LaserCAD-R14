# ADR 0010 — `create_drawing`: one validated JSON batch is one dispatch, one command, one revision

- **Status**: Accepted
- **Amended (1)**: 2026-09-29 — [ADR 0012](0012-document-layers-and-per-layer-export.md) §6
  (LCV-156): the root key set is `{version, entities}` plus an optional `layer` string naming an
  existing layer; the whole batch lands on it (else on the current layer). §5's "export preset"
  no longer exists. Nothing else changes.
- **Amended (2)**: 2026-09-30 — LCV-185: an item's key set is that type's fields plus `type`;
  another type's field (one the flat schema of §2 publishes) is tolerated only when `null`, and
  refused with `not a <type> key; a <type> takes <keys>` otherwise. A key no type publishes stays
  `unknown key`. The item properties of §2 are built from the per-type key lists. §2's schema rules
  are unchanged.
- **Amended (3)**: 2026-09-30 — LCV-192: every error of the **Errors** paragraph below ends
  `; expected <form>` (ADR 0007 Amended (12)), and the whole argument string is the path
  `(root)` (was `arguments`). The path, the 64-character key cut and the no-echo rule are
  unchanged.
- **Date**: 2026-09-27
- **Deciders**: architect (LCV-144; in the 1.0 scope by the 2026-09-27 scope
  decision recorded in `PLAN.md`)

## Context

LCV-144 wants one agent tool that appends many lines, circles and arcs at once,
atomically. The constraints that bind it are already written down:
[ADR 0007](0007-agent-turn-mutates-the-live-document.md) §D1 (the worker holds
no document state; `tests/lcv122_source_scans.rs` keeps every document type out
of `src/agent/`), §D2a (shape checks in the worker, hand-rolled, with per-field
messages; document checks at the apply site), §D12 (a turn is one flat history
group), §D13 (a step is one `Act`) and §D15 (a malformed call is a `Refused`
`Act`, not a dead turn). AGENTS.md §"Units and types": millimetres canonical,
radians in the kernel, degrees only at a presentation boundary.

## Decision

**1. Unit of work.** `create_drawing` is one tool call = one step = one `Act` =
one `Command` = one revision bump, whatever the entity count. It is always
advertised (no setting).

**2. Advertised schema: a provider-compatible hint. Code is the contract.**
Root `{version: integer, enum [1]; entities: array, minItems 1, maxItems 1000}`,
both required. Each item is one flat object: `type` (string, enum
`line|circle|arc`, required) plus every field any type uses (`x1 y1 x2 y2 cx cy
r start_deg end_deg` number, `ccw` boolean) as optional properties; which fields
each `type` requires is said in the item's `description`. No `oneOf`/`anyOf`/
`const`/`additionalProperties`: `tools.rs` has shipped only `type` /
`properties` / `required` / `description` so far, and several OpenRouter
providers reject the rest. Strictness lives in the parser, not the schema.

**3. Validation — pure, hand-rolled, complete before anything is sent.** New
file `src/agent/drawing.rs` (kernel-pure under §D8's `src/agent/` rules; names
no document type). Order:

1. **Byte cap before parsing.** `MAX_TOOL_ARGUMENT_BYTES = 1_048_576` on the
   raw argument string, checked in the dispatch closure **for every tool**
   before `serde_json` sees it. One universal cap; the scalar tools never
   approach it.
2. **Root.** An object whose key set is exactly `{version, entities}`;
   `version` is the integer 1; `entities` is an array of 1..=1000.
3. **Each entity.** An object; `type` ∈ {`line`, `circle`, `arc`}; its key set
   is that type's fields plus `type`; another type's field is tolerated only
   when `null` (Amended (2)) — unknown or missing keys are errors; numbers finite; `ccw` boolean; `r` through the **same** positive-
   finite helper `create_circle` / `create_arc` use (shared with `tools.rs`,
   not copied).
4. **Units.** mm in, mm out. `start_deg` / `end_deg` are converted to radians
   **once, here** — the same boundary `tools.rs` uses for `create_arc`.
   `ccw` has `create_arc`'s Y-up meaning.

**A batch of one is exactly the scalar call.** No rule a scalar tool lacks, none
it has missing. Therefore: **no bed-bounds check** (no scalar or human tool has
one; the bed is document state that can change mid-turn; LaserGRBL export does
not need it — a model that wants to stay on the bed reads it from
`query_entities`), no zero-length-line rejection, no magnitude cap. Any such
rule, if ever wanted, is added to the shared helper and binds both paths.

**Errors** stop at the first failure and name its path:
`create_drawing entities[17].r: -3 is not a positive finite number`. A new
`ToolCallError` variant carries `(index, field, reason)`. The payload is never
echoed; an unknown key's name is echoed truncated to 64 characters. Under §D15
the error is a `Malformed` `Act` → `Refused` → tool result + transcript row;
nothing applied, revision unchanged.

> **Amended (3), 2026-09-30 (LCV-192).** The error gains the expected form:
> `create_drawing entities[17].r: -3 is out of range; expected a positive
> number in mm`. One `ToolCallError::Arg { tool, path, reason, expected }`
> replaces the per-site variants; the root of the arguments is `(root)`.

**4. DTO.** `AgentAction::CreateDrawing { items: Vec<DrawingItem> }`, with
`DrawingItem { Line{x1,y1,x2,y2} | Circle{cx,cy,r} | Arc{cx,cy,r,start,end,ccw} }`
declared in `drawing.rs` and re-exported through `bridge`/`mod.rs`. Plain `f64`
fields, radians for arcs — the same shape as the scalar `AgentAction` variants,
and no geometry or document type crosses into `src/agent/`.

**5. Command.** `document::commands::CreateEntities` (name indicative), in
`create.rs` (200 LOC now; a sibling file only if it would cross 270): `do_`
records the pre-append length and appends every entity in order; `undo`
truncates back to it. It mirrors `CreateLine`: entities only — selection, bed
size and export preset are untouched. `DrawingItem → Entity` conversion happens
at the apply site (`src/app/`), which commits through
`History::commit_grouped` (§D12): the batch joins the turn's flat group as one
command, no per-entity composite.

**6. Atomicity is structural.** Validation is complete before the `Act` exists;
the apply is one command. A fence refusal (§D14) or a cancel before the frame
applies it leaves nothing behind and does not move the revision.

**7. Outcome.** From the live apply site after the commit: `Created N entities
(indices a..=b). The drawing now has M entities. Revision R.` Exact words are
`product-owner`'s; the numbers are the document's.

**8. Registration.** `tool_definitions()` appends `create_drawing` last (the
order comment in `tools.rs` moves with it); `parse_tool_call` gains one arm that
delegates to `drawing.rs`. The schema fragment is built in `drawing.rs` too, so
`tools.rs` (202 LOC) grows by the arm and the registration only.

**9. Seam.** `src/app/agent_apply.rs` is at 284. LCV-144 first moves its
narration helpers into `src/app/agent_narrate.rs` (ADR 0007 §D8, amendment 7),
then adds its arm.

## Consequences

- A 1000-entity drawing costs one step and one frame, and undoes with the rest
  of the turn in one `Ctrl+Z`.
- The assistant message carrying the batch stays in the turn's message list and
  is resent on each later request of that turn. Accepted: 1000 entities is
  ~60 KB; 1 MiB is a safety cap far above any model's output limit, and a
  provider context overflow already ends the turn as a transport error.
- LCV-144 depends on LCV-142 for §D12 (group) and §D15 (malformed → tool
  result). The implementation order already has 142 first.

## Alternatives considered

- **`serde` derive with `deny_unknown_fields`** — rejected for ADR 0007
  §Alternatives' reason: the messages lose the entity index and the field domain.
- **`oneOf` per type in the advertised schema** — precise, and rejected by
  enough providers to make the tool vanish on them.
- **One `Act` per entity, grouped** — 1000 steps and 1000 frames for one
  intention, and not atomic against a mid-batch fence.
- **Bed-bounds rejection** — a document check the scalar tools do not make;
  inconsistent, and the bed can change under the turn.
- **Command-line script / mini-language** — the user chose JSON entities
  (LCV-144 §Problem).
