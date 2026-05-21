# LCV-042 — SelectTool (point pick + window/crossing box)

- **Status**: Ready
- **Phase**: 4
- **Depends on**: LCV-041 (done), LCV-015 (done), LCV-027 (done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

The `SelectTool` stub shipped by LCV-040 is a no-op placeholder. Without a functional
SelectTool, the user cannot highlight entities to feed into move, delete, trim, or
extend commands. This demand replaces the stub with AutoCAD R14's default "command: "
selection behaviour: single-click pick, Shift+click toggle, window box (left-to-right
drag → fully-inside), and crossing box (right-to-left drag → intersects or inside).
Every selection change commits a `SelectionCommand` through the history stack so
Ctrl+Z restores the prior selection.

The existing `PointerEvent` type (LCV-041) does not carry modifier state, making
Shift+click impossible without a structural workaround. This demand closes the gap
by adding a `shift: bool` field to `PointerEvent::Press` and `PointerEvent::Release`,
and correspondingly adding `shift: bool` to `Tool::on_pointer_down` and
`Tool::on_pointer_up`. Every existing tool stub gets a mechanical one-line change
(`_shift: bool` in the parameter list); no logic changes are needed in those stubs.

## Scope

- **Replace the `SelectTool` stub** in `src/tools/select.rs` with a complete
  implementation. The stub's `pub struct SelectTool` declaration and `Default`
  impl are preserved; the body of every method changes.

- **`SelectTool` state machine** — three states (implementer names them freely):
  - *Idle*: nothing in progress.
  - *PressedAt(start: Vec2)*: primary button is down, cursor hasn't yet moved
    `DRAG_THRESHOLD_MM` away from `start`. `preview()` returns an empty `Vec`.
  - *Dragging { start: Vec2, current: Vec2 }*: cursor has moved past the drag
    threshold while the primary button is held. `preview()` returns four
    `Entity::Line` edges forming the rubber-band rectangle.

- **Constants** (defined in `src/tools/select.rs`):
  - `const PICK_THRESHOLD_MM: f64 = 5.0` — world-space pick radius for point pick.
  - `const DRAG_THRESHOLD_MM: f64 = 2.0` — minimum drag distance (mm) before the
    gesture is treated as a box rather than a click.

- **Point pick** (left-click, drag below threshold): executed at `on_pointer_up`.
  - For each entity at index `i` in `doc.entities`, compute a distance from `pos`:
    - `Entity::Line(l)` → `l.distance_to_point(pos)`
    - `Entity::Circle(c)` → `c.distance_to_point(pos).abs()`
    - `Entity::Arc(a)` → implementer's choice of a reasonable approximation
      (e.g., `(pos.distance(a.center) - a.r).abs()` clamped by angle-in-sweep check;
      exact formula is the implementer's discretion provided the acceptance criteria
      round-trip correctly).
  - Collect candidates where `distance < PICK_THRESHOLD_MM`. If multiple candidates,
    pick the one with the smallest distance. If none, the click lands on empty space.
  - Without Shift held:
    - Entity found → `SelectionCommand::new([idx])` → `history.commit`.
    - No entity → `SelectionCommand::new([])` (clear) → `history.commit`.
  - With Shift held:
    - Entity found and currently selected → commit `SelectionCommand` with that
      index removed from the current selection set.
    - Entity found and not currently selected → commit `SelectionCommand` with that
      index added to the current selection set.
    - No entity → **no-op** (Shift+click on empty space does not clear selection).

- **Window box** (primary button released after drag, `press_pos.x < release_pos.x`):
  - Build `box_rect = Rect::new(press_pos, release_pos)`.
  - Collect indices where `box_rect.contains_line`, `contains_circle`, or
    `contains_arc` returns `true` (matching on each `Entity` variant).
  - Commit `SelectionCommand::new(collected_indices)` → `history.commit`.
  - Always replaces the selection (no shift-additive mode for box drag).

- **Crossing box** (primary button released after drag, `press_pos.x >= release_pos.x`):
  - Build `box_rect = Rect::new(press_pos, release_pos)`.
  - Collect indices where `box_rect.crosses_line`, `crosses_circle`, or `crosses_arc`
    returns `true`.
  - Commit `SelectionCommand::new(collected_indices)` → `history.commit`.
  - Always replaces the selection.

- **Preview geometry**: `SelectTool::preview()` returns four `Entity::Line` values
  forming the four edges of `Rect::new(start, current)` only while in the *Dragging*
  state. In all other states it returns an empty `Vec`. Corner order: implementer's
  choice; the AC pins count (4) and type (all `Entity::Line`).

- **Cancel / Escape**: `SelectTool::cancel()` resets to *Idle* (clears any in-progress
  state). `on_key(egui::Key::Escape, app)` calls `self.cancel()` internally.

- **`name()`**: returns `"Select"` (unchanged from stub; matches existing manager and
  tool tests that assert this exact string).

- **Extend `PointerEvent`** in `src/tools/pointer_event.rs`:
  - Add `shift: bool` to `PointerEvent::Press { world_pos, button, shift }`.
  - Add `shift: bool` to `PointerEvent::Release { world_pos, button, shift }`.
  - `PointerEvent::Move` is unchanged.
  - Update existing construction sites (tests in `pointer_event.rs`,
    `manager.rs`, `app.rs`) to include `shift: false` (or the appropriate value).

- **Extend `Tool` trait** in `src/tools/tool.rs`:
  - `fn on_pointer_down(&mut self, pos: Vec2, shift: bool, doc: &mut Document, history: &mut History);`
  - `fn on_pointer_up(&mut self, pos: Vec2, shift: bool, doc: &mut Document, history: &mut History);`
  - `fn on_pointer_move` is unchanged (no modifier needed for move).
  - All existing tool stubs (line, circle, arc, move_, trim, extend, delete, polyline,
    rect, text, and the legacy test mocks in `manager.rs`) add `_shift: bool` and
    otherwise leave their bodies unchanged.

- **Update `ToolManager::on_pointer_event`** in `src/tools/manager.rs`:
  - Extract `shift` from `PointerEvent::Press { shift, … }` and pass it to
    `self.active.on_pointer_down(world_pos, shift, doc, history)`.
  - Extract `shift` from `PointerEvent::Release { shift, … }` and pass it to
    `self.active.on_pointer_up(world_pos, shift, doc, history)`.
  - Legacy wrappers `handle_pointer_down` and `handle_pointer_up` pass `shift: false`.

- **Update `App::update`** in `src/app.rs`:
  - When building `PointerEvent::Press { … }`, populate
    `shift: ctx.input(|i| i.modifiers.shift)`.
  - When building `PointerEvent::Release { … }`, populate
    `shift: ctx.input(|i| i.modifiers.shift)`.

## Out of scope

- **Shift-additive box drag** — box drag (window or crossing) always replaces the
  selection. Shift+box-additive is an AutoCAD extension not in v0.1.0 scope.
- **Zoom-aware pick threshold** — `PICK_THRESHOLD_MM = 5.0` is a fixed world-space
  constant. Making it scale with `camera.mm_per_px` (like snap tolerance does) is
  deferred to LCV-054.
- **Window vs crossing visual distinction** — both use the existing translucent amber
  preview stroke from LCV-037. Blue-vs-green dashed-box rendering (AutoCAD R14 style)
  is a future visual-polish demand.
- **Grips / handles** on selected entities for drag-resize. Modify operations go
  through dedicated tools (Move, Trim, Extend).
- **Selection filter by entity type** (e.g., "select all circles"). The selection set
  is a plain index set; filtering is the caller's responsibility.
- **Secondary-button context menu** for selection (reserved for a Phase 6 demand).
- **Touch / pen input** — not in v2 scope.
- **Shift+click on empty space clearing selection** — the no-op behaviour (Shift+click
  on empty space preserves selection) is intentional and matches AutoCAD R14.

## Acceptance criteria

1. `SelectTool::default().name()` returns exactly `"Select"`. Existing tests
   in `src/tools/manager.rs` and `src/tools/select.rs` that assert `"Select"` continue
   to pass without modification.

2. `PointerEvent::Press` has a `shift: bool` field; `PointerEvent::Release` has a
   `shift: bool` field; `PointerEvent::Move` is unchanged.
   `grep -nE 'PointerEvent::Press\b' src/tools/pointer_event.rs` shows the new field
   in the variant definition.

3. `Tool::on_pointer_down` signature is
   `fn on_pointer_down(&mut self, pos: Vec2, shift: bool, doc: &mut Document, history: &mut History)`.
   `Tool::on_pointer_up` signature is
   `fn on_pointer_up(&mut self, pos: Vec2, shift: bool, doc: &mut Document, history: &mut History)`.
   Both match across the trait definition and every `impl Tool` in the codebase.

4. `ToolManager::on_pointer_event` passes the `shift` value from `PointerEvent::Press`
   / `PointerEvent::Release` to `on_pointer_down` / `on_pointer_up` respectively.
   `ToolManager::handle_pointer_down` and `handle_pointer_up` pass `shift: false`.

5. In `App::update`, the `PointerEvent::Press` and `PointerEvent::Release` payloads
   carry `shift: ctx.input(|i| i.modifiers.shift)` at the point of construction.

6. **Point pick — selects closest entity within threshold**:  
   Given a `Document` containing one `Entity::Line` from `(0,0)` to `(10,0)`,
   `on_pointer_down` at `(5.0, 0.0)` then `on_pointer_up` at `(5.0, 0.0)` with
   `shift: false` commits exactly one `SelectionCommand` (so `history.can_undo()` is
   `true`) and leaves `doc.selection.is_selected(0)` as `true`.

7. **Point pick — click on empty space clears selection**:  
   Given the same document with entity 0 pre-selected (`doc.selection.add(0)`), a
   click at `(100.0, 100.0)` (further than `PICK_THRESHOLD_MM` from every entity)
   with `shift: false` commits one `SelectionCommand` and leaves
   `doc.selection.is_empty()` as `true`.

8. **Shift+click toggles membership**:
   - Add: entity 0 selected; Shift+click near entity 1 → `doc.selection` contains
     both indices 0 and 1.
   - Remove: entity 0 selected; Shift+click near entity 0 → `doc.selection.is_empty()`.
   - In both cases a `SelectionCommand` is committed (`history.can_undo()` is `true`
     and reflects the additional commit).

9. **Shift+click on empty space is a no-op**:  
   Entity 0 selected; Shift+click at `(100.0, 100.0)` → `doc.selection.is_selected(0)`
   remains `true` and no extra `SelectionCommand` is committed to history.

10. **Multi-entity pick — closest wins**:  
    Two `Entity::Line` entities side-by-side; cursor within `PICK_THRESHOLD_MM` of
    both; the entity with the smaller `distance_to_point(pos)` is the one selected
    after the click.

11. **Window box** (left-to-right drag):  
    One line fully inside the box, one line partially outside. A left-to-right drag
    (press X = 0, release X = 20, so `press_pos.x < release_pos.x`) commits a
    `SelectionCommand` selecting only the fully-inside entity.

12. **Crossing box** (right-to-left drag):  
    Same two lines. A right-to-left drag (press X = 20, release X = 0, so
    `press_pos.x >= release_pos.x`) commits a `SelectionCommand` selecting both
    entities (fully-inside + crossing).

13. **Drag threshold**:  
    Press at `(5.0, 5.0)`, release at `(5.0 + 1.9, 5.0)` (distance ≈ 1.9 mm < 2.0
    mm threshold) → treated as a point-pick click, not a box select.  
    Press at `(5.0, 5.0)`, release at `(5.0 + 2.1, 5.0)` (distance ≈ 2.1 mm ≥ 2.0
    mm threshold) → treated as a box select.

14. **Preview during drag returns exactly 4 `Entity::Line` values**:  
    After `on_pointer_down(start)` followed by `on_pointer_move(current)` where
    `start.distance(current) >= DRAG_THRESHOLD_MM`, `SelectTool::preview()` returns
    a `Vec<Entity>` of length 4 where every element is `Entity::Line(_)`.

15. **Preview is empty when not dragging**:  
    `SelectTool::default().preview().is_empty()` is `true`. After `cancel()`, the
    same holds. In the *PressedAt* state (before the drag threshold is crossed),
    `preview()` also returns an empty `Vec`.

16. **Escape cancels drag**:  
    Begin a drag; call `on_key(egui::Key::Escape, &mut app)` → `preview().is_empty()`.

17. **Undo restores previous selection**:  
    Click to select entity 0; then `history.undo(&mut doc)` → selection returns to
    its pre-click state (verified via `doc.selection.is_empty()` if selection was
    initially empty).

18. **Kernel purity**:  
    `grep -nE '^use (egui|eframe|rfd)' src/tools/select.rs` returns no matches.
    (`egui::Key` is used only via `on_key`'s parameter, which is declared by the
    trait in `src/tools/tool.rs` — not a `use` import in `select.rs` itself unless
    the implementer requires it, in which case it is acceptable since `tools/` is
    not part of the kernel purity boundary.)

19. **LOC budgets**: `src/tools/select.rs` ≤ 300 LOC;
    `src/tools/pointer_event.rs` ≤ 90 LOC; `src/tools/tool.rs` ≤ 90 LOC.

20. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and
    `cargo test --all` all exit 0.

## Expected tests

All tests live in `#[cfg(test)] mod tests` inside `src/tools/select.rs` unless noted.

- **Unit (AC 1)**: `select_tool_name_is_select` — `assert_eq!(SelectTool::default().name(), "Select")`.
- **Unit (AC 6)**: `point_pick_selects_nearest_entity` — one line at `(0,0)→(10,0)`;
  simulate down + up at `(5.0, 0.0)` with `shift: false`; assert
  `doc.selection.is_selected(0)` and `history.can_undo()`.
- **Unit (AC 7)**: `click_empty_space_clears_selection` — entity 0 pre-selected;
  click at `(100.0, 100.0)` with `shift: false`; assert `doc.selection.is_empty()`
  and `history.can_undo()`.
- **Unit (AC 8 — add)**: `shift_click_adds_entity_to_selection` — entities 0 and 1;
  click entity 0 (no shift); then shift+click entity 1; assert both selected.
- **Unit (AC 8 — remove)**: `shift_click_removes_entity_from_selection` — entity 0
  pre-selected; shift+click near entity 0; assert `!doc.selection.is_selected(0)`.
- **Unit (AC 9)**: `shift_click_empty_space_noop` — entity 0 selected; shift+click
  at `(100.0, 100.0)`; assert entity 0 still selected and that the history length did
  not increase (check `history.len()` before and after).
- **Unit (AC 10)**: `point_pick_closest_wins` — two lines within `PICK_THRESHOLD_MM`
  of cursor; the closer one is selected.
- **Unit (AC 11)**: `window_box_selects_fully_inside_only` — one line fully inside,
  one partly outside; left-to-right drag; assert only the fully-inside entity is
  selected.
- **Unit (AC 12)**: `crossing_box_selects_intersecting_entities` — one line fully
  inside, one crossing the box boundary; right-to-left drag; assert both selected.
- **Unit (AC 13 — below threshold)**: `drag_below_threshold_is_treated_as_click` —
  press at `(5.0, 5.0)`, release at `(6.9, 5.0)` (1.9 mm apart); assert `preview()`
  returned an empty vec at release time (not a box operation).
- **Unit (AC 13 — above threshold)**: `drag_above_threshold_is_treated_as_box` —
  press at `(5.0, 5.0)`, move to `(7.1, 5.0)` (2.1 mm), assert `preview()` has 4
  lines; release; assert a `SelectionCommand` was committed.
- **Unit (AC 14)**: `preview_during_drag_has_four_edge_lines` — after crossing drag
  threshold, `preview().len() == 4` and all items are `Entity::Line(_)`.
- **Unit (AC 15)**: `preview_idle_is_empty` — `SelectTool::default().preview().is_empty()`.
- **Unit (AC 16)**: `escape_cancels_drag` — begin drag; `on_key(Escape, &mut app)`; assert `preview().is_empty()`.
- **Unit (AC 17)**: `undo_restores_prior_selection` — click selects entity 0; undo;
  assert `doc.selection.is_empty()`.
- **Static check (AC 18)**: CI `grep` for `egui`/`eframe`/`rfd` top-level imports in
  `src/tools/select.rs`.
- **Build gate (AC 20)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

*(none)*

## Notes

- **`name()` value**: The stub and all existing tests assert `"Select"` (title-case).
  The brief mentioned `"SELECT"` (all-caps). Title-case is kept for consistency with
  shipped tests. If the convention should be all-caps, that is a one-line change in
  all Phase-4 tool stubs and a separate demand on the status-bar display layer.

- **`PointerEvent` `shift` field in `Move`**: `PointerEvent::Move` deliberately
  omits `shift` — the ortho lock demand (LCV-053) will add modifier-aware move
  handling when it needs it. No premature generalisation here.

- **Legacy `handle_pointer_down/up` wrappers**: These pass `shift: false`, which
  is correct because they are only used by the pre-LCV-041 test scaffolding. No
  tool other than `SelectTool` inspects `shift` in this demand.

- **Shift+click implementation detail**: the implementer builds the new selection set
  from `doc.selection.iter().collect::<HashSet<_>>()`, toggles membership for the
  picked index, then commits `SelectionCommand::new(new_set)`. One command per click,
  fully undoable.

- **Box direction uses original `press_pos.x` vs `release_pos.x`**, not the
  `Rect::min`/`max` (which are normalised). The direction check `press_pos.x <
  release_pos.x` (window) vs `press_pos.x >= release_pos.x` (crossing) must use
  the raw, un-normalised press and release positions. `Rect::new` is used only to
  build the normalised rect for the `contains_*` / `crosses_*` predicates.

- **Pick threshold is intentionally fixed at 5.0 mm**. At the default zoom
  (`mm_per_px = 1.0`) this is 5 screen pixels — tight. At 4× zoom-in
  (`mm_per_px = 0.25`) it becomes 20 screen pixels — comfortable. A zoom-aware
  formula (like `SNAP_TOLERANCE_PX × mm_per_px`) requires passing camera state
  into `on_pointer_down`, which is not in the current `Tool` trait. Defer to LCV-054.

- **Preview entity count**: four edges. A `Rect { min, max }` has corners BL, BR, TR,
  TL. A canonical order (bottom → right → top → left) is easy to test. All four must
  be `Entity::Line`; no `Circle` or `Arc` variants are valid for the rubber-band box.

- **LOC pressure**: with state enum, two constants, six trait method bodies, and ≈20
  unit tests, `select.rs` is expected to land around 250–280 LOC. If the test block
  pushes past 300, extract a helper `fn setup_doc_with_line() -> (Document, History)`
  into `mod tests` to deduplicate fixture setup.

- **Reference**: v1's `SelectTool.ts` (TypeScript) used `closest entity within hitRadius`
  for point-pick, left-vs-right drag for window/crossing, and `SelectionCommand`
  for all mutations. v2 mirrors that logic; `Rect::contains_*` / `crosses_*`
  (LCV-015) replace the v1 manual bbox loops.
