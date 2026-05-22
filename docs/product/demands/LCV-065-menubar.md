# LCV-065 — Menubar (File / Edit / View / Help)

- **Status**: Ready
- **Phase**: 6
- **Depends on**: LCV-062, LCV-060 (Done), LCV-069 (Done), LCV-031 (Done), LCV-034 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

LaserCAD v2 has no menubar. Infrequent but critical operations — opening and
saving a drawing, undoing an accidental delete, fitting the bed to screen — are
invisible to new operators and only reachable by remembering command-line
aliases or toolbar buttons that do not yet cover the full set. The AutoCAD R14
convention for a KISS CAD tool is a minimal top-of-window menubar: four menus
(File, Edit, View, Help) that are discoverable at a glance and mouse-first
accessible. Without it, saving a drawing for the first time requires knowing
`Ctrl+S` before any menu hints that it exists, blocking any operator who has
not read the manual.

## Scope

- **New file `src/ui/menubar.rs`** — contains exactly one public item:
  `pub fn draw_menubar(ui: &mut egui::Ui, app: &mut App)`.
  The function renders an `egui::menu::bar(ui, |ui| { … })` containing four
  top-level menus in left-to-right order: **File**, **Edit**, **View**, **Help**.
  File size ≤ 250 LOC (blank lines and comments included).

- **`src/ui/mod.rs`** — add `pub mod menubar;` and
  `pub use menubar::draw_menubar;`.

- **`src/app.rs` (`App` struct)** — add two new public boolean fields:

  ```rust
  /// Whether the grid is drawn in the viewport. Toggled by View → Grid (F7).
  pub grid_enabled: bool,
  /// Whether the snap engine runs each frame. Toggled by View → Snap (F3).
  pub snap_enabled: bool,
  ```

  Both must participate in `#[derive(Default)]` with value `true`; update
  `impl Default` (manual or struct-update syntax) accordingly.

- **`src/app.rs` (`App::update`)** — insert a
  `egui::TopBottomPanel::top("menubar").show(ctx, |ui| { draw_menubar(ui, self); })`
  call as the **very first** panel declaration, before the existing
  `TopBottomPanel::bottom("statusbar")`. The resulting declaration order is:

  ```
  TopBottomPanel::top("menubar")      // ← new, first
  TopBottomPanel::bottom("statusbar")
  TopBottomPanel::bottom("command_line")
  SidePanel::left("toolbar")
  CentralPanel::default()
  ```

- **`src/app.rs` (`App::update`)** — wrap the existing `draw_grid` call with
  `if self.grid_enabled { … }` and wrap the existing `resolve_snap` + snap
  assignment block with `if self.snap_enabled { … } else { self.active_snap = None; }`.
  (These two guards are also required by LCV-070; whichever demand is
  implemented second finds them already present and skips re-adding them.)

### Menu item contracts

#### File menu

| Label | Shortcut hint | Action |
|---|---|---|
| `New` | `Ctrl+N` | `app.action_new()` |
| `Open…` | `Ctrl+O` | `app.action_open()` |
| `Open Recent ▶` | — | submenu (see below) |
| `Save` | `Ctrl+S` | `app.action_save()` |
| `Save As…` | `Ctrl+Shift+S` | `app.action_save_as()` |
| *(separator)* | | `ui.separator()` |
| `Exit` | — | `ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close)` |

**Open Recent submenu** — rendered as a nested `egui::menu::menu_button`:

1. Call `crate::io::recent_files(&app.settings)` to get `&[String]`.
2. If the slice is empty, render one disabled label `"No recent files"`.
3. If non-empty, render one button per entry (index `0` first). Each button
   label is the last path component extracted via
   `std::path::Path::new(entry).file_name()` (fall back to the full string
   if `file_name()` returns `None`). Clicking entry at index `i` calls
   `crate::io::open_recent(i, &mut app.settings)` and, on `Ok(path)`, calls
   `app.action_open_path(path)`. An `Err` variant is silently ignored (stale
   path; the full error-toast workflow belongs to LCV-069 consumers).

#### Edit menu

| Label | Shortcut hint | Enabled condition | Action |
|---|---|---|---|
| `Undo` | `Ctrl+Z` | `app.history.can_undo()` | `app.history.undo(&mut app.document)` |
| `Redo` | `Ctrl+Y` | `app.history.can_redo()` | `app.history.redo(&mut app.document)` |
| *(separator)* | | always | `ui.separator()` |
| `Select All` | — | always | `app.document.selection.set(0..app.document.entity_count())` |

