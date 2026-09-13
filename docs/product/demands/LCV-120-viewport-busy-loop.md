# LCV-120 — The viewport never lets the app idle

- **Status**: Ready
- **Phase**: 11
- **Depends on**: none (interacts with LCV-105, LCV-116; invariant lives in `AGENTS.md` §Event flow → Repaint policy)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

`src/app/viewport.rs:46` calls `ctx.request_repaint()` unconditionally, inside
the `CentralPanel` closure, on every frame. `viewport::draw` runs once per frame
from `App::update_ui` (`src/app/mod.rs:228`) with no condition around it, so
**the application never idles by its own choice**: it tells the windowing system
"wake me again immediately" on every frame of a session in which nothing at all
is happening — no pointer, no keys, no tool in progress, no dirty document.
Independently reproduced twice: driving the real `App::update_ui` through
`egui::Context::run` on a clean, idle app, `FullOutput`'s `repaint_delay` is
**0 ns on consecutive frames**.

**What that costs is platform-dependent, and the original report overstated
it.** The demand as opened claimed the app "holds one CPU core at 100 percent
for the entire life of every session". That does not reproduce: on a Wayland
session, where the compositor throttles a surface that is not visibly changing,
the team lead measured **0.0% of a core over six seconds idle**. The request is
still made every frame; the platform is simply declining it. Somewhere that
declines less — a compositor without throttling, a remote or software-rendered
session, or any future platform target — pays the full price, and nothing in
this repo decides which. So this is a correctness-and-smell defect with a
platform-dependent cost, not an emergency. It is worth fixing for the second
reason below regardless of the first.

**The second reason is the real one: the blanket repaint masks the mechanism
LCV-116 AC 9 exists to provide.** `autosave::schedule_flush_repaint`
(`src/app/autosave.rs:84`, called from `src/app/mod.rs:237`) asks egui for one
bounded follow-up frame while an autosave write is still pending, and it is the
only thing that keeps a debounced write alive once the operator stops touching
the app. It is correct and tested at its own granularity — but with the viewport
requesting a frame every frame anyway, it is not currently what keeps autosave
alive; the busy loop is. It reads as dead code, and the invariant it encodes
cannot be observed through `App::update_ui` at all. `AGENTS.md` §Repaint policy
already records that it must not be deleted; this demand turns that comment into
an assertion.

**The comment on the line is half false, and that is probably why the line
survived review.** It reads "Always repaint so cursor-coords and smooth camera
motion stay live." There is **no camera animation anywhere in the tree**:
`Camera::pan`, `Camera::zoom_around` and `Camera::zoom_extents`
(`src/render/camera.rs:144`, `:125`, `:158`) all apply their change instantly
and return. "Smooth camera motion" describes a feature that does not exist, and
justified a cost that does. The cursor-coordinate half is real but does not need
this line either: `app.last_cursor_world` is written only inside `handle_hover`,
which already runs only when `response.hovered()` is true — so tying the repaint
to hover changes the status bar's readout in no way whatsoever.

## Scope

- One named private predicate in `src/app/viewport.rs` and one conditional
  around the existing `ctx.request_repaint()` call.
- A replacement doc comment that does not repeat the camera-animation claim.
- **Five** integration tests that observe `repaint_delay` through the real
  `App::update_ui`, including the negative controls that make them able to fail
  — one per term of the predicate plus the idle case and the autosave witness.
  *(Corrected after implementation: this line read "Three integration tests"
  while AC 4-7 and Expected tests already required four, and a fifth was needed
  to make mutation (b) able to fail. See the Expected-tests list.)*
- Correcting the two documents whose text becomes false the moment this lands.

## Out of scope

- **A scheduler, a frame-budget system, or an animation subsystem.** Three
  boolean terms and one `if`. Anything larger is a different demand and would
  need an ADR.
- **A fourth term in the predicate.** In particular **not**
  `tool_manager.anchor()`: a tool that is armed and waiting for its first click
  has nothing moving on screen, and adding it would silently restore the busy
  loop for the whole time a tool is active — which is most of a session.
  `response.dragged()` already covers every mouse button, including the
  middle-button pan handled immediately above it.
- **Any change to the other two repaint call sites.** The agent poll
  (`src/app/mod.rs:223-225`) and `autosave::schedule_flush_repaint` are correct,
  conditional and tested; they are read by this demand's tests and edited by
  none of them.
- **Deleting `autosave::schedule_flush_repaint`.** It stops being redundant the
  instant this lands. AC 6 is its witness; removing it would silently reopen
  LCV-116 AC 10 (a document left dirty and never written once the app goes
  idle).
- **A frame-rate cap, a vsync setting, a "power saving" preference, or any
  operator-visible control.** Nothing here reaches the UI.
