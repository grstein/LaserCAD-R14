# LCV-044 — PolylineTool

- **Status**: Ready
- **Phase**: 4
- **Depends on**: LCV-043 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

Users of AutoCAD R14 switch between `LINE` and `PLINE` depending on intent:
`LINE` produces independent disconnected segments; `PLINE` produces a connected
chain that reads as a single continuous path — a closed bracket, a rectangular
slot, a profile contour. Without `PLINE`, operators must consciously chain LINE
segments and accept that they cannot invoke the well-known `PLINE` alias.
LaserGRBL workflows for cutting closed profiles almost always originate from
`PLINE` muscle memory. This demand ships `PolylineTool` (`PLINE`) so the alias
and status-bar identity match AutoCAD R14, completing the two-command pair.

In v2, a "polyline" is not a distinct document entity: each segment is stored as
a separate `CreateLine` command, identical to `LineTool`. The only observable
difference is the tool name and status prompts. This preserves simplicity while
honouring the R14 UX contract.

## Scope

- **New file `src/tools/polyline.rs`** containing:
  - `pub struct PolylineTool` with the same two-variant private state machine as
    `LineTool`:
    - `Idle` — no point picked yet.
    - `WaitingSecondPoint { p1: Vec2, cursor: Vec2 }` — first point is fixed;
      `cursor` is the live preview endpoint, updated by `on_pointer_move`.
  - `impl Default for PolylineTool` — constructs in `Idle`.
  - `impl Tool for PolylineTool`:
    - `name() -> &'static str` — returns `"PLINE"`.
    - `status_text() -> &'static str`:
      - `Idle` → `"PLINE: Click to set start point"`.
      - `WaitingSecondPoint` → `"PLINE: Click to set next point  |  Enter/Esc to end"`.
    - `on_pointer_down(pos, _shift, doc, history)`:
      - `Idle` → transition to `WaitingSecondPoint { p1: pos, cursor: pos }`.
      - `WaitingSecondPoint { p1, .. }` → if `|pos − p1| > EPSILON`, commit
        `Box::new(CreateLine::new(Line::new(p1, pos)))` via
        `history.commit(cmd, doc)`, then transition to
        `WaitingSecondPoint { p1: pos, cursor: pos }` (chain). If
        `|pos − p1| ≤ EPSILON`, no-op (degenerate zero-length guard).
    - `on_pointer_move(pos, _doc)`:
      - `WaitingSecondPoint` → update `cursor = pos`.
      - `Idle` → no-op.
    - `on_pointer_up` — no-op (identical to `LineTool`).
    - `on_key(key, _app)`:
      - `Key::Enter` or `Key::Escape` in any state → `self.cancel()`.
      - All other keys → no-op.
    - `preview() -> Vec<Entity>`:
      - `WaitingSecondPoint { p1, cursor }` → if `|cursor − p1| > EPSILON`,
        returns `vec![Entity::Line(Line::new(p1, cursor))]`; otherwise `vec![]`.
      - `Idle` → returns `vec![]`.
    - `cancel(&mut self)` — resets state to `Idle`.
  - Module doc comment explaining the tool and noting it shares the commit
    pattern with `LineTool` (LCV-043).
  - `MUST NOT` import `eframe` or `rfd`. May import `egui` only for `egui::Key`.
  - File ≤ 200 LOC.

- **Update `src/tools/mod.rs`**:
  - Add `pub mod polyline;`.
  - Add `pub use polyline::PolylineTool;`.

## Out of scope

- **Polyline entity** — no `Entity::Polyline` variant; each segment is an
  independent `Entity::Line`. A compound polyline entity is an explicit non-goal
  for v0.1.0.
- **Close (`C`) option** — AutoCAD's `C` key in PLINE closes the chain to the
  start point. Not in this demand.
- **Width / half-width** — PLINE width properties are not in scope.
- **Arc segments within PLINE** — straight segments only; arc-in-polyline is not
  in scope.
- **Keyboard shortcut `PL` to activate PolylineTool** — belongs to LCV-070
  (command-line tool aliases).
