# LCV-051 — ExtendTool

- **Status**: Done
- **Phase**: 4
- **Depends on**: LCV-041 (Done), LCV-025 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: f0c6fb1 — feat(LCV-051): ExtendTool — extend entity to nearest boundary

## Problem

The user cannot lengthen a Line to an existing boundary without deleting and
re-drawing it. `ExtendEntity` (LCV-025) is fully implemented in the command
layer but nothing drives it from user input. The EXTEND workflow is central to
precision laser-cutting: the operator lays out guide lines, then extends work
pieces to meet them exactly rather than estimating endpoint coordinates. Without
a tool, every "line too short" correction costs three operations (delete, redraw,
undo if wrong) instead of one click.

## Scope

- **New file `src/tools/extend.rs`** — `ExtendTool` implementing `Tool`.
- **Update `src/tools/mod.rs`** — `pub mod extend;` + `pub use extend::ExtendTool;`.
- Single-click interaction: `on_pointer_move` resolves the hover candidate
  (target endpoint + nearest valid boundary) and shows a live preview;
  `on_pointer_down` commits one `ExtendEntity` command and returns to `Idle`.
- Constant `PICK_RADIUS_MM: f64 = 5.0` declared in `extend.rs`, used to
  find the nearest Line endpoint to the cursor.
- State machine: `Idle` / `Hover { target_idx, extend_endpoint, boundary_idx,
  preview_line }`.

## Out of scope

- **Arc targets or Arc boundaries** — `ExtendEntity` (LCV-025) does not
  support them; the entity scan silently skips `Entity::Arc` in both roles.
- **Circle targets** — a closed circle has no meaningful "extend" operation.
  Silently skipped.
- **Multi-boundary selection phase** — AutoCAD R14 requires the user to
  designate boundary edges before picking targets. The KISS v2 variant
  auto-selects the nearest valid boundary. This is intentional.
- **Numeric extend distance via command line** — Phase 6 concern.
- **Toolbar button / menu item for EXTEND** — LCV-066 (Phase 6).
- **Snap integration inside the extend pick logic** — the cursor position
  `pos` delivered to the tool is already snap-resolved by LCV-041; the extend
  pick uses it as-is. No additional snap filtering.
- **Ortho lock** — LCV-053.

## Acceptance criteria

1. `src/tools/extend.rs` exists. `ExtendTool::name()` returns exactly
   `"EXTEND"`. `ExtendTool` derives `Default`; `ExtendTool::default()`
   constructs successfully and starts in `Idle`.

2. **State machine** — `ExtendTool` has exactly two observable states,
   `Idle` and `Hover`. `cancel()` always transitions to `Idle` regardless of
   the current state; `preview()` returns an empty `Vec` in `Idle`.

3. **`status_text()`**:
   - `Idle` → `"EXTEND: Click near a line endpoint to extend it"`
   - `Hover` → `"EXTEND: Click to extend  |  Esc to cancel"`

4. **`on_pointer_move` — pick algorithm** (runs every frame the cursor is in
   the viewport; receives world-space mm `pos` and `&mut Document`):

   a. **Find target**: scan `doc.entities` for `Entity::Line` variants. For
      each Line at index `i`, compute `d0 = (pos − line.p1).length()` and
      `d1 = (pos − line.p2).length()`. Track the Line whose `min(d0, d1)`
      is smallest. If that minimum exceeds `PICK_RADIUS_MM`, no target —
      enter `Idle`, `preview()` empty.

   b. **Resolve endpoint**: `extend_endpoint = 0` when `d0 < d1`, `1`
      otherwise (tie-break: `1`).

   c. **Find boundary**: let `endpoint_pos` be `target.p1`
      (`extend_endpoint == 0`) or `target.p2` (`extend_endpoint == 1`).
      Iterate all entities `(j, entity)` where `j ≠ target_idx`:

      - `Entity::Line(b)`: call `line_line_infinite(&target, &b)`. If it
        returns `Some(x)`, compute `t = parametric_t(&target, x)`. Valid
        iff `t < −EPSILON` (endpoint 0) or `t > 1.0 + EPSILON` (endpoint
        1). Record `(|x − endpoint_pos|, j, x)` for valid hits.
      - `Entity::Circle(b)`: replicate the surrogate-segment approach from
        `src/document/commands/trim/line.rs`
        (`extend_line_to_circle`) to enumerate intersection points on the
        infinite-line projection. For each intersection point `x`, apply
        the same `t`-validity check. Record `(|x − endpoint_pos|, j, x)`
        for valid hits.
      - `Entity::Arc(_)`: skip.

      Among all recorded hits, pick the one with the smallest distance
      from `endpoint_pos` → `boundary_idx`, intersection point `x`.

      If no valid hit exists, enter `Idle`, `preview()` empty.

   d. **Compute `preview_line`**: the full extended Line — if
      `extend_endpoint == 0`, `Line::new(x, target.p2)`; if `1`,
      `Line::new(target.p1, x)`.

   e. Enter `Hover { target_idx, extend_endpoint, boundary_idx,
      preview_line }`.

