# LCV-043 — LineTool

- **Status**: Ready
- **Phase**: 4
- **Depends on**: LCV-041 (Done), LCV-023 (Done), LCV-037 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

After LCV-040 (Tool trait), LCV-041 (pointer plumbing + snap), and LCV-023
(CreateLine command), all the infrastructure is live. The operator cannot yet
draw a single line. Without LineTool, a laser-cutting session cannot start:
every workflow — box, bracket, fixture — opens with the LINE command.

This demand ships `LineTool` in `src/tools/line.rs`: a state-machine
implementation of AutoCAD R14's `LINE` command. The operator clicks a first
point, a rubber-band preview follows the cursor to a second click, the segment
is committed to history, and the command automatically chains for the next
segment. Enter or Escape terminates the chain cleanly. Snap coordinates
(already resolved by LCV-041) are used verbatim — no snap logic inside the
tool itself.

A `status_text()` method is also added to the `Tool` trait (with a default
implementation that returns `name()`) so LineTool can surface context-sensitive
hints without breaking existing tools.

## Scope

- **New file `src/tools/line.rs`** containing:
  - `pub struct LineTool` with a two-variant private state machine:
    - `Idle` — no point picked yet.
    - `WaitingSecondPoint { p1: Vec2, cursor: Vec2 }` — first point is fixed;
      `cursor` is the live preview endpoint, updated by `on_pointer_move`.
  - `impl Default for LineTool` — constructs in `Idle`.
  - `impl Tool for LineTool`:
    - `name() -> &'static str` — returns `"LINE"`.
    - `status_text() -> &'static str` — returns `"LINE: Specify first point"`
      in `Idle`; `"LINE: Specify next point or [Enter] to end"` in
      `WaitingSecondPoint`.
    - `on_pointer_down(pos, doc, history)`:
      - `Idle` → transition to `WaitingSecondPoint { p1: pos, cursor: pos }`.
      - `WaitingSecondPoint { p1, .. }` → if `|pos − p1| > EPSILON`, commit
        `Box::new(CreateLine::new(Line::new(p1, pos)))` via
        `history.commit(cmd, doc)`, then transition to
        `WaitingSecondPoint { p1: pos, cursor: pos }` (chain — next segment
        starts from the committed endpoint). If `|pos − p1| ≤ EPSILON`,
        no-op (degenerate zero-length line guard).
    - `on_pointer_move(pos, _doc)`:
      - `WaitingSecondPoint` → update `cursor = pos`.
      - `Idle` → no-op.
    - `on_pointer_up` — no-op (commit happens on `on_pointer_down`).
    - `on_key(key, app)`:
      - `Key::Enter` in `WaitingSecondPoint` → `self.cancel()` (end chain,
        no commit; last segment was already committed by the preceding click).
      - `Key::Escape` in any state → `self.cancel()`.
      - All other keys → no-op.
    - `preview() -> Vec<Entity>`:
      - `WaitingSecondPoint { p1, cursor }` → returns
        `vec![Entity::Line(Line::new(p1, cursor))]`.
      - `Idle` → returns `vec![]`.
    - `cancel(&mut self)` — resets state to `Idle`.
  - Module doc comment: "LineTool — AutoCAD R14 LINE command. State machine:
    Idle → WaitingSecondPoint. Commits via history.commit; never mutates doc
    directly."
  - `MUST NOT` import `eframe` or `rfd`. May import `egui` only for
    `egui::Key`.
  - File ≤ 200 LOC.

- **Update `src/tools/tool.rs`** — add one default method to `pub trait Tool`:
  ```rust
  fn status_text(&self) -> &'static str {
      self.name()
  }
  ```
  Placement: immediately after `name()`. Object-safety is preserved: the
  method has no generic parameters, takes only `&self`, and returns
  `&'static str`. No existing `Tool` implementor needs to change.

- **Update `src/tools/mod.rs`**:
  - Add `pub mod line;`.
  - Add `pub use line::LineTool;`.

## Out of scope

- **PolylineTool (LCV-044)** — LineTool commits individual disconnected
  segments; grouping into a polyline entity is LCV-044.
- **Ortho lock** (LCV-053) — LineTool uses `world_pos` from `PointerEvent`
  verbatim; ortho clamping is applied upstream in LCV-053.
- **Snap toggle / per-tool snap masks** (LCV-054) — snap is always-on at the
  plumbing layer (LCV-041); the F3 toggle and snap kind masks belong to
  LCV-054.
- **Command-line coordinate entry** (e.g. `10,20` or `@5<45`) — that is the
  command-line widget (LCV-068) responsibility. LineTool only accepts pointer
  input in this demand.
