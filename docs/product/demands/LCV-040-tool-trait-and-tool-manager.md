# LCV-040 — Tool trait + ToolManager

- **Status**: Ready
- **Phase**: 4
- **Depends on**: LCV-022, LCV-032, LCV-037
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: <to be filled by demand-manager>

## Problem

The render pipeline paints entities (LCV-035), selection highlights (LCV-036), and preview overlays (LCV-037). The viewport responds to pointer input (LCV-032) and stores `last_cursor_world`. The Command trait (LCV-022) and History stack (LCV-026) enable reversible mutations. But nothing connects user input to drawing actions — there is no tool abstraction.

Without a `Tool` trait, every drawing or selection demand (LCV-041 LineTool, LCV-042 SelectTool, LCV-046 CircleTool) would invent its own ad-hoc input-handling contract. Duplicate logic would creep in, preview rendering would be inconsistent, and switching tools would require scattered conditionals.

This demand ships:
1. The `Tool` trait — a state-machine interface that tools implement.
2. `ToolManager` — the owner of the active tool, routing pointer and key events.
3. `SelectTool` stub — a no-op placeholder so the app starts with a tool.
4. `App::commit` — the sole legal path for tools to mutate the document.
5. Wiring: `App` holds a `ToolManager`, calls `update_preview` each frame, and routes pointer events through the manager.

User outcome: after this demand lands, the framework is ready. The next demand can ship `LineTool` (LCV-041), implement `Tool` for it, and the operator draws lines. Every tool after that follows the same contract.

## Scope

- **New file `src/tools/tool.rs`** defining:
  - `pub trait Tool` with these methods:
    - `fn name(&self) -> &'static str` — tool name for statusbar display (e.g., `"Select"`, `"Line"`).
    - `fn on_pointer_down(&mut self, pos: Vec2, app: &mut App)` — left-button press at `pos` (world mm).
    - `fn on_pointer_move(&mut self, pos: Vec2, app: &mut App)` — cursor movement to `pos` (world mm).
    - `fn on_pointer_up(&mut self, pos: Vec2, app: &mut App)` — left-button release at `pos` (world mm).
    - `fn on_key(&mut self, key: egui::Key, app: &mut App)` — keyboard input (e.g., Escape to cancel).
    - `fn preview(&self) -> Vec<Entity>` — returns preview geometry for the current tool state; empty if no preview. The returned `Vec<Entity>` wires into `app.preview_entities` (LCV-037).
    - `fn cancel(&mut self)` — resets the tool to idle state (called on Escape or tool switch).
  - The trait is **object-safe** — no generic methods, no associated types. `ToolManager` stores `Box<dyn Tool>`.
  - Module doc comment stating: "Tools never mutate Document directly; they construct a `Box<dyn Command>` and call `App::commit(cmd)`."
  - File imports `egui::Key`, `crate::document::Entity`, and forward-declares `crate::app::App`. No `eframe`, no `rfd`.

- **New file `src/tools/manager.rs`** defining:
  - `pub struct ToolManager { active: Box<dyn Tool> }`.
  - `pub fn new(initial: Box<dyn Tool>) -> Self` — constructor.
  - `pub fn set_tool(&mut self, tool: Box<dyn Tool>)` — cancels the old tool (`self.active.cancel()`), replaces it with `tool`.
  - `pub fn active_tool_name(&self) -> &'static str` — delegates to `self.active.name()`.
  - `pub fn handle_pointer_down(&mut self, pos: Vec2, app: &mut App)` — delegates to `self.active.on_pointer_down(pos, app)`.
  - `pub fn handle_pointer_move(&mut self, pos: Vec2, app: &mut App)` — delegates to `self.active.on_pointer_move(pos, app)`.
  - `pub fn handle_pointer_up(&mut self, pos: Vec2, app: &mut App)` — delegates to `self.active.on_pointer_up(pos, app)`.
  - `pub fn handle_key(&mut self, key: egui::Key, app: &mut App)` — delegates to `self.active.on_key(key, app)`.
  - `pub fn preview(&self) -> Vec<Entity>` — delegates to `self.active.preview()`.
  - File imports `egui::Key`, `crate::geometry::Vec2`, `crate::document::Entity`, and forward-declares `crate::app::App`. No `eframe`, no `rfd`.

- **New file `src/tools/select.rs`** defining:
  - `pub struct SelectTool;` — a minimal stub.
  - `impl Default for SelectTool` — returns `SelectTool`.
  - `impl Tool for SelectTool`:
    - `name() -> "Select"`
    - `on_pointer_down/move/up` — no-ops (empty body).
    - `on_key` — no-op.
    - `preview() -> vec![]` — empty; no preview geometry.
    - `cancel()` — no-op.
  - Doc comment: "Placeholder for the real SelectTool (LCV-042). This stub lets the app start with a valid active tool."

