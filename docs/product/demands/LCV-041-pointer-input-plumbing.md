# LCV-041 — Pointer input plumbing through ToolManager

- **Status**: Done
- **Implementation**: a628859 — feat(LCV-041): PointerEvent type + ToolManager::on_pointer_event + snap resolve
- **Phase**: 4
- **Depends on**: LCV-040, LCV-016
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

After LCV-040 ships the `Tool` trait and `ToolManager`, `App::update` already routes raw egui pointer responses to the manager via ad-hoc `handle_pointer_down/move/up` calls. Three problems remain:

1. **No typed event boundary.** There is no `PointerEvent` type. Every tool author must study the egui `Response` contract to understand what pointer information exists; nothing enforces that tools receive world-space mm coordinates rather than screen pixels.
2. **Snap engine is never queried.** `App::active_snap` exists on the `App` struct but is `None` every frame. The snap engine (`crate::geometry::snap`) is fully implemented by LCV-016 but is never called during pointer movement. Tools that need to snap — LineTool, CircleTool, ArcTool — will each have to re-invent the call site if this isn't centralised first.
3. **Coordinate conversion is implicit.** The screen→world conversion happens inline in `App::update`. If a tool or a test ever needs to inspect what position was dispatched, there is no clean seam.

This demand closes all three gaps. After it lands, Phase-4 tools receive a `PointerEvent` whose `pos` field is always a world-space mm coordinate, already resolved through the snap engine. The snap engine is queried once per frame at the plumbing layer, and `App::active_snap` is the accurate, frame-current result that the snap marker renderer (LCV-038) already consumes.

## Scope

- **New file `src/tools/pointer_event.rs`** defining:
  - `pub enum PointerButton { Primary, Secondary, Middle }` — identifies which mouse button triggered a press or release. All three variants are defined now; only `Primary` routes to tools in this demand.
  - `pub enum PointerEvent { Move { pos: Vec2 }, Press { pos: Vec2, button: PointerButton }, Release { pos: Vec2, button: PointerButton } }` — the typed event that tools receive. `pos` is **world-space mm** in all variants; no screen pixels, no `egui::Pos2`.
  - Both types derive `Debug, Clone, Copy, PartialEq`.
  - Module-level doc comment: "All `pos` fields are world-space mm. Screen→world conversion and snap resolution happen in `App::update` before any `PointerEvent` is constructed."
  - No `egui`, `eframe`, or `rfd` imports. Only `crate::geometry::Vec2`.

- **Update `src/tools/mod.rs`**:
  - Add `pub mod pointer_event;`.
  - Re-export: `pub use pointer_event::{PointerButton, PointerEvent};`.

- **Update `src/tools/manager.rs`**:
  - Add `pub fn on_pointer_event(&mut self, event: PointerEvent, app: &mut App)` that dispatches:
    - `PointerEvent::Move { pos }` → `self.active.on_pointer_move(pos, app)`.
    - `PointerEvent::Press { button: PointerButton::Primary, pos }` → `self.active.on_pointer_down(pos, app)`.
    - `PointerEvent::Release { button: PointerButton::Primary, pos }` → `self.active.on_pointer_up(pos, app)`.
    - `PointerEvent::Press` or `PointerEvent::Release` with `Secondary` or `Middle` button → no-op (camera pan and future right-click menu are handled at `App::update` level, not tool level).
  - The existing `handle_pointer_down`, `handle_pointer_move`, `handle_pointer_up` methods stay intact (existing tests reference them).

- **Update `src/app.rs`** — add a named constant and a testable helper:
  - `pub const SNAP_TOLERANCE_PX: f64 = 10.0;` — snap tolerance in screen pixels. Multiplied by `camera.mm_per_px` to yield the world-space tolerance passed to the snap engine.
  - `pub fn resolve_snap(cursor: Vec2, entities: &[Entity], tolerance_mm: f64) -> (Vec2, Option<SnapResult>)`:
    - Converts `entities` slice to `Vec<SnapEntity>` via a trivial `match` on each `Entity` variant (Line→Line, Circle→Circle, Arc→Arc).
    - Calls `crate::geometry::snap::snap(cursor, tolerance_mm, &snap_entities)`.
    - Returns `(snap.point, Some(snap))` if a snap was found, `(cursor, None)` otherwise.
    - Pure function: no `egui`, no `&mut App`. Fully unit-testable.
  - **Inside `App::update`**, replace the current per-method pointer dispatch with snap-aware `on_pointer_event` dispatch:
    1. Compute `raw_world = self.camera.screen_to_world(hover_pos)`.
    2. Compute `tolerance_mm = SNAP_TOLERANCE_PX * self.camera.mm_per_px`.
    3. Call `let (pos, snap_result) = resolve_snap(raw_world, &self.document.entities, tolerance_mm);`.
    4. Set `self.active_snap = snap_result;`.
    5. Extract `tool_manager` via `std::mem::take(&mut self.tool_manager)` (borrow-split; existing pattern).
    6. On primary press: dispatch `PointerEvent::Press { pos, button: PointerButton::Primary }`.
    7. Always on hover: dispatch `PointerEvent::Move { pos }`.
    8. On primary release: dispatch `PointerEvent::Release { pos, button: PointerButton::Primary }`.
    9. Restore `self.tool_manager`.
  - When the cursor leaves the viewport (`!response.hovered()`), set `self.active_snap = None;` in that same frame.
  - LOC constraint: `src/app.rs` must stay ≤ 300 LOC. If adding `resolve_snap` and the constant pushes the file over the cap, extract `resolve_snap` and `SNAP_TOLERANCE_PX` into a new `src/app/snap.rs` sub-module; `App::update` calls the same function either way.

