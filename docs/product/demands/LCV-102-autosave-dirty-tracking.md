# LCV-102 — Autosave actually fires (History revision as the dirty signal)

- **Status**: Done
- **Phase**: 10
- **Depends on**: LCV-059 (Done), LCV-026 (Done); ADR 0002 §B
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: a958d29 — fix(LCV-102): track document revisions so autosave actually fires, with an 800 ms debounce

## Problem

Autosave is dead code. The flush check in the frame body only runs when
`app.dirty_since` is `Some`, and `dirty_since` is written in exactly one place —
`App::commit` — which only `TextTool` uses. Every other tool (Line, Polyline,
Rect, Circle, Arc, Move, Trim, Extend, Delete, Offset, Select) calls
`history.commit(cmd, doc)` directly, a deliberate LCV-041 design so tools need no
`&mut App`; the agent's five commit sites in `src/agent/tools.rs` do the same;
and undo/redo change the document without committing anything at all. The result
is that an operator can draw an entire part and crash out with **nothing** in
`~/.local/share/lasercad/autosave.json` — the exact scenario the feature was
built for. The debounce is also 5 s instead of v1's 800 ms, so even the one tool
that does mark the document dirty saves late.

ADR 0002 §B already decided the mechanism. This demand implements it.

## Scope

Implement ADR 0002 §B verbatim. It is a settled decision — the alternatives
(routing every tool commit through `App::commit`, hashing the `Document`,
sampling `history.len()`) are rejected in the ADR and must not be revisited here.

- **`src/document/history.rs`**: private `revision: u64` field, `pub fn
  revision(&self) -> u64`, incremented in `commit` and in `undo` / `redo` on the
  branch that actually did work. Kernel-pure.
- **`src/app`**: new field `last_synced_revision: u64` (defaults to `0`),
  private `fn sync_dirty(&mut self)`, and `pub fn mark_clean(&mut self)`.
- **`sync_dirty()` is the only writer of `dirty_since = Some(_)`**, called
  exactly once per frame at the end of the frame body, immediately before the
  autosave-flush check. The `dirty_since.get_or_insert_with(Instant::now)` line
  in `App::commit` is removed.
- **`mark_clean()` is the only writer of `dirty_since = None`.** The five
  `app.dirty_since = None;` sites in `src/io/file_actions.rs` and the one in the
  autosave-flush branch become `app.mark_clean()`.
- **`AUTOSAVE_DEBOUNCE` becomes `Duration::from_millis(800)`** (v1 parity).
- **`pub fn autosave_due(dirty_since: Option<Instant>, now: Instant) -> bool`**
  extracted so the debounce is unit-testable without sleeping and without
  touching disk.

## Out of scope

- **Statusbar autosave / dirty indicator** (a "●" marker, "saved 3 s ago" text,
  a modified-title asterisk). Marco 1 chrome. Do not add any UI.
- **Confirm-on-discard dialogs** for New / Open / quit with unsaved changes.
- **Changing the autosave file format or location** (`src/io/autosave.rs` is
  untouched; it stays JSON in world millimetres).
- **Autosave on quit, or a save-on-focus-loss hook.**
- **Routing tool commits through `App::commit`** — explicitly rejected in
  ADR 0002 §B ("Why this and not…"). Tools keep calling `history.commit`.
- **Splitting `src/app.rs`** — LCV-105 owns that. Add the ~15 lines where the
  code lives today.
- **The keyboard double-dispatch defect** that makes Ctrl+Z undo twice — that is
  LCV-103. This demand must not change key routing; its undo/redo criteria are
  asserted by calling `History::undo` / `History::redo` directly.

## Acceptance criteria

1. `History::revision()` exists and is `pub`; a freshly constructed `History`
   (`new`, `default`, `with_depth`) reports `0`.

2. `History::commit` increments the revision by exactly 1 per call, including
   commits that evict the oldest entry at the depth cap.

3. `History::undo` increments the revision by exactly 1 when it returns `true`
   and leaves it unchanged when it returns `false` (empty undo stack). Same for
   `History::redo` with the redo stack.

4. The revision is monotonically non-decreasing across any commit / undo / redo
   sequence: a commit-undo-commit sequence yields three distinct increasing
   values (this is the property `history.len()` does not have).

5. `src/document/history.rs` stays kernel-pure
   (`grep -nE '^use (egui|eframe|rfd)' src/document/history.rs` → no matches)
   and its implementation LOC (excluding `#[cfg(test)]`) stays ≤ 300.

6. `App` has a `last_synced_revision: u64` field initialised to `0` by
   `App::default()`.

