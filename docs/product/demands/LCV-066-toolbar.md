# LCV-066 — Toolbar — left-side tool palette

- **Status**: Done
- **Implementation**: e5e94e5 — feat(LCV-066): left-side toolbar with tool activation buttons
- **Phase**: 6
- **Depends on**: LCV-040 (Done), LCV-042, LCV-043, LCV-044, LCV-045, LCV-046, LCV-047, LCV-048, LCV-049, LCV-050, LCV-051, LCV-052
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

There is no graphical way to switch between drawing tools. Users must type
command-line tool names or rely on keyboard shortcuts that do not yet exist
(LCV-070). The AutoCAD R14 workflow centres on a vertical toolbar where
clicking a button activates the matching tool instantly. Without it, the
laser-cutting workflow (draw outline → switch to circle → drill holes → switch
to text → label the part → switch to erase → clean up) requires the user to
remember and type each tool name, blocking any mouse-first operator.

## Scope

- `src/ui/toolbar.rs` — new file; exports exactly one public item:
  `pub fn draw_toolbar(ui: &mut egui::Ui, app: &mut App)`.
- `src/ui/mod.rs` — add `pub mod toolbar;` and `pub use toolbar::draw_toolbar;`.
- `src/app.rs` (`App::update`) — add an `egui::SidePanel::left("toolbar")`
  call that invokes `draw_toolbar`, inserted **after**
  `egui::TopBottomPanel::bottom("statusbar")` and **before**
  `egui::CentralPanel::default()`.
- No other files are touched.

## Out of scope

- Icon images or SVG icons — text labels only in this demand.
- Tooltips on hover.
- Toolbar reordering, docking, or floating.
- Tool-specific sub-palettes (e.g., arc variant chooser).
- Any tool not in the ordered list in AC#4.
- Keyboard shortcut wiring (belongs to LCV-070).
- **Offset tool** — "No fillet / chamfer / offset" is an explicit v0.1.0
  non-goal (`docs/product/README.md`).

## Acceptance criteria

1. `src/ui/toolbar.rs` compiles, contains
   `pub fn draw_toolbar(ui: &mut egui::Ui, app: &mut App)`, and is ≤ 200 LOC
   (blank lines and comments included).

2. `src/ui/mod.rs` exposes `pub use toolbar::draw_toolbar` so callers
   can import via `use crate::ui::draw_toolbar`.

3. `App::update` renders the toolbar as a left-side panel using
   `egui::SidePanel::left("toolbar").show(ctx, |ui| { draw_toolbar(ui, self); })`.
   The panel is declared after `egui::TopBottomPanel::bottom("statusbar")` and
   before `egui::CentralPanel::default()`, preserving the ordering:
   `bottom("statusbar")` → `left("toolbar")` → `CentralPanel`.

4. The toolbar renders exactly **11 buttons in the following top-to-bottom
   order**, with the given text labels and active-tool-name contracts:

   | # | Label    | Tool type      | `active_tool_name()` value |
   |---|----------|----------------|----------------------------|
   | 1 | `SEL`    | `SelectTool`   | `"Select"`                 |
   | 2 | `LINE`   | `LineTool`     | `"LINE"`                   |
   | 3 | `PLINE`  | `PolylineTool` | `"PLINE"`                  |
   | 4 | `RECT`   | `RectTool`     | `"RECTANG"`                |
   | 5 | `CIRC`   | `CircleTool`   | `"CIRCLE"`                 |
   | 6 | `ARC`    | `ArcTool`      | `"ARC"`                    |
   | 7 | `TEXT`   | `TextTool`     | `"TEXT"`                   |
   | 8 | `MOVE`   | `MoveTool`     | `"MOVE"`                   |
   | 9 | `TRIM`   | `TrimTool`     | `"TRIM"`                   |
   |10 | `EXTEND` | `ExtendTool`   | `"EXTEND"`                 |
   |11 | `ERASE`  | `DeleteTool`   | `"ERASE"`                  |

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
  Covers AC#1 and AC#2.

- **Unit — `draw_toolbar_default_app_does_not_panic`**: construct
  `App::default()` (active tool is `SelectTool`); call `draw_toolbar` inside
  `egui::__run_test_ui` or an equivalent egui test harness; assert the call
  does not panic and `app.tool_manager.active_tool_name()` remains `"Select"`
  after the call. Covers AC#4 and AC#6 (no inadvertent tool switch on render).