## Out of scope

- **`Tool` trait method signature changes** — `on_pointer_down/move/up` remain left-button–only APIs; no button argument is added to the `Tool` trait in this demand.
- **Right-button / middle-button events routed to tools** — middle button continues to drive camera pan (LCV-032); right button is reserved for a future context-menu demand (Phase 6). Neither routes through `on_pointer_event` here.
- **Per-tool snap toggle** — LCV-054 (snap integration into drawing tools) adds F3-toggle snap on/off and per-tool snap-kind masks. In this demand snap is always-on: if the snap engine returns a candidate, tools receive the snapped position.
- **Snap for arc-arc / arc-line intersections** — the snap engine (LCV-016) documents this as a current limitation. No change here.
- **Snap tolerance as a user setting** — `SNAP_TOLERANCE_PX = 10.0` is a compile-time constant. A settings slider is a Phase-5 concern.
- **Ortho lock** (LCV-053) — ortho position clamping will intercept snapped positions before dispatch; that demand is its own scope. This demand creates the seam but does not implement ortho.
- **Secondary-button events dispatched to tools** — `PointerButton::Secondary` and `PointerButton::Middle` are defined for forward-compatibility but produce no tool call in this demand.
- **Touch / pen events** — not in v2.

## Acceptance criteria

1. `src/tools/pointer_event.rs` exists and defines `pub enum PointerButton` with exactly three variants (`Primary`, `Secondary`, `Middle`) and `pub enum PointerEvent` with exactly three variants (`Move { pos: Vec2 }`, `Press { pos: Vec2, button: PointerButton }`, `Release { pos: Vec2, button: PointerButton }`). Both types derive `Debug, Clone, Copy, PartialEq`.
2. `PointerEvent`'s `pos` field is `Vec2` from `crate::geometry`. No `egui::Pos2`, no `f32` fields. Verified: `grep -nE 'Pos2|: f32' src/tools/pointer_event.rs` returns no matches in field definitions.
3. `src/tools/mod.rs` re-exports `PointerButton` and `PointerEvent` via `pub use pointer_event::{…}`.
4. `src/tools/manager.rs` has `pub fn on_pointer_event(&mut self, event: PointerEvent, app: &mut App)`. A unit test confirms: `Move { pos }` dispatches to `on_pointer_move(pos, …)`; `Press { Primary, pos }` dispatches to `on_pointer_down(pos, …)`; `Release { Primary, pos }` dispatches to `on_pointer_up(pos, …)`; `Press { Secondary, pos }` and `Release { Middle, pos }` do NOT call `on_pointer_down` or `on_pointer_up`.
5. `pub const SNAP_TOLERANCE_PX: f64` is defined and equals `10.0`. It is accessible from tests without importing `egui`.
6. `pub fn resolve_snap(cursor: Vec2, entities: &[Entity], tolerance_mm: f64) -> (Vec2, Option<SnapResult>)` exists. Given a cursor within `tolerance_mm` of an entity endpoint, it returns `(snapped_point, Some(SnapResult { kind: Endpoint, … }))`. Given a cursor far from all entities, it returns `(cursor, None)`. Given an empty entities slice, it returns `(cursor, None)`.
7. In `App::update`, the snap engine is called once per frame while the cursor hovers the viewport. `self.active_snap` is set to the `Option<SnapResult>` returned by `resolve_snap`. Tools receive the snapped position when `active_snap` is `Some`, and the raw world position when `active_snap` is `None`.
8. When `!response.hovered()` (cursor has left the viewport), `self.active_snap` is set to `None` in the same frame. A future frame where the cursor re-enters restores snap normally.
9. `App::update` uses `on_pointer_event` (not the old `handle_pointer_down/move/up` direct calls) for the left-button dispatch. A `grep -n 'handle_pointer_down\|handle_pointer_move\|handle_pointer_up' src/app.rs` returns no matches (or only matches inside comments).
10. The borrow-splitting pattern is preserved: `std::mem::take(&mut self.tool_manager)` is used before calling `on_pointer_event`, and `self.tool_manager` is restored immediately after. No `unsafe` is introduced.
11. `src/tools/pointer_event.rs` ≤ 80 LOC; `src/tools/manager.rs` ≤ 180 LOC; `src/app.rs` ≤ 300 LOC (or each file in a sub-module split ≤ 300 LOC).
12. Kernel-purity: `grep -nE '^use (eframe|rfd)' src/tools/pointer_event.rs` returns no matches.
13. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: `pointer_event_all_variants_compile` — constructs `PointerEvent::Move { pos: Vec2::new(5.0, 10.0) }`, `PointerEvent::Press { pos: Vec2::new(0.0, 0.0), button: PointerButton::Primary }`, `PointerEvent::Release { pos: Vec2::new(1.0, 2.0), button: PointerButton::Secondary }`. Compile-check covers AC 1; `assert_eq!` on `Copy` round-trip covers derive.
- **Unit (AC 4 — dispatch to on_pointer_move)**: `on_pointer_event_move_dispatches_to_pointer_move` — `MockTool` records calls; call `tm.on_pointer_event(PointerEvent::Move { pos }, app)`; assert `mock.move_called_with == Some(pos)`.
- **Unit (AC 4 — dispatch to on_pointer_down)**: `on_pointer_event_primary_press_dispatches_to_pointer_down` — call `PointerEvent::Press { Primary, pos }`; assert `mock.down_called_with == Some(pos)`.
- **Unit (AC 4 — dispatch to on_pointer_up)**: `on_pointer_event_primary_release_dispatches_to_pointer_up` — call `PointerEvent::Release { Primary, pos }`; assert `mock.up_called_with == Some(pos)`.
- **Unit (AC 4 — non-primary noop)**: `on_pointer_event_secondary_press_is_noop` — call `PointerEvent::Press { Secondary, pos }` and `PointerEvent::Release { Middle, pos }`; assert `mock.down_called_with == None` and `mock.up_called_with == None`.
- **Unit (AC 6 — snap hit)**: `resolve_snap_returns_endpoint_when_cursor_near_line_end` — create `Entity::Line(Line { p1: Vec2::new(0.0, 0.0), p2: Vec2::new(10.0, 0.0) })`; call `resolve_snap(Vec2::new(0.2, 0.0), &entities, 1.0)`; assert result is `(Vec2::new(0.0, 0.0), Some(SnapResult { kind: SnapKind::Endpoint, … }))`.
- **Unit (AC 6 — snap miss)**: `resolve_snap_returns_raw_when_cursor_out_of_range` — same entity, cursor at `(20.0, 0.0)`, tolerance `1.0`; assert result is `(Vec2::new(20.0, 0.0), None)`.
- **Unit (AC 6 — empty entities)**: `resolve_snap_empty_entities_returns_cursor_unchanged` — empty slice, any cursor, any tolerance; assert `result.0 == cursor` and `result.1 == None`.
- **Static check (AC 12)**: `grep -nE '^use (eframe|rfd)' src/tools/pointer_event.rs` returns no matches.
- **Build gate (AC 13)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