- **Changing the autosave debounce, `sync_dirty`, `mark_clean`, or when
  `flush_if_due` runs.**
- **Changing what the status bar shows or when `last_cursor_world` is written.**
- **Touching `src/app/persist.rs` or the persistence paths.** That is LCV-119;
  the two demands share no line.

## Acceptance criteria

1. **The predicate is named, private, and lives in `viewport.rs`.** Not inline
   in the closure, and not a method on `App` — a named predicate is what a
   reviewer can check and a test can reason about:

   ```rust
   fn viewport_is_live(response: &egui::Response, app: &App) -> bool {
       response.hovered() || response.dragged() || !app.preview_entities.is_empty()
   }
   ```

   Exactly those three terms, in that order, no fourth. A bounded source scan
   over the function pins the three terms and asserts `anchor()` is absent, with
   a positive control.

2. **The repaint becomes conditional.** The unconditional
   `ctx.request_repaint()` at `src/app/viewport.rs:46` is replaced by
   `if viewport_is_live(&response, app) { ctx.request_repaint(); }` at the same
   position — after `paint()`, so `app.preview_entities` is **this frame's**
   value (`paint` assigns it from `tool_manager.preview()`), and after the
   middle-drag pan block. No `request_repaint` call is added anywhere else in
   `src/`.

3. **The comment stops claiming a feature that does not exist.** The replacement
   doc comment does not mention smooth camera motion, animation, or interpolated
   panning. It states what is true: the canvas asks for a follow-up frame only
   while the pointer is over it, a drag is in progress, or a tool preview is on
   screen; the status-bar coordinates are unaffected because `last_cursor_world`
   is only ever written under `response.hovered()`. It points at `AGENTS.md`
   §Repaint policy.

4. **Idle — the defect itself.** New integration test file
   `tests/lcv120_idle_repaint.rs`. A fresh `App::default()` is driven through
   `App::update_ui` with `harness::raw_input(vec![])`, and **no frame in the
   test carries any input event at all**. `repaint_delay` is
   `Duration::MAX` on frames 1 and 2. Frame 0 is excluded and the exclusion is
   commented: a fresh `egui::Context` reports 0 ns on its first frame regardless
   of what the body asked for (already documented in
   `tests/lcv116_autosave_repaint.rs::settle`). This assertion reads 0 ns on
   HEAD and `Duration::MAX` after the change; the implementer must confirm both,
   on a pristine checkout and on the patched tree.

5. **Hover, with the control that makes it falsifiable.** Same runner, same
   file. Frame A carries exactly one `egui::Event::PointerMoved(p)` for a `p`
   inside the central panel; frame B carries **no events** (settle); frames C
   and D carry **no events** and report `repaint_delay == Duration::ZERO`.
   Positive control: after frame A, `app.last_cursor_world.is_some()` — only
   `handle_hover` writes that field and only under `response.hovered()`, so this
   proves the chosen coordinate really is over the canvas rather than the test
   passing for the wrong reason. **Negative control, in the same file and
   mandatory**: the identical shape with `p` over the menubar reports
   `Duration::MAX` on frames C and D and leaves `app.last_cursor_world == None`.
   Without that second half, the hover assertion cannot fail.

6. **The autosave witness — `schedule_flush_repaint` becomes load-bearing in a
   test rather than in a comment.** Same idle shape as AC 4 (no pointer event in
   any frame, ever), with `dirty_since: Some(Instant::now())`. `repaint_delay`
   on frames 1+ is **non-zero and strictly under 800 ms**. Delete
   `schedule_flush_repaint` and this assertion reads `Duration::MAX`: an app
   that idles forever and never writes the operator's work. ADR 0002 §A4 rule 2
   is respected — the `dirty_since` stamp is fresh, the test sleeps for nothing,
   the debounce never elapses, and no write happens.

7. **The trap that has already produced an un-failable test in this project is
   written into the test file.** An asserting frame must carry **no input event
   at all**. If a frame carries any event — including a repeated
   `PointerMoved` at the same position — egui reports `repaint_delay == 0`
   regardless of what the predicate returned. The architect wrote that version
   first and **measured it passing against a mutant with `hovered()` deleted**.
   The shape that works is: one frame that makes the pointer resident, one
   settle frame, then assertions on empty frames only. This is recorded as a
   comment in `tests/lcv120_idle_repaint.rs`'s module header, next to the
   frame-0 rule, so the next person to extend the file does not re-derive it.

