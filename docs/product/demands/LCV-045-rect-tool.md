# LCV-045 — RectTool

- **Status**: Ready
- **Phase**: 4
- **Depends on**: LCV-043 (Done), LCV-041 (Done), LCV-023 (Done), LCV-037 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

After LineTool (LCV-043), the most frequent laser-cutting operation is a
rectangular profile — enclosures, panels, fixtures, labels. In AutoCAD R14 the
`RECTANG` command is a first-class tool: two corner clicks produce four closed
line segments in a single gesture. Without it, operators must fire LINE four
times and manually close the shape, which is error-prone (open corners break
LaserGRBL cut-path detection).

This demand ships `RectTool` in `src/tools/rect.rs`: a two-click state machine
that records the first corner on the initial click, rubber-bands a live
four-sided preview while the cursor moves, then on the second click derives all
four corners, guards against degenerate rectangles (zero width or zero height),
and commits exactly four `CreateLine` commands to the history stack — one per
side.

## Scope

- **New file `src/tools/rect.rs`** containing:
  - `pub struct RectTool` with a two-variant private state machine:
    - `Idle` — no corner captured.
    - `WaitingSecondCorner { p1: Vec2, cursor: Vec2 }` — first corner is fixed;
      `cursor` is the live opposite corner, updated by `on_pointer_move`.
  - `impl Default for RectTool` — constructs in `Idle`.
  - `impl Tool for RectTool`:
    - `name() -> &'static str` — returns `"RECTANG"`.
    - `status_text() -> &'static str`:
      - `Idle` → `"RECTANG: Click to set first corner"`.
      - `WaitingSecondCorner` → `"RECTANG: Click to set opposite corner  |  Esc to cancel"`.
    - `on_pointer_down(pos, _shift, doc, history)`:
      - `Idle` → transition to `WaitingSecondCorner { p1: pos, cursor: pos }`.
      - `WaitingSecondCorner { p1, .. }` → derive `p2 = pos`. Compute
        `width = (p2.x − p1.x).abs()` and `height = (p2.y − p1.y).abs()`.
        If `width ≤ EPSILON` or `height ≤ EPSILON`, do nothing and remain in
        `WaitingSecondCorner` (degenerate guard — zero-width or zero-height
        rectangles are meaningless as laser paths). Otherwise derive the four
        corners:
        ```
        A = Vec2::new(p1.x, p1.y)   // first click
        B = Vec2::new(p2.x, p1.y)   // x from p2, y from p1
        C = Vec2::new(p2.x, p2.y)   // second click
        D = Vec2::new(p1.x, p2.y)   // x from p1, y from p2
        ```
        Commit four `CreateLine` commands in order A→B, B→C, C→D, D→A via
        four separate calls to `history.commit(cmd, doc)`. Reset state to
        `Idle`.
    - `on_pointer_move(pos, _doc)`:
      - `WaitingSecondCorner` → `cursor = pos`.
      - `Idle` → no-op.
    - `on_pointer_up` — no-op (commit is on `on_pointer_down`).
    - `on_key(key, _app)`:
      - `Key::Escape` in any state → `self.cancel()`.
      - All other keys → no-op.
    - `preview() -> Vec<Entity>`:
      - `Idle` → `vec![]`.
      - `WaitingSecondCorner { p1, cursor }` → if
        `(cursor.x − p1.x).abs() > EPSILON && (cursor.y − p1.y).abs() > EPSILON`,
        return the four rubber-band sides in A→B, B→C, C→D, D→A order
        (same corner derivation as the commit path). Otherwise return `vec![]`
        (degenerate preview is suppressed).
    - `cancel(&mut self)` — resets state to `Idle`.
  - Module doc comment referencing the demand and the no-egui-import rule.
  - MUST NOT import `eframe` or `rfd`. May import `egui` for `egui::Key`.
  - File ≤ 300 LOC.

- **Update `src/tools/mod.rs`**:
  - Add `pub mod rect;`.
  - Add `pub use rect::RectTool;`.

## Out of scope

- **Polyline or closed-polygon entity** — RectTool commits four individual
  `CreateLine` segments; there is no `Polyline` wrapper here. That is
  LCV-044 territory.
- **Square mode (Shift-click)** — no width-equals-height clamping; the
  cursor position is used verbatim.
- **Chamfer / fillet corners** — not in AutoCAD R14 RECTANG default scope
  for v0.1.0.
- **Snapping** — snap coordinates are resolved upstream by LCV-041 and
  delivered as world-mm `pos` values; RectTool consumes them verbatim.