*(none)*

## Notes

- **`SnapEntity` conversion**: `crate::geometry::snap::SnapEntity` is a transitional type that mirrors `crate::document::Entity` exactly (`Line`, `Circle`, `Arc` variants carrying the same kernel value types). `resolve_snap` converts via a trivial `match`: `Entity::Line(l) => SnapEntity::Line(l)`, etc. When a future demand unifies the two types, the conversion disappears; `resolve_snap`'s signature stays stable.

- **`SNAP_TOLERANCE_PX` formula**: `tolerance_mm = SNAP_TOLERANCE_PX * camera.mm_per_px`. At the default zoom (`mm_per_px = 1.0`) this is 10 mm — generous for first use. At 10:1 zoom (`mm_per_px = 0.1`) it is 1 mm. The formula matches how v1 computed snap radius. Phase-5 settings can expose a sensitivity slider that replaces the constant.

- **Borrow split**: `App` owns `ToolManager`; `on_pointer_event` takes `&mut App`. The compile-time borrow conflict is resolved with `std::mem::take` — same pattern already live in `App::update` from LCV-040 (see lines 110–126 of `src/app.rs`). `ToolManager` already implements `Default` (LCV-040), so `take` is safe.

- **Why `PointerButton` on `PointerEvent` if tools ignore it**: the boundary between "what happened at the pointer" and "what the tool does" is clean. Ortho lock (LCV-053) will modify `PointerEvent::Move.pos` before forwarding to the tool; a future right-click-to-cancel demand can add `on_key`-equivalent routing for `PointerButton::Secondary` without changing the type. Adding the button now costs zero.

- **`App::active_snap` consumer**: `draw_snap_marker` in `src/render/snaps.rs` (LCV-038) already reads `app.active_snap` — it just always sees `None` today. After this demand it sees the correct snap result each frame with no changes to the renderer.

- **Reference**: v1's `InputHandler.ts` called `snapEngine.query(rawPos)` before forwarding to the tool, returning either the snapped point or the raw point. v2 follows the same contract.
