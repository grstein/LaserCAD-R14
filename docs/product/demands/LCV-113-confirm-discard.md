# LCV-113 — Confirm discard on New / Open / Exit when the document has unsaved changes

- **Status**: Ready
- **Phase**: 11
- **Depends on**: LCV-102 (Done), LCV-103 (Done), LCV-105 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

Every destructive entry point in v2 destroys the drawing without asking. File >
New and Ctrl+N run `action_new`, whose own doc comment admits it: *"Unsaved
changes are discarded without confirmation (a confirm-discard dialog is deferred
to a later demand)."* File > Open, Open Recent and Ctrl+O replace `document` and
`history` the moment the native dialog returns. File > Exit and the window close
button end the process. An operator who mis-types `Ctrl+N` after forty minutes of
layout work loses all of it, and the undo stack with it — Ctrl+Z cannot bring it
back, because `action_new` also replaces `History`.

v1 asked (`../LaserCAD-R14/src/io/file-actions.ts::confirmDiscard`), v2 does not.
The dialog itself already exists and has **zero callers**:
`src/ui/dialogs.rs::confirm_dialog` returns `Some(DialogResult::Confirmed)` /
`Some(DialogResult::Cancelled)` on the frame a button is clicked. This demand
wires it to the three destructive entry points behind a signal that means what
it says.

### Why `dirty_since` is the wrong signal (read this before writing code)

`App::dirty_since` is an **autosave debounce timer**, not an unsaved-changes
flag. `src/app/autosave.rs::flush_if_due` calls `app.mark_clean()` after every
flush, so `dirty_since` returns to `None` roughly every 800 ms while the
operator is drawing. A guard built on it would ask for confirmation only inside
the sub-second window between a change and the next autosave write — i.e. almost
never. `last_synced_revision` has the same problem for the same reason.

The correct signal is a second, independent mark: the `History::revision()` at
which the in-memory document was last known to be **safe to discard** (just
written to a file, just loaded from a file, or just reset to blank). Autosave
never touches it, because an autosave is crash recovery, not a save: the file on
disk — if there is one at all — is still stale.

## Scope

- **`App::saved_revision: Option<u64>`** — the revision at which the document
  was last safe to discard. `None` means "never was" (a fresh unsaved document,
  or one recovered from autosave at boot).
- **`App::has_unsaved_changes(&self) -> bool`** — the single predicate every
  entry point consults.
- **`App::mark_saved(&mut self)`** — sets `saved_revision = Some(history.revision())`
  and calls the existing `mark_clean()`. The five call sites in
  `src/io/file_actions.rs` switch from `mark_clean()` to `mark_saved()`;
  `flush_if_due` keeps calling `mark_clean()` and must not learn about
  `saved_revision`.
- **`PendingAction`** enum and `App::pending_action: Option<PendingAction>` —
  the destructive action parked while the dialog is up.
- **`request_new` / `request_open` / `request_open_path` / `request_exit`** —
  the guarded entry points. They run the action immediately on a clean document
  and park it on a dirty one.
- **Rewiring the three entry points**: File > New + `Ctrl+N`; File > Open… +
  `Ctrl+O` + File > Open Recent; File > Exit + the window close button (X).
- **The dialog**: `confirm_dialog` gains caller-supplied button labels, and a
  `draw_discard_dialog` driver that applies the parked action on **Discard** and
  drops it on **Cancel**.
- **A new file `src/app/file_ops.rs`** holding all of the above plus the five
  existing `App::action_*` delegating wrappers, moved verbatim out of
  `src/app/mod.rs` (which is at 293 implementation lines of a 300 cap and cannot
  absorb this demand otherwise).

## Out of scope

- **A "Save" button in the dialog.** Two buttons, Discard and Cancel. A third
  ("Save then continue") re-enters `action_save`, which for an unsaved document
  opens a native `rfd` dialog *on top of* an egui modal — a nested-modal
  lifecycle for a case the operator solves in two clicks (Cancel, Ctrl+S). v1
  shipped two buttons; so does this.
