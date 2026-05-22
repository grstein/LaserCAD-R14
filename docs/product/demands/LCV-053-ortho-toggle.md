# LCV-053 — Ortho lock toggle (F8)

- **Status**: Ready
- **Phase**: 4
- **Depends on**: LCV-041 (Done), LCV-043 (Done), LCV-044 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

When cutting parts that require right-angle geometry — bracket slots, rectangular
enclosures, connector cutouts — the operator must carefully watch the coordinate
readout to avoid accidentally drawing a diagonal segment where a perfectly
horizontal or vertical one is needed. This is slow and error-prone. AutoCAD R14
solves it with Ortho mode (F8): a single keypress forces all pointer movement to
the nearest cardinal axis relative to the last anchor point. Without Ortho mode,
every orthogonal layout job in LaserCAD v2 requires manual coordinate entry or a
post-draw fix. Ortho is the most-used precision shortcut in line-drawing workflows.

## Scope

- **New file `src/geometry/ortho.rs`** — pure geometry helper:
  - `pub fn ortho_constrain(anchor: Vec2, pos: Vec2) -> Vec2`
    - Computes `dx = |pos.x − anchor.x|` and `dy = |pos.y − anchor.y|`.
    - If `dx >= dy`: horizontal constraint → returns `Vec2::new(pos.x, anchor.y)`.
    - If `dy > dx`: vertical constraint → returns `Vec2::new(anchor.x, pos.y)`.
    - When `dx == dy == 0` (pos on anchor): returns `anchor` (horizontal branch wins,
      result is degenerate but harmless).
  - File-level doc comment referencing LCV-053.
  - No `egui`, `eframe`, or `rfd` imports — only `crate::geometry::Vec2`.
  - File ≤ 80 LOC (implementation + unit tests).

- **`src/geometry/mod.rs`** — wire the new module:
  - Add `pub mod ortho;`
  - Add `pub use ortho::ortho_constrain;`

- **`src/app.rs`** — add App field, F8 handler, and ortho application:
  - Add field `pub ortho: bool` to `App` with doc comment `/// Ortho lock: when
    true, pointer movement is constrained to the nearest horizontal or vertical
    axis from the active tool's last anchor point. Toggled by F8 (LCV-053).` and
    default `false`.
  - Add `use crate::geometry::ortho_constrain;` to the existing import block
    (no new sub-module needed; the function lives in `src/geometry/ortho.rs`).
  - Inside `App::update`, **outside** `response.hovered()` but still inside
    `CentralPanel`, add an F8 key handler after the existing Delete/Backspace
    handlers:
    ```rust
    if ctx.input(|i| i.key_pressed(egui::Key::F8)) {
        self.ortho = !self.ortho;
    }
    ```
  - Inside the `response.hovered()` block, **after** snap resolution and before
    setting `self.last_cursor_world`, apply the ortho constraint:
    ```rust
    let world_pos = if self.ortho {
        match self.tool_manager.anchor() {
            Some(anchor) => ortho_constrain(anchor, world_pos),
            None => world_pos,
        }
    } else {
        world_pos
    };
    ```
  - This single transformation propagates to `last_cursor_world`, all
    `PointerEvent` variants, and therefore to the tool's preview and committed
    coordinates — no per-tool changes required for the constraint itself.
  - `src/app.rs` must stay ≤ 300 LOC after this demand lands. The `ortho` field
    declaration and the F8 handler add at most 8 new lines; the constraint logic
    lives entirely in `src/geometry/ortho.rs`.

- **`src/tools/tool.rs`** — add one default method to `pub trait Tool`:
  ```rust
  /// The anchor point for ortho / snap constraints: the last committed
  /// endpoint that constrains the next cursor position.
  ///
  /// Returns `Some(p)` when the tool is awaiting a second point (i.e. has a
  /// fixed first point); returns `None` in idle state or for tools that have
  /// no meaningful anchor (select, move, etc.). Defaults to `None`.
  fn anchor(&self) -> Option<crate::geometry::Vec2> {
      None
  }
  ```
  Placement: after `status_text()`, before `preview()`. Object-safety is
  preserved: no generic parameters, takes only `&self`, returns
  `Option<Vec2>`. No existing `Tool` implementor needs to change.

