# LCV-048 — TextTool (Hershey-based)

- **Status**: Done
- **Implementation**: b59aa63 — feat(LCV-048): TextTool — click-to-place Hershey stroke text
- **Phase**: 4
- **Depends on**: LCV-055 (Done), LCV-043 (Done), LCV-040 (Done), LCV-041 (Done), LCV-037 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

LaserCAD v2 targets engraving workflows as a first-class use case: operators
need to place labels, part numbers, and dimension annotations directly on the
laser bed. Without a TextTool, every string must come from SVG import or the
agent harness — neither suitable for rapid manual annotation during a cutting
session.

LCV-055 shipped `layout_text(text, origin, height_mm, spacing_factor) ->
Vec<Entity>`, which renders any ASCII string as a flat list of `Entity::Line`
segments in world-space millimeters (Hershey Simplex Roman strokes). This
demand wires that kernel function into an interactive tool: the operator clicks
an anchor point, types the desired string, and the live Hershey strokes preview
updates character-by-character at the anchor. Pressing Enter commits all strokes
as a single undoable history entry.

Text is always exploded to `Entity::Line` segments before it touches the
document — no `TextEntity` variant is introduced, and the SVG exporter emits
only `<line>` elements (no `<text>`). This preserves the LaserGRBL SVG
contract (AGENTS.md §SVG export: "no live text").

## Scope

### New command: `CreateEntities` in `src/document/commands/create.rs`

`pub struct CreateEntities` — batch-insert a `Vec<Entity>` as one undoable
history entry:

- Fields (all private except through `new`):
  - `entities: Vec<Entity>` — the entities to insert.
  - `insert_at: Option<usize>` — index at which `do_` appended them; `None`
    before the first `do_` or after `undo`.
- `pub fn new(entities: Vec<Entity>) -> Self` — constructs with
  `insert_at: None`.
- `impl Command for CreateEntities`:
  - `do_`: record `insert_at = Some(doc.entities.len())`, then
    `doc.entities.extend(self.entities.iter().cloned())`.
  - `undo`: if `Some(start) = self.insert_at.take()`, call
    `doc.entities.drain(start .. start + self.entities.len())` to restore the
    pre-`do_` state exactly. A double-`undo` (when `insert_at` is already
    `None`) is a no-op — no panic, no corruption.
  - `label`: returns `"Place Text"`.
- Add `pub use create::CreateEntities;` to `src/document/commands/mod.rs`.

### New default method on `pub trait Tool` in `src/tools/tool.rs`

```rust
/// Character input from the keyboard (e.g. letter/digit typed by the
/// operator while a text tool is active). The default implementation is a
/// no-op; override in tools that accept typed input.
fn on_text_input(&mut self, _ch: char) {}
```

Placement: immediately after `on_key`. The method is object-safe: no generic
parameters, takes only `&mut self` and a `char`, returns `()`. No existing
`Tool` implementor needs to change.

### New method on `ToolManager` in `src/tools/manager.rs`

```rust
pub fn on_text_input(&mut self, ch: char) {
    self.active.on_text_input(ch);
}
```

No `&mut App` borrow needed; the method does not trigger the `std::mem::take`
dance used by `handle_key`.

### Character-input routing in `src/app.rs`

Inside `App::update`, after the existing `Backspace` key routing block, add:

```rust
// Character input → active tool (LCV-048 TextTool).
let typed_chars: Vec<char> = ctx.input(|i| {
    i.events
        .iter()
        .filter_map(|e| {
            if let egui::Event::Text(t) = e {
                Some(t.chars().collect::<Vec<char>>())
            } else {
                None
            }
        })
        .flatten()
        .collect()
});
for ch in typed_chars {
    self.tool_manager.on_text_input(ch);
}
```

This routing fires every frame that the egui context receives `Event::Text`
events (i.e., printable character presses). No `std::mem::take` is needed
because `on_text_input` does not borrow `App`.

### New file: `src/tools/text.rs`

`pub struct TextTool` — interactive text placement tool.

