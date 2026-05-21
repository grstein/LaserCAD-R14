# LCV-046 — CircleTool

- **Status**: Draft
- **Phase**: 4
- **Depends on**: LCV-041 (done), LCV-023 (done), LCV-037 (done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

Operators need to place circles on the drawing canvas — the fundamental AutoCAD R14 center-radius workflow. Without a CircleTool, the user cannot draw circles interactively. They must either rely on the agent harness (Phase 7) or have no circle capability at all, which breaks the core laser-cutting use case (cutting/engraving circles, holes, rounded features).

The standard AutoCAD R14 gesture is: click to fix the center point, move the cursor to see a live rubber-band preview of the circle at the current radius, then click again to commit. LaserGRBL-bound SVG export (LCV-056) emits circles as `<circle>` elements; the `Entity::Circle` variant (LCV-020) feeds that path.

## Scope

- New file `src/tools/circle.rs` defining `pub struct CircleTool`.
- **State machine** — private `enum CircleState`:
  - `Idle` — no point captured.
  - `WaitingRadius { center: Vec2 }` — center captured, awaiting second click.
  - `CircleTool` stores `state: CircleState` and `cursor: Vec2` (last known cursor position, default `Vec2::new(0.0, 0.0)`).
- **`on_pointer_down(pos, doc, history)`**:
  - `Idle` → records `pos` as center; transitions to `WaitingRadius { center: pos }`.
  - `WaitingRadius { center }` → computes `r = pos.distance(center)`. If `r < crate::geometry::EPSILON`, the click is degenerate (cursor coincides with center); do nothing and remain in `WaitingRadius`. Otherwise commit `Box::new(CreateCircle::new(Circle::new(center, r)))` via `history.commit(cmd, doc)` and reset to `Idle`.
- **`on_pointer_move(pos, doc)`**: stores `pos` in `self.cursor` (for use in `preview()`). No document mutation.
- **`on_pointer_up`**: no-op. The circle gesture is two clicks (down events), not a click-drag-release; `on_pointer_up` does nothing.
- **`preview() -> Vec<Entity>`**:
  - `Idle` → `vec![]`.
  - `WaitingRadius { center }` → `vec![Entity::Circle(Circle::new(center, self.cursor.distance(center)))]`. When `cursor == center` (r == 0.0), this yields a degenerate zero-radius circle in the preview; that is acceptable (the renderer draws nothing visible). The preview is ephemeral and never committed.
- **`cancel()`**: resets state to `Idle`, zeroes `cursor`. Clears any in-progress preview implicitly (since `preview()` returns `vec![]` in `Idle`).
- **`on_key(key, app)`**: `egui::Key::Escape` → calls `self.cancel()`. All other keys are no-ops.
- **`name() -> &'static str`**: returns `"CIRCLE"`.
- Update `src/tools/mod.rs`: add `pub mod circle;` and `pub use circle::CircleTool;`.
- File size: `src/tools/circle.rs` ≤ 300 LOC.
- Per-module unit tests under `#[cfg(test)] mod tests` in `circle.rs`. No new file under `tests/`.
- Must NOT import `eframe` or `rfd`. Importing `egui` is allowed (required for `on_key`).

## Out of scope

- **Snap integration** — LCV-054 adds snap-aware point resolution for all drawing tools. In LCV-046, the tool simply receives whatever world-space `pos` the plumbing (LCV-041) delivers; if snap is live, that position is already snapped.
- **Ortho lock** — not applicable to CircleTool (center-radius gesture has no axis constraint).
- **Status bar / command-line prompts** — LCV-067 (status bar) and LCV-068 (command-line widget) wire the tool name and per-state prompts. In this demand `name()` returns `"CIRCLE"` and per-state strings (`"CIRCLE: Specify center point"` / `"CIRCLE: Specify radius"`) are noted for documentation but not delivered to any UI widget.
- **Three-point circle** — not in AutoCAD R14's CIRCLE command default; not in v0.1.0 scope.
- **Circle-by-diameter** — rejected; keep the center-radius workflow only.
- **Keyboard entry for radius** — the command-line input (`@x,y`, distance entry) is LCV-068 territory.
- **Selection side-effects** — committed circles are not auto-selected. A separate `SelectionCommand` (LCV-027) handles that.
- **Undo** — the existing history stack (LCV-026) handles undo/redo of `CreateCircle` automatically; CircleTool adds nothing here.

## Acceptance criteria

1. `src/tools/circle.rs` exists and defines `pub struct CircleTool` implementing `crate::tools::Tool`. `src/tools/mod.rs` re-exports it via `pub use circle::CircleTool;`.
2. **`name()` exact**: `CircleTool::default().name() == "CIRCLE"`.
3. **Object-safe**: `let _: Box<dyn crate::tools::Tool> = Box::new(CircleTool::default());` compiles.
4. **Idle preview is empty**: `CircleTool::default().preview().is_empty()` is `true`.
5. **First click transitions to WaitingRadius**: after `on_pointer_down(Vec2::new(10.0, 20.0), &mut doc, &mut hist)`, `preview()` on a subsequent `on_pointer_move(Vec2::new(10.0, 30.0), &mut doc)` returns exactly one `Entity::Circle` with `center == Vec2::new(10.0, 20.0)` and `r` approximately `10.0` (within `EPSILON`).
6. **Preview radius matches cursor distance**: in `WaitingRadius`, `preview()[0]` is `Entity::Circle(Circle { center, r })` where `r == cursor.distance(center)` within `EPSILON`. Tested with center `(0.0, 0.0)` and cursor at `(3.0, 4.0)` → `r ≈ 5.0`.
7. **Second click commits circle**: starting from `Document::default()`, click center at `(0.0, 0.0)`, then click at `(5.0, 0.0)` (r = 5.0). After the second `on_pointer_down`, `doc.entity_count() == 1` and `doc.entities[0] == Entity::Circle(Circle::new(Vec2::new(0.0, 0.0), 5.0))`.
8. **Committed circle is undoable**: after the commit in AC 7, `history.undo(&mut doc)` leaves `doc.entity_count() == 0`.
9. **State resets to Idle after commit**: after the second click commits, `preview().is_empty()` is `true` (tool is back in `Idle`).
10. **Degenerate click ignored**: in `WaitingRadius { center: Vec2::new(5.0, 5.0) }`, calling `on_pointer_down(Vec2::new(5.0, 5.0), &mut doc, &mut hist)` (r == 0.0) leaves `doc.entity_count() == 0` and the tool remains in `WaitingRadius` (next `preview()` call still returns one `Entity::Circle`).
11. **Escape cancels from WaitingRadius**: after the first click, calling `on_key(egui::Key::Escape, &mut app)` resets the tool; `preview().is_empty()` is `true`.
12. **Escape in Idle is a no-op**: `on_key(egui::Key::Escape, &mut app)` on a fresh `CircleTool::default()` does not panic; `preview()` stays empty.
13. **`cancel()` resets from WaitingRadius**: after one click (center captured), `cancel()` causes `preview().is_empty()` to be `true`.
14. **`on_pointer_move` updates preview continuously**: in `WaitingRadius { center: (0.0, 0.0) }`, after `on_pointer_move(Vec2::new(3.0, 4.0), &mut doc)` the preview radius is ≈ 5.0; after a subsequent `on_pointer_move(Vec2::new(6.0, 8.0), &mut doc)` the preview radius is ≈ 10.0.
15. Kernel-purity: `grep -nE '^use (eframe|rfd)' src/tools/circle.rs` returns no matches.
16. Size: `wc -l src/tools/circle.rs` reports `<= 300`.
17. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 2)**: `circle_tool_name_is_circle` — `assert_eq!(CircleTool::default().name(), "CIRCLE")`.
- **Unit (AC 3)**: `circle_tool_is_object_safe` — `let _: Box<dyn Tool> = Box::new(CircleTool::default());`.
- **Unit (AC 4)**: `idle_preview_is_empty` — `CircleTool::default().preview().is_empty()`.
- **Unit (AC 5, AC 6)**: `preview_in_waiting_radius_returns_circle` — click center `(0.0, 0.0)`, move to `(3.0, 4.0)`, assert preview is one `Entity::Circle` with center `(0.0, 0.0)` and `r ≈ 5.0` within `EPSILON`.
- **Unit (AC 7)**: `second_click_commits_circle_to_document` — full gesture from `Document::default()`; assert `entity_count() == 1` and exact `Entity::Circle` content.
- **Unit (AC 8)**: `committed_circle_is_undoable` — same setup as AC 7; call `history.undo(&mut doc)`; assert `entity_count() == 0`.
- **Unit (AC 9)**: `tool_resets_to_idle_after_commit` — after second click, `preview().is_empty()`.
- **Unit (AC 10)**: `degenerate_click_does_not_commit` — `WaitingRadius { center: (5.0, 5.0) }`, click at `(5.0, 5.0)`, assert `entity_count() == 0` and `preview()` still returns one entity (still in WaitingRadius).
- **Unit (AC 11)**: `escape_cancels_waiting_radius` — after first click, `on_key(Escape, &mut app)`; assert `preview().is_empty()`.
- **Unit (AC 12)**: `escape_in_idle_is_noop` — call `on_key(Escape, app)` on default tool; no panic, `preview().is_empty()`.
- **Unit (AC 13)**: `cancel_resets_from_waiting_radius` — after first click, `cancel()`; assert `preview().is_empty()`.
- **Unit (AC 14)**: `preview_radius_updates_on_each_pointer_move` — two consecutive `on_pointer_move` calls; assert radius changes correctly each time.
- **Static check (AC 15)**: `grep -nE '^use (eframe|rfd)' src/tools/circle.rs` returns no matches.
- **Size check (AC 16)**: `wc -l src/tools/circle.rs` reports `<= 300`.
- **Build gate (AC 17)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

