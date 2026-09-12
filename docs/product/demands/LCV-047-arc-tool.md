# LCV-047 — ArcTool

- **Status**: Done
- **Implementation**: d0656a3 — feat(LCV-047): ArcTool — 3-point arc drawing with circumcenter math and preview
- **Phase**: 4
- **Depends on**: LCV-041 (Done), LCV-023 (Done), LCV-037 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

LaserCAD v2 can render arcs (LCV-035) and store them via `CreateArc` (LCV-023), but the operator has no interactive way to draw one. Without an `ArcTool`, any workflow that requires a curved cut or engrave path forces the operator to use SVG import or the agent — neither of which is suitable for precise manual geometry construction.

The AutoCAD R14 three-point arc workflow is the minimal interactive arc-drawing primitive: click a start point, click an end point, then drag the cursor to bend the arc through a third point, and click to commit. This produces a deterministic `Entity::Arc` fully defined by the circumscribed circle through the three operator-supplied points.

## Scope

- **New file `src/tools/arc.rs`** implementing `ArcTool`:
  - Private state enum `ArcToolState` with three variants:
    - `Idle`
    - `WaitingEnd { start: Vec2 }` — start point captured, waiting for end click.
    - `WaitingMid { start: Vec2, end: Vec2 }` — start and end captured, cursor defines the arc's curvature (the point on the arc between them).
  - `pub struct ArcTool { state: ArcToolState, cursor: Vec2 }` — `cursor` is updated every `on_pointer_move` frame; `Vec2::default()` when `state == Idle`.
  - `impl Default for ArcTool` — starts in `Idle`, cursor `Vec2::default()`.
  - Full `impl Tool for ArcTool` (see acceptance criteria).

- **Free function `arc_from_3_points(a: Vec2, b: Vec2, c: Vec2) -> Option<Arc>`**:
  - Computes the circumscribed circle through A, B, C using the direct Cartesian formula (`D = 2*(ax*(by-cy) + bx*(cy-ay) + cx*(ay-by))`).
  - Returns `None` if `|D| < EPSILON` (collinear) or if any two points coincide within `EPSILON`.
  - On success returns `Arc { center, r, start_angle: atan2(a.y-cy, a.x-cx), end_angle: atan2(c.y-cy, c.x-cx), ccw }` where `ccw` is derived from the signed area of triangle (A, B, C): positive → `ccw = true`, negative → `ccw = false`.
  - Placement: if `src/geometry/arc.rs` is ≤ 260 LOC after the addition, place it there as `pub fn arc_from_3_points(...)`. Otherwise place it as a module-level `fn arc_from_3_points(...)` (crate-private) in `src/tools/arc.rs`. Either way, the function must have at least three unit tests (see Expected tests).

- **Update `src/tools/mod.rs`**: add `pub mod arc;` and re-export `pub use arc::ArcTool;`.

## Out of scope

- **Snap integration** — LCV-054 adds F3-toggle snap to all drawing tools. `ArcTool` receives already-snapped world positions via the pointer plumbing (LCV-041); it does not call the snap engine itself.
- **Command-line input for arc coordinates** — LCV-068 wires typed coordinate entry to tools. `ArcTool` here is purely mouse-driven.
- **Arc-by-center-start-end or arc-by-start-center-end** — AutoCAD R14 has multiple arc sub-commands. Only 3-point arc is in scope for v0.1.0.
- **Status bar or command prompt text updates** — LCV-067 / LCV-068 are not Done. State-specific prompts ("Specify start point:", "Specify second point:", "Specify end point:") are out of scope for this demand; they will be wired when those demands land.
- **Ortho lock** — LCV-053 is a separate demand.
- **Degenerate fallback commit** (collinear → commit a line instead) — if the three points are collinear the operator clicked incorrectly; doing nothing and staying in the current state is the correct behavior.

## Acceptance criteria

1. `src/tools/arc.rs` exists, defines `pub struct ArcTool`, and `ArcTool` implements `crate::tools::Tool`. The file is ≤ 300 LOC.

