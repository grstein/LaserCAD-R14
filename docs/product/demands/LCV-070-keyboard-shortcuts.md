# LCV-070 — Keyboard shortcuts (tool activation hotkeys)

- **Status**: Done
- **Implementation**: 35a804a — feat(LCV-070): keyboard shortcuts — L/C/A/P/R/E/M tool hotkeys + F8 ortho
- **Phase**: 6
- **Depends on**: LCV-040 (Done), LCV-053, LCV-062, LCV-065
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

Without keyboard shortcuts, every tool switch in LaserCAD requires a mouse trip
to the toolbar. In a typical laser-cutting session an operator cycles between
Line, Circle, Erase and Move dozens of times per drawing; mouse-only switching
adds friction and breaks flow. AutoCAD R14 makes this frictionless through
single-letter hotkeys: press `L`, start drawing a line; press `E`, erase the
selection. LaserCAD v2 must honour the same muscle memory. Additionally,
Ctrl+Z/Y for undo/redo — already provided by the history stack (LCV-026) — are
not yet wired to any key, so every accidental click is currently unrecoverable
without restarting.

## Scope

- **`src/ui/shortcuts.rs`** — new file; exports exactly one public function:
  `pub fn process_shortcuts(ctx: &egui::Context, app: &mut crate::app::App)`.
- **`src/ui/mod.rs`** — add `pub mod shortcuts;` and
  `pub use shortcuts::process_shortcuts;`.
- **`src/app.rs`** (`App` struct) — add two new boolean fields:
  `pub snap_enabled: bool` (default `true`) and
  `pub grid_enabled: bool` (default `true`).
- **`src/app.rs`** (`App::update`) — add one call,
  `crate::ui::process_shortcuts(ctx, self);`, at the **very start** of `update`
  before any panel is rendered. Also wrap the existing `resolve_snap` call with
  `if self.snap_enabled { … }` and set `self.active_snap = None` when disabled.
  Wrap the existing `draw_grid` call with `if self.grid_enabled { … }`.
- **`src/app.rs`** (`App::default` / `#[derive(Default)]`) — `snap_enabled` and
  `grid_enabled` both initialise to `true`; update `Default` impl (manual or
  struct-update syntax) accordingly.
- No other files are touched by this demand.

## Out of scope

- `Escape` key (already handled in `app.rs`, not moved here).
- `Delete` / `Backspace` keys (already handled in `app.rs` via LCV-052, not
  moved here).
- Zoom-extents keys `F` / `Ctrl+0` (already handled in `app.rs`, not moved
  here).
- The `O` key (Offset) — Offset is an explicit v0.1.0 non-goal.
- The `T` key for **Text** tool — see Notes for conflict resolution.
- Any shortcut that opens a floating dialog (e.g. settings, about).
- Customisable / rebindable shortcuts.
- On-screen shortcut hint overlays or tooltips.
- macOS ⌘ modifier — `ctrl` only; multi-platform is Phase 9.

## Acceptance criteria

### Module and wiring

1. `src/ui/shortcuts.rs` compiles, contains
   `pub fn process_shortcuts(ctx: &egui::Context, app: &mut crate::app::App)`,
   and is ≤ 150 LOC (blank lines and comments included).

2. `src/ui/mod.rs` exposes `pub use shortcuts::process_shortcuts` so callers
   import via `use crate::ui::process_shortcuts`.

3. `App::update` calls `process_shortcuts(ctx, self)` once per frame as the
   first statement of the function body, before any
   `egui::TopBottomPanel` / `egui::SidePanel` / `egui::CentralPanel` call.

4. `App` gains two new public fields: `snap_enabled: bool` and
   `grid_enabled: bool`. Both default to `true`. `App::default()` satisfies
   `app.snap_enabled == true && app.grid_enabled == true`.

### Tool activation keys

