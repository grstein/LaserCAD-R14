# LCV-067 — Status bar (coords / units / active tool)

- **Status**: Ready
- **Phase**: 6
- **Depends on**: LCV-030 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

Once the camera, tools, and snap engine are in place, the operator has no at-a-glance read-out of where the cursor is in world space, which tool is active, or how many entities the document holds. In a CAD-to-laser workflow this matters: the user needs to confirm millimeter-precise cursor coordinates while placing geometry, verify at a glance that the right tool is engaged (LINE vs. CIRCLE vs. SELECT), and see how many entities are in the drawing without opening a properties dialog. The status bar is the AutoCAD R14 idiom for all three: a single persistent strip at the bottom of the window showing live coordinates, active tool, and document state.

## Scope

- New file `src/ui/statusbar.rs` containing:
  - `pub fn draw_statusbar(ui: &mut egui::Ui, app: &App)` — renders the three status segments.
  - Coordinates segment: when `app.last_cursor_world` is `Some(p)`, renders `X: {p.x:.2}  Y: {p.y:.2}  mm`; when `None`, renders `X: —  Y: —  mm`. Values are formatted with exactly two decimal places.
  - Tool segment: renders `Tool: {name}` where `name = app.tool_manager.active_tool_name()` uppercased.
  - Entity count segment: renders `Entities: {n}` where `n = app.document.entities.len()`.
  - All three segments laid out horizontally via `ui.horizontal(|ui| { ... })`. Segments are separated by a single `ui.separator()` between each pair.
  - File stays `≤ 150 LOC`. No `rfd` import anywhere in the file.
- `src/ui/mod.rs`: add `pub mod statusbar;` and re-export `pub use statusbar::draw_statusbar;`.
- `src/app.rs` `App::update`: call `egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| { draw_statusbar(ui, self); });` **before** the `CentralPanel` block, so egui allocates the bottom strip first and the central panel fills the remainder.

## Out of scope

- Snap mode indicator (F3 toggle) — owned by LCV-054 / LCV-070.
- Ortho mode indicator (F8 toggle) — owned by LCV-053 / LCV-070.
- Units toggle (mm ↔ inches) — there is no unit-toggle feature in v2; mm is canonical and immutable.
- Grid spacing display — belongs to LCV-033 if ever needed.
- Selection count (how many entities are selected) — no current demand; defer.
- Any clickable / interactive element inside the status bar — the bar is read-only in this demand.
- Theme / visual polish beyond `egui` defaults — owned by LCV-071.
- Zoom level display — no current demand; defer.

## Acceptance criteria

1. `src/ui/statusbar.rs` exists, is `≤ 150 LOC`, and contains no `rfd` import.
2. `src/ui/mod.rs` declares `pub mod statusbar;` and re-exports `draw_statusbar` as `pub use statusbar::draw_statusbar;`.
3. When `app.last_cursor_world = Some(Vec2 { x: 123.456, y: 7.8 })`, the rendered label text contains `X: 123.46` and `Y: 7.80` (two decimal places, standard rounding).
4. When `app.last_cursor_world = None`, the rendered label text contains `X: —` and `Y: —`.
5. The tool segment always renders the active tool name uppercased: `ToolManager::default()` → label contains `Tool: SELECT`.
6. The entity count segment renders `Entities: 0` on a default document; after inserting one entity via `app.document.entities.push(...)`, renders `Entities: 1`.
7. `App::update` in `src/app.rs` calls `draw_statusbar` inside `TopBottomPanel::bottom("statusbar")`, placed **before** the `CentralPanel` block.
8. `cargo run` shows a status bar strip at the bottom of the window. Moving the cursor over the viewport updates the X/Y readout in real time. (Manual smoke.)
9. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

All tests in `#[cfg(test)] mod tests` inside `src/ui/statusbar.rs`.

- **AC#3 — coord formatting with Some**: call a private helper `format_coords(Some(Vec2::new(123.456, 7.8)))` and assert the result equals `"X: 123.46  Y: 7.80  mm"`. (Extract the format string into a `pub(crate) fn format_coords(pos: Option<Vec2>) -> String` helper so it can be unit-tested without an egui context.)
- **AC#4 — coord formatting with None**: call `format_coords(None)` and assert the result equals `"X: —  Y: —  mm"`.
- **AC#5 — tool name uppercased**: construct a `ToolManager::default()` and assert `active_tool_name().to_uppercase() == "SELECT"`. (The uppercasing lives in `draw_statusbar`; the test verifies that `active_tool_name()` returns the expected base string so the format is deterministic.)
- **AC#6 — entity count**: construct `App::default()`, assert entity count label would be `"Entities: 0"`; push one entity, assert count is `1`. (Test via `app.document.entities.len()` directly — the rendering path is covered by the manual smoke.)
- **AC#7 — panel ordering (static read)**: `grep` for `TopBottomPanel::bottom` appearing before `CentralPanel` in `src/app.rs` — verified by code review; not a runtime test.
- **AC#9 — build gate**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- `app.document.entities` is a `Vec<Entity>` (declared in `src/document/entity.rs`, owned by `Document`). Use `.len()` directly; `entity_count()` is also available on `Document` but both are equivalent.
- `ToolManager::active_tool_name()` returns `&'static str` (e.g. `"Select"`, `"Line"`, `"Circle"`). Uppercase it with `.to_uppercase()` at the call site in `draw_statusbar` for the `Tool: LINE` display style.
- The `format_coords` helper should use Rust's `format!("X: {:.2}  Y: {:.2}  mm", p.x, p.y)` for the `Some` branch and `"X: —  Y: —  mm".to_string()` for `None`. The em-dash `—` (U+2014) matches the AutoCAD R14 convention for "no value".
- `TopBottomPanel::bottom` must be called before `CentralPanel` because egui processes panels in declaration order: a bottom panel declared after the central panel has zero height. See egui layout docs.
- `src/ui/statusbar.rs` MUST NOT import `eframe` or `rfd` (kernel-purity rule in AGENTS.md §Purity rule applies to statusbar as a pure UI widget).
- `egui` import is allowed and required for `egui::Ui`.
- The LOC cap is 150, well within the module hard-cap of 300. The implementation should be simple: one `pub fn`, one `pub(crate) fn` helper, and a `#[cfg(test)] mod tests` block.
- v1 reference: the TypeScript v1 showed a bottom status bar with coordinate readout and tool name label. Same UX intent here.