- **Ortho lock** — applied upstream (LCV-053); not RectTool's concern.
- **Command-line coordinate entry** — LCV-068 territory.
- **Keyboard shortcut `REC` to activate RectTool** — LCV-070.
- **Toolbar button** — LCV-066.
- **Undo semantics** — the existing history stack (LCV-026) handles undo/redo
  of each `CreateLine` automatically; four separate undos remove the four
  sides one by one. No grouped-undo macro command is introduced here.
- **Zero-area error dialog** — degenerate clicks are silently ignored; no
  error message.

## Acceptance criteria

1. `src/tools/rect.rs` exists. `RectTool` is a `pub struct` that implements
   `Tool`. `RectTool::default()` starts in `Idle` state: `preview()` returns
   an empty `Vec<Entity>`.

2. `name()` returns the exact string `"RECTANG"`.

3. `status_text()` returns `"RECTANG: Click to set first corner"` in `Idle`
   state, and `"RECTANG: Click to set opposite corner  |  Esc to cancel"` in
   `WaitingSecondCorner` state.

4. `on_pointer_down` in `Idle` with any `pos` transitions to
   `WaitingSecondCorner`. After the call, `status_text()` contains
   `"opposite corner"`.

5. `on_pointer_move` in `WaitingSecondCorner` updates the preview: after
   `on_pointer_move(new_pos, _doc)`, `preview()` reflects `new_pos` as the
   opposite corner (when non-degenerate).

6. `preview()` in `WaitingSecondCorner { p1, cursor }` where both
   `|cursor.x − p1.x| > EPSILON` and `|cursor.y − p1.y| > EPSILON` returns
   exactly four `Entity::Line` values in order A→B, B→C, C→D, D→A with corners
   derived as specified. Each endpoint must be bit-exact over the stored values.

7. `preview()` in `WaitingSecondCorner` where `|cursor.x − p1.x| ≤ EPSILON`
   (zero-width condition) returns `vec![]`.

8. `preview()` in `WaitingSecondCorner` where `|cursor.y − p1.y| ≤ EPSILON`
   (zero-height condition) returns `vec![]`.

9. `on_pointer_down` in `WaitingSecondCorner { p1 }` with `p2` where
   `|p2.x − p1.x| ≤ EPSILON` (zero-width degenerate): no entity is added,
   no history entry is pushed, state remains `WaitingSecondCorner`.

10. `on_pointer_down` in `WaitingSecondCorner { p1 }` with `p2` where
    `|p2.y − p1.y| ≤ EPSILON` (zero-height degenerate): no entity is added,
    no history entry is pushed, state remains `WaitingSecondCorner`.

11. `on_pointer_down` in `WaitingSecondCorner { p1 }` with a non-degenerate
    `p2`:
    a. Exactly four `Entity::Line` entries are added to `doc.entities`.
    b. The four lines are, in commit order: A→B, B→C, C→D, D→A with corners
       A=(p1.x,p1.y), B=(p2.x,p1.y), C=(p2.x,p2.y), D=(p1.x,p2.y).
       Endpoint coordinates must be bit-exact.
    c. `history.can_undo()` returns `true` after the commit.
    d. The tool resets to `Idle`: `status_text()` contains `"first corner"` and
       `preview()` returns `vec![]`.

12. The four committed lines form a closed loop: the start point of each line
    equals the end point of the preceding line, and the end point of the fourth
    line equals the start point of the first line.

13. `on_key(Key::Escape, _app)` in `WaitingSecondCorner` transitions to `Idle`:
    `preview()` returns `vec![]` and no entity has been added.

14. `on_key(Key::Escape, _app)` in `Idle` is a no-op: no panic, tool remains
    `Idle`.

15. `cancel()` in `WaitingSecondCorner` resets to `Idle`: `preview()` returns
    `vec![]`.

16. `RectTool` is object-safe: `let _: Box<dyn Tool> = Box::new(RectTool::default());`
    compiles.

17. `src/tools/mod.rs` contains `pub mod rect;` and `pub use rect::RectTool;`.

18. Kernel-purity: `grep -nE '^use (eframe|rfd)' src/tools/rect.rs` returns no
    matches.

19. `wc -l src/tools/rect.rs` ≤ 300.

20. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

All tests live in `src/tools/rect.rs` inside `#[cfg(test)] mod tests`.

- **(AC 1, 4)** `default_is_idle_preview_empty` — `RectTool::default()`, assert
  `preview().is_empty()` and `status_text()` contains `"first corner"`.

- **(AC 2)** `name_exact` — `assert_eq!(RectTool::default().name(), "RECTANG")`.

- **(AC 3)** `status_text_idle` — assert exact string
  `"RECTANG: Click to set first corner"`.

- **(AC 3, 4)** `status_text_waiting` — after `on_pointer_down(p1, ...)`, assert
  exact string `"RECTANG: Click to set opposite corner  |  Esc to cancel"`.

