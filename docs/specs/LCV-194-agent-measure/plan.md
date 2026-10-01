# LCV-194 — Plan

## Approach

The geometry goes in the kernel. `src/geometry/distance.rs` finds the closest points of two bounded
primitives (point, line, circle, arc). If they intersect, the distance is 0. Otherwise the minimum
is taken over a set of candidates:

- each endpoint of one primitive against the other primitive;
- for a pair with a circle or an arc, the mutual-normal points on the line through the centres
  (circle or arc pairs) or through the foot of the perpendicular (line–curve pairs);
- each candidate is kept only when it lies inside the arc's span.

`src/geometry/overlap.rs` detects a shared segment or a shared arc span.

The agent side follows `check_drawing` (LCV-190), and the operand rules live in the parser:

- `measure` parses in `src/agent/tools/measure.rs` into `AgentAction::Measure(MeasureRequest)`.
- The parser checks the operand counts per query (AC8).
- `src/app/agent_apply/measure.rs::answer` resolves the operands:
  - indices go through `set.rs::out_of_range`;
  - ids go through LCV-188's `set.rs::resolve`.
- It checks entity kinds: `angle` takes lines only.
- It formats the answer as `Planned::Answer`.

AC7 holds by construction. An answer moves no revision, so `agent_poll.rs::apply_fenced` counts a
step and advances no fence.

Lands after LCV-188 (ids, `resolve`), LCV-190 (`check_drawing` is the registry neighbour) and
LCV-192 (refusal shape).

Answer formats, pinned character for character in the tests. Every number is `{:.3}`, with `-0`
normalised to `0`:

- `distance: 5.000 mm, dx 3.000, dy 4.000, from (0.000, 0.000) to (3.000, 4.000)`
- `length: 31.416 mm`
- `bbox: min (0.000, 0.000), max (40.000, 20.000), width 40.000 mm, height 20.000 mm` or
  `bbox: the drawing is empty`
- `intersections: (10.000, 0.000); (20.000, 0.000)` or `intersections: none` or
  `intersections: overlap`
- `angle: 270.000° counter-clockwise from the first line to the second; 90.000° between the lines`

Distance points are listed in operand order: points first, then entities. dx and dy are the second
point minus the first. A directed angle that rounds to 360.000 prints 0.000. A zero-length line has
no direction, and `angle` refuses it.

## Touches

- `src/geometry/distance.rs` (new): `closest(a: Prim, b: Prim) -> (Vec2, Vec2)`, where
  `Prim = Point | Line | Circle | Arc`.
- `src/geometry/overlap.rs` (new): `overlaps(a, b) -> bool`.
- `src/geometry/mod.rs`: the `pub use` lines.
- `src/agent/bridge/action/measure.rs` (new, kernel-pure): `MeasureQuery`, `MeasureRequest
  {query, points, targets}`.
- `src/agent/bridge/action.rs`: the `Measure` variant and its `tool_name` arm.
- `src/agent/bridge.rs`: re-export.
- `src/agent/tools/measure.rs` (new, kernel-pure): `parse`.
- `src/agent/tools.rs::parse_tool_call`: the arm. The registry inserts `measure` after
  `check_drawing`; `create_drawing` stays last.
- `src/agent/tools/schema.rs::base_definitions`: the flat `measure` schema:
  - `query` is a string enum;
  - `points` is an array of `{x, y}`;
  - `indices` is an integer array and `ids` a string array;
  - no `oneOf`.
- `src/agent/tools/args.rs::expected_form`: rows for `query`, `points` and `points[k].x`.
- `src/app/agent_apply/measure.rs` (new): `answer(&MeasureRequest, &Document) -> Planned`.
- `src/app/agent_apply.rs::plan`: one arm.
- `src/agent/prompt.rs::DEFAULT_PROMPT`: the `measure` line.
- `AGENTS.md`: the purity list gains `bridge/action/measure.rs` and `tools/measure.rs`.
- ADRs: none. This is a read-only tool on the existing `query_entities` path (ADR 0007 §D2a, §D14).

## Risks

- LOC cap:

  | File | LOC after LCV-188 | Change | Result |
  |---|---|---|---|
  | `bridge/action.rs` | ~232 | +~6 | under 240 |
  | `agent_apply.rs` | ~235 | +~3 | 238 |
  | `tools.rs` | 168 | +3 | 171 |
  | `tools/schema.rs` | 93 | +~25 | ~118 |

  The formatting stays in `agent_apply/measure.rs`, not `agent_narrate.rs`. No seam needed.
- Mutation testing: **yes**. Targets:
  - `src/agent/tools/measure.rs`: the operand counts and the exclusivity of `indices` and `ids`;
  - `geometry/distance.rs`: the candidate set and the span filter. A missing candidate gives a
    plausible but wrong minimum, which only a mutant run finds.
- Near-tangent and near-parallel cases use `EPSILON`. The T1 tests pin:
  - a tangent line and circle give distance 0 and one intersection;
  - parallel segments give the endpoint–segment distance;
  - concentric circles give |r1 − r2|.
- `overlap` comes before the point list. Two collinear segments that touch end to end are one point,
  not an overlap; T3 pins it.
- Text formatting: `-0.000` is avoided by `+ 0.0`; T8 pins it.
