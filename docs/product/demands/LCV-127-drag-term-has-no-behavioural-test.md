# LCV-127 — The drag term of `viewport_is_live` has no behavioural test

- **Status**: Draft
- **Phase**: 11
- **Depends on**: LCV-120
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

LCV-120 made the canvas ask for a follow-up frame only while something is
moving, and `viewport_is_live` (`src/app/viewport.rs:68`) is the whole of that
decision:

```rust
response.hovered() || response.dragged() || !app.preview_entities.is_empty()
```

Two of those three terms have a behavioural witness in
`tests/lcv120_idle_repaint.rs` — `a_hovered_canvas_keeps_asking_for_frames`
and `a_live_preview_keeps_asking_for_frames_with_the_pointer_away`, each paired
with a negative control. The middle term has none. It is covered only by a
source scan (`the_predicate_has_exactly_three_terms`), which proves the text
`response.dragged()` is present in the file and nothing whatever about what
happens when the operator pans.

That gap is not academic. `dragged()` is the term that keeps the camera moving
while the middle button is held: the pan handler two lines above it runs under
`response.dragged_by(Middle)`, and a pan that stops asking for frames stutters
or freezes mid-stroke — the operator drags and the drawing does not follow.
Deleting the term today breaks that and the suite stays green, which is this
repository's most-repeated defect class: a test that cannot fail. LCV-120's
review closed with the term measured, not guessed: a middle-button press
followed by six `PointerMoved` frames does pan the camera under the headless
harness, and an empty frame taken mid-drag still reports a repaint request. The
test is roughly ten lines and it was deferred for scope, not ruled out as
infeasible. This demand writes it.

## Scope

- One integration test added to the existing `tests/lcv120_idle_repaint.rs`,
  covering the `response.dragged()` term of `viewport_is_live` behaviourally.
- Whatever adjustment to that test's own event sequence is needed to make it
  fail when the term is deleted (see AC 3 — the mutation is the criterion, the
  sequence is only the starting point).
- A line in the file's module doc recording that the third term now has a
  witness too.

## Out of scope

- **Any change to `src/`.** `viewport_is_live`, `handle_pan`, `Camera::pan`
  and the repaint policy are correct and shipped. This demand adds a test to a
  behaviour that already works. If the test fails against `main` on first run,
  stop and report — that is a finding, not a licence to edit the predicate.
- **A new `tests/lcv127_*.rs` binary.** `boot`, `frame_delay` and
  `click_events` are private helpers of `tests/lcv120_idle_repaint.rs` and are
  exactly what this test needs; copying them into a second binary is the
  fragile path and duplicates the two documented traps with them.
- **Testing the pan arithmetic.** Whether a 40 px drag moves the camera by the
  right number of millimetres is `Camera::pan`'s business and has its own unit
  tests. This test asserts the camera moved *at all*, and only as a control
  that the drag was real.
- **Wheel zoom, touch, kinetic panning, drag thresholds as a tunable.** Not in
  the product, not in this demand.
- **Reopening LCV-120.** It is shipped and reviewed; this is the follow-on it
  named.

## Acceptance criteria

1. **A new test exists, named for the behaviour.** One `#[test]` in
   `tests/lcv120_idle_repaint.rs` named
   `a_middle_drag_keeps_asking_for_frames`, with a doc comment stating that it
   is the witness for the `response.dragged()` term of `viewport_is_live` and
   that the assertion is taken on an **empty** frame, per that file's trap 2.

2. **It asserts on an empty frame taken mid-drag.** The middle button is
   pressed and **never released** before the asserting frames. The frames that
   assert carry no input event at all — any event, including a repeated
   `PointerMoved`, makes egui report `repaint_delay == 0` regardless of what
   the frame body asked for, and that is how the LCV-120 review caught a test
   that could not fail. At least two consecutive empty frames must read
   `Duration::ZERO`.