- **(AC 5, 6)** `preview_4_segments_after_move` — first click at `(0.0, 0.0)`,
  `on_pointer_move(Vec2::new(30.0, 20.0), &mut doc)`, assert `preview()` has
  exactly 4 `Entity::Line` items. Verify all four endpoint pairs:
  `(0,0)→(30,0)`, `(30,0)→(30,20)`, `(30,20)→(0,20)`, `(0,20)→(0,0)`.

- **(AC 7)** `preview_zero_width_suppressed` — first click at `(10.0, 10.0)`,
  `on_pointer_move(Vec2::new(10.0, 40.0), &mut doc)` (same x, different y),
  assert `preview().is_empty()`.

- **(AC 8)** `preview_zero_height_suppressed` — first click at `(10.0, 10.0)`,
  `on_pointer_move(Vec2::new(50.0, 10.0), &mut doc)` (different x, same y),
  assert `preview().is_empty()`.

- **(AC 9)** `degenerate_zero_width_second_click_ignored` — first click at
  `(5.0, 5.0)`, second click at `(5.0, 50.0)`, assert `doc.entity_count() == 0`
  and `!history.can_undo()` and tool still in `WaitingSecondCorner`.

- **(AC 10)** `degenerate_zero_height_second_click_ignored` — first click at
  `(5.0, 5.0)`, second click at `(50.0, 5.0)`, assert `doc.entity_count() == 0`
  and `!history.can_undo()` and tool still in `WaitingSecondCorner`.

- **(AC 11 a–d)** `second_click_commits_4_lines_and_resets` — first click
  `(0.0, 0.0)`, second click `(10.0, 5.0)`. Assert `doc.entity_count() == 4`,
  `history.can_undo() == true`, `status_text()` contains `"first corner"`,
  `preview().is_empty()`.

- **(AC 11b, 12)** `committed_corners_exact_and_closed` — same setup as above.
  Extract all four `Entity::Line` entries. Assert in order:
  - `[0]`: p1=`(0,0)`, p2=`(10,0)`
  - `[1]`: p1=`(10,0)`, p2=`(10,5)`
  - `[2]`: p1=`(10,5)`, p2=`(0,5)`
  - `[3]`: p1=`(0,5)`, p2=`(0,0)`
  (Verify with `approx_eq(_, EPSILON)` for floating-point safety.)

- **(AC 13)** `escape_in_waiting_resets_to_idle` — first click, call
  `on_key(Key::Escape, &mut app)`. Assert `preview().is_empty()`,
  `doc.entity_count() == 0`.

- **(AC 14)** `escape_in_idle_is_noop` — fresh tool, call
  `on_key(Key::Escape, &mut app)`. No panic, `preview().is_empty()`.

- **(AC 15)** `cancel_resets_to_idle` — first click,
  `on_pointer_move(Vec2::new(20.0, 15.0), &mut doc)`, assert preview non-empty,
  then `cancel()`, assert `preview().is_empty()`.

- **(AC 16)** `object_safe` —
  `let _: Box<dyn Tool> = Box::new(RectTool::default());`.

## Open questions

*(none)*

## Notes

- **Four individual commits, not a macro command.** Each side is a separate
  `history.commit(Box::new(CreateLine::new(...)), doc)` call. This means Ctrl+Z
  undoes one side at a time — matching the AutoCAD R14 behaviour where RECTANG
  is syntactic sugar over four LINE operations. No grouped-undo wrapper is
  introduced; that would be a separate architectural feature.

- **Corner derivation is deterministic.** Corners are derived algebraically from
  `p1` and `p2` — no sorting or normalisation of coordinates. The commit order
  (A→B→C→D→A) is fixed regardless of which screen quadrant `p2` lies relative
  to `p1`. The result is always a closed, axis-aligned rectangle.

- **No chaining after second click.** Unlike `LineTool` (which stays in
  `WaitingSecondPoint` to chain the next segment), `RectTool` resets to `Idle`
  after committing the four sides. `RECTANG` in AutoCAD R14 exits the command
  after one rectangle; chaining would require a re-activation.

- **`EPSILON`**: use `crate::geometry::EPSILON` for both the degenerate-click
  guard and the preview suppression condition — the same constant used across
  the kernel.

- **Preview suppression matches commit guard.** The `preview()` method uses
  the same `> EPSILON` check for width and height as `on_pointer_down`. The
  user therefore never sees a preview that would be rejected on click.

- **`on_pointer_up` is a no-op.** Consistent with `LineTool` and `CircleTool`:
  the commit gesture is down-down, not press-drag-release.

- **File placement**: `src/tools/rect.rs`. Mirror the structure and imports of
  `src/tools/line.rs` as the canonical reference.

- **v1 reference**: v1 had no dedicated `RectTool`; users chained four LINE
  calls. The v2 implementation closes that UX gap.
