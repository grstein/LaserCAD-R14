# LCV-032 — Viewport wiring (pointer input → tools)

- **Status**: Done
- **Phase**: 3
- **Depends on**: LCV-031
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: 33bae5a — feat(LCV-032): viewport wiring — pointer hover, wheel zoom, middle-drag pan, zoom-extents key

## Problem

The viewport is allocated (LCV-030) and the Camera math exists (LCV-031), but the cursor is still inert. Phase-4 tools (LineTool, CircleTool, SelectTool) cannot start without a single, single-sourced answer to "what is the cursor's world position right now?". The snap engine (LCV-016) needs the same value. Without this demand the renderer can draw a grid but the operator cannot interact with it; they cannot pan, zoom, or zoom-extents from the keyboard.

This demand widens the viewport's `Sense` to `click_and_drag()`, reads pointer events out of the egui `Response`, stores the cursor's last-known world position on `App`, and wires three camera actions to keyboard / mouse input: wheel → zoom-around-cursor; middle-button-drag → pan; `F` key (or `Ctrl+0`) → zoom-extents.

User outcome: the operator moves the mouse and the status bar (Phase 6) will show live mm coordinates from `app.last_cursor_world`. The wheel zooms toward the cursor; the middle button pans 1:1; pressing `F` fits the drawing. No drawing tool yet — but the camera responds to input.

## Scope

