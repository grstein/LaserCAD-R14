# LCV-159 — Plan

## Approach

Polar entry is **one parser arm with no new variant**. In `cmdline/parse.rs::parse`, text
holding `<` goes through a new `parse_polar(s) -> Option<Vec2>`. `@d<a` becomes the
existing `CommandInput::Relative(d·(cos a, sin a))` and `d<a` becomes `CommandInput::Point(…)`.
The app's existing dispatch then does the rest: the anchor is added, a missing anchor
gets `NO_BASE_POINT` (AC4), and typed input bypasses snap. ADR 0003's revisit criteria already
name polar entry as "one parser arm; none reopens this ADR". Degrees exist only inside the
parser, which is the text boundary. What leaves it is a Vec2 in mm, so no degree value reaches
state. Multiples of 90° use exact unit vectors, so `@10<90` yields (0, 10) exactly.

DIST is a new tool, `tools/dist.rs::DistTool`, with states Idle → WaitingSecond{first}.
`anchor()` returns the first point, so `@`/polar input works. It never touches `Document`
or `History`. Its result travels through a new object-safe default method
`Tool::take_message(&mut self) -> Option<String>`, a single-shot sibling of `take_successor`.
It is drained in `app/viewport.rs::poll_successor` *before* the successor is installed, so
the pointer path and the typed path share one body. The drained text goes into
`app.command_feedback`. `take_successor` returns `SelectTool` once the result is ready. The
angle is shown in degrees in [0, 360), CCW from +X, and every value has 3 decimals, with no
`-0.000`.

## Touches

- `src/cmdline/parse.rs::{parse, parse_polar}`: the polar arm, placed before the `,` and number
  branches (187 → ~205 impl LOC).
- `src/cmdline/mod.rs::ToolKind`: new `Dist` variant; doc tables gain `dist`/`di`.
- `src/cmdline/parse.rs::tool_alias`: word rows `"dist" | "di"` (the word axis is open; the
  variant is the ADR-level part).
- `src/tools/dist.rs`: new `DistTool` (~110 impl LOC); `src/tools/mod.rs::make` gets the `Dist` arm and re-exports it.
- `src/tools/tool.rs::Tool::take_message`: default `None`. `src/tools/manager.rs::take_message`
  forwards it.
- `src/app/viewport.rs::poll_successor`: drains the message into `command_feedback` (+3 lines).
- `src/app/cmdline.rs`: **no change**, at 271 LOC, over the 270 mark. Polar adds no variant.
- ADRs: ADR 0003 amendment (5) (commit `868bb01`) admits `ToolKind::Dist`, the polar arm
  (no new `CommandInput` variant) and `Tool::take_message`.
- No toolbar button and no bare key for DIST, because the letter axis is closed.
- No agent tool: DIST is a query the agent can compute from `query_entities`.

## Risks

- LOC cap: `app/cmdline.rs` is at 271, so the plan keeps it untouched. `app/viewport.rs` is at 241
  and `tools/manager.rs` at 218; both have room.
- Mutation testing: no (not `agent/`, not `export.rs`, not `History`).
- `-0.000` and angle wrap (`@10<-45` → 315° in DIST). Unit tests pin the formatter on
  negative zero, 0°, 90°, 180° and 359.9995°.
- `<` inside an unknown word: the polar arm runs only after the tool, toggle, zoom and layer
  lookups, and anything it cannot parse is `Unknown(trimmed)`, the same as today.
- Message ordering: the message is drained before `set_tool`, so the SELECT hand-over cannot
  swallow the result. An end-to-end test through `submit_command` and a pointer test pin this.