**Fields** (all private):
- `state: TextToolState` — the state machine variant (see below).
- `height_mm: f64` — cap height for the placed text in millimeters. Default:
  `5.0`. Constant `DEFAULT_TEXT_HEIGHT_MM: f64 = 5.0` declared at module level.
- `spacing_factor: f64` — passed verbatim to `layout_text`. Default: `1.0`.
  Constant `DEFAULT_SPACING_FACTOR: f64 = 1.0` at module level.

**Private state enum `TextToolState`** (derive `Debug`, `Clone`):
```
Idle
WaitingInput { anchor: Vec2, text: String }
```
- `Idle`: No anchor is set. The tool is ready for the first click.
- `WaitingInput { anchor, text }`: Anchor is fixed at `anchor` (world mm).
  `text` grows as the operator types. `preview()` returns live Hershey strokes
  at `anchor`.

**`impl Default for TextTool`**: constructs in `Idle`, `height_mm =
DEFAULT_TEXT_HEIGHT_MM`, `spacing_factor = DEFAULT_SPACING_FACTOR`.

**`impl Tool for TextTool`**:

- `name() -> &'static str`: returns `"TEXT"`.

- `status_text() -> &'static str`:
  - `Idle` → `"TEXT: Click to set insertion point"`.
  - `WaitingInput` → `"TEXT: Type text, Enter to confirm, Escape to cancel"`.

- `on_pointer_down(pos, _shift, _doc, _history)`:
  - `Idle` → transition to `WaitingInput { anchor: pos, text: String::new() }`.
    No entity is committed; the click only fixes the anchor.
  - `WaitingInput` → no-op (anchor is already fixed; a second click does not
    move the anchor and does not commit).

- `on_pointer_move(_pos, _doc)` — no-op in all states. Text position is fixed
  at the anchor set by the first click; cursor movement does not affect it.

- `on_pointer_up` — no-op.

- `on_text_input(ch)`:
  - `WaitingInput { text, .. }` → push `ch` onto `text`. Printable ASCII (space
    through `~`) is accepted; `layout_text` silently drops unsupported code
    points, so any `char` is safe to accumulate.
  - `Idle` → no-op.

- `on_key(key, app)`:
  - `Key::Backspace` in `WaitingInput { text, .. }` → pop the last Unicode
    scalar from `text` via `text.pop()` (correct for multi-byte UTF-8;
    acceptable since only ASCII is laid out). Stays in `WaitingInput`.
  - `Key::Backspace` in `Idle` → no-op.
  - `Key::Enter` in `WaitingInput { anchor, text }`:
    - If `text.is_empty()`: transition to `Idle` (nothing to commit).
    - If `text` is non-empty: call
      `let lines = crate::text::layout_text(&text, anchor, self.height_mm, self.spacing_factor);`
      then `app.commit(Box::new(CreateEntities::new(lines)));` then
      transition to `Idle`.
  - `Key::Enter` in `Idle` → no-op.
  - `Key::Escape` in any state → `self.cancel()`.
  - All other keys → no-op.

- `preview() -> Vec<Entity>`:
  - `Idle` → `vec![]`.
  - `WaitingInput { anchor, text }` if `text.is_empty()` → `vec![]`.
  - `WaitingInput { anchor, text }` if non-empty → returns
    `crate::text::layout_text(text, *anchor, self.height_mm, self.spacing_factor)`.
    Recomputed fresh each call.

- `cancel(&mut self)`: resets `state` to `Idle`. No document mutation. Safe
  to call multiple times (idempotent on `Idle`).

**Module doc comment**: "TextTool — AutoCAD R14 TEXT command. Click to set
anchor, type to build the string, Enter to commit as `Entity::Line` strokes.
Uses `layout_text` (LCV-055). Never mutates the document directly."

**Must NOT** import `eframe` or `rfd`. May import `egui` only for `egui::Key`.
File ≤ 300 LOC.

### Update `src/tools/mod.rs`

- Add `pub mod text;`.
- Add `pub use text::TextTool;`.

## Out of scope

- **Multi-line text**: `\n` in the pending string is silently accepted by
  `layout_text` as an unsupported character (advances cursor, no strokes). True
  multi-line layout is a future demand.