- Update `src/app.rs`:
  - Add `pub last_cursor_world: Option<Vec2>` to the `App` struct. Default = `None` (cursor not yet over viewport).
  - Inside `App::update`, after the viewport rect is allocated, request a `click_and_drag`-sensing response: `let response = ui.interact(rect, ui.id().with("viewport"), egui::Sense::click_and_drag());` (or `ui.allocate_rect(rect, egui::Sense::click_and_drag())` — whichever current egui idiom matches the version in `Cargo.lock`; the implementer follows the egui-best-practices skill).
  - Update `self.camera.viewport_size_px = [rect.width(), rect.height()];` (LCV-031 already added this; verify it still happens before the interact call).
  - **Pointer hover**: if `response.hovered()` and `let Some(pos) = response.hover_pos()` (or `ctx.input(|i| i.pointer.hover_pos())` — pick one source), set `self.last_cursor_world = Some(self.camera.screen_to_world(pos))`. Otherwise leave it unchanged (`hovered() == false` and the cursor left the viewport means the last-seen value persists — that's fine; consumers check `hovered()` if they care).
  - **Wheel zoom**: if `response.hovered()`, read `ctx.input(|i| i.smooth_scroll_delta.y)` (or `raw_scroll_delta.y` — implementer picks the egui-recommended one for wheel-tick events). For each unit of wheel scroll, compute `factor = if delta > 0.0 { 1.1 } else { 0.9 }` and call `self.camera.zoom_around(hover_pos, factor)`. The zoom anchor is the cursor's screen position. If scroll delta is zero, skip.
  - **Middle-button pan**: if `response.dragged_by(egui::PointerButton::Middle)`, call `self.camera.pan(response.drag_delta())`. (`drag_delta()` returns the per-frame delta since the last frame; calling `pan` per frame integrates correctly.)
  - **Zoom-extents key**: in the same `update` body, check `ctx.input(|i| i.key_pressed(egui::Key::F))`. If pressed and the viewport is focused or hovered (implementer picks the simpler of the two — being hovered is sufficient for v2), call `self.camera.zoom_extents(self.document.bounds(), [rect.width(), rect.height()])`. Also accept `Ctrl+0` (Ctrl + the digit zero) as a second binding for the same action — matches AutoCAD's "Zoom Extents" shortcut on numeric keypad-style bindings.
  - **Repaint discipline**: KISS — call `ctx.request_repaint()` at the end of `update` so the cursor-coords-in-statusbar (Phase 6) updates smoothly. The skill's "egui doesn't repaint unless something changed" caveat is acknowledged; v2 will optimize repaint scheduling later if profiling shows wasted frames. For now, always repaint.
- File size: `src/app.rs` stays ≤300 LOC. Split into `src/app/input.rs` if the body grows past the cap. The implementer estimates ~80 additional LOC; well within budget.
- The kernel-purity rule does not apply to `src/app.rs`.

## Out of scope

- **Tool dispatch**: this demand does not pick which tool consumes a click. `last_cursor_world` is set; click/release events are stored on the `Response` but no tool consumes them yet. The Tool trait and ToolManager arrive in **LCV-040**.
- **Click → entity selection**: owned by **LCV-042** (SelectTool).
- **Keyboard tool shortcuts** (`L` for line, `C` for circle, etc.): owned by **LCV-070**. The `F` and `Ctrl+0` zoom-extents keys are scoped to this demand because they directly drive Camera, which this demand owns.
- **Right-click context menu**: not in v2 yet (Phase 6 dialogs).
- **Status bar widget** showing live coords: owned by **LCV-067**. This demand only stores `last_cursor_world`; nothing reads it yet beyond unit tests.
- **Touch / pen / multi-finger pan-zoom**: egui supports it but v2 does not require it; out of scope.
- **Wheel sensitivity slider**: a settings concern (Phase 5). Fixed factor of 1.1/0.9 per wheel notch is sufficient.
- **Repaint optimization** (only repaint when state changes): documented as a future tweak; this demand always repaints.

## Acceptance criteria

1. `src/app.rs` `App` struct carries `pub last_cursor_world: Option<Vec2>`. `App::default().last_cursor_world == None`.
2. Inside `App::update`, the viewport rect is allocated with `egui::Sense::click_and_drag()` (or equivalent — must accept clicks and drags, not just hover). `grep -nE 'Sense::(click_and_drag|drag|click)' src/app.rs` returns at least one match in the viewport allocation.
3. When the pointer hovers over the viewport, `self.last_cursor_world` is updated each frame to `Some(camera.screen_to_world(hover_pos))`. (Verified via the unit test in AC 8 below — a helper function or a re-entrant unit test on `Camera::screen_to_world` since direct egui hover testing is impractical.)
4. Mouse-wheel scroll over the viewport calls `camera.zoom_around(hover_pos, factor)` with `factor > 1.0` for positive scroll and `factor < 1.0` for negative scroll. The zoom anchor is the cursor's screen position. (Manual smoke — see Expected tests.)
5. Middle-mouse-button drag over the viewport calls `camera.pan(drag_delta)`. (Manual smoke.)
6. Pressing the `F` key while the viewport is hovered calls `camera.zoom_extents(self.document.bounds(), [rect.width(), rect.height()])`. The same action is triggered by `Ctrl+0`. (Manual smoke + unit test of the dispatch function below.)
7. `ctx.request_repaint()` is called once per `update` invocation. (Implementer documents in a code comment if they decide to optimize to "only when state changed"; the simpler form is acceptable.)
8. The pointer→camera dispatch logic is extracted into a testable helper, e.g., `fn handle_wheel_zoom(camera: &mut Camera, anchor: egui::Pos2, scroll_delta: f32)` and `fn handle_zoom_extents(camera: &mut Camera, doc: &Document, viewport_size: [f32; 2])`. Each helper has a unit test exercising it without egui's UI loop: pass a known camera + scroll delta, assert post-state.
9. Size: `wc -l src/app.rs` reports `<= 300` (or `wc -l src/app/*.rs` each reports `<= 300` if split).
10. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `app_default_has_no_cursor_world` — `assert_eq!(App::default().last_cursor_world, None);`.
- **Unit (AC 3)**: test `screen_to_world_via_camera_writes_cursor_world` — constructs a `Camera` with a known viewport, calls `camera.screen_to_world(...)`, and verifies the result matches a hand-computed value. (Direct egui hover simulation is not required; testing the math is sufficient. AC 3's behavior is covered by AC 7 in LCV-031 plus the dispatch helper test.)
- **Unit (AC 8)**: tests `wheel_zoom_dispatch_positive_scroll_zooms_in`, `wheel_zoom_dispatch_negative_scroll_zooms_out`, `wheel_zoom_dispatch_zero_scroll_is_noop`, `zoom_extents_dispatch_calls_camera`. Each constructs a `Camera` (and a `Document` with known bounds where applicable), invokes the helper, and asserts the camera's post-state matches expectation.
- **Manual smoke (AC 4)**: run `cargo run`. Move the cursor over the viewport; scroll the mouse wheel up — the viewport zooms in, with the world point under the cursor staying under the cursor (visually verifiable once the grid lands in LCV-033; in this demand, manually check by recording `app.last_cursor_world` via a temporary `dbg!` in development if needed). Record success in the demand's `Implementation:` line.
- **Manual smoke (AC 5)**: hold the middle mouse button and drag across the viewport. With a future grid (LCV-033) the grid translates 1:1 with the cursor; in this demand, the change is invisible (no rendered content) but the camera's `center_world` shifts. Manual verification deferred to LCV-033 acceptance is acceptable.
- **Manual smoke (AC 6)**: with an empty document, press `F` — no visible change (bounds is None; zoom_extents resets to default). Manual verification of the keypress firing the helper is satisfied by AC 8's unit test.
- **Build gate (AC 10)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **Testability**: directly testing egui input handling inside `update` requires `egui_kittest` or a stub `Ui`. The egui-best-practices skill recommends NOT adding `egui_kittest` preemptively. The compromise: extract the dispatch into pure helper functions (`handle_wheel_zoom`, `handle_zoom_extents`) that take camera + numeric inputs, leaving `update` as a thin shim that reads egui state and calls helpers. The helpers are unit-testable; `update` is verified by manual smoke. This pattern repeats in LCV-041 (Tool dispatch) and Phase 6 chrome demands.
- **Wheel sensitivity**: `factor = 1.1` per wheel tick is a common default; AutoCAD's default is similar. v2 does not expose a sensitivity setting in Phase 3; if a user complains, Phase 5 settings can add a slider.
- **Why `F` and `Ctrl+0`**: `F` matches FreeCAD / Inkscape (fit-view); `Ctrl+0` matches Chrome / web zoom-to-100% and is on the home row of every keyboard. Pick both; muscle memory carries either way. AutoCAD R14 uses `Z` then `E` (zoom-extents subcommand) — that flow is reserved for the Phase-6 command line (LCV-068) and is not in scope here.
- **`response.hovered()` vs `response.contains_pointer()`**: depending on the egui version, one or both may be available. The implementer picks whichever returns "the cursor is over the viewport rect AND the viewport has hover focus" semantics. Both are acceptable for v2.
- **`drag_delta()` vs `interact_pointer_pos()`**: `drag_delta()` is the per-frame translation since the last frame and matches `pan`'s contract. Using `interact_pointer_pos() - drag_started_pos()` would accumulate; not what we want.
- **Repaint discipline**: KISS says "always repaint". This burns CPU when idle but eliminates a class of "the coordinates froze" bugs. Phase 6 (statusbar) and Phase 7 (agent) introduce more state changes that need repaint anyway; profiling can revisit.
- Reference: v1's `App.tsx` bound the same actions: `onWheel` → camera.zoom; middle-drag → camera.pan; `F` → camera.zoomToExtents. v2 keeps the same shortcuts.