- **Update `src/tools/mod.rs`**:
  - Add `pub mod tool;`, `pub mod manager;`, `pub mod select;`.
  - Add `pub use tool::Tool;`, `pub use manager::ToolManager;`, `pub use select::SelectTool;`.

- **Update `src/app.rs`**:
  - Add `pub tool_manager: ToolManager` to the `App` struct.
  - Implement `Default` for `App` such that `tool_manager` initializes to `ToolManager::new(Box::new(SelectTool::default()))`.
  - Add method `pub fn commit(&mut self, cmd: Box<dyn Command>)`:
    - Executes the command via `self.history.commit(cmd, &mut self.document)`.
    - This is the **only legal path** for tools to mutate the document.
  - In `App::update`, **after** reading pointer events but **before** drawing:
    - If left mouse button was pressed this frame, call `self.tool_manager.handle_pointer_down(world_pos, self)` (requires splitting borrow — see Notes).
    - If pointer moved, call `self.tool_manager.handle_pointer_move(world_pos, self)`.
    - If left mouse button was released this frame, call `self.tool_manager.handle_pointer_up(world_pos, self)`.
    - If Escape key was pressed, call `self.tool_manager.handle_key(egui::Key::Escape, self)`.
  - Update `self.preview_entities = self.tool_manager.preview();` each frame before `draw_preview`.

- File sizes: `src/tools/tool.rs` ≤150 LOC, `src/tools/manager.rs` ≤150 LOC, `src/tools/select.rs` ≤50 LOC.
- The `src/tools/*.rs` files may import `egui` for `egui::Key`. They MUST NOT import `eframe` or `rfd`.

## Out of scope

- **LineTool** (LCV-041), **SelectTool** full implementation (LCV-042), **CircleTool** (LCV-046), **ArcTool** (LCV-047), **TrimTool** (LCV-050), **ExtendTool** (LCV-051), **DeleteTool** (LCV-048), **MoveTool** (LCV-049) — each is its own demand.
- **Tool shortcuts** (`L` for Line, `C` for Circle, `Escape` to switch to Select) — owned by LCV-070 (keyboard shortcuts).
- **Toolbar buttons** for tool switching — owned by LCV-064 (toolbar UI).
- **Right-click to cancel** — v2 uses Escape only; no right-click cancel.
- **Multi-button pointer events** (middle, right) — only left-button goes through tools; middle is pan (LCV-032), right is reserved for context menus (Phase 6).
- **Touch gestures** — not in v2.
- **Tool state persistence** across sessions — tools start fresh each run.
- **Tool options panel** (e.g., line width, snap settings per tool) — deferred; not in Phase 4.

## Acceptance criteria