5. Pressing the following keys — **without any modifier** and only when
   `ctx.wants_keyboard_input()` returns `false` — activates the corresponding
   tool by calling
   `app.tool_manager.set_tool(Box::new(<ToolType>::default()))`:

   | Key | `egui::Key` variant | Tool type     | `active_tool_name()` after |
   |-----|---------------------|---------------|----------------------------|
   | `L` | `egui::Key::L`      | `LineTool`    | `"LINE"`                   |
   | `P` | `egui::Key::P`      | `PolylineTool`| `"PLINE"`                  |
   | `R` | `egui::Key::R`      | `RectTool`    | `"RECT"`                   |
   | `C` | `egui::Key::C`      | `CircleTool`  | `"CIRCLE"`                 |
   | `A` | `egui::Key::A`      | `ArcTool`     | `"ARC"`                    |
   | `M` | `egui::Key::M`      | `MoveTool`    | `"MOVE"`                   |
   | `E` | `egui::Key::E`      | `DeleteTool`  | `"ERASE"`                  |
   | `T` | `egui::Key::T`      | `TrimTool`    | `"TRIM"`                   |
   | `X` | `egui::Key::X`      | `ExtendTool`  | `"EXTEND"`                 |

6. When `ctx.wants_keyboard_input()` returns `true` (a text widget — e.g. the
   command-line input — has keyboard focus), **none** of the single-letter tool
   keys in AC#5 may activate a tool. The active tool is unchanged.

7. A tool key pressed with any modifier (`Ctrl`, `Shift`, `Alt`) must **not**
   activate a tool. Only the bare, unmodified key press counts.

### Undo / Redo

8. `Ctrl+Z` (i.e. `ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift &&
   i.key_pressed(egui::Key::Z))`) calls
   `app.history.undo(&mut app.document)`. When `app.history.can_undo()` is
   `false`, the call is a no-op (the existing `undo` contract returns `false`
   and leaves the document untouched).

9. `Ctrl+Y` calls `app.history.redo(&mut app.document)`. Same guard: no-op
   when `can_redo()` is `false`.

10. `Ctrl+Z` and `Ctrl+Y` fire unconditionally regardless of
    `ctx.wants_keyboard_input()`. (Standard behaviour: undo/redo must work even
    when the command-line input is focused.)

### File shortcuts