- **Moving the anchor after the first click**: second pointer click in
  `WaitingInput` is a no-op. Re-anchoring requires Escape and a new click.
- **Right-to-left / non-ASCII rendering**: only Simplex Roman ASCII 32–126.
- **Font or size selection UI**: `height_mm` and `spacing_factor` are
  constants in this demand. A settings-driven font panel is Phase 6+.
- **Command-line coordinate entry for the anchor point** (`10,20`): LCV-068.
- **Keyboard shortcut `T` to activate TextTool**: LCV-070.
- **Toolbar button for TextTool**: LCV-066.
- **Arrow-key cursor movement inside the text buffer** (caret navigation):
  append-and-backspace only in this demand.
- **Copy / paste into the text buffer**: out of scope.
- **A `TextEntity` variant in `Entity`**: text is always exploded to
  `Entity::Line` by `layout_text` before touching the document.
- **SVG `<text>` elements**: the exporter emits only `<line>` elements;
  text strokes are just lines.
- **Zero-height or negative-height guard**: `layout_text` already returns an
  empty `Vec` for non-positive `height_mm`. TextTool stores a positive constant
  and never passes ≤ 0.
- **Undo within the text buffer** (Ctrl+Z mid-type): history undo (LCV-026) is
  handled at the App level and reverts whole committed commands, not individual
  keystrokes.

## Acceptance criteria

1. `src/document/commands/create.rs` defines `pub struct CreateEntities`.
   `CreateEntities::new(entities)` constructs with `insert_at: None`.
   `src/document/commands/mod.rs` re-exports it as
   `pub use create::CreateEntities;` so
   `crate::document::commands::CreateEntities` resolves.

2. `CreateEntities` round-trip: starting from `Document::default()`, call
   `cmd.do_(&mut doc)` with a two-entity vec (`Entity::Line(l1)`,
   `Entity::Line(l2)`); assert `doc.entities.len() == 2` and both entities
   match. Call `cmd.undo(&mut doc)`; assert `doc.entities.is_empty()`.

3. `CreateEntities` does not remove pre-existing entities on undo: seed the
   document with one entity, `do_` with two new entities, `undo`. Assert
   exactly the original one entity remains at index 0.

4. Double-`undo` on a `CreateEntities` that has already been undone is a no-op:
   no panic, document unchanged.

5. `CreateEntities::label()` returns the exact string `"Place Text"`.

6. `CreateEntities` is object-safe: `let _: Box<dyn Command> =
   Box::new(CreateEntities::new(vec![]));` compiles.

7. `src/tools/tool.rs` has a new default method
   `fn on_text_input(&mut self, _ch: char) {}` on `pub trait Tool`.
   A unit test confirms `SelectTool::default().on_text_input('x')` does not
   panic (no-op via default).

8. `src/tools/manager.rs` has a method
   `pub fn on_text_input(&mut self, ch: char)`. Calling it on a
   `ToolManager::default()` with any char does not panic (SelectTool
   no-ops it).

9. `src/tools/text.rs` exists. `TextTool` is a `pub struct` that implements
   `crate::tools::Tool`. `TextTool::default()` starts in `Idle`.

10. `name()` returns the exact string `"TEXT"`.

11. `status_text()` returns `"TEXT: Click to set insertion point"` in `Idle`,
    and `"TEXT: Type text, Enter to confirm, Escape to cancel"` in
    `WaitingInput`.

12. `preview()` returns an empty `Vec<Entity>` in `Idle`.

13. `on_pointer_down(pos, false, &mut doc, &mut hist)` in `Idle` transitions to
    `WaitingInput`. After the call, `preview()` returns an empty `Vec<Entity>`
    (text buffer is still empty at this point — no strokes before any character
    is typed).

14. `on_text_input('H')` in `WaitingInput` causes `preview()` to return at
    least one `Entity::Line`. Every entity in the preview is `Entity::Line(_)`.
    All preview endpoint Y values satisfy `y >= anchor.y - EPSILON` (Hershey
    text ascends above the baseline).