8. **The three surviving repaint sites are each conditional.** `src/` contains
   exactly three **implementation** `request_repaint*` call sites, excluding
   comments and test sections — `src/app/mod.rs` (agent turn in flight),
   `src/app/autosave.rs` (pending write), `src/app/viewport.rs` (live canvas) —
   and each is inside an `if`. A static check asserts the count and that none is
   at statement level without a guard.

   **A bare `grep -rn "request_repaint" src/` does not return three and never
   did** — it returns **8** on the shipped tree and returned **7** before this
   demand landed. The extra matches are doc-comment prose (`src/app/autosave.rs`
   module header) and needles inside `#[cfg(test)]` sections
   (`src/app/viewport.rs`, `src/app/autosave.rs`). *(Corrected after
   implementation: the criterion as originally written named that grep and its
   literal output, which was never true. What the shipped test asserts — a walk
   over `src/` that slices each file at the bare `#[cfg(test)]` and skips
   comment lines — is what this criterion has always meant, and is the wording
   above.)* The count is over implementation lines only; a `grep` figure is not
   the criterion and must not be re-introduced as one.

9. **`tests/lcv116_autosave_repaint.rs`'s module header is rewritten.** Its
   section **"Why this drives `schedule_flush_repaint` and not `App::update_ui`"**
   becomes false the moment this lands: it says the unconditional viewport
   repaint "pins `repaint_delay` at zero on every full frame, clean or dirty, so
   the clean half of the contract is not observable through `update_ui` today",
   and that removing that line is "a rendering-policy change this demand did not
   ask for". Both statements expire here. Rewrite the section: the clean case is
   now observable through `update_ui`, `tests/lcv120_idle_repaint.rs` asserts it
   there, and this file keeps its narrower unit-granularity assertions on
   `schedule_flush_repaint` itself. No other sentence in that header may be left
   standing if this change makes it false.

10. **`AGENTS.md` §Event flow → Repaint policy is updated in the same commit.**
    The sentence "One violation is live today, `src/app/viewport.rs:46`, and
    LCV-120 closes it: do not add a second, and do not delete
    `schedule_flush_repaint` when it lands…" expires on landing. Replace it
    with: there are no unconditional repaints left; the three conditional sites,
    named; `schedule_flush_repaint` is now load-bearing and is pinned by
    `tests/lcv120_idle_repaint.rs`; an unconditional per-frame repaint remains a
    review blocker.