2. `ArcTool` has a private enum `ArcToolState` with exactly three variants: `Idle`, `WaitingEnd { start: Vec2 }`, `WaitingMid { start: Vec2, end: Vec2 }`. All variants derive `Debug` and `Clone`. Both `Vec2` fields are in world-space millimeters.

3. `Tool::name()` returns exactly the string `"ARC"` (uppercase, no whitespace).

4. **Idle state — first click**: `on_pointer_down(pos, doc, history)` when `state == Idle` transitions to `WaitingEnd { start: pos }`. `preview()` in `Idle` returns an empty `Vec<Entity>`.

5. **WaitingEnd — rubber-band preview**: `on_pointer_move(pos, doc)` updates `self.cursor = pos`. `preview()` in `WaitingEnd` returns exactly `[Entity::Line(Line::new(start, self.cursor))]`. No document mutation occurs on move.

6. **WaitingEnd — second click**: `on_pointer_down(pos, doc, history)` when `state == WaitingEnd` transitions to `WaitingMid { start, end: pos }`. No `CreateArc` or any command is committed at this point.

7. **WaitingMid — arc preview (valid case)**: `on_pointer_move` updates `self.cursor`. When `arc_from_3_points(start, self.cursor, end)` returns `Some(arc)`, `preview()` returns exactly `[Entity::Arc(arc)]`. The arc is recomputed every call to `preview()` from current `start`, `self.cursor`, `end`.

8. **WaitingMid — arc preview (degenerate case)**: When `arc_from_3_points(start, self.cursor, end)` returns `None` (start, cursor, end are collinear or any two coincide), `preview()` returns exactly `[Entity::Line(Line::new(start, end))]` as a visual fallback. The state does NOT change.

9. **WaitingMid — commit**: `on_pointer_down(pos, doc, history)` when `state == WaitingMid`:
   - If `arc_from_3_points(start, pos, end)` returns `Some(arc)`: call `history.commit(Box::new(CreateArc::new(arc)), doc)`, then transition to `Idle`. The committed arc satisfies:
     - `arc.center` is the circumcenter of (start, pos, end) within `EPSILON` mm.
     - `arc.r` is the circumradius within `EPSILON` mm.
     - `arc.start_angle == atan2(start.y - arc.center.y, start.x - arc.center.x)` within `EPSILON` rad.
     - `arc.end_angle == atan2(end.y - arc.center.y, end.x - arc.center.x)` within `EPSILON` rad.
     - `arc.ccw == true` iff the signed area of triangle (start, pos, end) is positive: `(pos.x - start.x)*(end.y - start.y) - (pos.y - start.y)*(end.x - start.x) > 0`.
   - If `arc_from_3_points(start, pos, end)` returns `None`: do nothing, stay in `WaitingMid`. No command is committed.

10. **Escape at any state**: `on_key(egui::Key::Escape, app)` calls `self.cancel()`. After `cancel()`, `state == Idle` and `preview()` returns an empty `Vec<Entity>`.

11. **`cancel()` contract**: resets `self.state` to `Idle` and `self.cursor` to `Vec2::default()`. No document mutation. Safe to call multiple times in sequence (idempotent on the Idle state).

12. **`on_pointer_up` is a no-op**: `on_pointer_up(pos, doc, history)` does not change state, does not commit any command, and does not alter the preview. `ArcTool` is click-on-press only.

13. **`arc_from_3_points` function**:
    - Signature: `fn arc_from_3_points(a: Vec2, b: Vec2, c: Vec2) -> Option<crate::geometry::Arc>` (public if placed in `geometry`, crate-private if placed in `tools/arc.rs`).
    - Returns `None` when `|D| < EPSILON` where `D = 2*(a.x*(b.y-c.y) + b.x*(c.y-a.y) + c.x*(a.y-b.y))`.
    - Returns `None` when any two of A, B, C are within `EPSILON` of each other (Euclidean distance).
    - Returns `Some(arc)` otherwise, with `arc.r > 0` and `arc.r` equal to `(a - center).length()` within `EPSILON`.
    - Pure function: no `egui`, no document access.

14. **Module registration**: `src/tools/mod.rs` has `pub mod arc;` and re-exports `pub use arc::ArcTool;`. `ArcTool` is reachable as `crate::tools::ArcTool` from tests.