- **Escape closing the dialog.** Escape is routed to the active tool by
  `src/app/input.rs` and there is exactly one keyboard reader (ADR 0002 §A6).
  Teaching the dialog to read Escape adds a second reader for a key that is
  already spoken for. The Cancel button is the way out.
- **Content comparison.** Undoing back to the saved geometry still counts as
  unsaved, because `History::revision()` is monotonic (LCV-102 §Risks documents
  this). The guard is conservative by design: it may ask when nothing would be
  lost, never the reverse.
- **A modified-document title bar marker** (`*untitled.svg`) and any statusbar
  indicator — LCV-116 owns the statusbar; a window title is a separate demand.
- **Autosave-recovery UX.** A document recovered at boot is treated as unsaved
  (it is), but this demand adds no "recovered from crash" banner or prompt.
- **Guarding anything else.** Tool switches, Select All, undo/redo and the agent
  are non-destructive to the document as a whole and stay unguarded.
- **Changing what `action_new` / `action_open` / `action_open_path` do** once
  they run. They keep clearing the autosave file and resetting history.

## Acceptance criteria

1. **The field.** `App` carries `saved_revision: Option<u64>`, `App::default()`
   sets it to `None`, and its doc comment states the two-rule meaning in §Scope.
   `App::new()` needs no code for the autosave-recovery case: a recovered
   document leaves `saved_revision == None` with a non-empty document, which
   rule 2 below already reports as unsaved.

2. **The predicate.** `pub fn has_unsaved_changes(&self) -> bool` returns:
   - `Some(r)` → `self.history.revision() != r`;
   - `None` → `self.document.entity_count() > 0`.

   Truth table, each row a test case:

   | state | expected |
   |---|---|
   | `App::default()` (blank, `None`) | `false` |
   | blank, one `history.commit` (`None`, 1 entity) | `true` |
   | after `mark_saved()` | `false` |
   | after `mark_saved()` then one commit | `true` |
   | after `mark_saved()` then `mark_clean()` (an autosave flush) | `false` — then one commit → `true` |
   | after `mark_saved()`, one commit, then `mark_clean()` (autosave flushed the change) | **`true`** — the change is autosaved but not saved |
   | after `mark_saved()` then `history.undo` | `true` |
   | `saved_revision = None` with entities pushed directly (boot recovery) | `true` |

3. **`mark_saved` is the only writer.** `pub fn mark_saved(&mut self)` sets
   `saved_revision = Some(self.history.revision())` **and** calls
   `mark_clean()`. `grep -rn "saved_revision" src/` shows assignments only in
   `App::default` and `mark_saved`. `src/app/autosave.rs` does not mention
   `saved_revision` at all, and LCV-102 AC 13 still holds (`dirty_since = None`
   appears exactly once, inside `mark_clean`).

4. **The five file actions mark saved, not clean.** In `src/io/file_actions.rs`,
   `action_new`, `action_open`, `action_open_path`, `action_save` and
   `action_save_as` call `app.mark_saved()` in place of today's
   `app.mark_clean()`, and always **after** any `app.history` replacement.
   `grep -n "mark_clean" src/io/file_actions.rs` returns no matches.

5. **The parked action.** `pub enum PendingAction { New, Open, OpenPath(PathBuf), Exit }`
   (`Debug`, `PartialEq`) and `App::pending_action: Option<PendingAction>`,
   `None` by default.

6. **Guarded entry points.** In `src/app/file_ops.rs`:
   - `pub fn request_new(&mut self)`, `request_open(&mut self)`,
     `request_open_path(&mut self, path: PathBuf)`: when
     `!has_unsaved_changes()` they call the matching `action_*` immediately and
     leave `pending_action == None`; otherwise they set `pending_action` to the
     matching variant and change **nothing** else — `document`, `history`,
     `current_file` and `settings` are untouched.
   - `pub fn request_exit(&mut self) -> bool`: returns `true` when the document
     is clean (the caller closes the window) and `false` after parking
     `PendingAction::Exit` (the caller cancels the close). It returns `bool`
     because it is the only one of the four whose action needs an
     `egui::Context`; the other three perform theirs directly.
   - All four are no-ops returning the "do not proceed" answer while
     `pending_action.is_some()`, so a second Ctrl+N or a second click on the
     window X cannot queue a second action or swap the parked one.