Undo and Redo must be rendered with `ui.add_enabled(condition, egui::Button::new(label))` so
they appear visually grayed out when the stack is empty. The shortcut hint is
part of the label string (e.g. `"Undo\tCtrl+Z"`).

#### View menu

| Label | Shortcut hint | Action |
|---|---|---|
| `Zoom In` | — | `app.camera.zoom_in(1.25)` |
| `Zoom Out` | — | `app.camera.zoom_out(1.25)` |
| `Fit to Bed` | — | see below |
| *(separator)* | | `ui.separator()` |
| `Grid` *(checkmark)* | `F7` | `app.grid_enabled = !app.grid_enabled` |
| `Snap` *(checkmark)* | `F3` | `app.snap_enabled = !app.snap_enabled` |

**Fit to Bed** — compute the bed bounding box from `app.bed` and call
`app.camera.zoom_extents`:

```rust
let min = app.bed.origin_world;
let max = crate::geometry::Vec2::new(
    app.bed.origin_world.x + app.bed.size_mm[0],
    app.bed.origin_world.y + app.bed.size_mm[1],
);
app.camera.zoom_extents(Some((min, max)), app.camera.viewport_size_px);
```

**Grid / Snap checkmarks** — render with `ui.checkbox(&mut app.grid_enabled, "Grid")` and
`ui.checkbox(&mut app.snap_enabled, "Snap")`. The checkbox provides its own
check indicator; no separate `SelectableLabel` is needed.

#### Help menu

| Label | Action |
|---|---|
| `About` | `app.about_open = true` |

## Out of scope

- Keyboard shortcut handling (`Ctrl+N/O/S`, `F3`, `F7`, `Ctrl+Z/Y`): the
  menubar shows shortcut hints as text only. Actual key dispatch is LCV-070.
- Any menu item not in the tables above (no Layers, no DXF, no G-code, no
  settings, no Print).
- Custom icons or images on menu items — text labels only.
- Tooltips on menu items.
- Menu bar drag-to-reorder or tear-off menus.
- An "Open Recent" keyboard shortcut for individual recent-file slots (LCV-070).
- Error toasts when `action_open_path` fails (belongs to LCV-069 consumers).
- Persisting `grid_enabled` / `snap_enabled` to `Settings` (LCV-058 concern;
  they are session-only booleans for now).
- The `Ctrl+Shift+S` keyboard shortcut handler for Save As (LCV-070).

## Acceptance criteria

1. `src/ui/menubar.rs` compiles, contains
   `pub fn draw_menubar(ui: &mut egui::Ui, app: &mut App)`, and reports
   ≤ 250 LOC via `wc -l src/ui/menubar.rs`.

2. `src/ui/mod.rs` exposes `pub use menubar::draw_menubar` so callers can
   import via `use crate::ui::draw_menubar`.

3. `App::update` declares `TopBottomPanel::top("menubar")` as its **first**
   panel, before `TopBottomPanel::bottom("statusbar")`. Running `cargo run`
   and inspecting the window shows the menubar strip at the very top, above the
   canvas and all other chrome.

4. `App` gains `pub grid_enabled: bool` and `pub snap_enabled: bool`. Both
   default to `true`: `App::default().grid_enabled == true` and
   `App::default().snap_enabled == true`.

5. The File menu renders exactly six items in the following order: `New`,
   `Open…`, `Open Recent ▶`, `Save`, `Save As…`, a separator, and `Exit`.
   No other items are present.

6. The Edit menu renders exactly four items: `Undo`, `Redo`, a separator, and
   `Select All`. `Undo` is grayed out when `app.history.can_undo()` is `false`;
   `Redo` is grayed out when `app.history.can_redo()` is `false`.

7. The View menu renders exactly six items: `Zoom In`, `Zoom Out`, `Fit to Bed`,
   a separator, `Grid` (checkbox checked when `app.grid_enabled`), and `Snap`
   (checkbox checked when `app.snap_enabled`).

8. The Help menu renders exactly one item: `About`.

9. Clicking `Grid` in the View menu toggles `app.grid_enabled`; clicking `Snap`
   toggles `app.snap_enabled`.

10. Clicking `About` sets `app.about_open = true`, causing the About dialog
    (LCV-069) to open.

11. Clicking `Select All` when the document has `n` entities results in
    `app.document.selection.len() == n`.

12. Clicking `Zoom In` decreases `app.camera.mm_per_px` (world becomes larger
    on screen); clicking `Zoom Out` increases it.