1. `src/tools/tool.rs` exists and defines `pub trait Tool` with exactly seven methods: `name`, `on_pointer_down`, `on_pointer_move`, `on_pointer_up`, `on_key`, `preview`, `cancel` — matching the signatures in Scope.
2. The `Tool` trait is **object-safe**: the test file constructs `let _: Box<dyn Tool> = Box::new(SelectTool);` and compiles.
3. `src/tools/manager.rs` exists and defines `pub struct ToolManager` with the API surface in Scope: `new`, `set_tool`, `active_tool_name`, `handle_pointer_down`, `handle_pointer_move`, `handle_pointer_up`, `handle_key`, `preview`.
4. `src/tools/select.rs` exists and defines `pub struct SelectTool` implementing `Default` and `Tool`. `SelectTool::default().name()` returns the exact string `"Select"`.
5. `src/tools/mod.rs` re-exports `Tool`, `ToolManager`, and `SelectTool`.
6. `src/app.rs` `App` struct carries `pub tool_manager: ToolManager`. `App::default().tool_manager.active_tool_name()` returns `"Select"`.
7. `App` has a method `pub fn commit(&mut self, cmd: Box<dyn Command>)` that executes the command via `self.history.commit(cmd, &mut self.document)`.
8. Calling `app.commit(Box::new(CreateLine::new(...)))` adds the line to `app.document.entities` and pushes onto `app.history` — verifiable via a unit test.
9. In `App::update`, `self.preview_entities` is updated each frame from `self.tool_manager.preview()` **before** `draw_preview` is called.
10. In `App::update`, when the left mouse button is pressed while hovering the viewport, `handle_pointer_down(world_pos, app)` is called on the active tool.
11. In `App::update`, when the pointer moves while hovering the viewport, `handle_pointer_move(world_pos, app)` is called on the active tool.
12. In `App::update`, when the left mouse button is released while hovering the viewport, `handle_pointer_up(world_pos, app)` is called on the active tool.
13. In `App::update`, when the Escape key is pressed, `handle_key(Key::Escape, app)` is called on the active tool.
14. `ToolManager::set_tool` calls `cancel()` on the old tool before replacing it. Verified via a unit test with a custom `Tool` impl that tracks cancel calls.
15. The `preview()` method on `SelectTool` returns an empty `Vec<Entity>`.
16. Kernel-purity: `grep -nE '^use (eframe|rfd)' src/tools/*.rs` returns no matches.
17. Size: `wc -l src/tools/tool.rs` reports `<= 150`; `wc -l src/tools/manager.rs` reports `<= 150`; `wc -l src/tools/select.rs` reports `<= 50`.
18. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `tool_trait_has_seven_methods` — constructs a `SelectTool`, calls all seven methods through the `Tool` trait to compile-check the surface.
- **Unit (AC 2)**: test `tool_is_object_safe` — `let _: Box<dyn Tool> = Box::new(SelectTool);` and `let _: &dyn Tool = &SelectTool;`.
- **Unit (AC 4)**: test `select_tool_name_exact` — `assert_eq!(SelectTool::default().name(), "Select");`.
- **Unit (AC 6)**: test `app_default_tool_manager_has_select` — `assert_eq!(App::default().tool_manager.active_tool_name(), "Select");`.
- **Unit (AC 7, 8)**: test `app_commit_adds_entity_and_pushes_history` — creates an `App::default()`, calls `app.commit(Box::new(CreateLine::new(...)))`, asserts `app.document.entity_count() == 1` and `app.history.can_undo() == true`.
- **Unit (AC 14)**: test `tool_manager_set_tool_cancels_old` — implement a `MockTool` with a `RefCell<bool> cancelled` flag, wrap in `ToolManager`, call `set_tool(Box::new(SelectTool))`, assert the flag is `true`.
- **Unit (AC 15)**: test `select_tool_preview_is_empty` — `assert!(SelectTool::default().preview().is_empty());`.
- **Static check (AC 16)**: `grep -nE '^use (eframe|rfd)' src/tools/*.rs` returns no matches.
- **Size check (AC 17)**: verify LOC counts.
- **Build gate (AC 18)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **Borrow splitting in `App::update`**: the tool methods take `&mut App` but `ToolManager` is a field of `App`. The implementer must split the borrow: extract `tool_manager` into a local variable before calling methods, or restructure the call to avoid simultaneous `&mut self` and `&mut self.tool_manager`. A common pattern: `let preview = self.tool_manager.preview(); self.preview_entities = preview;` — no conflict. For pointer dispatch, consider:
  ```rust
  // Extract tool_manager temporarily
  let mut tm = std::mem::take(&mut self.tool_manager);
  tm.handle_pointer_down(world_pos, self);
  self.tool_manager = tm;
  ```
  Or restructure so tool methods take component references (`&mut Document`, `&mut History`, `&Camera`) instead of `&mut App`. The latter is cleaner but requires updating the trait signature — implementer picks the least invasive approach for this demand.

- **Why `&mut App` in tool methods**: tools need read access to camera (for snap math), document (for hit-testing), and write access to preview (via return value) and commit (to push commands). Passing `&mut App` is the simplest signature; future demands can refine if it causes borrow-checker pain.

- **`preview()` returns `Vec<Entity>`**: this matches `app.preview_entities: Vec<Entity>` from LCV-037. The tool constructs ephemeral entities each frame; no persistence.

- **`cancel()` vs Escape key**: `on_key(Key::Escape, app)` is the input event; it typically calls `self.cancel()` internally. `cancel()` is also called by `set_tool` to clean up the old tool before switching. They are related but distinct: `on_key` is event-driven, `cancel` is state-driven.

- **SelectTool stub**: the stub does nothing; the real SelectTool (LCV-042) will implement box-select, click-to-select, shift-click-to-toggle, etc. This demand only needs enough for the app to compile and start.

- **Reference**: v1's `tools/Tool.ts` had similar methods: `onPointerDown`, `onPointerMove`, `onPointerUp`, `onKeyDown`, `getPreview`, `cancel`. v2 aligns names and semantics.

- **Object safety load-bearing**: `Box<dyn Tool>` is the storage shape in `ToolManager`. Adding a generic method to `Tool` would break this. Reviewers should flag any such addition.