7. **Menubar.** `File > New`, `File > Open…`, each `File > Open Recent` entry and
   `File > Exit` call `request_new` / `request_open` / `request_open_path` /
   `request_exit` respectively. Exit sends
   `egui::ViewportCommand::Close` only when `request_exit()` returned `true`.
   `grep -n "action_new()\|action_open()\|action_open_path(" src/ui/menubar.rs`
   returns no matches.

8. **Shortcuts.** In `src/ui/shortcuts.rs::dispatch_shortcuts`, `Ctrl+N` calls
   `request_new` and `Ctrl+O` calls `request_open`. `Ctrl+S` / `Ctrl+Shift+S`
   are unchanged (saving is not destructive). No new key is read anywhere:
   `dispatch_shortcuts` and `src/app/input.rs` remain the only two readers
   (ADR 0002 §A6), and the discard dialog reads no key at all.

9. **The window close button.** `pub fn poll_close_request(ctx: &egui::Context, app: &mut App)`
   is called once per frame from `App::update_ui`, immediately before
   `panels::draw_dialogs`. When `ctx.input(|i| i.viewport().close_requested())`
   is true it calls `request_exit()`; if that returns `false` it sends
   `egui::ViewportCommand::CancelClose` **in the same frame** (verified: eframe
   0.29.1 `epi_integration.rs:288-298` checks that frame's
   `viewport_output[ROOT].commands` and closes otherwise). On a clean document it
   sends nothing and the app closes.

10. **The dialog.** `confirm_dialog(ctx, title, message, confirm_label, cancel_label)
    -> Option<DialogResult>` — the two hard-coded `"Yes"` / `"No"` strings become
    parameters, rendered in that order in the existing horizontal row; everything
    else about the window (centred anchor, not resizable, not collapsible, no ×)
    is unchanged. The discard call passes:
    - title: `"Discard unsaved changes?"`
    - message: `"The current drawing has unsaved changes. Continuing will discard them and the undo history."`
    - confirm: `"Discard"`, cancel: `"Cancel"`.

11. **The driver.** `pub fn draw_discard_dialog(ctx: &egui::Context, app: &mut App)`
    is called from `panels::draw_dialogs`, renders nothing while
    `pending_action.is_none()`, and on a click delegates to
    `pub fn apply_dialog_result(ctx: &egui::Context, app: &mut App, result: DialogResult)`.
    Splitting the decision out of the render is mandatory: it is what makes AC 12
    and AC 13 testable without simulating a pointer click.

12. **Discard performs the parked action exactly once.**
    `apply_dialog_result(.., Confirmed)` with `pending_action == Some(New)` runs
    `action_new` once and leaves `pending_action == None`; with `Some(Exit)` it
    sends `ViewportCommand::Close` and leaves `pending_action == None`.
    (`Open` / `OpenPath` follow the same shape but must not be exercised in an
    automated test — see §Expected tests.)

13. **Cancel discards nothing.** `apply_dialog_result(.., Cancelled)` sets
    `pending_action = None` and leaves `document.entity_count()`,
    `history.revision()`, `current_file`, `saved_revision` and `dirty_since`
    exactly as they were, and sends no viewport command. The app stays open.

14. **Autosave does not suppress the prompt.** A document whose changes have
    been autosaved (`dirty_since == None`, `last_synced_revision` current) but
    never written to a file still reports `has_unsaved_changes() == true` and
    still raises the dialog. This is row 6 of AC 2 and gets its own named test —
    it is the criterion most likely to be "optimised away".

15. **File split and caps.** `src/app/file_ops.rs` exists, is declared in
    `src/app/mod.rs` and re-exported per the AGENTS.md `mod.rs` rule; it holds
    `PendingAction`, the four `request_*` functions, `has_unsaved_changes`,
    `mark_saved`, `draw_discard_dialog`, `apply_dialog_result`,
    `poll_close_request`, and the five `App::action_*` wrappers moved verbatim
    from `mod.rs`. Every touched file stays ≤ 300 implementation lines (lines
    before the first `#[cfg(test)]`), `src/app/mod.rs` included. `file_ops.rs`
    must not import `eframe` or `rfd` (it may import `egui`).

16. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`
    and `cargo test --all` all exit 0.

## Expected tests

**Quality bar (Marco 1).** Every criterion above is covered by an automated test
or tagged **[manual]** below. Everything that touches `App::update_ui` carries a
headless regression test driving the real frame body through
`tests/harness/mod.rs` (`SCREEN`, `key_events`, `raw_input`, `frame`, `tap`).
The three egui 0.29.1 traps — key-repeat rewrite, pointer warm-up frame,
one-frame focus lag — are documented in ADR 0002 §A4; follow it rather than
rediscovering it. The May-2026 drive shipped 72 demands with a green unit gate
and ten integration defects; a green `cargo test` that never drives `update_ui`
is not evidence.

**Two hard test rules specific to this demand:**

- **Never call `request_open` / `request_open_path` on a clean document from a
  test, and never confirm a parked `Open` / `OpenPath`.** Both paths reach
  `src/io/file_actions.rs`, which opens a blocking native `rfd` dialog and hangs
  the run (ADR 0002 §A4 rule 1). The *dirty* path is safe and is what the tests
  exercise: it only parks the action.
- `Ctrl+N` is safe in both directions and is the end-to-end vehicle.

- **Unit (AC 1, 2, 14)** in `src/app/file_ops.rs`:
  `has_unsaved_changes_truth_table` — one assertion per row of AC 2, built with
  `App::default()` + `history.commit(CreateLine …)`;
  `autosave_flush_does_not_clear_unsaved_changes` — the row-6 regression test,
  doc comment naming `dirty_since` as the wrong signal.
- **Unit (AC 3, 4)**: `mark_saved_sets_revision_and_clears_dirty`; static check
  — the two greps.
- **Unit (AC 5, 6)**: `request_new_on_clean_document_acts_immediately` (asserts
  the document is reset and `pending_action` stays `None`);
  `request_new_on_dirty_document_parks_and_changes_nothing`;
  `request_open_on_dirty_document_parks_without_touching_the_filesystem`;
  `request_exit_returns_false_and_parks_when_dirty` / `..._true_when_clean`;
  `second_request_while_pending_is_a_no_op`.
- **Unit (AC 12, 13)**: `confirm_runs_the_parked_action_once`,
  `cancel_restores_nothing_and_clears_the_pending_action` — both call
  `apply_dialog_result` directly inside a `ctx.run(…)` closure so the Exit case
  can assert on viewport commands.
- **Integration (AC 7, 8, 9, 11)** in `tests/lcv113.rs` with `mod harness;`:
  - `ctrl_n_on_clean_document_resets_immediately` — `tap(Ctrl+N)`, assert no
    dialog was parked.
  - `ctrl_n_on_dirty_document_opens_the_dialog_and_keeps_the_drawing` — commit
    two lines through `app.history`, `tap(Ctrl+N)`, assert
    `pending_action == Some(PendingAction::New)` **and**
    `document.entity_count() == 2`. This is the defect this demand fixes; the
    doc comment says so.
  - `ctrl_o_on_dirty_document_parks_without_opening_a_native_dialog` — asserts
    `pending_action == Some(PendingAction::Open)`; the test stops there (never
    confirms).
  - `close_request_on_a_dirty_document_is_cancelled` — builds its own
    `egui::RawInput` inline (the harness's `raw_input` cannot express viewport
    events, and ADR 0002 §A3's five-item harness list stays untouched), setting
    `viewports` for `ViewportId::ROOT` to a `ViewportInfo` whose `events` contain
    `egui::ViewportEvent::Close`, drives one `ctx.run(input, |ctx| app.update_ui(ctx))`,
    and asserts the returned `FullOutput.viewport_output[&ViewportId::ROOT].commands`
    contains `egui::ViewportCommand::CancelClose` and
    `pending_action == Some(PendingAction::Exit)`.
    (Verified against egui 0.29.1: `RawInput::default()` already seeds
    `viewports` with `ViewportId::ROOT`, so `i.viewport()` never panics in the
    existing harness; `ViewportOutput::commands` is a public `Vec<ViewportCommand>`
    and `ViewportCommand` derives `PartialEq`.)
  - `close_request_on_a_clean_document_is_not_cancelled` — same input, empty
    document, asserts `CancelClose` is **absent**.
- **Unit (AC 10)** in `src/ui/dialogs.rs`: extend
  `confirm_dialog_returns_none_without_click` to the new signature and add
  `confirm_dialog_renders_custom_labels` — renders through `ctx.run` with
  `"Discard"` / `"Cancel"` and asserts no panic and `None` without a click.
- **Static check (AC 15)**: file exists, `wc -l` to the first `#[cfg(test)]` for
  every touched file, purity grep `grep -nE '^use (eframe|rfd)' src/app/file_ops.rs`.