5. **`preview()`** — returns `vec![Entity::Line(preview_line)]` in `Hover`,
   empty `Vec` in `Idle`.

6. **`on_pointer_down`** — if in `Hover`, commits
   `ExtendEntity::new(target_idx, boundary_idx, extend_endpoint)` via
   `history.commit(Box::new(cmd), doc)`, then transitions to `Idle`.
   If in `Idle`, does nothing and `history` remains unchanged.

7. **`on_pointer_up`** — always a no-op.

8. **`on_key(egui::Key::Escape, app)`** — calls `self.cancel()`.
   All other keys are no-ops.

9. **`on_pointer_down` after Escape** — if `on_key(Escape)` was called
   between a hover and a click, the tool is in `Idle`, so the click is a
   no-op; no command is committed.

10. **Registration** — `src/tools/mod.rs` contains:
    ```
    pub mod extend;
    pub use extend::ExtendTool;
    ```
    The existing module list and re-exports are unmodified.

11. **Object-safe** — `let _: Box<dyn Tool> = Box::new(ExtendTool::default());`
    compiles without warnings.

12. **Kernel-purity** — `grep -nE '^use (eframe|rfd)' src/tools/extend.rs`
    returns no matches. The file may import `egui` (for `egui::Key`).

13. **LOC** — `wc -l src/tools/extend.rs` reports `≤ 300`.

14. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0 with no regressions.

## Expected tests

All tests live in `#[cfg(test)] mod tests` inside `src/tools/extend.rs`.

- **Unit (AC 1)**: `name_is_extend` — `assert_eq!(ExtendTool::default().name(), "EXTEND")`.

- **Unit (AC 1 + 11)**: `is_object_safe` —
  `let _: Box<dyn Tool> = Box::new(ExtendTool::default());`.

- **Unit (AC 2)**: `cancel_from_idle_stays_idle` — `cancel()` on a fresh
  `ExtendTool`; assert `preview().is_empty()`.

- **Unit (AC 2 + 8)**: `escape_key_resets_to_idle` — set up a one-Line +
  one-boundary doc, call `on_pointer_move` to enter `Hover`, then call
  `on_key(egui::Key::Escape, &mut app)`; assert `preview().is_empty()` and
  `status_text()` contains `"Click near"`.

- **Unit (AC 3)**: `status_text_transitions` — build a doc with
  `Entity::Line(Line::new((0,0), (5,0)))` and
  `Entity::Line(Line::new((10,-1), (10,1)))`; call `on_pointer_move` with
  cursor at `(5.2, 0.0)` (within `PICK_RADIUS_MM` of `p2`); assert
  `status_text()` contains `"Click to extend"`.

- **Unit (AC 4 — empty doc)**: `no_line_in_doc_stays_idle` — empty `Document`,
  cursor anywhere; assert `preview().is_empty()`.

- **Unit (AC 4 — cursor far)**: `cursor_far_from_all_endpoints_stays_idle` —
  doc with `Line((0,0),(10,0))`; cursor at `(50.0, 50.0)`;
  assert `preview().is_empty()`.

- **Unit (AC 4 + 5 — no boundary)**: `single_line_no_boundary_stays_idle` —
  doc with exactly one `Entity::Line`; cursor near `p2`; assert
  `preview().is_empty()` (no boundary entity → `Idle`).

- **Unit (AC 4 + 6 — line boundary)**: `hover_line_boundary_preview` — doc
  with `Entity::Line(Line::new((0.0,0.0),(5.0,0.0)))` at index 0 and
  `Entity::Line(Line::new((10.0,-1.0),(10.0,1.0)))` at index 1; cursor at
  `(5.2, 0.0)` (near `p2`). After `on_pointer_move`:
  - `status_text()` contains `"Click to extend"`.
  - `preview()` returns `vec![Entity::Line(_)]`.
  - The preview Line has `p1.approx_eq((0.0, 0.0), EPSILON)` and
    `p2.approx_eq((10.0, 0.0), EPSILON)`.

- **Unit (AC 4 + 6 — circle boundary)**: `hover_circle_boundary_preview` —
  doc with `Entity::Line(Line::new((-5.0,0.0),(-3.0,0.0)))` and
  `Entity::Circle(Circle::new((0.0,0.0), 2.0))`; cursor near `p2`
  at `(-2.8, 0.0)`. After `on_pointer_move`, preview Line has
  `p2.approx_eq((-2.0, 0.0), EPSILON)`.