11. `Ctrl+N` calls `app.action_new()`, `Ctrl+O` calls `app.action_open()`, and
    `Ctrl+S` calls `app.action_save()` — the same methods that the menubar
    (LCV-065) uses for the File → New / Open / Save menu items as established
    by LCV-062. These fire unconditionally (same rule as AC#10).

### Toggle keys

12. `F8` toggles `app.ortho_enabled` (the `bool` field added to `App` by
    LCV-053). After each press the value is logically negated: if it was `true`
    it becomes `false` and vice versa.

13. `F3` toggles `app.snap_enabled`. After each press the value is logically
    negated.

14. `F7` toggles `app.grid_enabled`. After each press the value is logically
    negated.

15. Toggle keys fire unconditionally regardless of `ctx.wants_keyboard_input()`
    (consistent with how function keys behave in AutoCAD R14).

### Snap and grid suppression

16. In `App::update`, the `resolve_snap` call and `self.active_snap` assignment
    are executed only when `self.snap_enabled == true`. When `snap_enabled` is
    `false`, `self.active_snap` is set to `None` each frame so no snap marker
    is rendered.

17. In `App::update`, the `draw_grid` call is executed only when
    `self.grid_enabled == true`. When `grid_enabled` is `false`, no grid is
    painted.

### Build gate

18. All of `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, and
    `cargo test --all` pass with no regressions after this demand lands.

## Expected tests

*At least one test per acceptance criterion.*

- **Unit — `shortcuts_module_compiles`** (AC#1, AC#2): `use
  lasercad::ui::process_shortcuts;` resolves at compile time.

- **Unit — `app_default_snap_and_grid_enabled`** (AC#4): `let app =
  App::default(); assert!(app.snap_enabled); assert!(app.grid_enabled);`.

- **Unit — `tool_key_l_activates_line_tool`** (AC#5): construct a
  `ToolManager` defaulting to `SelectTool`; simulate
  `ctx.input(Key::L, no modifiers, wants_keyboard_input=false)` via the
  egui test harness (or a direct call to a testable inner function — see
  Notes); assert `app.tool_manager.active_tool_name() == "LINE"`.

- **Unit — `tool_key_p_activates_polyline_tool`** (AC#5): same pattern, `Key::P`
  → `"PLINE"`.

- **Unit — `tool_key_r_activates_rect_tool`** (AC#5): `Key::R` → `"RECT"`.

- **Unit — `tool_key_c_activates_circle_tool`** (AC#5): `Key::C` → `"CIRCLE"`.

- **Unit — `tool_key_a_activates_arc_tool`** (AC#5): `Key::A` → `"ARC"`.

- **Unit — `tool_key_m_activates_move_tool`** (AC#5): `Key::M` → `"MOVE"`.

- **Unit — `tool_key_e_activates_delete_tool`** (AC#5): `Key::E` → `"ERASE"`.

- **Unit — `tool_key_t_activates_trim_tool`** (AC#5): `Key::T` → `"TRIM"`.

- **Unit — `tool_key_x_activates_extend_tool`** (AC#5): `Key::X` → `"EXTEND"`.

- **Unit — `tool_key_blocked_when_wants_keyboard_input`** (AC#6): simulate
  `wants_keyboard_input = true` and press `Key::L`; assert active tool is still
  `SelectTool`.

- **Unit — `tool_key_blocked_with_ctrl_modifier`** (AC#7): simulate `Ctrl+L`;
  assert active tool is still `SelectTool`.

- **Unit — `ctrl_z_calls_undo`** (AC#8): build an `App` with one committed
  `CreateLine` command; call `process_shortcuts` with a simulated `Ctrl+Z`
  input; assert `app.document.entity_count() == 0` and
  `app.history.can_redo() == true`.

- **Unit — `ctrl_z_noop_when_empty_history`** (AC#8): fresh `App`, simulate
  `Ctrl+Z`; assert `history.can_undo() == false` and document unchanged.

- **Unit — `ctrl_y_calls_redo`** (AC#9): commit one line, undo it, simulate
  `Ctrl+Y`; assert `app.document.entity_count() == 1`.

- **Unit — `ctrl_z_fires_when_wants_keyboard_input`** (AC#10): set
  `wants_keyboard_input = true`; simulate `Ctrl+Z` after a commit; assert undo
  still fires (entity removed).

- **Unit — `f8_toggles_ortho_enabled`** (AC#12): `app.ortho_enabled = false`;
  simulate `F8`; assert `app.ortho_enabled == true`; simulate `F8` again;
  assert `false`.

- **Unit — `f3_toggles_snap_enabled`** (AC#13): `app.snap_enabled = true`;
  simulate `F3`; assert `app.snap_enabled == false`; simulate `F3`; assert
  `true`.

- **Unit — `f7_toggles_grid_enabled`** (AC#14): same pattern with
  `app.grid_enabled` and `F7`.

- **Unit — `snap_disabled_clears_active_snap`** (AC#16): build an `App` with
  a document entity; set `app.snap_enabled = false`; confirm that after the
  snap-suppression branch in `App::update` runs (or a call to the extracted
  helper), `app.active_snap == None`.

- **Manual smoke — tool keys** (AC#5, AC#6): `cargo run`; click on the canvas
  (so no text widget has focus); press L → toolbar shows LINE highlighted; press
  C → CIRCLE highlighted; click in the command-line input; press L → active tool
  does NOT change.

- **Manual smoke — undo/redo** (AC#8, AC#9): draw two lines; press Ctrl+Z
  once → one line remains; press Ctrl+Z again → canvas empty; press Ctrl+Y →
  one line reappears.

- **Manual smoke — toggles** (AC#12–14): press F7 → grid disappears; press F7
  again → grid returns. Press F3 → snap markers stop appearing; press F3 →
  snap returns. Press F8 → status bar shows ORTHO ON (once LCV-067 status
  bar label lands); press F8 → ORTHO OFF.

## Open questions

*(none — demand is Ready)*

## Notes

### T key conflict resolution (T=Trim, not T=Text)

AutoCAD R14's `acad.pgp` maps both `T → MTEXT` and `TR → TRIM`. As single-key
hotkeys these conflict: pressing `T` is the first keystroke of `TR`, so they
cannot coexist as unambiguous single-letter shortcuts.

**Decision:** `T` activates `TrimTool`. Trim is the higher-frequency modify
command in laser-cutting workflows (cutting paths often produce overlapping
segments that need trimming). The Text tool (LCV-048) is available via the
toolbar (LCV-066) and, when LCV-068 lands, via the command line (`TEXT` or
`T⏎`). This is the conservative choice: keyboard muscle memory for Trim is
more disruptive to break than for Text.

### X key for Extend

`X` is chosen for `ExtendTool` because the two-letter AutoCAD alias is `EX`,
and `E` is already taken by `ERASE`. `X` is the natural single-character
contraction. The command line will still accept `EX⏎` once LCV-068 ships.

### Preventing egui key conflicts

egui claims keyboard events on widgets that have focus. `ctx.wants_keyboard_input()`
returns `true` whenever any egui text-input field is active (e.g. the
command-line dock from LCV-068). Single-letter tool keys must be guarded by
`!ctx.wants_keyboard_input()`. Ctrl/function key shortcuts bypass this guard —
undo/redo and file operations should work even while the operator is typing in
the command line.

### Testability of `process_shortcuts`

`process_shortcuts` takes a `&egui::Context`, which is not trivially
constructible in unit tests without running a frame. The recommended approach
is to extract the dispatch logic into a smaller, pure function:

```rust
/// Inner, testable dispatch. Called from `process_shortcuts`.
pub fn dispatch_shortcuts(
    tool_key: Option<egui::Key>,
    modifiers: egui::Modifiers,
    wants_kbd: bool,
    app: &mut App,
) { … }
```

Unit tests call `dispatch_shortcuts` directly, passing synthetic values.
`process_shortcuts` reads `ctx.input(…)` and delegates to `dispatch_shortcuts`.
This keeps the egui coupling thin and the logic testable. The inner function is
`pub(crate)` or `pub` — implementer's choice; tests import it directly.

### Ortho toggle dependency

`app.ortho_enabled` is the boolean field that LCV-053 is expected to add to
`App`. If LCV-053 has not yet landed when this demand is implemented, the
implementer must add `ortho_enabled: bool` (default `false`) to `App` as part
of this demand, removing the LCV-053 dependency on that field. The toggle
behaviour (clamping movement to 0°/45°/90°) is still LCV-053's responsibility;
LCV-070 only wires the F8 key to the toggle.

### File shortcut action methods

`action_new()`, `action_open()`, and `action_save()` are the methods that
LCV-062 adds to `App`. The exact signatures will be confirmed when LCV-062 is
refined. If those methods do not yet exist when LCV-070 is implemented, stub
them out in `app.rs` as `pub fn action_new(&mut self) {}` etc. so the shortcuts
compile. The real implementations land with LCV-062.

### LOC budget

`shortcuts.rs` must stay ≤ 150 LOC. The tool-dispatch table can be expressed as
a local array of `(egui::Key, fn() -> Box<dyn Tool>)` pairs iterated in a loop
to avoid repetitive `if` chains.

### Purity

`src/ui/shortcuts.rs` may import `egui` and `crate::app::App` (and transitively
`crate::tools::*`). It must **not** directly import from
`src/geometry/`, `src/document/`, or `src/io/svg/`. All document mutation goes
through `App`'s public interface (`app.history`, `app.document`,
`app.tool_manager`).