15. `on_text_input` appends characters: after typing `'A'`, `'B'`, `'C'` in
    `WaitingInput`, `preview()` returns strictly more lines than after typing
    only `'A'`.

16. `on_key(Key::Backspace, &mut app)` in `WaitingInput` with one character
    typed: removes that character; `preview()` returns `vec![]` (buffer now
    empty).

17. `on_key(Key::Backspace, &mut app)` in `WaitingInput` with an empty buffer:
    no panic, `preview()` stays empty.

18. `on_key(Key::Backspace, &mut app)` in `Idle`: no panic.

19. `on_key(Key::Enter, &mut app)` in `WaitingInput` after typing at least one
    character (anchor at `Vec2::new(0.0, 0.0)`, text `"H"`):
    a. `app.document.entity_count() > 0` — at least one `Entity::Line` was
       committed.
    b. Every committed entity is `Entity::Line(_)`.
    c. `app.history.can_undo() == true`.
    d. `preview()` returns `vec![]` (tool is back in `Idle`).

20. `on_key(Key::Enter, &mut app)` in `WaitingInput` with an empty buffer:
    no entity is added to the document, tool returns to `Idle`, no panic.

21. `on_key(Key::Enter, &mut app)` in `Idle`: no-op, no panic.

22. `on_key(Key::Enter, &mut app)` in `WaitingInput` with text `"H"` commits
    **one** history entry (one `CreateEntities` command): `app.history.len()`
    increases by exactly 1. `app.history.undo(&mut app.document)` removes all
    the committed lines in a single undo step, restoring
    `app.document.entity_count() == 0`.

23. `on_key(Key::Escape, &mut app)` in `WaitingInput` after typing: no entity
    is committed, `preview()` returns `vec![]`, tool is in `Idle`.

24. `on_key(Key::Escape, &mut app)` in `Idle`: no-op, no panic.

25. `cancel()` in `WaitingInput` transitions to `Idle`; `preview()` returns
    `vec![]` afterwards.

26. `cancel()` in `Idle` is idempotent: no panic.

27. `on_pointer_down` in `WaitingInput` is a no-op: anchor does not move,
    buffer is unchanged, no entity committed.

28. `on_pointer_move` in any state is a no-op: no document mutation, no state
    change, no panic.

29. `on_pointer_up` in any state is a no-op.

30. `TextTool` is object-safe:
    `let _: Box<dyn Tool> = Box::new(TextTool::default());` compiles.

31. `src/tools/mod.rs` re-exports `TextTool` as `pub use text::TextTool`.

32. Kernel-purity: `grep -nE '^use (eframe|rfd)' src/tools/text.rs` returns no
    matches.

33. Size: `wc -l src/tools/text.rs` ≤ 300.

34. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

All tool tests live in `#[cfg(test)] mod tests` inside `src/tools/text.rs`.
Command tests live in `#[cfg(test)] mod tests` in
`src/document/commands/create.rs` (alongside the existing `CreateLine` tests).

### `CreateEntities` command (in `create.rs`)

- **(AC 1)**: `create_entities_constructor_reachable` — constructs a
  `CreateEntities::new(vec![])` and a `CreateEntities::new(vec![Entity::Line(...)])`.
- **(AC 2)**: `create_entities_roundtrip_two_lines` — `do_` then `undo` on
  empty document; assert entity count returns to 0 and both entities were
  present after `do_`.
- **(AC 3)**: `create_entities_undo_does_not_remove_preexisting` — seed doc
  with one line, `do_` adds two more, `undo`; assert only the original line
  remains at index 0.
- **(AC 4)**: `create_entities_double_undo_is_noop` — `do_`, `undo`, `undo`
  again; assert no panic and entity count equals the pre-`do_` count.
- **(AC 5)**: `create_entities_label_exact` — `assert_eq!(cmd.label(), "Place Text")`.
- **(AC 6)**: `create_entities_object_safe` — `let _: Box<dyn Command> = Box::new(CreateEntities::new(vec![]));`.

### `Tool` trait + `ToolManager` (in respective files)