15. **Kernel-purity**: `src/tools/arc.rs` does not import `eframe` or `rfd`. Verified: `grep -nE '^use (eframe|rfd)' src/tools/arc.rs` returns no matches.

16. **LOC cap**: `src/tools/arc.rs` ≤ 300 LOC. If `arc_from_3_points` is placed in `src/geometry/arc.rs`, the geometry file must also remain ≤ 300 LOC.

17. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

All tests live in `#[cfg(test)] mod tests` in the relevant file.

### Structural

- **AC 1, 3**: `arc_tool_name_exact` — `assert_eq!(ArcTool::default().name(), "ARC")`.
- **AC 1**: `arc_tool_is_object_safe` — `let _: Box<dyn Tool> = Box::new(ArcTool::default());`.

### State machine — happy path

- **AC 4**: `idle_first_click_transitions_to_waiting_end` — construct `ArcTool::default()`, call `on_pointer_down(Vec2::new(10.0, 0.0), &mut doc, &mut history)`, assert `preview()` is a line from `(10, 0)` to `(0, 0)` (cursor not yet moved) and the variant is `WaitingEnd`.
- **AC 5**: `waiting_end_move_updates_rubber_band_preview` — after first click at `(0, 0)`, call `on_pointer_move(Vec2::new(5.0, 5.0), &mut doc)`, assert `preview() == [Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0)))]`.
- **AC 6**: `waiting_end_second_click_transitions_to_waiting_mid` — first click at `(0, 0)`, second click at `(10, 0)`, assert state is `WaitingMid { start: (0,0), end: (10,0) }` and `doc.entity_count() == 0`.
- **AC 7**: `waiting_mid_valid_cursor_shows_arc_preview` — start `(0, 0)`, end `(10, 0)`, move cursor to `(5, 5)`: `arc_from_3_points((0,0), (5,5), (10,0))` must return `Some`; assert `preview()` returns exactly one `Entity::Arc`.
- **AC 9**: `waiting_mid_commit_produces_correct_arc` — start `(0, 0)`, end `(10, 0)`, click `(5, 5)`: assert `doc.entity_count() == 1`, the entity is `Entity::Arc`, the arc's center is at `(5, -2.5)` within 1e-9 mm, radius is `√((5² + 7.5²))` ≈ `9.014` mm within 1e-9 mm, `ccw == true` (cursor above the chord), and the tool is back in `Idle`.
  - Known exact value: `arc_from_3_points((0,0), (5,5), (10,0))` → center `(5.0, -2.5)`, r `= sqrt(25 + 56.25)/1 = sqrt(56.25 + 6.25)` — implementer verifies analytically and pins the assertion.
- **AC 9 — cw case**: `waiting_mid_commit_ccw_false_when_below_chord` — start `(0, 0)`, end `(10, 0)`, click `(5, -5)` (below chord): assert `arc.ccw == false`.
- **AC 9 — undo**: `arc_tool_commit_is_undoable` — commit arc then call `history.undo(&mut doc)`, assert `doc.entity_count() == 0`.

### State machine — degenerate / escape

- **AC 10**: `waiting_mid_collinear_cursor_shows_line_fallback` — start `(0, 0)`, end `(10, 0)`, move cursor to `(5, 0)` (on the chord); assert `preview() == [Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)))]`.
- **AC 9 — collinear noop**: `waiting_mid_collinear_click_does_not_commit` — start `(0, 0)`, end `(10, 0)`, click `(5, 0)`; assert `doc.entity_count() == 0` and tool stays in `WaitingMid`.
- **AC 10**: `escape_from_idle_is_noop` — `on_key(Escape, app)` on a fresh `ArcTool`; assert `preview().is_empty()`, no panic.
- **AC 10**: `escape_from_waiting_end_resets_to_idle` — click once at `(0, 0)`, then `on_key(Escape)`; assert `preview().is_empty()`.
- **AC 10**: `escape_from_waiting_mid_resets_to_idle` — click at `(0,0)`, click at `(10,0)`, then `on_key(Escape)`; assert `preview().is_empty()`.
- **AC 11**: `cancel_is_idempotent` — call `cancel()` twice on `Idle`; assert no panic and `preview().is_empty()`.
- **AC 12**: `pointer_up_is_noop` — in any state, `on_pointer_up` does not mutate doc or change state.