- **`src/tools/manager.rs`** — add delegation:
  ```rust
  /// The active tool's anchor point for ortho / snap constraints.
  /// Returns `None` when the active tool has no live anchor.
  pub fn anchor(&self) -> Option<crate::geometry::Vec2> {
      self.active.anchor()
  }
  ```

- **`src/tools/line.rs`** — add `anchor()` override:
  ```rust
  fn anchor(&self) -> Option<Vec2> {
      match self.state {
          State::WaitingSecondPoint { p1, .. } => Some(p1),
          State::Idle => None,
      }
  }
  ```

- **`src/tools/polyline.rs`** — add identical `anchor()` override (same state
  machine shape as `LineTool`).

- **`src/ui/statusbar.rs`** — add ORTHO indicator:
  - Add a `pub(crate) fn format_ortho(ortho: bool) -> Option<&'static str>`
    helper that returns `Some("ORTHO")` when `ortho == true`, `None` otherwise.
  - In `draw_statusbar`, after the entity count segment, conditionally render:
    ```rust
    if let Some(badge) = format_ortho(app.ortho) {
        ui.separator();
        ui.label(badge);
    }
    ```
  - File must remain ≤ 150 LOC.

## Out of scope

- **F3 snap toggle** — belongs to LCV-054 (snap integration into drawing tools).
- **F7 grid toggle** — belongs to LCV-033 or LCV-070; not this demand.
- **Keyboard shortcut registrations in LCV-070** — LCV-053 wires F8 directly in
  `App::update`; the central shortcuts table (LCV-070) will document it but
  does not need to land first.
- **CircleTool, ArcTool, RectTool** — these tools do not have a fixed "first
  point → second point" anchor in the same sense. RectTool (LCV-045) operates on
  diagonal corners; CircleTool/ArcTool use radius/angle. None gain `anchor()`
  overrides in this demand. Adding ortho to them is deferred; the `None` default
  is the correct no-op.
- **Right-click or toolbar button for Ortho** — keyboard only.
- **Visual indication on the cursor (crosshair lock style)** — the status bar
  badge is sufficient for v0.1.0. Cursor styling is LCV-071 territory.
- **Ortho in the command-line coordinate entry path** — that path (LCV-068) will
  receive the raw typed coordinate; ortho does not constrain typed input.
- **Polar tracking (non-cardinal angles)** — AutoCAD's PolarSnap is not in
  scope for v0.1.0.
- **Ortho state persistence across sessions** — `ortho` defaults to `false` on
  every launch. The settings store (LCV-058) does not persist this toggle.
- **Any change to `SelectTool`, `MoveTool`, `DeleteTool`, `TrimTool`,
  `ExtendTool`** — none of these have a line-anchor; they inherit `anchor() →
  None` and remain unaffected.

## Acceptance criteria

1. `App` has a `pub ortho: bool` field that defaults to `false`.

2. Pressing F8 once while the viewport is focused sets `app.ortho` to `true`;
   pressing F8 a second time sets it back to `false`. State toggles on each
   F8 press with no other side-effect.

3. When `app.ortho == false`, the `world_pos` delivered to all three
   `PointerEvent` variants equals the snap-resolved position unchanged.

4. When `app.ortho == true` and the active tool's `anchor()` returns `None`
   (e.g. `LineTool` in `Idle` state), `world_pos` is not modified — the
   constraint is a no-op.