- **Build gate (AC 16)**: the three cargo commands.
- **[manual] smoke** — the criteria no headless test covers (the actual button
  clicks and the real window chrome): `cargo run`; draw two lines; press Ctrl+N →
  the dialog appears, the drawing is still on screen behind it; click **Cancel** →
  dialog closes, both lines still there, Ctrl+Z still undoes the second line;
  press Ctrl+N again → click **Discard** → empty canvas. Repeat with File > Open…
  and confirm the native file dialog appears only *after* Discard. Draw one line,
  wait 2 s (autosave fires), then press Ctrl+N → the dialog **still** appears.
  Save the file (Ctrl+S), then press Ctrl+N → **no** dialog. Draw a line and click
  the window's X → the dialog appears and the window stays open; click Discard →
  the app exits. Click X on a saved document → it exits with no dialog.

## Risks

- **The action runs after the dialog, not with it.** `confirm_dialog` is
  immediate-mode: the parked `PendingAction` is the state machine. Forgetting to
  clear it on either branch leaves a dialog that reopens every frame. AC 12 and
  AC 13 both assert the clear.
- **`request_open` on a clean document opens a native dialog.** That is correct
  behaviour and a test hazard; it is called out twice above on purpose.
- **Merge overlap.** LCV-114 also edits `src/ui/menubar.rs` (File > Bed size…)
  and `src/io/file_actions.rs` (adopting the file's bed). LCV-115 edits
  `action_save` / `action_save_as` too. The File menu is the shared surface;
  whoever lands second re-reads it.
- **`src/app/mod.rs` is at 293/300 implementation lines.** The file split in
  AC 15 is not optional tidy-up; without it this demand cannot land inside the
  cap.
- **Conservative prompts.** Undo-to-saved-state asks anyway. Accepted (§Out of
  scope); the alternative is content hashing, rejected in ADR 0002 §B for the
  autosave signal and rejected here for the same reason.

## Open questions

*(none — demand is Ready)*

## Notes

- v1 reference: `../LaserCAD-R14/src/io/file-actions.ts:33` (`confirmDiscard`,
  guarded on `state.entities.length === 0`) and `:117,121` (New and Open). v1 did
  not guard Exit; v2 does, because v2 is a native window with an X button and no
  browser "leave site?" fallback.
- v1's guard was "the document is non-empty", which prompts after every open
  even when nothing changed. v2's revision-based predicate is strictly better
  and falls back to v1's rule exactly where it has to (`saved_revision == None`).
- `App::default()` is the test constructor and `App::new()` is boot-only
  (ADR 0002 §A2). No test may call `App::new()`, so the autosave-recovery row of
  AC 2 is simulated by pushing entities into a default `App` with
  `saved_revision == None`.
- `DialogResult` already exists with `Confirmed` / `Cancelled` variants and is
  re-exported from `crate::ui`; no new result type.
- LCV-116 adds an autosave indicator to the status bar. It reads
  `dirty_since`, not `saved_revision`; the two signals answer different
  questions ("is a write pending?" vs "would I lose work?") and both stay.
