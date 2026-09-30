# LCV-185 — Plan

## Approach

`drawing.rs::item` keeps its per-type key lists (`LINE_KEYS`, `CIRCLE_KEYS`, `ARC_KEYS`) and
splits today's single "unknown key" refusal in three: a key published for another type with a
`null` value is skipped (AC1); the same key with a non-null value is refused with
`not a <type> key; a <type> takes <keys>` (AC2); a key no type publishes keeps the current
`unknown key` reason (AC3). The published schema stays the flat object (ADR 0010 §2); new
tests prove it equals the union of the key lists and still carries no union keywords (AC5–6).
The prompt's create_drawing paragraph says other types' keys may be omitted or null (AC7).

## Touches

- `src/agent/drawing.rs::item` — three-way key check; `ALL_KEYS` helper (union of the lists).
- `src/agent/drawing.rs::schema` — build item properties from the key lists (one source).
- `src/agent/prompt.rs::DEFAULT_PROMPT` — create_drawing paragraph: "keys of other types may be
  omitted or null"; drop "exactly that type's other keys".
- ADRs: ADR 0010 §2 amended — "exactly that type's fields plus type" becomes "that type's fields
  plus type; another type's field is tolerated only when null".

## Decisions (self-approved per user goal)

- Only JSON `null` is tolerated; `0`, `""`, `false` on a foreign key are refused (spec open question).
- `<keys>` in the AC2 reason lists the type's keys in schema order, comma-separated, no `type`.

## Risks

- LOC cap: `drawing.rs` is at 266. Seam if it passes 270: move `schema`, `layer_schema` and the
  key lists to `src/agent/drawing/schema.rs` (kernel-pure; add it to the AGENTS.md purity list).
- Mutation testing: yes — `src/agent/` is high-risk; the null/non-null/unpublished branches
  must each be killed by a test.
- A too-lax check would let a wrong-type value slip through: AC2 test uses a non-null foreign key
  on each of the three types.
- Cross-spec overlap: `prompt.rs` is also edited by LCV-189, 186 and 187 (land in order).