5. When `app.ortho == true` and the active tool's `anchor()` returns
   `Some(p1)`:
   - If the cursor is closer to horizontal from `p1` (`|dx| >= |dy|`),
     `world_pos.y` is clamped to `p1.y`; `world_pos.x` is unchanged.
   - If the cursor is closer to vertical from `p1` (`|dy| > |dx|`),
     `world_pos.x` is clamped to `p1.x`; `world_pos.y` is unchanged.
   This applies to `Move`, `Press`, and `Release` events.

6. `ortho_constrain(anchor, pos)` satisfies the following, verified by unit
   tests — all coordinates in mm:
   a. `anchor = (0, 0)`, `pos = (10, 3)` → `(10, 0)` (horizontal).
   b. `anchor = (0, 0)`, `pos = (3, 10)` → `(0, 10)` (vertical).
   c. `anchor = (0, 0)`, `pos = (5, 5)` → `(5, 0)` (horizontal wins the tie).
   d. `anchor = (5, 3)`, `pos = (12, 7)` → `(12, 3)` (horizontal; dx=7, dy=4).
   e. `anchor = (5, 3)`, `pos = (6, 9)` → `(5, 9)` (vertical; dx=1, dy=6).
   f. `anchor = (2, 2)`, `pos = (2, 2)` → `(2, 2)` (degenerate, on anchor).

7. `LineTool::anchor()` returns `None` when the tool is in `Idle` state.

8. `LineTool::anchor()` returns `Some(p1)` when the tool is in
   `WaitingSecondPoint { p1, cursor }` state; the returned point is exactly
   the `p1` stored in the state machine (bit-exact).

9. `PolylineTool::anchor()` satisfies the same contract as AC 7 and AC 8.

10. `ToolManager::anchor()` delegates to `self.active.anchor()` without
    modification.

11. `Tool::anchor()` default implementation returns `None`. A unit test
    confirms `SelectTool::default().anchor() == None` without an explicit
    override on `SelectTool`.

12. The status bar renders an `"ORTHO"` label (as a separate segment after a
    `ui.separator()`) when `app.ortho == true`. The label is absent when
    `app.ortho == false` — no empty space or placeholder is shown.

13. `format_ortho(true)` returns `Some("ORTHO")`; `format_ortho(false)` returns
    `None`.

14. Manual smoke: with `LineTool` active, click a first point anywhere on the
    canvas, then move the cursor at a shallow diagonal from that point — the
    live preview segment locks to horizontal. Move the cursor to a steep
    diagonal — the preview locks to vertical. The status bar shows `"ORTHO"`.
    Press F8: the preview immediately becomes free again; the `"ORTHO"` badge
    disappears.

15. `src/geometry/ortho.rs` ≤ 80 LOC; `src/ui/statusbar.rs` ≤ 150 LOC;
    `src/tools/tool.rs` ≤ 100 LOC; `src/tools/manager.rs` ≤ 200 LOC;
    `src/tools/line.rs` ≤ 230 LOC; `src/tools/polyline.rs` ≤ 230 LOC.

16. Kernel-purity: `grep -nE '^use (egui|eframe|rfd)' src/geometry/ortho.rs`
    returns no matches.

17. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

### `src/geometry/ortho.rs` — `#[cfg(test)] mod tests`

- **(AC 6a)** `ortho_constrain_horizontal_dx_gt_dy` —
  `assert_eq!(ortho_constrain(Vec2::new(0.0, 0.0), Vec2::new(10.0, 3.0)), Vec2::new(10.0, 0.0))`.

- **(AC 6b)** `ortho_constrain_vertical_dy_gt_dx` —
  `assert_eq!(ortho_constrain(Vec2::new(0.0, 0.0), Vec2::new(3.0, 10.0)), Vec2::new(0.0, 10.0))`.

- **(AC 6c)** `ortho_constrain_tie_prefers_horizontal` —
  `assert_eq!(ortho_constrain(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0)), Vec2::new(5.0, 0.0))`.

- **(AC 6d)** `ortho_constrain_offset_anchor_horizontal` —
  `assert_eq!(ortho_constrain(Vec2::new(5.0, 3.0), Vec2::new(12.0, 7.0)), Vec2::new(12.0, 3.0))`.

