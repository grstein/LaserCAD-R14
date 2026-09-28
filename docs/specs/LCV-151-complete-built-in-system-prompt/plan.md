# LCV-151 — Plan

## Approach

Text-only change on top of LCV-143: rewrite `src/agent/prompt.rs::DEFAULT_PROMPT` as a
static, sectioned English prompt (what LaserCAD is · units and Y-up world · tools ·
entity indices · command-line routing · fence · step budget · reply style) and replace
143's golden test with the new text. No templating: the tool list is written by hand and
kept honest by a test that walks `tool_definitions()` and fails if any tool name or any
schema property name is missing from that tool's paragraph. The standard messages are
quoted verbatim from the constants the code emits, and a test pins them as substrings.
`resolve`, `TurnConfig`, `turn_config` and the editor are untouched (LCV-143 AC 5).

## Content decisions (the code is the contract; user decision at plan approval)

- Tool arguments are the real schema names: `create_line {x1,y1,x2,y2}`,
  `create_circle {cx,cy,r}`, `create_arc {cx,cy,r,start_deg,end_deg,ccw}`,
  `delete_entity {index}`, `move_entity {index,dx,dy}`, `query_entities {}`,
  `query_selection {}` (AC 2).
- Index contract: entity indices are **zero-based and positional** (ADR 0007 §D5, the
  tool schemas) (AC 3).
- Fence: quotes `crate::app::AGENT_FENCE_REFUSAL` (the tool result the model reads) and
  `FENCE_STOP_PLACEHOLDER` (calls left in the batch). No `Fenced: revision mismatch`
  string exists in the crate; none is invented (AC 5).
- Budget: default `AGENT_STEP_BUDGET_DEFAULT` (256), range 1..=4096, one tool call = one
  step, a batch that would cross it is refused whole and the turn ends with
  `step budget exceeded (N tool calls per turn)`, shown to the operator only; the model
  gets no budget message (AC 6).
- Routing (ADR 0007 §D9 amendment 5): only `:` and `/ai` input reaches the model;
  everything else is a CAD command (AC 4).
- Keeps 143's content: mm, degrees in arc tools / radians in kernel, "if create_drawing is
  advertised", capture only if advertised, honesty and stop-on-fence rules.

## Touches

- `src/agent/prompt.rs::DEFAULT_PROMPT` — new text (~90 lines); golden test updated.
- `src/app/agent_worker.rs` `#[cfg(test)]` — AC 1, AC 5, AC 6 wire-level and substring tests (need the
  `pub(crate)` `FENCE_STOP_PLACEHOLDER` and `drive_turn`, so they live here).
- `tests/it/lcv151_default_prompt.rs` — tool/argument coverage and section needles.
- ADRs: none.

## Seams for later specs

- LCV-144: `tool_definitions()` gains `create_drawing` → the T1 coverage test fails until
  its paragraph lists its properties; 144 edits `DEFAULT_PROMPT` in the same task.
- LCV-145: `tool_definitions(vision: bool)`; T1's loop changes to iterate `[false, true]`
  (the spec's AC 3 wording) and `capture_canvas` gets its paragraph. The prompt stays
  static; the tool is described "if advertised", never interpolated from `TurnConfig`.

## Risks

- LOC cap: `prompt.rs` ~60 → ~150 impl LOC (the const is implementation). Fine; if it
  ever nears 270, `include_str!("prompt.txt")` is the seam.
- Mutation testing: no — no logic changes, only a string constant and content tests.
- Substring coverage for short names (`r`, `x1`) is vacuous against the whole prompt; the
  test scopes each check to the tool's own paragraph and matches whole words.
- AC 8: the exact text is authored by the implementer in T4; T6 presents it to the user,
  who reviews it word for word before the spec is marked Done.