- **Keyboard shortcut `L` to activate LineTool** — LCV-070.
- **Toolbar button for LineTool** — LCV-066.
- **Right-click to end chain** — v2 uses Enter/Escape only; no right-click
  cancel in Phase 4.
- **Undo mid-chain** (Ctrl+Z while chaining) — history undo/redo (LCV-026)
  is separate. LineTool does not catch `Key::Z`; Ctrl+Z is handled at the App
  level and falls through to the history stack naturally.
- **Zero-length line as error** — degenerate clicks are silently ignored; no
  error dialog or status message.

## Acceptance criteria

1. `src/tools/line.rs` exists. `LineTool` is a `pub struct` that implements
   `Tool`. `LineTool::default()` starts in `Idle` state.

2. `name()` returns the exact string `"LINE"`.

3. `status_text()` returns `"LINE: Specify first point"` when the tool is in
   `Idle` state, and `"LINE: Specify next point or [Enter] to end"` when in
   `WaitingSecondPoint` state.

4. `preview()` returns an empty `Vec<Entity>` in `Idle` state.

5. `preview()` in `WaitingSecondPoint { p1, cursor }` returns exactly
   `vec![Entity::Line(Line::new(p1, cursor))]` — the entity is bit-exact over
   the stored `p1` and `cursor` values.

6. `on_pointer_down` in `Idle` with any `pos` transitions the tool to
   `WaitingSecondPoint`. After the call, `preview()` returns a single
   `Entity::Line` whose both endpoints equal `pos` (degenerate segment at the
   first click, before the cursor has moved).

7. `on_pointer_move` in `WaitingSecondPoint` updates the preview endpoint:
   after calling `on_pointer_move(new_pos, _)`, `preview()` returns
   `Entity::Line` whose second endpoint equals `new_pos`.

8. `on_pointer_down` in `WaitingSecondPoint { p1 }` with `p2` where
   `|p2 − p1| > EPSILON`:
   a. Calls `history.commit(Box::new(CreateLine::new(Line::new(p1, p2))), doc)`.
   b. The document gains exactly one new `Entity::Line(Line { p1, p2 })`.
   c. The tool stays in `WaitingSecondPoint` with `p1 = p2` (chaining).
   d. `history.can_undo()` returns `true` after the commit.

9. `on_pointer_down` in `WaitingSecondPoint { p1 }` with `p2` where
   `|p2 − p1| ≤ EPSILON` (degenerate click): no entity is added to the
   document, no history entry is pushed, the state remains `WaitingSecondPoint`.

10. Chaining: three successive `on_pointer_down` calls on a fresh tool at
    `p1`, `p2`, `p3` (all distinct beyond `EPSILON`) produce exactly two
    committed `Entity::Line` entries in `doc.entities`: `Line(p1, p2)` and
    `Line(p2, p3)`.

11. `on_key(Key::Enter, _app)` in `WaitingSecondPoint` transitions the tool
    to `Idle` without adding any entity to the document. `preview()` returns
    `vec![]` immediately after.

12. `on_key(Key::Escape, _app)` in `WaitingSecondPoint` transitions the tool
    to `Idle` without adding any entity. `preview()` returns `vec![]`
    immediately after.

13. `on_key(Key::Escape, _app)` in `Idle` is a no-op (no panic, tool remains
    `Idle`).

14. `cancel()` in `WaitingSecondPoint` transitions to `Idle`; `preview()`
    returns `vec![]` afterwards.

15. `src/tools/tool.rs` has a new default method `fn status_text(&self) ->
    &'static str` that returns `self.name()`. A unit test confirms
    `SelectTool.status_text() == "Select"` without any explicit override on
    `SelectTool`.

16. `LineTool` is object-safe: `let _: Box<dyn Tool> = Box::new(LineTool::default());`
    compiles.

17. `src/tools/mod.rs` re-exports `LineTool` as `pub use line::LineTool`.

18. Kernel-purity: `grep -nE '^use (eframe|rfd)' src/tools/line.rs` returns
    no matches.

19. `wc -l src/tools/line.rs` ≤ 200.

20. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

All tests live in `src/tools/line.rs` inside `#[cfg(test)] mod tests`.

- **(AC 1, 4)** `line_tool_default_is_idle_preview_empty` — constructs
  `LineTool::default()`, asserts `preview().is_empty()`.

- **(AC 2)** `line_tool_name_exact` — `assert_eq!(LineTool::default().name(), "LINE")`.

- **(AC 3)** `status_text_idle` — `assert_eq!(tool.status_text(), "LINE: Specify first point")`.