### `arc_from_3_points` unit tests

- **AC 13 — valid CCW**: `arc_from_3_points_semicircle_ccw` — `a=(−r,0), b=(0,r), c=(r,0)` with r=10: assert center ≈ `(0,0)`, radius ≈ `10.0`, `ccw == true`.
- **AC 13 — valid CW**: `arc_from_3_points_semicircle_cw` — `a=(−r,0), b=(0,−r), c=(r,0)`: assert `ccw == false`.
- **AC 13 — collinear returns None**: `arc_from_3_points_collinear_returns_none` — `a=(0,0), b=(5,0), c=(10,0)`: assert `result.is_none()`.
- **AC 13 — duplicate points returns None**: `arc_from_3_points_duplicate_points_returns_none` — `a=(1.0,1.0), b=(1.0,1.0), c=(5.0,5.0)`: assert `result.is_none()`.
- **AC 13 — r > 0**: `arc_from_3_points_radius_positive` — any valid triple: assert `arc.r > 0.0`.

### Build gate

- **AC 17**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

*(none)*

## Notes

### Circumcenter formula

Given three points A, B, C (all in mm):

```
D  = 2 * (a.x*(b.y - c.y) + b.x*(c.y - a.y) + c.x*(a.y - b.y))
|D| < EPSILON  →  collinear, return None

sq_a = a.x*a.x + a.y*a.y
sq_b = b.x*b.x + b.y*b.y
sq_c = c.x*c.x + c.y*c.y

center.x = (sq_a*(b.y - c.y) + sq_b*(c.y - a.y) + sq_c*(a.y - b.y)) / D
center.y = (sq_a*(c.x - b.x) + sq_b*(a.x - c.x) + sq_c*(b.x - a.x)) / D
r = (a - center).length()
```

### CCW determination

Signed area of triangle (A = start, B = mid, C = end):

```
signed_area = (b.x - a.x)*(c.y - a.y) - (b.y - a.y)*(c.x - a.x)
ccw = signed_area > 0.0
```

The sign matches the geometry: a cursor above the chord `start→end` (in the standard Y-up convention) produces `ccw = true`; below produces `ccw = false`. The `Entity::Arc` convention (AGENTS.md: CCW-positive from +X) is preserved.

Note that `src/geometry/arc.rs` uses `f64` radians throughout, consistent with the kernel's angle convention. No degree conversion at any point in this tool.

### `Arc` entity convention reminder

`Entity::Arc` wraps `geometry::Arc { center, r, start_angle, end_angle, ccw }`. The arc is swept from `start_angle` to `end_angle` in the direction indicated by `ccw`. The render pipeline (LCV-035) and SVG exporter (LCV-056) both honour this convention.

### LOC budget

`src/geometry/arc.rs` is 281 lines as of LCV-013. Adding `arc_from_3_points` (≈ 25 lines of code + doc comment) would push it to ≈ 306 lines — just over the 300-LOC cap. The implementer should therefore place `arc_from_3_points` in `src/tools/arc.rs` as a crate-private function unless the geometry file has been trimmed. If placed in `tools/arc.rs`, its tests live in the same file's `#[cfg(test)] mod tests`.

### Reference to CircleTool (LCV-046)

`CircleTool` (LCV-046) establishes the 2-click tool pattern: first click → center, move → radius preview, second click → commit. `ArcTool` follows the same pattern extended to three clicks. Reviewing LCV-046's implementation before starting LCV-047 is strongly recommended.

### `on_pointer_down` vs `on_pointer_up`

State transitions happen in `on_pointer_down` (press), not `on_pointer_up` (release). This matches the click-first-then-drag AutoCAD R14 interaction model. `on_pointer_up` is a documented no-op (AC 12).

### Borrow split for `on_key`

`on_key` takes `&mut App`. The Escape handler only calls `self.cancel()`, so no `App` field is read. No borrow-split gymnastics required.