7. `App::sync_dirty()` sets `dirty_since` via `get_or_insert_with(Instant::now)`
   **iff** `history.revision() != last_synced_revision`, then stores the new
   revision. Calling it twice with no intervening mutation does not change
   `dirty_since` (idempotent, and the first dirty instant is preserved — the
   debounce is measured from the first unsaved change, not the latest).

8. `App::mark_clean()` sets `dirty_since = None` **and**
   `last_synced_revision = history.revision()`.

9. **A direct `history.commit` dirties the document.** With `app` built by
   `App::default()`: `app.history.commit(Box::new(CreateLine::new(line)), &mut
   app.document)` followed by `app.sync_dirty()` leaves `app.dirty_since ==
   Some(_)`. This is the defect: before the fix it stays `None`.

10. **Undo dirties the document.** Commit, `sync_dirty`, `mark_clean`, then
    `app.history.undo(&mut app.document)`, then `sync_dirty` → `dirty_since` is
    `Some(_)` again. Same for `redo`.

11. **A no-op undo does not dirty the document.** On a clean `App::default()`,
    `app.history.undo(&mut app.document)` returns `false`; after `sync_dirty()`,
    `dirty_since` is still `None`.

12. **A fresh `History` does not re-dirty a just-loaded document.**
    `app.history = History::default(); app.mark_clean();` followed by
    `sync_dirty()` leaves `dirty_since == None` — the case that motivates
    `mark_clean` resyncing the revision (ADR 0002 §B).

13. `grep -rn "dirty_since = None" src/` returns exactly one line, inside
    `mark_clean`. `grep -rn "dirty_since" src/` shows no
    `get_or_insert_with` outside `sync_dirty`, and `App::commit` no longer
    mentions `dirty_since`.

14. `App::commit` still commits through the history stack and is otherwise
    unchanged; `TextTool` keeps working (its existing tests stay green).

15. `AUTOSAVE_DEBOUNCE == Duration::from_millis(800)`.

16. `pub fn autosave_due(dirty_since: Option<Instant>, now: Instant) -> bool`
    exists, is reachable as `lasercad::app::autosave_due`, and satisfies:
    - `autosave_due(None, now) == false`;
    - `autosave_due(Some(now - 799 ms), now) == false`;
    - `autosave_due(Some(now - 800 ms), now) == true` (boundary is `>=`);
    - `autosave_due(Some(now - 5 s), now) == true`.
    It takes no `&self`, performs no I/O, and no test of it sleeps.

17. The frame body calls `self.sync_dirty()` exactly once per frame, on a path
    that executes unconditionally (not inside `if response.hovered()` and not
    inside any dialog branch), immediately before the autosave-flush check —
    in `App::update` today, or in `App::update_ui` if LCV-103 has landed first.
    `grep -rn "sync_dirty" src/` returns its definition plus exactly one call
    site.

18. The flush branch calls `autosave_due(self.dirty_since, Instant::now())`,
    writes via `crate::io::save_autosave(&self.document)`, and then calls
    `self.mark_clean()` **regardless of the write result** (a failed write is
    dropped, not retried every frame; the next document change re-arms the
    debounce). The `Result` is explicitly discarded with `let _ = …` or matched.

19. **No test writes an autosave file.** No test lets the debounce elapse, calls
    `save_autosave`, or sleeps; assertions are on `dirty_since`,
    `History::revision()` and `autosave_due` only (ADR 0002 §A4 rule 2).
    `grep -rn "sleep" src/app.rs src/document/history.rs tests/` returns no new
    matches.

20. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`
    and `cargo test --all` all exit 0, with no regression in
    `tests/lcv070.rs`, `tests/app_tool.rs` or `src/io/file_actions.rs`'s tests.

## Expected tests

All tests are in-crate unit tests: `#[cfg(test)] mod tests` in
`src/document/history.rs` and in the file defining `App` (`sync_dirty` is
private, so integration tests under `tests/` cannot reach it).

- **Unit (AC 1)**: `revision_starts_at_zero` — `new`, `default`, `with_depth(3)`.
- **Unit (AC 2)**: `commit_increments_revision` — three commits → `3`; plus a
  `with_depth(2)` case proving eviction still increments.
- **Unit (AC 3)**: `undo_and_redo_increment_revision_only_when_work_is_done` —
  covers both the `true` and the `false` branch of each.
- **Unit (AC 4)**: `revision_is_monotonic_across_commit_undo_commit` — asserts
  strictly increasing values while `history.len()` goes 1 → 0 → 1.
- **Static check (AC 5)**: purity grep + `wc -l` minus the test module.
- **Unit (AC 6)**: `app_default_last_synced_revision_is_zero`.
- **Unit (AC 7)**: `sync_dirty_is_idempotent_without_mutation` — commit,
  `sync_dirty`, capture `dirty_since`, `sync_dirty` again, assert the instant is
  unchanged.