13. Clicking `Fit to Bed` calls `camera.zoom_extents` with the bounds derived
    from `app.bed.origin_world` and `app.bed.size_mm` as the bounding box.

14. The `Open Recent` submenu renders the label `"No recent files"` (disabled)
    when `crate::io::recent_files(&app.settings)` is empty.

15. `src/ui/menubar.rs` contains no `use rfd`, no `use eframe`, no direct import
    from `src/geometry/`, `src/document/`, `src/io/svg/`, `src/agent/`, or
    `src/text/`. Permitted imports: `egui`, `crate::app::App`,
    `crate::io::{recent_files, open_recent}`, and `crate::geometry::Vec2`
    (needed only for the Fit to Bed bounds).

16. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0 with no regressions.

## Expected tests

Tests live in `#[cfg(test)] mod tests` inside `src/ui/menubar.rs` and
`src/app.rs`. All egui tests use `egui::Context::default()` +
`ctx.run(egui::RawInput::default(), |ctx| { … })` or equivalent.

- **Unit — `menubar_module_compiles`** (AC#1, AC#2):
  `use lasercad::ui::draw_menubar;` resolves at compile time.
  Covers AC#1 and AC#2.

- **Unit — `app_default_grid_and_snap_enabled`** (AC#4):
  ```rust
  let app = App::default();
  assert!(app.grid_enabled);
  assert!(app.snap_enabled);
  ```

- **Unit — `draw_menubar_default_app_does_not_panic`** (AC#3):
  Construct `App::default()`; call `draw_menubar` inside `ctx.run(…)` via
  `egui::__run_test_ui` or equivalent. Assert the call does not panic and
  `app.about_open` remains `false`. Covers AC#3 (no crash at startup).

- **Unit — `view_grid_checkbox_toggles_grid_enabled`** (AC#9):
  Construct `App::default()` with `grid_enabled = true`. Simulate a click on
  the `Grid` checkbox via the egui test harness or by calling an extracted
  toggle helper directly. Assert `app.grid_enabled == false`. Toggle a second
  time; assert `true`. Covers AC#9.

- **Unit — `view_snap_checkbox_toggles_snap_enabled`** (AC#9):
  Same pattern for `snap_enabled` and the `Snap` checkbox.

- **Unit — `help_about_sets_about_open`** (AC#10):
  Construct `App::default()`. Simulate a click on the `About` menu item;
  assert `app.about_open == true`. Covers AC#10.

- **Unit — `select_all_covers_all_entities`** (AC#11):
  Construct an `App` with three `Entity::Line` entries in `app.document.entities`.
  Simulate a click on `Select All`; assert
  `app.document.selection.len() == 3`. Covers AC#11.

- **Unit — `zoom_in_decreases_mm_per_px`** (AC#12):
  `app.camera.mm_per_px` is `1.0` before the click; assert it is `< 1.0` after
  simulating `Zoom In`. Covers AC#12.

- **Unit — `zoom_out_increases_mm_per_px`** (AC#12):
  Inverse of above. Covers AC#12.

- **Unit — `fit_to_bed_uses_bed_bounds`** (AC#13):
  Set `app.bed.size_mm = [200.0, 100.0]` and `app.bed.origin_world = Vec2::new(0.0, 0.0)`.
  Set `app.camera.viewport_size_px = [800.0, 600.0]`. Simulate `Fit to Bed`.
  Assert `app.camera.center_world` is approximately `Vec2::new(100.0, 50.0)`.
  Covers AC#13.

- **Unit — `open_recent_submenu_empty_message`** (AC#14):
  Construct `App::default()` (settings has empty `recent_files`). Run one frame
  calling `draw_menubar`. Assert the call does not panic. The "No recent files"
  path is covered by this non-crash assertion; full label content is verified
  by manual smoke. Covers AC#14.

- **Static check — purity** (AC#15):
  `grep -rn 'use rfd\|use eframe' src/ui/menubar.rs` returns no output.

- **Static check — LOC budget** (AC#1):
  `wc -l src/ui/menubar.rs` ≤ 250.

- **Manual smoke — full menu walkthrough** (AC#3 through AC#14):
  `cargo run`. Confirm:
  1. Menubar appears at the top of the window in the order File / Edit / View / Help.
  2. File → New clears the canvas; File → Open opens a native file dialog;
     File → Save As opens a native save dialog; File → Exit closes the app.
  3. Draw two lines. Edit → Undo removes one; Edit → Undo removes the other.
     Edit → Redo restores one. Undo is grayed out when stack is empty.
  4. Draw three circles. Edit → Select All highlights all three.
  5. View → Zoom In enlarges the drawing; View → Zoom Out shrinks it.
     View → Fit to Bed frames the entire bed rectangle with margin.
  6. View → Grid unchecks and grid disappears; re-check and grid returns.
     View → Snap unchecks and snap markers stop; re-check and snap returns.
  7. Help → About opens the About dialog; clicking × closes it.
  8. Open a recent file: File → Open Recent shows the filename; clicking it
     loads the drawing. If no recent files, menu shows "No recent files" grayed out.

## Open questions

*(none — demand is Ready)*

## Notes

### Panel declaration order is load-bearing

`egui` claims panel space in declaration order. `TopBottomPanel::top` reserves
screen-top space before any `bottom` panel has a chance to shrink the remaining
area. Declaring `top("menubar")` first ensures it sits flush against the OS
window titlebar. Declaring it after a `bottom` panel would still work visually
but deviates from the egui idiom and may produce layout glitches on some
platforms.

### `action_*` method contracts (LCV-062)

The following methods must be present on `App` before `menubar.rs` compiles.
LCV-062 is responsible for their implementations; this demand only calls them:

| Method | Signature | Notes |
|---|---|---|
| `action_new` | `pub fn action_new(&mut self)` | Replaces `self.document` with `Document::default()`, clears history, resets `dirty_since`. |
| `action_open` | `pub fn action_open(&mut self)` | Opens a native file dialog; on confirm, delegates to `action_open_path`. |
| `action_open_path` | `pub fn action_open_path(&mut self, path: std::path::PathBuf)` | Loads an SVG file at `path` into `self.document`; pushes path to `self.settings.recent_files`. |
| `action_save` | `pub fn action_save(&mut self)` | Saves to the current file path; opens Save As dialog when no path is set. |
| `action_save_as` | `pub fn action_save_as(&mut self)` | Always opens a native save dialog; updates the current path on confirm. |

If LCV-062 has not yet landed when this demand is implemented, stub all five
methods as `pub fn action_…(&mut self[, …]) {}` in `app.rs` so this demand
compiles. The real implementations land with LCV-062.

### `grid_enabled` / `snap_enabled` field ownership

LCV-070 §4 also adds `snap_enabled` and `grid_enabled` to `App`. Whichever
demand is implemented first adds the field declarations and the `Default`
values. The implementer of the second demand finds the fields already present
and skips re-declaring them — no merge conflict arises because both demands
prescribe identical types and defaults (`bool`, `true`).

### Grayed-out Undo / Redo

Use `ui.add_enabled(cond, egui::Button::new("Undo\tCtrl+Z")).clicked()` inside
the Edit menu body. `add_enabled(false, …)` renders the button with
`ui.visuals().weak_text_color()` automatically — no manual color override
needed. Prefer this over `ui.button(…)` + a post-click guard because it
communicates intent to the user before they click.

### Open Recent: stale-path tolerance

`crate::io::open_recent` returns `Ok(PathBuf)` even if the file has since been
deleted (it only validates the index; the file read happens in
`action_open_path`). When `action_open_path` fails on a missing file, the error
is surfaced by the LCV-069 `error_dialog` mechanism — the menubar itself does
not need error handling beyond dropping the `Err` from `open_recent` (which
only errors on invalid indices, which cannot happen here because the index comes
from an enumerated slice).

### Exit via `ViewportCommand::Close`

`egui::ViewportCommand::Close` is the eframe ≥ 0.27 replacement for the
deprecated `frame.quit()`. Verify the variant name against the `egui` version
pinned in `Cargo.lock` (`0.29.x` as of this writing). Do not call
`std::process::exit`; let the eframe runtime perform a clean shutdown so the
`on_exit` hook (autosave flush) fires.

### `Vec2` import in `menubar.rs`

`Fit to Bed` needs `crate::geometry::Vec2` to construct the `(min, max)` tuple
for `camera.zoom_extents`. This is the only kernel type permitted in
`menubar.rs`. It is not a violation of the purity rule because `render/*` and
`ui/*` are both allowed to consume `geometry` types.

### LOC budget

250 LOC for four menus is generous. A data-driven approach (a `const` array of
`(&str, fn(&mut App))` per menu) can cover File and Edit in ~60 lines each;
View and Help are shorter. The implementer's choice of pattern is free; the
cap is the contract.