- **(AC 6e)** `ortho_constrain_offset_anchor_vertical` —
  `assert_eq!(ortho_constrain(Vec2::new(5.0, 3.0), Vec2::new(6.0, 9.0)), Vec2::new(5.0, 9.0))`.

- **(AC 6f)** `ortho_constrain_degenerate_pos_on_anchor` —
  `assert_eq!(ortho_constrain(Vec2::new(2.0, 2.0), Vec2::new(2.0, 2.0)), Vec2::new(2.0, 2.0))`.

- **(AC 6, negative coords)** `ortho_constrain_negative_horizontal` —
  `ortho_constrain(Vec2::new(0.0, 0.0), Vec2::new(-8.0, 2.0))` → `Vec2::new(-8.0, 0.0)`.

- **(AC 6, negative coords)** `ortho_constrain_negative_vertical` —
  `ortho_constrain(Vec2::new(0.0, 0.0), Vec2::new(1.0, -9.0))` → `Vec2::new(0.0, -9.0)`.

### `src/tools/tool.rs` (or `src/tools/select.rs`) — `#[cfg(test)] mod tests`

- **(AC 11)** `tool_default_anchor_is_none` —
  `assert_eq!(SelectTool::default().anchor(), None)`.

### `src/tools/line.rs` — `#[cfg(test)] mod tests`

- **(AC 7)** `line_tool_anchor_idle_is_none` —
  fresh `LineTool::new()`: `assert_eq!(t.anchor(), None)`.

- **(AC 8)** `line_tool_anchor_waiting_returns_p1` — call
  `on_pointer_down(Vec2::new(7.0, 4.0), false, &mut doc, &mut h)`;
  `assert_eq!(t.anchor(), Some(Vec2::new(7.0, 4.0)))`.

- **(AC 8, precision)** `line_tool_anchor_updates_after_second_commit` —
  click at `(0.0, 0.0)`, then `(10.0, 0.0)` (commits a segment and chains);
  `assert_eq!(t.anchor(), Some(Vec2::new(10.0, 0.0)))` (anchor advances to
  the newly committed endpoint).

### `src/tools/polyline.rs` — `#[cfg(test)] mod tests`

- **(AC 9)** `polyline_tool_anchor_idle_is_none` —
  fresh tool: `assert_eq!(t.anchor(), None)`.

- **(AC 9)** `polyline_tool_anchor_waiting_returns_p1` — call
  `on_pointer_down(Vec2::new(3.0, 8.0), false, &mut doc, &mut h)`;
  `assert_eq!(t.anchor(), Some(Vec2::new(3.0, 8.0)))`.

### `src/tools/manager.rs` — `#[cfg(test)] mod tests`

- **(AC 10)** `tool_manager_anchor_delegates_to_active_tool` —
  construct `ToolManager::new(Box::new(LineTool::new()))`;
  assert `manager.anchor() == None` (idle); call
  `manager.handle_pointer_down(Vec2::new(1.0, 2.0), &mut app)`;
  assert `manager.anchor() == Some(Vec2::new(1.0, 2.0))`.

### `src/ui/statusbar.rs` — `#[cfg(test)] mod tests`

- **(AC 13)** `format_ortho_true_returns_some_badge` —
  `assert_eq!(format_ortho(true), Some("ORTHO"))`.

- **(AC 13)** `format_ortho_false_returns_none` —
  `assert_eq!(format_ortho(false), None)`.

### Manual smoke (AC 14)