- **Unit (AC 4 + 6 — nearest boundary wins)**: `nearest_boundary_selected` —
  doc with three entities: target `Line((0,0),(5,0))`, boundary A
  `Line((10,-1),(10,1))`, boundary B `Line((20,-1),(20,1))`; cursor near
  `p2`. Assert preview `p2.approx_eq((10.0, 0.0), EPSILON)` (boundary A,
  not B).

- **Unit (AC 6)**: `hover_extend_endpoint_zero` — doc with
  `Entity::Line(Line::new((5.0,0.0),(10.0,0.0)))` and
  `Entity::Line(Line::new((0.0,-1.0),(0.0,1.0)))`; cursor at `(5.2, 0.0)`
  (near `p1`). After `on_pointer_move`, preview Line has
  `p1.approx_eq((0.0, 0.0), EPSILON)` and `p2.approx_eq((10.0, 0.0), EPSILON)`.

- **Unit (AC 6)**: `tiebreak_endpoint_is_one` — doc with a Line and a
  boundary; cursor placed exactly equidistant from `p1` and `p2`
  (midpoint); assert `extend_endpoint` observed through the preview
  is `1` (i.e., preview `p2` changed, `p1` unchanged).

- **Unit (AC 6 + 7)**: `pointer_down_commits_and_resets_to_idle` — same
  setup as `hover_line_boundary_preview`; call `on_pointer_move` to enter
  `Hover`, then `on_pointer_down(pos, false, &mut doc, &mut hist)`:
  - `doc.entities[0]` is `Entity::Line` with `p2.approx_eq((10.0,0.0), EPSILON)`.
  - `hist.can_undo()` is `true`.
  - `preview().is_empty()` (transitioned to `Idle`).

- **Unit (AC 7 — undo)**: `extend_via_tool_is_undoable` — after
  `on_pointer_down` commits, call `hist.undo(&mut doc)`; assert the target
  Line is restored to its original geometry (`p2.approx_eq((5.0,0.0), EPSILON)`).

- **Unit (AC 6 + 8)**: `pointer_down_idle_is_noop` — fresh `ExtendTool`, doc
  with one Line; call `on_pointer_down` without a prior `on_pointer_move`
  hover; assert `hist.can_undo()` is `false` and entity count is unchanged.

- **Static check (AC 12)**: `grep -nE '^use (eframe|rfd)' src/tools/extend.rs`
  returns no matches.

- **Build gate (AC 14)**: `cargo fmt --all -- --check &&
  cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

*(none — demand is Ready)*

## Notes

- `ExtendEntity` is exported from `crate::document` as
  `crate::document::ExtendEntity` (via `document/mod.rs` re-export). The
  tool constructs it the same way as any other command and calls
  `history.commit(Box::new(ExtendEntity::new(…)), doc)`.

- `parametric_t` is `pub(crate)` in
  `src/document/commands/trim/mod.rs`. The tool may call it as
  `crate::document::commands::trim::parametric_t(&line, point)`.
  Alternatively, the implementer may inline the two-liner —
  `(p − line.p1).dot(line.p2 − line.p1) / (line.p2 − line.p1).length_squared()` —
  if that keeps the file under the 300-LOC cap more cleanly.

- The Circle-boundary hit logic mirrors `extend_line_to_circle` in
  `src/document/commands/trim/line.rs` (lines 81–116). If duplicating that
  logic bloats `extend.rs` past 300 LOC, promote the surrogate-segment
  approach to a `pub(crate) fn intersect_infinite_line_circle(target: &Line,
  boundary: &Circle) -> Vec<Vec2>` helper in `trim/line.rs` and call it from
  both `extend_line_to_circle` and `ExtendTool`. Document the split in the
  commit message.

- AutoCAD R14 runs EXTEND in two phases: first the user picks boundary edges,
  then picks entities to extend. The v2 KISS variant merges these into a single
  click by auto-selecting the nearest boundary. This is intentional per the
  product principle "prefer small tools that compose over smart tools with
  hidden behavior" — the simplification trades some flexibility for
  predictability.

- `PICK_RADIUS_MM = 5.0` matches the rough world-space snap radius at default
  zoom (`SNAP_TOLERANCE_PX = 10.0` × `mm_per_px ≈ 0.5` at typical bed zoom).
  A future Phase-5 demand may expose it as a settings slider.

- After LCV-066 lands, the toolbar button should call
  `app.tool_manager.set_tool(Box::new(ExtendTool::default()))`. The
  `name()` return value `"EXTEND"` is load-bearing for Phase-6
  command-line dispatch (LCV-068); do not change it.

- Reference: v1's `tools/extendTool.ts` used the same hover-to-preview /
  click-to-commit pattern. v2 matches the shape but relies on the
  already-implemented `ExtendEntity` command rather than re-computing the
  geometry in the tool.

- Structural references: `src/tools/delete.rs` for a simple modify tool;
  `src/tools/line.rs` for the state-machine pattern with preview.
