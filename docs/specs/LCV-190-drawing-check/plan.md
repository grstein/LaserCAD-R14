# LCV-190 — Plan

## Approach

One kernel function, `document/check.rs::check_drawing(&Document) -> CheckReport`, finds the problems once.
Every consumer prints the same `CheckReport::lines()`. The operator reaches it through the `check` word or a rail
button, and the agent through the `check_drawing` tool. It reads only, so nothing is committed, and the agent
side is a `Planned::Answer` like `query_entities`. The revision never moves, so the fence and the applied count
stay as they are (`agent_poll.rs::apply_fenced`), and the step is counted like any other action (§D13).

- **Scope:** entities on layers with Output on. Indices are document indices, zero-based, the same as
  `query_entities`.
- **Endpoints (AC2, AC3):** each line or arc that is neither degenerate nor a later duplicate gives two
  endpoints. An endpoint *meets* when another endpoint is within `EPSILON`. Endpoints that meet nothing are paired
  greedily by ascending distance while that distance is < 0.5 mm, and each pair is one gap. Unpaired ones are open
  ends. This makes a gap never also count as two open ends.
- **Duplicates (AC4):** entity *j* is a duplicate of the lowest *i < j* with the same geometry within `EPSILON`.
  Lines compare as unordered endpoint pairs. Circles compare centre and r. Arcs are first normalised to CCW (a CW
  arc swaps its start and end points), then compare centre, r and both endpoints. Endpoints are used, not angles,
  so wrap-around needs no special case.
- **Degenerate (AC5):** a line of length ≤ `EPSILON`, an arc with `sweep_angle() ≤ EPSILON`, or a circle or arc
  with r ≤ `EPSILON`.
- **Off-bed (AC6):** the entity's exact `bbox()` leaves `[0, bed_w] × [0, bed_h]` by more than `EPSILON`. Arc
  bboxes include their extreme points, so a bulge that crosses the edge counts.
- **Report (AC1, AC7):** summary lines come first, in the fixed kind order open end, gap, duplicate, degenerate,
  off-bed, and only for kinds with a count, for example `CHECK: 2 open ends`. One line per finding follows, in the
  same kind order and by ascending index. Coordinates use `{:.3}` mm. With nothing found, the only line is
  `CHECK: no problems found.`
- **Operator UI:** the command dock shows one line at a time and a history window was rejected (LCV-139, LCV-165
  out of scope). So `run_check` puts the summary lines, joined with `; `, in `command_feedback`, and opens a
  read-only **Check** window that lists every report line. The window is non-modal, has a Close button, reads no
  key (ADR 0002 §A6) and caps its body at 426 pt with a scroll area (ADR 0009). It shows a snapshot, and running
  `check` again refreshes it. A clean check opens no window.

## Touches

- `src/document/check.rs` (new, kernel via `document/`): `check_drawing`, `CheckReport`, `Finding`. Its tests go
  in `check/tests.rs`. `src/document/mod.rs` gets `mod check` and the re-export.
- `src/cmdline/mod.rs`: `CommandInput::Check`. `src/cmdline/parse.rs`: the word `check`.
- `src/app/check.rs` (new): `run_check`. `src/app/cmdline.rs`: one arm. `src/app/mod.rs` and `src/app/init.rs`:
  the field `check_report: Option<Vec<String>>`.
- `src/ui/check_dialog.rs` (new) and `src/ui/mod.rs`. `src/app/panels.rs::draw_dialogs`: one call.
- `src/ui/icons/modify.rs`: the `check` glyph. `src/ui/toolbar.rs::draw_toolbar`: a `CHECK` icon button in the
  bottom row next to `AI`, with the tooltip `Check — CHECK`. `TOOLS` does not change, because CHECK is not a tool.
- `src/agent/bridge/action.rs`: `AgentAction::CheckDrawing`, plus its `tool_name` arm (LCV-192).
  `src/agent/tools.rs`: the `"check_drawing"` arm, argument-free like the queries. `src/agent/tools/schema.rs`:
  the definition after `query_selection`. `src/app/agent_apply.rs::plan`: the answer arm.
  `src/agent/prompt.rs`: one tool line.
- `DESIGN.md` §7 (rail bottom row, Check window) and `CHANGELOG.md`.
- ADRs: none. This is a new read-only action inside the existing ADR 0007 rules. There is no new key and no
  module-boundary change.

## Decisions (self-approved per user goal)

- The report is shown in a read-only Check window with a one-line dock summary. There is no command-history pane,
  no live panel and no canvas marker.
- CHECK is a command, not a tool: it has no `ToolKind` and no Tools-menu row. The rail button sits beside `AI`.
- The only command word is `check`. No letter is added (ADR 0003 §A2a closed axis).
- Circles and arcs with zero radius also count as degenerate. Degenerate entities and later duplicates stay out of
  the endpoint analysis, so a doubled line is not hidden as "closed".
- Gaps pair only endpoints that meet nothing, nearest first. A T-junction end is an open end.
- Location per kind: the point for an open end, the midpoint and width for a gap, the start point or centre of
  the later entity for a duplicate, the entity's first point for a degenerate entity, and its bbox for an off-bed
  entity.

## Risks

- LOC cap: `src/app/mod.rs` is at 287, and the new field adds 3 lines for 290. There is no seam, but LCV-190 is
  the last field that fits: the next one moves `check_report` and `shortcuts_open` into a `Dialogs` struct.
  `src/app/cmdline.rs` is at 271 and gains 1 line, so the handler lives in `app/check.rs`. `src/agent/tools.rs`
  is at 263, or less after the LCV-192 split, and gains 1 line. `bridge/action.rs` is at 237 and gains 3.
  `check.rs` is estimated at about 220 lines; if it passes 270, the formatting moves to `check/report.rs`.
- Order: this work is implemented after LCV-192, LCV-191 and LCV-187, and after `agent-harness` has been rebased
  onto the line that carries LCV-183. The rail glyph needs `ui/icons/`, which does not exist on this branch yet.
  The rebase also brings in `tool_name` (LCV-192), which needs the new arm.
- Mutation testing: yes, for `src/agent/` and for the kernel check (the tolerance comparisons, gap pairing,
  duplicate normalisation and bbox bounds).
- False open ends after save and reopen: SVG writes `{:.4}` and import rebuilds the arc centre. A closed
  line-and-arc contour that goes through `io::svg` export and import must report nothing (T9).
- Cost: endpoint and duplicate matching are O(n²), fine because the check runs on demand, never per frame.