*(none)*

## Notes

- **Commit call site**: `history.commit(Box::new(CreateCircle::new(Circle::new(center, r))), doc)` — `Circle` is `crate::geometry::Circle { center: Vec2, r: f64 }`. `CreateCircle` is `crate::document::commands::CreateCircle` (LCV-023, shipped in `src/document/commands/create.rs`).
- **`on_pointer_down` drives the gesture, not `on_pointer_up`**: circles are committed on the second *click down*, matching AutoCAD R14's CIRCLE command. `on_pointer_up` is a no-op.
- **Why `cursor` field and not transient local**: `preview()` is a `&self` method with no arguments — it has no access to a current position unless the tool stores the last known cursor position. `on_pointer_move` keeps this field current each frame so `preview()` is always correct.
- **Preview with r ≈ 0 renders as a dot or invisible**: `egui::Painter` draws circles with radius < 0.5 screen-pixels as an effective no-op. This is acceptable — the preview disappears at the center and grows smoothly as the cursor moves away. No special-casing needed.
- **Status text** (for reference when LCV-068 lands): `Idle` → `"CIRCLE: Specify center point"`, `WaitingRadius` → `"CIRCLE: Specify radius"`. The command-line widget demand owns the delivery of these strings to the UI; CircleTool need only expose `name()`.
- **Floating-point degenerate threshold**: `r < crate::geometry::EPSILON` (where `EPSILON ≈ 1e-10` mm) rather than `r == 0.0` exactly. This guards against sub-epsilon float noise from coordinate arithmetic, consistent with the kernel's normalization contract (`Vec2::normalize` uses the same guard).
- **Wiring into `App`**: the ToolManager (LCV-040) and pointer plumbing (LCV-041) already call `manager.preview()` each frame and write `app.preview_entities`; the draw_preview renderer (LCV-037) already consumes it. No changes to `app.rs` are required for this demand.
- **Reference**: AutoCAD R14 CIRCLE command, `center-radius` mode: `Command: CIRCLE`, `3P/2P/TTR/<Center point>: <click>`, `Diameter/<Radius>: <click>`. v2 mirrors this two-click flow precisely.
- **LOC budget guidance**: the full implementation (struct, state enum, two impl blocks — Tool + inherent, tests) fits comfortably within 200 LOC based on the comparable LineTool pattern. Keep tests inside the file to stay within 300 LOC.