- **Unit — `clicking_line_button_activates_line_tool`**: using an `App` with
  `SelectTool` active, simulate a click on the `LINE` button (synthesise an
  `egui::Response` with `clicked() == true` or use the egui test harness);
  assert `app.tool_manager.active_tool_name() == "LINE"` afterward.
  Covers AC#5.

- **Manual smoke**: `cargo run`; confirm the left side panel is visible on
  startup with 11 text buttons in the specified order; click each button and
  verify the active one appears pressed/highlighted while all others are
  un-selected; confirm `active_tool_name()` changes with each click
  (observable via the status bar once LCV-067 lands, or via `println!` debug
  output during development). Covers AC#3, AC#4, AC#6, AC#7.

## Open questions

*(none — demand is Ready)*

## Notes

- **Panel ordering is load-bearing.** `egui` panels are claimed in declaration
  order; `CentralPanel` consumes all remaining space and must be declared last.
  The current `App::update` has `TopBottomPanel::bottom("statusbar")` followed
  immediately by `CentralPanel::default()`. Insert `SidePanel::left("toolbar")`
  between the two.

- **`active_tool_name()` strings are load-bearing contracts.** Each tool demand
  must honour its declared string exactly:
  - `SelectTool`   → `"Select"`  (confirmed: `src/tools/select/mod.rs`)
  - `LineTool`     → `"LINE"`    (confirmed: `src/tools/line.rs`)
  - `PolylineTool` → `"PLINE"`   (confirmed: `src/tools/polyline.rs`)
  - `RectTool`     → `"RECTANG"` (confirmed: LCV-045 AC#2 — **not** `"RECT"`)
  - `CircleTool`   → `"CIRCLE"`  (confirmed: `src/tools/circle.rs`)
  - `ArcTool`      → `"ARC"`     (confirmed: `src/tools/arc.rs`)
  - `TextTool`     → `"TEXT"`    (pre-defined by this demand; LCV-048 must
                                  declare `name()` returning exactly `"TEXT"`)
  - `MoveTool`     → `"MOVE"`    (confirmed: LCV-049 AC#1)
  - `TrimTool`     → `"TRIM"`    (pre-defined by this demand; LCV-050 must
                                  declare `name()` returning exactly `"TRIM"`)
  - `ExtendTool`   → `"EXTEND"`  (pre-defined by this demand; LCV-051 must
                                  declare `name()` returning exactly `"EXTEND"`)
  - `DeleteTool`   → `"ERASE"`   (confirmed: `src/tools/delete.rs` and
                                  LCV-052 AC#1)

- **All 11 tool types must implement `Default`.** The toolbar constructs each
  tool via `<ToolType>::default()`. `SelectTool`, `LineTool`, `PolylineTool`,
  `RectTool`, `CircleTool`, `ArcTool`, `MoveTool`, and `DeleteTool` already
  have `impl Default` in their respective demands/source. LCV-048 (`TextTool`),
  LCV-050 (`TrimTool`), and LCV-051 (`ExtendTool`) must also provide
  `impl Default` — this demand pre-defines that requirement.

- **LCV-048 / LCV-050 / LCV-051 have no demand files yet.** This demand
  pre-defines their `name()` contracts above. Those demands must honour these
  strings. The toolbar will not compile until all 11 tool types exist; the
  implementer should claim this demand only after all deps are Done.

- **`ToolManager::set_tool` calls `cancel()` automatically** on the old tool
  before switching — the toolbar does not need to call `cancel()` explicitly.

- **Purity rule** (`AGENTS.md`): `toolbar.rs` must not import from
  `src/geometry/`, `src/document/`, or `src/io/svg/`. Permitted imports:
  `egui`, `crate::app::App`, and the 11 tool types from
  `crate::tools::{SelectTool, LineTool, PolylineTool, RectTool, CircleTool,
  ArcTool, TextTool, MoveTool, TrimTool, ExtendTool, DeleteTool}`.

- **LOC budget.** Keep `toolbar.rs` ≤ 200 LOC. A data-driven approach avoids
  repetition: a local array of `(&str, fn() -> Box<dyn Tool>)` pairs (label,
  constructor closure) iterated in a single `for` loop keeps the file compact.
  The choice of pattern is the implementer's.
