# LCV-066 — Toolbar with tool buttons

- **Status**: Ready
- **Phase**: 6
- **Depends on**: LCV-040 (Done), LCV-042, LCV-043, LCV-044, LCV-045, LCV-046, LCV-047, LCV-049, LCV-052
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

There is no graphical way to switch between drawing tools. Users must type
command-line tool names or rely on keyboard shortcuts that do not yet exist
(LCV-070). The AutoCAD R14 workflow centres on a vertical toolbar where
clicking a button activates the matching tool instantly. Without it, the
laser-cutting workflow (draw outline → switch to circle → draw holes → switch
to erase → clean up) requires the user to remember and type each tool name,
blocking any mouse-first operator.

## Scope

- `src/ui/toolbar.rs` — new file; exports exactly one public item:
  `pub fn draw_toolbar(ui: &mut egui::Ui, app: &mut App)`.
- `src/ui/mod.rs` — add `pub mod toolbar;` and `pub use toolbar::draw_toolbar;`.
- `src/app.rs` (`App::update`) — add an `egui::SidePanel::left("toolbar")`
  call that invokes `draw_toolbar` **before** `egui::CentralPanel::default()`.
- No other files are touched.

## Out of scope

- Icon images or SVG icons — text labels only.
- Tooltips on hover.
- Toolbar reordering, docking, or floating.
- Tool-specific sub-palettes (e.g. arc variants).
- Any tool not in the ordered list below.
- Keyboard shortcut wiring (belongs to LCV-070).

## Acceptance criteria

1. `src/ui/toolbar.rs` compiles, contains `pub fn draw_toolbar(ui: &mut
   egui::Ui, app: &mut App)`, and is ≤ 150 LOC (blank lines and comments
   included).

2. `src/ui/mod.rs` exposes `pub use toolbar::draw_toolbar` so callers
   can import via `use crate::ui::draw_toolbar`.

3. `App::update` renders the toolbar as a left-side panel using
   `egui::SidePanel::left("toolbar").show(ctx, |ui| { draw_toolbar(ui, self); })`,
   placed before `egui::CentralPanel::default()`.

4. The toolbar renders exactly **8 buttons in the following top-to-bottom
   order**, with the given text labels:

   | # | Label  | Tool type      | `active_tool_name()` value |
   |---|--------|----------------|----------------------------|
   | 1 | `SEL`  | `SelectTool`   | `"Select"`                 |
   | 2 | `LINE` | `LineTool`     | `"LINE"`                   |
   | 3 | `PLINE`| `PolylineTool` | `"PLINE"`                  |
   | 4 | `RECT` | `RectTool`     | `"RECT"`                   |
   | 5 | `CIRC` | `CircleTool`   | `"CIRCLE"`                 |
   | 6 | `ARC`  | `ArcTool`      | `"ARC"`                    |
   | 7 | `MOVE` | `MoveTool`     | `"MOVE"`                   |
   | 8 | `ERASE`| `DeleteTool`   | `"ERASE"`                  |

5. Clicking a button calls
   `app.tool_manager.set_tool(Box::new(<ToolType>::default()))` for the
   corresponding tool. No other side effects.

6. Each button is rendered with `egui::Button::new(label).selected(is_active)`
   where `is_active = app.tool_manager.active_tool_name() == <expected_name>`.
   Exactly one button is in the `selected` state at any time (matching the
   currently active tool); all others are un-selected.

7. Buttons are laid out vertically; each occupies the full panel width.
   Minimum panel width is not constrained beyond egui defaults.

8. All of `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, and
   `cargo test --all` pass with no regressions after this demand lands.

## Expected tests

- **Unit — `toolbar_module_compiles`**: `use lasercad::ui::draw_toolbar;`
  resolves at compile time (compile-only check; no runtime assertion needed).

- **Unit — `draw_toolbar_with_select_active_highlights_sel`**: construct an
  `App` (default → active tool is `SelectTool`); render `draw_toolbar` using
  `egui::__run_test_ui` or an equivalent test harness; assert that the
  response for the `"SEL"` button reports `selected == true` (or use the
  returned `egui::Response::is_pointer_button_down_on` / widget introspection
  approach available in the egui test API). If egui's test API cannot assert
  `selected` state directly, at minimum assert the function does not panic.

- **Unit — `set_tool_called_on_click`**: using an `App` with `SelectTool`
  active, simulate a click on the `"LINE"` button (via `Response::clicked()`);
  assert `app.tool_manager.active_tool_name() == "LINE"` afterward.

- **Manual smoke**: `cargo run`; confirm the left panel is visible on
  startup with 8 text buttons; click each button and verify the active one
  appears pressed/highlighted and `active_tool_name()` changes accordingly
  (observable via the status bar once LCV-067 lands, or via println! debug
  output during development).

## Open questions

*(none — demand is Ready)*

## Notes

- `egui::SidePanel::left` must precede `egui::CentralPanel::default()` in
  `App::update`; egui panels are claimed in declaration order and the central
  panel consumes all remaining space.

- The `active_tool_name()` strings in the table above are **load-bearing
  contracts**. Each tool demand must honour its declared string exactly:
  - `SelectTool` → `"Select"` (title-case; confirmed in `src/tools/select/mod.rs`)
  - `LineTool` → `"LINE"` (confirmed in `src/tools/line.rs`)
  - `PolylineTool` → `"PLINE"` (confirmed in LCV-044 AC#2)
  - `RectTool` → `"RECT"` (required by this demand; LCV-045 must declare this)
  - `CircleTool` → `"CIRCLE"` (confirmed in `src/tools/circle.rs`)
  - `ArcTool` → `"ARC"` (confirmed in `src/tools/arc.rs`)
  - `MoveTool` → `"MOVE"` (required by this demand; LCV-049 must declare this)
  - `DeleteTool` → `"ERASE"` (confirmed in LCV-052 AC#1)

- `ToolManager::set_tool(&mut self, tool: Box<dyn Tool>)` calls `cancel()` on
  the old tool before switching — the toolbar does not need to call `cancel()`
  explicitly.

- `toolbar.rs` must **not** import from `src/geometry/`, `src/document/`, or
  `src/io/svg/` (purity rule in AGENTS.md). It imports `egui` and
  `crate::{app::App, tools::{SelectTool, LineTool, PolylineTool, RectTool,
  CircleTool, ArcTool, MoveTool, DeleteTool}}`.

- Keep `toolbar.rs` ≤ 150 LOC. The repetitive button list can use a local
  `const` array of `(&str, &str)` pairs (label, name) plus a match/dispatch
  macro or closure to avoid repetition; the choice is the implementer's.

- LCV-045 (RectTool) and LCV-049 (MoveTool) have PLAN.md entries but no
  demand files yet; this demand is blocked on those demands being refined and
  confirming `name()` returns `"RECT"` and `"MOVE"` respectively.