3. **The mutation gate — this is the acceptance criterion, not the event
   sequence.** With `response.dragged() ||` deleted from `viewport_is_live` in
   a scratch `git worktree`, `a_middle_drag_keeps_asking_for_frames` must
   **fail by name**. If it still passes, the asserting frame is being carried
   by `hovered()` instead and the test shape is wrong: hold the button down and
   move the pointer **off the canvas** before asserting, so that `hovered()` is
   `false` while egui still reports the captured drag. Report which of the two
   shapes was needed.

4. **Positive controls, so a pass means what it says.** In the same test:
   - the camera actually moved — capture `app.camera` before the drag and
     assert the pan changed it, proving egui crossed its drag threshold and
     `dragged_by(Middle)` fired rather than the test measuring nothing;
   - `app.preview_entities.is_empty()` at the asserting frames, so the third
     term cannot be the one answering;
   - `app.dirty_since.is_none()`, so `schedule_flush_repaint` cannot be the one
     answering either.

5. **The existing negative control is reused, not rewritten.**
   `a_pointer_outside_the_canvas_lets_the_app_idle` stays exactly as it is and
   is cited in the new test's doc comment as the paired control: same harness,
   same empty-frame shape, `Duration::MAX`. No existing test in the file is
   modified other than the module doc line in AC 6.

6. **The module doc is honest again.** The header of
   `tests/lcv120_idle_repaint.rs` gains one line recording that all three terms
   of the predicate now have a behavioural witness, and LCV-120's §Risks bullet
   naming this gap is left alone — it points here and stays accurate as
   history.

7. **Harness rules are respected.** ADR 0002 §A4: `App::default()`, never
   `App::new()`; no `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S`; both injected paths
   stay `None`. Harness rule 3: the frame carrying `PointerButton` is preceded
   by a frame carrying `PointerMoved` alone, or the widget rect is not yet
   registered for hit-testing and the press lands on nothing. `cargo fmt --all
   -- --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test
   --all` exit 0.

## Expected tests

- **Integration (AC 1, 2, 4, 5)**: `a_middle_drag_keeps_asking_for_frames` in
  `tests/lcv120_idle_repaint.rs`. Starting recipe, measured working during the
  LCV-120 review: `boot(App::default())`; one warm-up frame carrying
  `PointerMoved(canvas.center())`; one frame carrying the middle
  `PointerButton { pressed: true }`; six frames each carrying a single
  `PointerMoved` stepping across the canvas; then the empty asserting frames
  with the button still down.
- **Mutation (AC 3)**, run by the implementer in a scratch `git worktree`
  before reporting done, and reported with the exact failure line:
  - delete `response.dragged() ||` from `viewport_is_live` → the new test fails
    by name, and `a_hovered_canvas_keeps_asking_for_frames` and
    `a_live_preview_keeps_asking_for_frames_with_the_pointer_away` still pass
    (proving the new test is the one that discriminates);
  - delete the whole `if viewport_is_live(…) { ctx.request_repaint(); }` block
    → the new test fails together with the other two, as it should.
- **Control (AC 4)**: remove the camera-moved assertion and confirm the test
  still passes — that assertion is a control, not a behaviour under test, and
  knowing it is not load-bearing keeps a future edit from chasing it.
- **[manual] smoke**: none. The behaviour is already in LCV-120's manual smoke
  list ("Middle-drag to pan — the pan is smooth and does not stutter or stop
  mid-drag") and this demand exists precisely so that line stops being the only
  check.

## Notes

- Origin: `product-owner` correction to LCV-120 §Risks (2026-09-13). That
  bullet previously claimed a headless drag "would be a fragile assertion";
  the reviewer had already measured otherwise, and the text now says the
  coverage was deferred, not ruled out, and forwards the gap here.
- `tests/lcv120_idle_repaint.rs`'s module header documents the two traps that
  have each already produced a test that cannot fail in this file: frame 0 of a
  fresh `egui::Context` always reads 0 ns, and an asserting frame must carry no
  event. Read it before writing a line.
- The source-scan test `the_predicate_has_exactly_three_terms` in
  `src/app/viewport.rs` stays as it is: it guards the *shape* of the predicate
  (three terms, no fourth, still private) and this demand guards its
  *behaviour*. Neither replaces the other.
- Small demand, intentionally. One test, one term, no production change.