- **(AC 7)**: `select_tool_on_text_input_is_noop` — in `src/tools/select/mod.rs`
  (or the select test module): `SelectTool::default().on_text_input('x')` does
  not panic; the method is inherited from the default.
- **(AC 8)**: `tool_manager_on_text_input_noop_for_select` — construct
  `ToolManager::default()`, call `on_text_input('a')`; no panic.

### `TextTool` state machine (in `src/tools/text.rs`)

- **(AC 9, 12)**: `text_tool_default_is_idle_preview_empty` — construct
  `TextTool::default()`, assert `name() == "TEXT"`, assert `preview().is_empty()`.

- **(AC 11)**: `status_text_idle` —
  `assert_eq!(TextTool::default().status_text(), "TEXT: Click to set insertion point")`.

- **(AC 11)**: `status_text_waiting_input` — after `on_pointer_down` with any
  position, assert `status_text() == "TEXT: Type text, Enter to confirm, Escape to cancel"`.

- **(AC 13)**: `first_click_anchors_and_preview_still_empty` — call
  `on_pointer_down(Vec2::new(10.0, 5.0), false, &mut doc, &mut hist)`; assert
  `preview().is_empty()` (buffer is empty before typing).

- **(AC 14)**: `typing_H_produces_line_preview` — after first click, call
  `on_text_input('H')`; assert `!preview().is_empty()` and every element is
  `Entity::Line(_)`.

- **(AC 14)**: `preview_y_coords_above_baseline` — after first click at
  `Vec2::new(0.0, 0.0)` and `on_text_input('H')`, assert all preview endpoint
  Y values are `>= 0.0 - EPSILON`.

- **(AC 15)**: `more_chars_produce_more_preview_lines` — type `'A'`, record
  `n1 = preview().len()`; type `'B'`, record `n2 = preview().len()`; type `'C'`,
  record `n3 = preview().len()`; assert `n3 >= n2 && n2 >= n1 && n3 > n1`.

- **(AC 16, 17)**: `backspace_removes_char_and_empties_buffer` — first click,
  `on_text_input('X')`, assert preview non-empty, `on_key(Key::Backspace, &mut app)`,
  assert `preview().is_empty()`.

- **(AC 17)**: `backspace_on_empty_buffer_no_panic` — first click (no char
  typed), `on_key(Key::Backspace, &mut app)`; no panic, `preview().is_empty()`.

- **(AC 18)**: `backspace_in_idle_no_panic` — fresh tool,
  `on_key(Key::Backspace, &mut app)`; no panic.

- **(AC 19 a–d)**: `enter_with_H_commits_lines_and_returns_idle` — first click
  at `(0.0, 0.0)`, `on_text_input('H')`, `on_key(Key::Enter, &mut app)`.
  Assert `app.document.entity_count() > 0`, all entities are `Entity::Line(_)`,
  `app.history.can_undo() == true`, `preview().is_empty()`.

- **(AC 20)**: `enter_with_empty_buffer_no_commit` — first click, then
  `on_key(Key::Enter, &mut app)` with no characters typed; assert
  `doc.entity_count() == 0` and `preview().is_empty()`.

- **(AC 21)**: `enter_in_idle_no_panic` — fresh tool,
  `on_key(Key::Enter, &mut app)`; no panic, `preview().is_empty()`.

- **(AC 22)**: `commit_is_one_history_entry_and_undoable` — first click, type
  `'H'`, Enter; assert `app.history.len() == 1`. Call
  `app.history.undo(&mut app.document)`; assert
  `app.document.entity_count() == 0`.

- **(AC 23)**: `escape_in_waiting_discards_typed_text` — first click, type
  `'H'`, `on_key(Key::Escape, &mut app)`; assert
  `app.document.entity_count() == 0`, `preview().is_empty()`.

- **(AC 24)**: `escape_in_idle_no_panic` — fresh tool,
  `on_key(Key::Escape, &mut app)`; no panic.

- **(AC 25)**: `cancel_in_waiting_resets_to_idle` — first click, type `'H'`,
  `cancel()`; assert `preview().is_empty()`.

- **(AC 26)**: `cancel_is_idempotent` — `cancel()` twice on fresh tool; no
  panic, `preview().is_empty()`.

