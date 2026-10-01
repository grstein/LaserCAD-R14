# LCV-190 — Tasks

Prerequisite: `agent-harness` rebased onto the line carrying LCV-183 (`ui/icons/`); LCV-192, 191, 187 Done.

- [x] T1 [AC2][AC3] Test: open ends and gaps — open L-polyline → 2 open ends; closed rectangle and
  line+arc fillet contour → none; 0.3 mm gap → one gap, no open ends; 0.6 mm → two open ends; ends
  within `EPSILON` meet; a T-junction end is open; entities on an Output-off layer are ignored
  (files: src/document/check/tests.rs)
- [x] T2 [AC2][AC3] `check_drawing`, `CheckReport`, `Finding`; endpoint collection, meet test,
  greedy nearest-first gap pairing (files: src/document/check.rs, src/document/mod.rs)
- [x] T3 [AC4] Test: duplicates — reversed line, same circle, CW arc vs its CCW twin, three copies
  → two findings against the lowest index, different span → none, a doubled open line still shows
  its open ends (files: src/document/check/tests.rs)
- [x] T4 [AC4] Duplicate detection with CCW-normalised arcs; later duplicates leave the endpoint
  analysis (files: src/document/check.rs)
- [x] T5 [AC5][AC6] Test: degenerate zero-length line, zero-span arc, zero-radius circle; off-bed
  line past the right edge, arc whose endpoints are inside but bulge crosses y=0, entity touching
  an edge exactly → not off-bed (files: src/document/check/tests.rs)
- [x] T6 [AC5][AC6] Degenerate and off-bed findings (files: src/document/check.rs)
- [x] T7 [AC1][AC7] Test: `CheckReport::lines()` — summary lines in kind order, singular/plural,
  zero-count kinds omitted, finding lines by kind then index with `{:.3}` mm, clean drawing →
  exactly `CHECK: no problems found.` (files: src/document/check/tests.rs)
- [x] T8 [AC1][AC7] `CheckReport::lines()`; move formatting to `check/report.rs` if `check.rs`
  passes 270 (files: src/document/check.rs, src/document/check/report.rs)
- [x] T9 [AC1][AC7][AC8] Test (headless app): typing `check` fills the dock with the joined summary
  and `check_report` with the lines; clean drawing → dock `CHECK: no problems found.`, no report;
  revision, selection, undo depth and dirty flag unchanged; a line+arc contour exported and
  re-imported through `io::svg` reports nothing (files: tests/it/cmdline/check_command.rs,
  tests/it/cmdline/mod.rs)
- [x] T10 [AC1] `CommandInput::Check` and the word `check` (parse test included) (files:
  src/cmdline/mod.rs, src/cmdline/parse.rs)
- [x] T11 [AC1][AC8] `App::check_report` field and its default (files: src/app/mod.rs,
  src/app/init.rs)
- [x] T12 [AC1][AC8] `app/check.rs::run_check` and the `CommandInput::Check` arm (files:
  src/app/check.rs, src/app/cmdline.rs)
- [x] T13 [AC1] Test (paint harness): the Check window paints every report line, its body stays
  ≤426 pt with a long report, Close clears `check_report` (files: tests/it/ui/check_dialog.rs,
  tests/it/ui/mod.rs)
- [x] T14 [AC1] `ui/check_dialog.rs::check_dialog` and its `draw_dialogs` call (files:
  src/ui/check_dialog.rs, src/ui/mod.rs, src/app/panels.rs)
- [x] T15 [AC1] Test (paint harness): the rail paints a CHECK icon button beside `AI`, tooltip
  `Check — CHECK`; clicking it runs the check (files: tests/it/ui/icon_tool_rail.rs)
- [ ] T16 [AC1] `check` glyph and the rail button (files: src/ui/icons/modify.rs, src/ui/toolbar.rs)
- [ ] T17 [AC9] Test: `check_drawing` parses to `CheckDrawing` whatever its arguments; the tool list
  order includes it after `query_selection`; in a turn its result equals
  `check_drawing(&doc).lines().join("\n")`, counts one step, leaves the revision, and a following
  mutation in the same turn is not fenced (files: tests/it/agent/check_drawing.rs,
  tests/it/agent/mod.rs, src/agent/tools/tests.rs)
- [ ] T18 [AC9] `AgentAction::CheckDrawing` (+ `tool_name` arm), `"check_drawing"` arm, schema entry
  (files: src/agent/bridge/action.rs, src/agent/tools.rs, src/agent/tools/schema.rs)
- [ ] T19 [AC9] Answer arm in `agent_apply.rs::plan`; prompt tool line and its pinned text (files:
  src/app/agent_apply.rs, src/agent/prompt.rs, tests/it/agent/system_prompt.rs)
- [ ] T20 Mutation testing: `scripts/mutants.sh <base>` (MUTANTS_TARGET_DIR set); a test for every
  missed mutant (files: src/document/check/tests.rs, tests/it/agent/check_drawing.rs)
- [ ] T21 DESIGN.md §7: rail bottom row (CHECK beside AI) and the Check window (files: DESIGN.md)
- [ ] T22 CHANGELOG line (files: CHANGELOG.md)