- **Unit (AC 8)**: `mark_clean_clears_and_resyncs` — after `mark_clean`,
  `dirty_since.is_none()` and a following `sync_dirty()` leaves it `None`.
- **Unit (AC 9)**: `direct_history_commit_marks_document_dirty` — the
  regression test for the defect; its doc comment names it
  ("tools bypass `App::commit`").
- **Unit (AC 10)**: `undo_marks_document_dirty` and `redo_marks_document_dirty`.
- **Unit (AC 11)**: `no_op_undo_does_not_dirty`.
- **Unit (AC 12)**: `replacing_history_then_mark_clean_stays_clean`.
- **Static check (AC 13, 17)**: the three greps.
- **Unit (AC 14)**: existing `TextTool` and `App::commit` tests run unmodified.
- **Unit (AC 15)**: `autosave_debounce_is_800ms` —
  `assert_eq!(AUTOSAVE_DEBOUNCE, Duration::from_millis(800))`.
- **Unit (AC 16)**: `autosave_due_boundaries` — the four cases, built with
  `Instant::now()` and `checked_sub`, no sleeping.
- **Static check (AC 18)**: reviewer reads the flush branch.
- **Static check (AC 19)**: the `sleep` grep; reviewer confirms no test drives a
  frame long enough to flush.
- **Build gate (AC 20)**: the three cargo commands.
- **Manual smoke**: launch the app, draw two lines with the LINE tool (which
  never calls `App::commit`), wait ~1 s, then kill the process
  (`pkill -9 lasercad`). Expect `~/.local/share/lasercad/autosave.json` to exist
  and to contain both lines; relaunch and confirm the drawing is restored.
  Repeat with a single Ctrl+Z before the kill and confirm the autosave reflects
  the undone state.

## Risks

- **`src/app.rs` grows past its already-over-cap size** (482 implementation
  lines today). Accepted for Marco 0: ADR 0002 assigns the split to LCV-105 and
  designs it so this demand's `sync_dirty` / `mark_clean` / `autosave_due` move
  into `src/app/autosave.rs` unchanged. Keep the addition minimal and self-
  contained so that move is mechanical.
- **Placement of the `sync_dirty()` call is the whole demand.** Put it inside a
  conditional branch (hover, dialog, panel) and autosave silently stays broken
  for exactly the sessions it matters in. AC 17 is the guard; the reviewer must
  read the call site, not just the grep count.
- **800 ms plus a per-frame `request_repaint`** means an autosave write roughly
  every 800 ms during continuous editing. Each write is one small JSON document
  via an atomic temp-and-rename; v1 shipped the same cadence. Accepted.
- **Merge overlap with LCV-103 / LCV-105**, both of which restructure the frame
  body. Whichever lands second must preserve the "once per frame,
  unconditionally, immediately before the flush" placement; call it out in the
  handoff.
- **A revision counter only proves "something changed", not "changed away from
  disk".** Undo-back-to-the-saved-state still marks the document dirty and
  triggers one autosave write. That is strictly safe (it never loses work) and
  cheaper than any content comparison.

## Open questions

*(none — demand is Ready)*

## Notes

- Authority: `docs/adr/0002-headless-input-tests-and-dirty-tracking.md` §B
  (decision, rejected alternatives, and the "Committed to" line
  "`History::revision()` is the document-dirty signal"). D3 in that ADR's
  Context is this defect.
- The five `app.dirty_since = None;` sites in `src/io/file_actions.rs` are in
  `action_new`, `action_open`, `action_save`, `action_open_path` and
  `action_save_as`. In the three that also replace `app.history` with a fresh
  `History`, `mark_clean()` must be called **after** the replacement, otherwise
  the resynced revision is the old one.
- `src/agent/tools.rs` commits with `&mut Document` + `&mut History` and no
  `App`. It needs no change: the counter lives on `History`, so agent-driven
  geometry is covered for free. No separate test is required for it.
- ADR 0002 §A4 rule 2 forbids tests that let the debounce elapse — a fired
  autosave writes to the developer's real
  `~/.local/share/lasercad/autosave.json`. That is why AC 16 exists in the shape
  it does: `autosave_due` takes `now` as a parameter precisely so the clock can
  be faked.
- ADR 0002 §"The 300-LOC cap" reading: inline `#[cfg(test)] mod tests` blocks do
  not count toward the 300-LOC cap.
- If LCV-103 lands first, the frame body is `App::update_ui` and a headless
  regression test for the end-to-end path becomes possible via
  `tests/harness/mod.rs`. It is **not** required here — do not add a dependency
  on LCV-103 to ship this demand.