- **(AC 27)**: `second_pointer_down_in_waiting_is_noop` — first click at
  `(0.0, 0.0)`, type `'A'`, second `on_pointer_down` at `(99.0, 99.0)`;
  assert preview is still for anchor `(0.0, 0.0)` (all line X coords are close
  to 0, not 99) and `doc.entity_count() == 0`.

- **(AC 30)**: `text_tool_is_object_safe` —
  `let _: Box<dyn Tool> = Box::new(TextTool::default());`.

- **(AC 32)** Static check: `grep -nE '^use (eframe|rfd)' src/tools/text.rs`
  returns no matches.

- **(AC 34)** Build gate: `cargo fmt --all -- --check && cargo clippy
  --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

*(none)*

## Notes

### Why `CreateEntities` and not multiple `CreateLine` calls

A text string of N characters produces K stroke segments (K ≥ 1). Committing
K individual `CreateLine` commands would require K undo steps to erase a
single text placement. A single `CreateEntities` command makes the entire
string placement reversible with one Ctrl+Z, matching the AutoCAD R14 `UNDO`
behaviour for the TEXT command.

### `on_key(Key::Enter)` vs `on_pointer_up` for commit

AutoCAD R14 TEXT commits on Enter, not on click release. Pressing Enter is the
operator's explicit "I'm done typing" signal. `on_pointer_up` is a documented
no-op (AC 29).

### App::commit vs history.commit

`on_key` receives `&mut App`, which exposes `App::commit`. The Enter handler
calls `app.commit(Box::new(CreateEntities::new(lines)))`. This is the same
legal path used by tools when they need App-level access (see LCV-043 notes).

### `layout_text` import path

```rust
use crate::text::layout_text;
use crate::geometry::Vec2;
use crate::document::{commands::CreateEntities, Entity};
```

`layout_text` takes `spacing_factor: f64` as the last argument; pass
`self.spacing_factor` (default `1.0`).

### Actual `layout_text` signature (as shipped in LCV-055)

```rust
pub fn layout_text(text: &str, origin: Vec2, height_mm: f64, spacing_factor: f64) -> Vec<Entity>
```

`spacing_factor = 1.0` yields normal Hershey advance widths. The LCV-055
demand notes mention `0.2` as a conventional default, but the implemented tests
use `1.0` as the baseline. TextTool uses `1.0`.

### `on_text_input` and `egui::Event::Text`

`egui::Event::Text(String)` is fired by egui for every printable key press,
respecting the OS input method (shift, dead keys, etc.). It carries a `String`
(may contain multiple code points for composition sequences). The routing in
`app.rs` iterates `chars()` on the string and feeds each `char` individually to
`ToolManager::on_text_input`. No `std::mem::take` is needed because
`on_text_input` does not take `&mut App`.

### Preview performance

`preview()` calls `layout_text` on every frame while the operator is typing.
For the typical annotation string (< 64 characters), this is a tight inner loop
over a static `&[&[(i8, i8)]]` array — no allocation beyond the output `Vec`.
Flamegraph profiling is not required for this demand; if performance becomes
visible at > 200 characters, a caching layer is a future demand.

### `CreateEntities` placement in `create.rs`

The existing `create.rs` holds `CreateLine`, `CreateCircle`, `CreateArc`. Add
`CreateEntities` in the same file to keep all primitive-insertion commands
co-located. If the file approaches 350 LOC, the implementer may split it into
`create_single.rs` and `create_batch.rs` — document the split in a brief module
comment.

### v1 reference

v1 had an unreleased TEXT command in the backlog. The Hershey-based stroke
approach is new to v2. There is no v1 implementation to compare against.

### `text.rs` LOC budget

The tool state machine (struct + enum + two `impl` blocks + tests) is more
complex than `LineTool` but comparable to `ArcTool`. All tests inside the
same file; keep at ≤ 300 LOC. If the test block alone grows beyond 200 lines,
split tests into a sibling `src/tools/text_tests.rs` under `#[cfg(test)]` with
`#[path = "text_tests.rs"] mod tests;` in `text.rs`.
