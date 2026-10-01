# LCV-196 — Plan

## Approach

Expansion happens in the parser, so `AgentAction::CreateDrawing { items: Vec<DrawingItem> }`, the
apply arm, the single `CreateEntities` commit and the outcome sentence do not change. AC9 holds by
construction: the count is the expanded `items.len()`, and LCV-188 adds the `New ids` suffix.

`drawing.rs` (229 LOC) is split into three files:

- `drawing/keys.rs`: the per-type key table (name, kind, optional). Both the parser and
  `drawing/schema.rs` read it, so the schema and the LCV-185 null-tolerance rule cannot drift
  apart.
- `drawing/items.rs`: parses each new type into a private `Shape`.
- `drawing/expand.rs`: turns each `Shape` into `DrawingItem`s.

Item `i`'s output range is recorded, so an array can copy it. Expansion follows the AC rules:

- **Arrays.** `of` is resolved to the closure of base items: a listed array brings its own `of`,
  plus itself. Duplicates are refused like `indices` duplicates. Copy k:
  - `linear_array` copies the outputs in batch order, translated by k·(dx, dy);
  - `polar_array` rotates them by k·`step_deg` about (cx, cy), arcs' angles included, through
    `geometry::Transform::Rotate`.
- **Count before allocation.** The expanded count is computed from the shapes first, and the batch
  is refused when it is over 1000 or 0. A 1000×1000 array never allocates.
- **Text.** It goes through a new kernel `text::layout::text_strokes(text, origin, height,
  spacing) -> Vec<(Vec2, Vec2)>`. `layout_text` becomes a map over it. `DEFAULT_SPACING_FACTOR`
  moves to `text::layout`, and `tools/text.rs` uses it, so AC4 parity is structural.
- **Rect corners.** They are CCW quarter arcs centred inset by `corner_radius`. A side of zero
  length (radius = half the short side, within `EPSILON`) is omitted.
- **Polygon.** It has `sides` lines; vertex k is at `start_deg` + k·360/`sides`.

Decisions (self-approved per user goal):

- An array copies the *whole output* of what it lists: a listed array means its sources plus its
  copies. So a grid is `linear_array {of:[row_array]}`, where `row_array` lists the hole.
- `closed: true` with 2 points is refused, and so is a closing point equal to the first. Both are
  "repeated consecutive point" in the cyclic sense.
- A batch whose expansion draws nothing (for example, text of spaces only) is refused at
  `entities`.
- `sides` and `count` must be integers; `of` entries are integer item indices.

## Touches

- `src/agent/drawing.rs`: `parse` drives `items` → `expand`. `ENTITY_KEYS` moves out, and the
  doc comment is updated.
- `src/agent/drawing/keys.rs` (new): the type list and the per-type key table (`Num`, `Int`,
  `Bool`, `Str`, `Points`, `IndexList`; optional flag).
- `src/agent/drawing/items.rs` (new): `Shape` and the per-type parse, refusing at
  `entities[i].<key>` and `entities[i].points[k].x` in the LCV-192 shape.
- `src/agent/drawing/expand.rs` (new): the output ranges, the closure of `of`, the count and the
  copies.
- `src/agent/drawing/schema.rs::schema`: `type` enum of 9 types, typed properties from `keys.rs`
  (`points` items `{x, y}`, `of` integer array), and the item description listing each type's
  keys.
- `src/text/layout.rs`: `text_strokes`, `DEFAULT_SPACING_FACTOR`.
- `src/tools/text.rs`: uses the shared constant.
- `src/agent/prompt.rs::DEFAULT_PROMPT`: the `create_drawing` paragraph names the new types, `of`
  by position, and the cap counted after expansion.
- `AGENTS.md`: the purity list gains `drawing/*.rs`.
- ADRs: ADR 0010, see "ADR amendment" below.

## ADR amendment

T15 appends this to ADR 0010's header as the next free `Amended (n)`:

> **Amended (n)**: <date> — LCV-196: §2 and §3.3 gain six item types: `polyline {points, closed}`,
> `rect {x, y, width, height, corner_radius}`, `polygon {cx, cy, r, sides, start_deg}`,
> `text {x, y, height, text}`, `linear_array {of, count, dx, dy}` and
> `polar_array {of, count, cx, cy, step_deg}`.
>
> - **Expansion.** `drawing.rs` expands them into §4's `Line|Circle|Arc` before the `Act` exists.
>   The §3.2 cap of 1000 applies to the expanded count, which is computed before any copy is made.
>   §4's DTO is unchanged.
> - **Kernel calls.** Expansion may call the kernel's `crate::geometry` and `crate::text` (both
>   kernel-pure). No document type crosses into `src/agent/`, and text goes through
>   `text::layout::text_strokes`, the function `TEXT` uses.
> - **Arrays.** `of` references earlier items by batch position. A listed array contributes its
>   whole output, two levels at most.
> - **Unchanged.** `version` stays 1, since every earlier payload stays valid. §2's schema rules
>   are unchanged (still no `oneOf`/`anyOf`/`allOf`/`const`/`additionalProperties`), and §1's
>   one dispatch = one command = one revision holds.

## Risks

- LOC cap:
  - `drawing.rs` 229 → ~150 after the move.
  - `items.rs`, `expand.rs` and `keys.rs` are each ≤200; split `items.rs` per family if it passes
    270.
  - `drawing/schema.rs` 41 → ~90. `text/layout.rs` +~10.
- Mutation testing: **yes**. Targets:
  - the boundaries of the item rules: the 2/1000 points, 3/64 sides, 2/1000 count, 256 chars and
    1000 expanded;
  - the depth rule and the `of` closure;
  - the zero-side omission;
  - the cyclic repeated point.
- Arc orientation under arrays: `Transform::Rotate` on the CCW `(start, end)` keeps `ccw`. T9
  checks the end points of a rotated arc copy numerically.
- The schema stays provider-safe. T10 extends the existing scan of the forbidden keywords to the
  new nested `points` items.
- Optional keys: a rect with `corner_radius` 0 or null makes 4 lines; `start_deg` null means 0 for
  `polygon` only. Arc's `start_deg` stays required. The key table carries an optional flag per
  type, not per key, and T3 pins this.