> 1. `cargo run`.
> 2. Activate `LineTool` (activate via toolbar or future keyboard shortcut).
> 3. Click anywhere on the canvas to place the first point.
> 4. Without pressing F8, move the cursor at a ~20° angle from the first point.
>    Verify the rubber-band segment is NOT constrained (it follows the diagonal).
> 5. Press F8. Verify `"ORTHO"` appears in the status bar.
> 6. Move the cursor: verify the preview segment locks to horizontal when the
>    cursor is within 45° of horizontal from the anchor, and to vertical
>    otherwise.
> 7. Click to commit a segment: verify the committed entity is exactly
>    horizontal or vertical (coordinate readout confirms).
> 8. Press F8 again: verify `"ORTHO"` disappears and the preview is free again.
> 9. Press Escape to reset the tool.

## Open questions

*(none)*

## Notes

- **Constraint seam**: the ortho application sits in `App::update` between snap
  resolution and `PointerEvent` construction. This is the same seam described in
  the LCV-041 demand notes: "Ortho lock (LCV-053) will modify `PointerEvent::Move.pos`
  before forwarding to the tool; a future right-click-to-cancel demand can add
  `on_key`-equivalent routing for `PointerButton::Secondary` without changing
  the type." The application order is: raw cursor → snap resolution →
  ortho clamping → `PointerEvent` → tool. This means ortho overrides snap when
  both are active; snap provides the starting point, ortho clamps to the nearest
  axis from there.

- **Why `src/geometry/ortho.rs`, not `src/app/ortho.rs`**: `ortho_constrain` is
  a pure geometric function — it touches only `Vec2` arithmetic. The geometry
  module is the canonical home for pure kernel functions (AGENTS.md §Architecture:
  "Hard rules on the tree: `geometry/` — pure kernel; no egui, eframe, rfd").
  Placing it under `src/app/` would scatter pure geometry across two modules and
  compromise the kernel-purity contract. The import in `app.rs` is a single
  line: `use crate::geometry::ortho_constrain;`.

- **Why anchor on Tool trait, not App**: the anchor is the tool's current first
  point — it is part of the tool's internal state. Duplicating it on `App` would
  require tools to write back to `App` on every state change, violating the
  "tools never mutate App directly" contract from AGENTS.md §State. A read-only
  `fn anchor(&self) -> Option<Vec2>` default method costs nothing and does not
  break object-safety.

- **`last_cursor_world` reflects constrained coords**: because ortho is applied
  before `self.last_cursor_world = Some(world_pos)`, the status bar's X/Y
  coordinate readout shows the constrained (clamped) position while Ortho is
  on. This matches AutoCAD R14 behaviour: the status bar always shows where
  the cursor will actually commit.

- **`src/app.rs` LOC**: the `pub ortho: bool` field declaration and the F8
  handler add at most 8 new lines. The pure constraint logic and its tests live
  entirely in `src/geometry/ortho.rs`. If the file still exceeds 300 LOC after
  this demand, the implementer may split the in-file tests into
  `src/app/tests.rs` (same pattern as existing `src/app/snap.rs`).

- **`ortho_constrain` precision**: the function performs only subtraction,
  comparison, and field replacement — no trigonometry, no floating-point error
  accumulation. The result coordinate is taken directly from either `pos.x` /
  `anchor.x` or `pos.y` / `anchor.y`, preserving whatever precision the upstream
  snap engine produced.

- **LineTool chain after ortho commit**: after a constrained commit, the chaining
  logic stores the committed endpoint as the new `p1`. On the next move event,
  `anchor()` returns the new `p1`, and ortho constrains relative to it. This
  gives the correct behaviour for drawing a staircase: each new segment is
  independently constrained to H or V from the last committed point.

- **F8 outside `response.hovered()`**: the key handler is placed outside
  `response.hovered()` so the user can toggle ortho while the cursor is outside
  the viewport (e.g. when reading dimensions elsewhere on the monitor). This
  matches the Escape / Delete key handler placement already in `app.rs`.

- **v1 reference**: v1 (`InputHandler.ts`, `App.ts`) tracked an `orthoMode: bool`
  on the `App` instance, toggled by `keydown === 'F8'`, applied as a coordinate
  clamp in `resolveWorldPos()` before tool dispatch. The same pipeline here.
