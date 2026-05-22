# LCV-069 — Modal dialogs (confirm / error / about)

- **Status**: Ready
- **Phase**: 6
- **Depends on**: LCV-030 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

The operator needs three lightweight in-app dialogs to support the Phase 6 UI chrome and the Phase 7
agent settings demand (LCV-076): a **confirmation dialog** (e.g., "Discard unsaved changes?"), an
**error dialog** (e.g., "File could not be opened"), and an **About dialog** (app name, version,
license). Without them, destructive actions have no guard, error paths are silent, and the Help menu
has nothing to open. All three must be pure egui floating windows — no OS dialog (no `rfd`), no
blocking thread, no extra crate.

## Scope

- New file `src/ui/dialogs.rs` containing:
  - `pub enum DialogResult { Confirmed, Cancelled }` — derives `Debug, PartialEq, Clone, Copy`.
  - `pub fn confirm_dialog(ctx: &egui::Context, title: &str, message: &str) -> Option<DialogResult>` —
    egui `Window` anchored to viewport center, non-resizable, not collapsible, no close (×) button;
    body shows `message`; two buttons: **Yes** → `Some(Confirmed)`, **No** → `Some(Cancelled)`;
    returns `None` while neither button has been pressed in the current frame.
  - `pub fn error_dialog(ctx: &egui::Context, title: &str, message: &str) -> bool` — same window
    style as above; body shows `message`; single **OK** button; returns `true` on the frame the
    button is clicked, `false` every other frame.
  - `pub fn about_dialog(ctx: &egui::Context, open: &mut bool)` — `egui::Window::new("About LaserCAD")
    .open(open)`, resizable: false, collapsible: false; body shows the literal string `"LaserCAD v2"`,
    then the version string produced by `env!("CARGO_PKG_VERSION")`, then the license string
    `"MIT OR Apache-2.0"`; egui's built-in × button sets `*open = false`.
- `src/ui/mod.rs` re-exports `dialogs::{DialogResult, confirm_dialog, error_dialog, about_dialog}`.
- `src/app.rs` `App` struct gains `pub about_open: bool` (participates in `#[derive(Default)]`,
  defaults to `false`); `App::update` calls
  `crate::ui::about_dialog(ctx, &mut self.about_open)` **after** the `CentralPanel::default().show`
  block.
- File size: `src/ui/dialogs.rs` ≤ 200 LOC.

## Out of scope

- True OS-level modal blocking (only visual convention; egui does not natively block pointer events
  to other panels).
- Any rfd import or OS dialog.
- A "Help → About" menu item — that belongs to LCV-065 (Menubar). This demand only adds the field,
  the wiring in `App::update`, and the dialog logic; the menubar sets `about_open = true`.
- Custom styling, icons, or branded imagery inside the dialogs — that is LCV-071 (Theme).
- A persistent open-state field on `App` for `confirm_dialog` or `error_dialog` — callers own their
  own `bool` flag (see Notes).
- Internationalization, RTL, or non-ASCII button labels.

## Acceptance criteria

1. `src/ui/dialogs.rs` declares `pub enum DialogResult { Confirmed, Cancelled }` with
   `#[derive(Debug, PartialEq, Clone, Copy)]`.  
   _Verify_: `grep -n 'enum DialogResult' src/ui/dialogs.rs` returns exactly one match.

2. `confirm_dialog(ctx, title, message)` renders an `egui::Window` whose title equals `title`, is
   anchored to the viewport center (`Align2::CENTER_CENTER`), is not resizable, is not collapsible,
   and has no × close button (`egui::Window::…collapsible(false).resizable(false)`); the window body
   contains `message` as a label and exactly two buttons labelled **"Yes"** and **"No"**; when
   neither is clicked the return value is `None`.  
   _Verify_: unit test (see Expected tests §1).

3. `confirm_dialog` returns `Some(DialogResult::Confirmed)` on the same frame the **Yes** button is
   clicked, and `Some(DialogResult::Cancelled)` on the same frame the **No** button is clicked.  
   _Verify_: manual smoke (see Expected tests §4).

4. `error_dialog(ctx, title, message)` renders a window with the same modal-style constraints as
   `confirm_dialog`; the body contains `message` and a single **"OK"** button; returns `true` on the
   frame OK is clicked, `false` every other frame.  
   _Verify_: unit test (see Expected tests §2).

5. `about_dialog(ctx, open)` renders `egui::Window::new("About LaserCAD").open(open)` with
   `resizable(false)` and `collapsible(false)`; the body contains the exact string `"LaserCAD v2"`,
   followed by the crate version obtained via `env!("CARGO_PKG_VERSION")`, followed by
   `"MIT OR Apache-2.0"`; clicking the × button sets `*open = false`.  
   _Verify_: unit test (see Expected tests §3) + manual smoke (see Expected tests §4).