11. **Caps, purity, gates.** `src/app/viewport.rs` is at 195 of the 300
    implementation-LOC cap (ADR 0004's `awk` recipe, never `wc -l`) and must
    stay under it. It still MUST NOT import `eframe` or `rfd`, and MUST NOT read
    a key. `cargo fmt --all -- --check`,
    `cargo clippy --all-targets -- -D warnings` and `cargo test --all` exit 0.

### Measured reference

Verified by the architect against pristine and patched copies of `32334ec`.
These are the numbers the criteria above encode; if the implementer measures
something different, stop and report rather than adjusting the assertion.

| input shape | HEAD today | after the fix |
|---|---|---|
| no pointer event ever, frames 1+ | 0 ns | `Duration::MAX` |
| `dirty_since = Some(now)`, no pointer, frames 1+ | 0 ns | 776 ms |
| pointer moved into the canvas, then empty frames | 0 ns | 0 ns |
| pointer moved onto the menubar, then empty frames | 0 ns | `Duration::MAX` |

## Expected tests

- **Integration (AC 4, 5, 6, 7)** in `tests/lcv120_idle_repaint.rs`, with
  `mod harness;`. `repaint_delay` is observable through the ADR 0002 harness
  **with no harness change**: `harness::raw_input` is already `pub`, and this
  file adds its own short runner that keeps the `FullOutput` instead of dropping
  it — exactly what `tests/lcv116_autosave_repaint.rs::scheduled_delay` already
  does, except the closure body is `app.update_ui(ctx)`:

  ```rust
  fn frame_delay(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) -> Duration {
      let out = ctx.run(raw_input(events), |ctx| app.update_ui(ctx));
      out.viewport_output[&egui::ViewportId::ROOT].repaint_delay
  }
  ```

  **Five** tests: `an_idle_app_asks_for_no_repaint` (AC 4);
  `a_hovered_canvas_keeps_asking_for_frames` (AC 5, with the
  `last_cursor_world` positive control);
  `a_pointer_outside_the_canvas_lets_the_app_idle` (AC 5's negative control);
  `a_pending_autosave_still_wakes_an_idle_app` (AC 6); and
  `a_live_preview_keeps_asking_for_frames_with_the_pointer_away` (AC 1's third
  predicate term, and the only test that can fail mutation (b) below).

  *(Corrected after implementation: this list named four tests while mutation
  (b) requires a live-preview case that none of the four exercises. The fifth
  test was added by the implementer and confirmed by the reviewer as required
  rather than scope creep, so it is named here. It puts a preview on screen with
  the pointer **away** from the canvas, so `hovered()` and `dragged()` are both
  false and the assertion can only be carried by
  `!app.preview_entities.is_empty()`.)*
- **Unit (AC 1, 2, 3, 8)** in `src/app/viewport.rs`'s test module: the bounded
  source scan over `viewport_is_live` (three terms present, `anchor` absent,
  positive control); a scan asserting the `request_repaint` call sits inside the
  `if`; a scan asserting the replacement comment contains none of
  `"smooth camera"`, `"animation"`, `"animat"`. Bound every haystack to the
  implementation section — a scan that reads its own file whole and matches its
  own `.find()` arguments has shipped three times in this project.
- **Mutation checks the reviewer will run, so run them first and record the
  results**: (a) delete `response.hovered()` from the predicate → AC 5's hover
  test must fail by name (if it still passes, the asserting frame is carrying an
  event — re-read AC 7); (b) delete `!app.preview_entities.is_empty()` →
  `a_live_preview_keeps_asking_for_frames_with_the_pointer_away` must fail by
  name; (c) delete `schedule_flush_repaint`'s call from `update_ui` → AC 6 must
  fail by name; (d) revert the `if` to an unconditional call → AC 4 must fail by
  name.
- **[manual] smoke**: `cargo run`. Move the pointer over the canvas — the
  status-bar coordinates track it exactly as before. Middle-drag to pan — the
  pan is smooth and does not stutter or stop mid-drag (this is the
  `response.dragged()` term; it has no automated test **in this demand** — the
  test is writable and was deferred, not ruled out; see §Risks and LCV-127).
  Start the LINE tool, click once, move the mouse — the rubber-band preview
  follows. Move the pointer off the window entirely and leave it for ten
  seconds — the app is quiet; on a platform with a per-process CPU readout it
  should sit at or near zero. Draw a line and stop touching the app — within
  ~1 s the autosave indicator still goes from `● autosave pending` to
  `○ autosaved`. That last step is the one that fails if
  `schedule_flush_repaint` is disturbed.

## Risks

- **The recurring bug class in this project is a test that cannot fail**, and
  this demand's central assertion is an *absence*. The architect already wrote a
  version of AC 5 that passed against a mutant with `hovered()` deleted, because
  its asserting frames each carried a `PointerMoved`. AC 7 exists to stop that
  recurring; the mutation list in Expected tests is not optional.
- **Frame 0 always reads 0 ns.** An assertion placed on the first frame of a
  fresh `Context` is vacuous. Start at frame 1.
- **`response.dragged()` has no automated coverage** and is the one term carried
  by the source scan and the manual smoke alone. **That coverage is deferred,
  not infeasible.** *(Corrected after implementation. This bullet previously
  claimed a headless drag "would be a fragile assertion". The reviewer measured
  otherwise: a middle-button press followed by six `PointerMoved` frames does
  pan the camera under the existing harness, and an **empty** frame mid-drag
  still reports `0ns` even with `hovered()` deleted — so a non-fragile ~10-line
  test, paired against the existing pointer-over-the-menubar control, is
  available. It was simply not written here.)* The gap is carried forward as
  **LCV-127**. If the pan stutters in the smoke test, the term is wrong — report
  it, do not add a fourth term.
- **`preview_entities` must be read after `paint()`.** `paint` is what assigns
  it from `tool_manager.preview()`; reading a stale value would make a live
  preview drop a frame behind.
- **Do not delete `schedule_flush_repaint`.** It looks redundant on HEAD and
  stops being redundant at the exact moment this lands. `AGENTS.md` §Repaint
  policy and AC 6 both say so.

## Notes

- No ADR. The architect decided this does not need one; the durable invariant
  went into `AGENTS.md` §Event flow as the **Repaint policy** paragraph, which
  AC 10 keeps current.
- **Two claims from the demand as opened have been corrected in §Problem and
  must not be reintroduced downstream.** (1) "The application holds one CPU core
  at 100 percent for the entire life of every session" — does not reproduce;
  0.0% of a core over six seconds idle on Wayland, where the compositor
  throttles the surface. The honest claim is that the app never idles by its own
  choice and the cost is platform-dependent. (2) The open question "smooth
  camera motion (pan/zoom animation, if any)" is **answered: none exists**.
  `Camera::pan`, `zoom_around` and `zoom_extents` are all instantaneous. The
  original Open questions section is closed and removed on that basis.
- `git blame src/app/viewport.rs` attributes line 46 to `0d1d52b` (LCV-105),
  which did not set out to introduce it; it is a pre-existing line that
  LCV-116's review happened to measure.
- Related: LCV-102 (autosave dirty tracking), LCV-116 AC 9 / AC 10 (the repaint
  scheduling this defect masks), ADR 0002 §A3/§A4 (the headless harness and its
  rules), ADR 0004 (the LOC cap and how to measure it).