- **Toolbar button** — belongs to LCV-066.
- **Ortho lock** — applied upstream (LCV-053); `PolylineTool` uses `pos`
  verbatim.
- **Snap masks / F3 toggle** — belongs to LCV-054; snap is always-on at the
  plumbing layer.
- **Command-line coordinate entry** — belongs to LCV-068.
- **Any change to `LineTool`** — LCV-043 is `Done`; this demand does not touch
  `src/tools/line.rs`.
- **Any change to `Tool` trait** — `status_text()` default already ships with
  LCV-043; no trait changes needed.

## Acceptance criteria

1. `src/tools/polyline.rs` exists. `PolylineTool` is a `pub struct` that
   implements `Tool`. `PolylineTool::default()` starts in `Idle` state.

2. `name()` returns the exact string `"PLINE"`.

3. `status_text()` returns `"PLINE: Click to set start point"` in `Idle` state,
   and `"PLINE: Click to set next point  |  Enter/Esc to end"` in
   `WaitingSecondPoint` state.

4. `preview()` returns an empty `Vec<Entity>` in `Idle` state.

5. `preview()` in `WaitingSecondPoint { p1, cursor }` where
   `|cursor − p1| > EPSILON` returns exactly
   `vec![Entity::Line(Line::new(p1, cursor))]` — endpoints are bit-exact to
   the stored `p1` and `cursor`.

6. `preview()` in `WaitingSecondPoint { p1, cursor }` where
   `|cursor − p1| ≤ EPSILON` returns `vec![]`.

7. `on_pointer_down` in `Idle` with any `pos` transitions to
   `WaitingSecondPoint`. `status_text()` returns the "next point" prompt
   immediately after.

8. `on_pointer_move` in `WaitingSecondPoint` updates the preview endpoint:
   after calling `on_pointer_move(new_pos, _)`, `preview()` returns
   `Entity::Line` whose second endpoint equals `new_pos`.

9. `on_pointer_down` in `WaitingSecondPoint { p1 }` with `p2` where
   `|p2 − p1| > EPSILON`:
   a. Calls `history.commit(Box::new(CreateLine::new(Line::new(p1, p2))), doc)`.
   b. The document gains exactly one new `Entity::Line(Line { p1, p2 })`.
   c. The tool remains in `WaitingSecondPoint` with the new `p1 = p2` (chaining).
   d. `history.can_undo()` returns `true` after the commit.

10. `on_pointer_down` in `WaitingSecondPoint { p1 }` with `p2` where
    `|p2 − p1| ≤ EPSILON`: no entity is added, no history entry is pushed, the
    state remains `WaitingSecondPoint`.

11. Chaining: three successive `on_pointer_down` calls on a fresh tool at
    `p1`, `p2`, `p3` (all distinct beyond `EPSILON`) produce exactly two
    committed `Entity::Line` entries in `doc.entities`: `Line(p1, p2)` then
    `Line(p2, p3)`.

12. `on_key(Key::Enter, _app)` in `WaitingSecondPoint` transitions the tool to
    `Idle` without adding any entity. `preview()` returns `vec![]` immediately
    after.

13. `on_key(Key::Escape, _app)` in `WaitingSecondPoint` transitions the tool to
    `Idle` without adding any entity. `preview()` returns `vec![]` immediately
    after.

14. `on_key(Key::Escape, _app)` in `Idle` is a no-op: no panic, tool remains
    `Idle`, `preview()` is still empty.

15. `cancel()` in `WaitingSecondPoint` transitions to `Idle`; `preview()`
    returns `vec![]` afterwards.

16. `PolylineTool` is object-safe:
    `let _: Box<dyn Tool> = Box::new(PolylineTool::default());` compiles.

17. `src/tools/mod.rs` re-exports `PolylineTool` as `pub use polyline::PolylineTool`.

18. Kernel-purity: `grep -nE '^use (eframe|rfd)' src/tools/polyline.rs` returns
    no matches.

19. `wc -l src/tools/polyline.rs` ≤ 200.

20. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

All tests live in `src/tools/polyline.rs` inside `#[cfg(test)] mod tests`.