6. `src/ui/mod.rs` re-exports all four public items (`DialogResult`, `confirm_dialog`,
   `error_dialog`, `about_dialog`) from the `dialogs` submodule.  
   _Verify_: `use lasercad::ui::{DialogResult, confirm_dialog, error_dialog, about_dialog};`
   compiles without error (exercised by the unit tests which import via `crate::ui::`).

7. `App` in `src/app.rs` has a `pub about_open: bool` field; `App::default().about_open == false`;
   `App::update` calls `crate::ui::about_dialog(ctx, &mut self.about_open)` at least once per frame,
   after the `CentralPanel::default().show` block.  
   _Verify_: unit test (see Expected tests §5).

8. `wc -l src/ui/dialogs.rs` reports ≤ 200; `grep -n 'rfd' src/ui/dialogs.rs` returns no matches.  
   _Verify_: static check commands above exit 0 / empty output.

9. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and
   `cargo test --all` all exit 0.

## Expected tests

Tests live in `#[cfg(test)] mod tests` inside `src/ui/dialogs.rs` and `src/app.rs`.
All egui tests use `egui::Context::default()` + `ctx.run(egui::RawInput::default(), |ctx| { … })`.

- **§1 — unit (AC 2 + AC 3, none case)**  
  Construct a fresh `egui::Context`. Run one frame calling `confirm_dialog(ctx, "T", "M")` without
  synthesizing any button click. Assert the return value is `None`.

- **§2 — unit (AC 4, none case)**  
  Construct a fresh `egui::Context`. Run one frame calling `error_dialog(ctx, "E", "Msg")` without
  a button click. Assert the return value is `false`.

- **§3 — unit (AC 5, open stays true)**  
  Construct a fresh `egui::Context`. Set `let mut open = true`. Run one frame calling
  `about_dialog(ctx, &mut open)` without simulating a close event. Assert `open == true`.

- **§4 — manual smoke (AC 3 + AC 5)**  
  Temporarily hard-code `about_open: true` in `App::default()` (revert before commit, or use a
  `--feature dev-open-about` guard). Run `cargo run`. Confirm the About dialog appears at viewport
  center showing "LaserCAD v2", the version string, and the license. Click ×; confirm the dialog
  closes. Re-run with `about_open: false`; confirm no dialog appears at startup. Record pass/fail in
  the `Implementation:` line.

- **§5 — unit (AC 7)**  
  In `src/app.rs` tests:  
  `let app = App::default(); assert!(!app.about_open);`

- **§6 — static check (AC 8)**  
  `wc -l src/ui/dialogs.rs` ≤ 200.  
  `grep -n 'rfd' src/ui/dialogs.rs` returns empty output.

- **§7 — build gate (AC 9)**  
  `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all`
  exits 0.

## Open questions

(none)

## Notes

- **Caller-owned open state**: `confirm_dialog` and `error_dialog` are stateless helpers; they
  render the window every frame they are called. The caller is responsible for holding a `bool` flag
  and stopping the call once the dialog is dismissed. Example usage pattern:
  ```rust
  if self.confirm_discard {
      if let Some(r) = crate::ui::confirm_dialog(ctx, "Discard changes?", "Unsaved work will be lost.") {
          self.confirm_discard = false;
          if r == DialogResult::Confirmed { self.new_document(); }
      }
  }
  ```
  This keeps `App` free of per-dialog boolean fields for every future confirm/error site.

- **`about_open` on `App`**: the About dialog is the only one that has a persistent driver on `App`
  because it is triggered by the Help menu (LCV-065), which needs a durable toggle. The menubar will
  do `if ui.button("About").clicked() { self.about_open = true; }`.

- **egui Window anchoring**: `egui::Window::new(title).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])`
  pins the window to the viewport center. This is the correct call for egui ≥ 0.27; implementer
  verifies against the egui version pinned in `Cargo.lock`.

- **No `open()` on confirm/error windows**: these windows must not expose egui's × button (which
  would allow dismissal without a definitive Yes/No/OK answer). Omitting `.open(&mut bool)` is the
  correct approach; do NOT pass an open flag to these two windows.

- **`env!("CARGO_PKG_VERSION")`**: expands at compile time to the `version` field in `Cargo.toml`.
  No runtime lookup needed.

- **300 LOC cap from AGENTS.md**: the 200 LOC cap in this demand is stricter than the repo-wide 300
  LOC cap; three small functions should fit comfortably in 150–180 lines including doc comments and
  tests.

- **Phase 7 consumer**: LCV-076 (Agent settings dialog) depends on LCV-069 because it will call
  `confirm_dialog` when the user clears the API key. It may also call `error_dialog` on invalid
  endpoint URLs.
