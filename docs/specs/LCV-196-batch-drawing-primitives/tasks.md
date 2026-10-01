# LCV-196 — Tasks

Prerequisites: LCV-185, LCV-192 Done; LCV-188 Done (the outcome's `New ids` suffix).

- [x] T1 Refactor (no behaviour change): move `ENTITY_KEYS` into `drawing/keys.rs` as a typed key
  table that `parse` and `schema` both read; the tests are unchanged (files: src/agent/drawing.rs,
  src/agent/drawing/keys.rs, src/agent/drawing/schema.rs)
- [ ] T2 [AC4] Test, then `text_strokes` plus `layout_text` as a map over it and the shared
  `DEFAULT_SPACING_FACTOR` (files: src/text/layout.rs, src/tools/text.rs)
- [ ] T3 [AC1][AC2][AC3][AC4][AC8] Test (parser), shapes, one case per rule:
  - `polyline` open and closed;
  - `rect` without a radius, with a radius, and with a radius of half the short side (two arcs
    meet, no zero-length sides);
  - a 6-sided `polygon` with `start_deg` 30;
  - `text` "AB" equals `layout_text`'s lines.

  Refusals at their exact paths:
  - a repeated point;
  - `closed` with 2 points;
  - a closing point equal to the first;
  - an oversize `corner_radius`;
  - `sides` 2 and 65;
  - text empty, 257 chars, or containing `\n`;
  - a null other-type key tolerated.

  (files: src/agent/drawing/tests.rs)
- [ ] T4 [AC1][AC2][AC3][AC4][AC8] `drawing/items.rs`: `Shape`, per-type parse; `expand.rs` for
  the shapes (files: src/agent/drawing/items.rs, src/agent/drawing/expand.rs,
  src/agent/drawing.rs)
- [ ] T5 [AC5][AC6][AC7][AC8] Test (parser), arrays:
  - `linear_array` of two items, count 3 → items plus 4 copies in batch order;
  - `polar_array` of a line and an arc by 90° → rotated endpoints and arc angles;
  - a grid = an array of an array.

  Refusals at their exact paths:
  - `of` naming itself, a later item or a 3rd-level array;
  - `of[1]` duplicated;
  - `count` 1 and 1001;
  - an expansion of 1001 → refused at `entities`, without allocating (1000×1000);
  - text of spaces only → refused at `entities`.

  (files: src/agent/drawing/tests.rs)
- [ ] T6 [AC5][AC6][AC7][AC8] Arrays in `expand.rs`: output ranges, the `of` closure, the depth
  check, the count before allocation, and the copies via `Transform::Rotate`
  (files: src/agent/drawing/expand.rs, src/agent/drawing/items.rs)
- [ ] T7 [AC9] Test (headless turn): a batch mixing `rect` and `polar_array` is one step, one undo
  and one revision. The outcome reports the expanded count, the `a..=b` range and the new-id
  suffix (files: tests/it/agent/drawing_batch.rs)
- [ ] T8 [AC4] Test (headless app): `text` in a batch and the `TEXT` command at the same point,
  height and string create identical entities (files: tests/it/agent/drawing_batch.rs)
- [ ] T9 [AC6] Test: a polar copy of a CCW arc keeps `ccw`, and its endpoints equal the rotated
  source endpoints within `EPSILON` (files: src/agent/drawing/tests.rs)
- [ ] T10 [AC10] Test: the schema lists the 9 types and every new key with its JSON type (`points`
  items `{x, y}`, `of` integer array). A recursive scan finds no
  `oneOf`/`anyOf`/`allOf`/`const`/`additionalProperties`. The description names each type's keys
  (files: src/agent/tools/tests.rs)
- [ ] T11 [AC10] `drawing/schema.rs` from the key table (files: src/agent/drawing/schema.rs)
- [ ] T12 [AC10] Test, then the `DEFAULT_PROMPT` `create_drawing` paragraph covering the new types,
  `of` and the expanded cap (files: src/agent/prompt.rs, tests/it/agent/default_prompt.rs)
- [ ] T13 Docs: AGENTS.md purity list (`drawing/*.rs`); CHANGELOG line (files: AGENTS.md,
  CHANGELOG.md)
- [ ] T14 `scripts/mutants.sh` on the diff (`src/agent/drawing*`, `src/text/layout.rs`). Kill the
  survivors or justify them in the commit body.
- [ ] T15 Docs: append plan.md's "ADR amendment" to ADR 0010 as the next free `Amended (n)`
  (files: docs/adr/0010-declarative-drawing-batch-tool.md)