- **(AC 1, 4)** `name_and_idle_status` — constructs `PolylineTool::default()`,
  asserts `name() == "PLINE"` and `preview().is_empty()`.

- **(AC 3)** `status_text_idle` — fresh tool: `status_text()` contains
  `"start"` (matches `"PLINE: Click to set start point"`).

- **(AC 3)** `status_text_waiting` — after `on_pointer_down(p1, …)`:
  `status_text()` contains `"next"`.

- **(AC 5, 6, 7)** `first_click_transitions_to_waiting` — call
  `on_pointer_down(Vec2::new(1.0, 2.0), false, …)`, assert
  `status_text()` contains `"next"` and `preview().is_empty()` (cursor == p1,
  degenerate segment suppressed).

- **(AC 5, 8)** `preview_rubber_band_after_move` — after first click at
  `(0.0, 0.0)` and `on_pointer_move(Vec2::new(10.0, 0.0), …)`, assert
  `preview().len() == 1` and the returned `Entity::Line` has `p1 = (0,0)`,
  `p2 = (10,0)`.

- **(AC 10)** `degenerate_second_click_ignored` — call `on_pointer_down(p, …)`
  twice at the same point; assert `doc.entity_count() == 0` and
  `!history.can_undo()`.

- **(AC 9 a–d)** `second_click_commits_and_chains` — click at `(0,0)` then
  `(10,0)`; assert `doc.entity_count() == 1`, entity is `Entity::Line` with
  correct coordinates, `history.can_undo() == true`, and tool is still in
  waiting state (`status_text()` contains `"next"`).

- **(AC 11)** `chain_draws_second_segment` — clicks at `(0,0)`, `(10,0)`,
  `(10,10)`; assert `doc.entity_count() == 2`.

- **(AC 12)** `enter_key_ends_chain_without_commit` — after first click, call
  `on_key(Key::Enter, &mut app)`; assert `doc.entity_count() == 0` and
  `preview().is_empty()`.

- **(AC 13)** `escape_key_cancels_in_waiting_state` — after first click, call
  `on_key(Key::Escape, &mut app)`; assert `doc.entity_count() == 0` and
  `preview().is_empty()`.

- **(AC 14)** `escape_in_idle_is_noop` — fresh tool, call
  `on_key(Key::Escape, &mut app)`; no panic, `preview().is_empty()`.

- **(AC 15)** `cancel_resets_to_idle` — after first click, call `cancel()`;
  assert `preview().is_empty()`.

- **(AC 16)** `object_safe_and_idle_cancel_noop` — fresh tool: `cancel()` is a
  no-op; `let _: Box<dyn Tool> = Box::new(PolylineTool::new());` compiles.

## Open questions

*(none)*

## Notes

- **Structural parity with `LineTool`**: `PolylineTool` is an intentional clone
  of `LineTool` (LCV-043) with `name()` returning `"PLINE"` and status prompts
  updated accordingly. No logic divergence is introduced. This is by design:
  v2 does not have a compound polyline entity; the chain-of-`CreateLine` model
  is the KISS choice.

- **Preview suppresses degenerate segment**: `LineTool` in the shipped code
  suppresses the preview when `|cursor − p1| ≤ EPSILON` (see AC 6). Match that
  behaviour exactly.

- **`on_pointer_up` is a no-op**: commit happens on `on_pointer_down`, matching
  AutoCAD R14 and `LineTool`.

- **Chain endpoint precision**: the new `p1` stored after a commit is exactly
  `p2` (the committed endpoint), not a re-snapped value. Prevents floating-point
  drift across long chains.

- **`preview()` ownership**: returns an owned `Vec<Entity>` constructed fresh
  each call from the current state. This matches the LCV-037 contract (ephemeral,
  per-frame, no stored handle).

- **Committed segments are already in the document**: the render pipeline paints
  all document entities every frame, so previously committed segments of the
  chain appear automatically without any "ghost" logic in the tool.

- **File placement**: `src/tools/polyline.rs`. Register it in `src/tools/mod.rs`
  exactly as `LineTool` is registered.

- **LOC budget**: with the identical state machine, tests, and doc comments,
  the file should sit well within the 200-LOC cap.
