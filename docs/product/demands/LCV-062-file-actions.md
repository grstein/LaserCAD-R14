# LCV-062 — File actions — New / Open / Save / Save As wired to dialogs and SVG I/O

- **Status**: Ready
- **Phase**: 5
- **Depends on**: LCV-056 (done), LCV-057 (done), LCV-061 (done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

A laser-cutter operator draws geometry in LaserCAD and needs to persist it as an
SVG file they can drag into LaserGRBL, then reopen the same file to refine the
cut path. Without wired file actions — New, Open, Save, Save As — the SVG
export and import functions that LCV-056 and LCV-057 delivered are dead code:
no keyboard shortcut or menu item reaches them, and there is no way to track
which file is currently open. This demand connects the keyboard shortcuts
(Ctrl+N / Ctrl+O / Ctrl+S / Ctrl+Shift+S) to the existing dialog wrappers and
SVG pipeline, giving the operator a complete save/restore workflow before the
Phase 6 menubar arrives.

## Scope

- **New file `src/io/file_actions.rs`** declaring four `pub` functions, all
  with signature `fn(app: &mut App)`:
  - `action_new` — resets the session to blank.
  - `action_open` — presents an open dialog, imports SVG, loads into document.
  - `action_save` — saves to the current path; falls back to `action_save_as`
    when no path is known.
  - `action_save_as` — presents a save dialog, exports SVG, writes file.
- **`App` struct additions** in `src/app.rs`:
  - `pub current_file: Option<PathBuf>` — path of the file most recently opened
    or saved, `None` for an unsaved new document.
  - `pub error_message: Option<String>` — when `Some`, a modal error window is
    rendered on the next frame; cleared when the user dismisses it.
- **Shortcut wiring** in `App::update`: detect global keyboard shortcuts with
  `ctx.input(…)` and dispatch to the corresponding action function.
- **Error display** in `App::update`: render `crate::ui::error_dialog` when
  `error_message.is_some()`; clear the field on OK.
- **Module wiring**: `src/io/mod.rs` declares `pub mod file_actions;` and
  re-exports the four functions.

## Out of scope

- Menubar File menu items — owned by LCV-065.
- Full keyboard-shortcut registry (L, P, R, C, A, F3, F7, F8, …) — owned by
  LCV-070.
- Confirm-discard dialog before New or Open when the document is dirty — a
  follow-on demand. `action_new` and `action_open` discard unsaved changes
  without confirmation for now.
- Recent-files submenu integration in the menubar — owned by LCV-065.
- Exit / close-window action (`Ctrl+Q`, `Alt+F4`) — out of scope; eframe
  handles the OS close event independently.
- Window-title update to reflect the current filename — "while you're at it";
  not in scope.
- Y-axis transformation or coordinate conversion — SVG export/import are
  passthrough by LCV-056 / LCV-057 contract.
- Per-entity layer assignment on import — all imported entities are placed in
  the document flat list; colour metadata is not preserved.

## Acceptance criteria

1. `src/io/file_actions.rs` exists and declares exactly four `pub fn` items —
   `action_new`, `action_open`, `action_save`, `action_save_as` — all with
   signature `fn(app: &mut App)`. No other public items are exported from this
   file.

2. `App` in `src/app.rs` gains two fields that default to `None`:
   `pub current_file: Option<std::path::PathBuf>` and
   `pub error_message: Option<String>`. `App::default()` returns `None` for
   both; the existing unit tests for `App::default()` continue to pass.

3. `action_new(app)` performs all of the following atomically within one call:
   - sets `app.document = Document::default()` (clears all entities and
     selection),
   - sets `app.history = History::default()` (clears undo/redo stack),
   - sets `app.current_file = None`,
   - sets `app.dirty_since = None`,
   - calls `crate::io::clear_autosave()`.

4. `action_open(app)` calls `crate::io::open_file_dialog()`. If the dialog
   returns `None` (user cancelled), the function returns immediately without
   modifying any `App` field.

5. When `action_open(app)` receives a path from the dialog, it reads the file
   with `std::fs::read_to_string`. On I/O failure it sets
   `app.error_message = Some(<descriptive string that includes the OS error>)`
   and returns without modifying the document.

6. When `action_open(app)` successfully reads the file, it parses the content
   with `crate::io::svg::import_svg`. On `Err(e)` it sets
   `app.error_message = Some(format!("SVG import failed: {e}"))` and returns
   without modifying the document.

7. After a successful open `action_open(app)` performs all of the following:
   - replaces `app.document` with a `Document` whose `entities` are the
     imported `Vec<Entity>` (selection, history, and other `Document` fields
     are reset to `Document::default()` values),
   - sets `app.history = History::default()`,
   - sets `app.current_file = Some(path.clone())`,
   - sets `app.dirty_since = None`,
   - calls `app.settings.push_recent_file(path.to_string_lossy().into_owned())`,
   - calls `app.settings.save()` — errors are silently swallowed (not set on
     `error_message`),
   - calls `crate::io::clear_autosave()`.

8. `action_save(app)` delegates to `action_save_as(app)` when
   `app.current_file` is `None`.

9. `action_save(app)` when `app.current_file` is `Some(path)`:
   - calls `crate::io::svg::export_svg(&app.document)` to obtain the SVG
     string,
   - writes the string to `path` with `std::fs::write`,
   - on I/O failure sets `app.error_message = Some(<descriptive message>)` and
     returns,
   - on success sets `app.dirty_since = None` and calls
     `crate::io::clear_autosave()`.
   Neither `export_svg` nor any other step in the success path modifies
   `app.current_file`.

10. `action_save_as(app)` calls `crate::io::save_file_dialog(default_name)`
    where `default_name` is:
    - the filename component (UTF-8) of `app.current_file` if it is `Some` and
      has a filename, or
    - `"untitled.svg"` otherwise.
    If the dialog returns `None` (user cancelled), the function returns
    immediately without modifying any `App` field.

11. `action_save_as(app)` ensures the chosen path carries a `.svg` extension
    (case-sensitive string comparison on `path.extension()`). If the extension
    is absent or different, `path.set_extension("svg")` is applied before
    writing.

12. After a successful save-as, `action_save_as(app)` performs all of the
    following:
    - writes the `export_svg` string to the (possibly extension-adjusted) path;
      on I/O failure sets `app.error_message` and returns,
    - sets `app.current_file = Some(path.clone())`,
    - sets `app.dirty_since = None`,
    - calls `app.settings.push_recent_file(path.to_string_lossy().into_owned())`,
    - calls `app.settings.save()` (errors silently swallowed),
    - calls `crate::io::clear_autosave()`.

13. `App::update()` processes the following global keyboard shortcuts via
    `ctx.input(|i| …)` — active regardless of which panel is hovered — and
    dispatches to the corresponding action:
    - `Ctrl+N` (ctrl pressed, N pressed, shift not pressed) → `action_new(self)`,
    - `Ctrl+O` (ctrl pressed, O pressed, shift not pressed) → `action_open(self)`,
    - `Ctrl+S` (ctrl pressed, S pressed, shift not pressed) → `action_save(self)`,
    - `Ctrl+Shift+S` (ctrl pressed, shift pressed, S pressed) →
      `action_save_as(self)`.
    Each check uses `i.key_pressed(egui::Key::*)` so that the action fires on
    exactly one frame per key-down event (not held-key repeat).

14. `App::update()` renders the error modal after all panels: when
    `self.error_message` is `Some(msg)`, it calls
    `crate::ui::error_dialog(ctx, "Error", msg)`; when `error_dialog` returns
    `true` (OK clicked), `self.error_message` is set to `None`.

15. `src/io/file_actions.rs` MUST NOT contain a top-level `use egui`, `use
    eframe`, or `use rfd` statement. Interaction with the dialog layer goes
    through `crate::io::open_file_dialog` / `crate::io::save_file_dialog`.

16. `src/io/file_actions.rs`:
    - carries a `//!` module header,
    - carries `///` doc comments on every `pub fn`,
    - contains no `unwrap()` or `expect()` outside `#[cfg(test)]` blocks,
    - stays within the 300-LOC hard cap.

17. `src/io/mod.rs` adds `pub mod file_actions;` and
    `pub use file_actions::{action_new, action_open, action_save, action_save_as};`
    so that all four functions are accessible as `crate::io::action_*` from
    any module.

18. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

All tests live in `#[cfg(test)] mod tests` inside `src/io/file_actions.rs`
unless noted.

- **Unit (AC 2):** `app_default_current_file_is_none` — `App::default().current_file`
  is `None`. *(Placed in `src/app.rs` tests alongside existing AC tests.)*

- **Unit (AC 2):** `app_default_error_message_is_none` — `App::default().error_message`
  is `None`. *(Placed in `src/app.rs` tests.)*

- **Unit (AC 3):** `action_new_clears_document_and_history` — construct an `App`,
  call `app.commit(Box::new(CreateLineCmd(…)))` to place one entity and record
  history, then call `action_new(&mut app)`. Assert `app.document.entity_count() == 0`
  and `!app.history.can_undo()`.

- **Unit (AC 3):** `action_new_resets_current_file` — set
  `app.current_file = Some(PathBuf::from("foo.svg"))`, call `action_new(&mut app)`,
  assert `app.current_file.is_none()`.

- **Unit (AC 3):** `action_new_clears_dirty_since` — set
  `app.dirty_since = Some(std::time::Instant::now())`, call `action_new(&mut app)`,
  assert `app.dirty_since.is_none()`.

- **Unit (AC 9):** `action_save_writes_svg_to_current_path` — create a temp
  file path; set `app.current_file = Some(tmp_path.clone())`; push one
  `Entity::Line` into `app.document`; call `action_save(&mut app)`. Assert the
  temp file exists and its content contains `"<svg"`.

- **Unit (AC 9):** `action_save_clears_dirty_since` — same setup plus
  `app.dirty_since = Some(Instant::now())`; call `action_save(&mut app)`;
  assert `app.dirty_since.is_none()`.

- **Unit (AC 9):** `action_save_io_error_sets_error_message` — set
  `app.current_file = Some(PathBuf::from("/nonexistent_dir/canary.svg"))`;
  call `action_save(&mut app)`; assert `app.error_message.is_some()`. (The
  write must fail because the directory does not exist.)

- **Unit (AC 11):** `action_save_as_appends_svg_extension` — cannot be tested
  headlessly (requires a dialog). Covered implicitly by the compile-time
  function-signature check below.

- **Compile check (AC 1, 17):** `file_actions_fn_signatures` — assert function
  pointer types in a no-op test:
  ```rust
  let _: fn(&mut App) = crate::io::action_new;
  let _: fn(&mut App) = crate::io::action_open;
  let _: fn(&mut App) = crate::io::action_save;
  let _: fn(&mut App) = crate::io::action_save_as;
  ```

- **Static check (AC 15):** `grep -nE '^use (egui|eframe|rfd)' src/io/file_actions.rs`
  returns no matches.

- **Static check (AC 16):** `wc -l src/io/file_actions.rs` reports ≤ 300.

- **Build gate (AC 18):** `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

### Module wiring checklist

```
src/io/file_actions.rs   — new file; four pub fn(app: &mut App)
src/io/mod.rs            — add: pub mod file_actions;
                                pub use file_actions::{action_new, action_open,
                                                       action_save, action_save_as};
src/app.rs               — add two fields to App struct;
                           add shortcut detection in update();
                           add error_message dialog rendering in update()
```

### Imports `file_actions.rs` will need

```rust
use std::fs;
use std::path::PathBuf;
use crate::app::App;
use crate::document::{Document, History};
use crate::io::{clear_autosave, open_file_dialog, save_file_dialog};
use crate::io::svg::{export_svg, import_svg};
```

No `egui`, `eframe`, or `rfd` imports are needed or permitted.

### Circular module concern

`io::file_actions` imports `crate::app::App`, which in turn imports
`crate::io::settings::Settings`. This is a function-level dependency, not a
type-level cycle, and is legal within a single Rust crate. `App` contains no
`file_actions` types; there is no structural cycle.

### Keyboard shortcut detection pattern

```rust
// In App::update(), before or after the panel closures — both placements
// are valid; choose after panels so the canvas response does not capture
// keyboard input first.
ctx.input(|i| {
    if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::N) {
        crate::io::action_new(self);
    }
    if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::O) {
        crate::io::action_open(self);
    }
    if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::S) {
        crate::io::action_save(self);
    }
    if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::S) {
        crate::io::action_save_as(self);
    }
});
```

Note: `ctx.input(…)` borrows `ctx` immutably and `self` mutably; the action
calls inside the closure take `self` as `&mut App`. The implementer must take
the `self` borrow into account — if the borrow checker rejects the inline form,
read the flags into local booleans first and dispatch after the closure.

### Error modal placement

`error_dialog` must be called after all panel content so it floats above the
canvas (same pattern as `about_dialog`):

```rust
// At the bottom of App::update(), after about_dialog:
if let Some(msg) = self.error_message.clone() {
    if crate::ui::error_dialog(ctx, "Error", &msg) {
        self.error_message = None;
    }
}
```

### `.svg` extension guard

The rfd save dialog on Linux (GTK3) does not automatically append the
extension. Always call `path.set_extension("svg")` when
`path.extension().and_then(|e| e.to_str()) != Some("svg")`.

### Settings save on open/save-as

`app.settings.save()` writes to the platform config directory and is a
best-effort call. Errors here (e.g., read-only filesystem) must **not** be
surfaced via `error_message`; log with `eprintln!` or silently swallow.
The user's drawing is already safe on disk at this point.

### Relation to LCV-065 (Menubar) and LCV-070 (Shortcuts)

- LCV-065 (Phase 6) will call the same four action functions from the File menu.
  Keep the function signatures stable.
- LCV-070 will add the remaining shortcuts (L, P, R, C, A, F3, F7, F8,
  Ctrl+Z / Ctrl+Y). The four Ctrl+N/O/S shortcuts added here may be
  consolidated into `src/ui/shortcuts.rs` in LCV-070 without breaking
  existing behaviour.

### Testing action_open and action_save_as headlessly

Both functions open a native dialog and cannot be driven in a CI environment
without a display. Limit their test coverage to:

1. The compile-time function-signature check (proves the code compiles and
   the signatures match).
2. Manual smoke test: run the app, press Ctrl+O / Ctrl+Shift+S, verify the
   dialog appears and the document updates.

For `action_save` with `current_file = Some(path)` there is no dialog, making
it fully unit-testable — see the three `action_save_*` tests above.