- **(AC 3)** `status_text_waiting` — call `on_pointer_down(p1, &mut doc, &mut hist)`,
  then `assert_eq!(tool.status_text(), "LINE: Specify next point or [Enter] to end")`.

- **(AC 5, 6)** `preview_after_first_click_is_degenerate_segment` — after
  `on_pointer_down(Vec2::new(10.0, 20.0), ...)`, assert `preview()` returns
  one `Entity::Line` where both `p1` and `p2` equal `Vec2::new(10.0, 20.0)`.

- **(AC 7)** `preview_updates_on_pointer_move` — after first click at
  `(0.0, 0.0)`, call `on_pointer_move(Vec2::new(5.0, 5.0), &mut doc)`, assert
  preview line's second endpoint is `Vec2::new(5.0, 5.0)`.

- **(AC 8 a–d)** `pointer_down_commits_line_and_chains` — fresh tool, call
  `on_pointer_down(Vec2::new(0.0, 0.0), ...)` then
  `on_pointer_down(Vec2::new(10.0, 0.0), ...)`. Assert:
  `doc.entities.len() == 1`, entity is `Entity::Line` with `p1 = (0,0)` and
  `p2 = (10,0)`, `history.can_undo() == true`, tool still in
  `WaitingSecondPoint` (preview is non-empty).

- **(AC 9)** `degenerate_click_is_ignored` — call `on_pointer_down(p, ...)` twice
  at the same point; assert `doc.entities.is_empty()` and
  `history.can_undo() == false`.

- **(AC 10)** `three_clicks_produce_two_segments` — click at `(0,0)`, `(5,0)`,
  `(5,5)`. Assert `doc.entities.len() == 2`, verify both segments by coordinate.

- **(AC 11)** `enter_key_ends_chain_without_commit` — after first click, call
  `on_key(Key::Enter, &mut app)`. Assert `doc.entities.is_empty()`,
  `preview().is_empty()`.

- **(AC 12)** `escape_key_cancels_in_waiting_state` — after first click, call
  `on_key(Key::Escape, &mut app)`. Assert `doc.entities.is_empty()`,
  `preview().is_empty()`.

- **(AC 13)** `escape_in_idle_is_noop` — fresh tool, call
  `on_key(Key::Escape, &mut app)`. No panic, `preview().is_empty()`.

- **(AC 14)** `cancel_resets_to_idle` — after first click, call `cancel()`.
  Assert `preview().is_empty()`.

- **(AC 15)** `select_tool_status_text_falls_back_to_name` — in
  `src/tools/select.rs` tests: `assert_eq!(SelectTool.status_text(), "Select")`.

- **(AC 16)** `line_tool_is_object_safe` —
  `let _: Box<dyn Tool> = Box::new(LineTool::default());`.

## Open questions

*(none)*

## Notes

- **Committing via `history.commit`**: `on_pointer_down` receives
  `&mut Document` and `&mut History`. The tool commits with
  `history.commit(Box::new(CreateLine::new(Line::new(p1, p2))), doc)` — not
  via `App::commit`. Only `on_key` has access to `&mut App`; Enter/Escape
  require no commit so `App::commit` is not needed here.

- **`on_pointer_up` is a no-op**: AutoCAD R14 LINE commits on mouse-down, not
  on mouse-up. The press-release cycle is: down commits + chains, up is
  ignored. This avoids accidental double-commits on slow clicks.

- **`EPSILON`**: use `crate::geometry::EPSILON` (the same constant used
  throughout the kernel) as the degenerate-click threshold. A zero-length line
  is geometrically meaningless and would produce a zero-length path in SVG
  export.

- **`status_text` default**: the new default method on `Tool` reads
  `self.name()`. Because `name()` is object-dispatched at runtime, this is
  safe. No change to `SelectTool` or any other existing tool is required; they
  inherit the default.

- **Preview entity ownership**: `preview()` returns an owned `Vec<Entity>`
  constructed fresh each call from the current state. This matches the LCV-037
  contract (ephemeral, per-frame, no stored handle).

- **Chain endpoint precision**: the new `p1` stored after a commit is exactly
  `p2` (the committed endpoint), not a re-snapped value. This prevents
  accumulating floating-point drift across long chains.

- **v1 reference**: v1's `LineTool.ts` followed the same state machine:
  `idle → awaitingSecondPoint → (commit + loop)`. Enter/Escape called
  `deactivate()`, which mapped to `cancel()` here.

- **File placement**: `src/tools/line.rs`. The module tree in `AGENTS.md`
  already lists `src/tools/line.rs` in the planned module tree.
