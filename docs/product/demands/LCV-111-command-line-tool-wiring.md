# LCV-111 — Wire the parser into the tools: `on_command_input`, R14 prompts, focus-on-typing

- **Status**: Ready
- **Phase**: 11
- **Depends on**: LCV-110 (the parser, `ToolKind`, `tools::make`, the recall ring)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

LCV-110 builds a parser that nothing calls. This demand makes typing a
coordinate draw geometry — the single change that turns v2 from a mouse toy
into a CAD program.

Right now an operator who wants a 100 mm slot must click twice and hope. There
is no exact input path: `Tool::on_command_input` has a no-op default and zero
implementors, so `l` ⏎ `0,0` ⏎ `@100,0` ⏎ produces nothing at all. For laser
cutting that is disqualifying — a kerf-compensated pocket is 12.2 mm, not
"about 12", and the operator gets the number from a caliper, not from a pixel.
This demand delivers the three exact-entry forms every AutoCAD user already
has in their fingers:

- **absolute** `100,50` — put the point here;
- **relative** `@100,0` — 100 mm to the right of the last point;
- **direct distance** `50` — 50 mm in the direction the cursor is pointing.

It also gives the command line something to *say*. Today the strip shows
`LINE: Click to set start point` — a hint written for a mouse. After this
demand it shows `LINE Specify first point:`, the R14 prompt, and it changes
with every phase of every drawing tool, so the operator always knows what the
program is waiting for. And typing a digit anywhere focuses the field, so the
keyboard-first flow the product promises actually starts with the keyboard.

## Scope

- **`ToolInput`** (ADR 0003 §B1) in `src/cmdline/mod.rs`, with `as_point()`.
- **`Tool::on_command_input`** re-signed to
  `(&mut self, ToolInput, &mut Document, &mut History) -> bool`, default
  `false`; the `ToolManager` forward re-signed to match.
- **Six tools implement it**: `LineTool`, `PolylineTool`, `RectTool`,
  `CircleTool`, `ArcTool`, `MoveTool`. `SelectTool`, `TrimTool`, `ExtendTool`,
  `DeleteTool` keep the `false` default.
- **`anchor()` overrides** for `CircleTool`, `ArcTool` and `MoveTool` (see
  product decision 3) — `RectTool` deliberately gets none.
- **`src/app/cmdline.rs`**, a new `app` phase file holding
  `pub fn submit(app: &mut App, raw: &str)` — the one place that resolves
  `@dx,dy` and direct-distance entry against the anchor, the cursor and ortho.
- **Four `App` fields**: `command_history`, `command_feedback`,
  `focus_command_line`, `command_line_focused`.
- **R14 prompts**: new `status_text()` literals for the six drawing tools plus
  `SelectTool`, and new `status_text()` overrides on `CircleTool` and `ArcTool`,
  which have none today.
- **Focus-on-typing** and **`ArrowUp`/`ArrowDown` recall** (ADR 0003 §E2, §E3),
  including `dispatch_shortcuts -> bool` threaded into `process_input`.
- **The feedback line** rendered by `src/ui/command_line.rs`.
- **`Camera::ZOOM_STEP`** — one constant shared by the View menu and typed
  `zoom in` / `zoom out`.
- **`tests/harness/mod.rs` additions** `text_events`, `type_command`,
  `submit_command` (ADR 0003 §F1) and the trap-5 note in its module header.

## Out of scope

- **TEXT.** `TextTool` is not wired here and `Tool::on_text_input` is **not
  deleted** — LCV-112 owns `wants_raw_input` / `on_raw_input`, the TEXT state
  machine and the removal of `on_text_input`. See §Risks: this demand leaves
  TEXT string entry temporarily non-functional, by design and for exactly one
  demand.
- **Relative (`@dx,dy`) input for RECT's opposite corner.** Product decision 4
  explains why; `r` ⏎ `0,0` ⏎ `100,50` ⏎ still draws an exact rectangle.
- **Snap interaction with typed points.** A typed number is exact and bypasses
  snap entirely (ADR 0003 §B5). No "snap the typed point to the nearest
  endpoint" behaviour, ever.
- **The ADR 0003 §E6 switch** — removing the ten bare tool letters so every
  printable key reaches the command line (true R14 uniformity, where `L` ⏎ runs
  LINE). The mechanism this demand ships supports it: deleting the `TOOL_KEYS`
  loop from `dispatch_shortcuts` is the entire change. It is a **product**
  question with a real cost (it breaks a shipped, toolbar-advertised binding
  set from LCV-070/LCV-104), so it gets its own demand and its own decision,
  after Marco 1 has shipped and the flow has been used. Marco 1 keeps the bare
  letters.
- **Keeping the command line focused after a successful submit.** That is the
  §E6 switch by the back door: a permanently focused field makes
  `ctx.wants_keyboard_input()` permanently true and silently kills all ten tool
  keys. Only raw mode (LCV-112) holds focus.
- **Renaming `status_text`, `Cow<'static, str>` prompts, interpolated prompts**
  (`"Specify radius <25.0>:"`), and any prompt for `TrimTool`, `ExtendTool` or
  `DeleteTool` beyond what they show today.
- **A `CommandOutcome` enum** instead of `bool` (ADR 0003 §B2), per-tool
  rejection strings, and command echo/transcript history in a panel.
- **Polar entry `@50<30`, `SPACE` as Enter, empty-Enter "repeat last command",
  LINE's `c`/`u` options, `zoom window`/`previous`** — as in LCV-110.
- **Caret placement after a recall.** See §Risks; accepted as-is.

## Product decisions (do not re-open these)

1. **A typed point is exactly a click at that point.** Every wired tool's
   `on_command_input` body is ADR 0003 §B3 verbatim — it calls its own
   `on_pointer_down`. No tool grows a second state machine for typed input, and
   every fix to the click path is inherited by the typed path for free.
2. **Deterministic refusal beats invented geometry.** When `@` arrives with no
   anchor, or a bare distance arrives with no direction, the app **mutates
   nothing** and says why. It does not default the direction to +X, does not
   reuse the last direction, and does not treat the distance as an X coordinate.
   A wrong line that looks plausible costs an operator a sheet of material; a
   refusal costs a keystroke.
3. **`anchor()` is exposed wherever an ortho clamp still yields valid
   geometry.** `Tool::anchor()` has one consumer today — the F8 ortho pipeline
   at `src/app/viewport.rs:102` — and this demand gives it a second: it is the
   base point for `@dx,dy` and the origin for direct-distance entry. Only
   `LineTool` and `PolylineTool` override it today, which is why `@` and bare
   distances would not work for MOVE, CIRCLE or ARC. Adding it to those three
   also turns ortho on for them, and in all three cases that is R14-correct and
   the geometry still commits: MOVE gets the classic "move it 50 mm right",
   CIRCLE gets an axis-aligned radius pick, ARC gets an axis-aligned chord.
4. **RECT is the exception and gets no `anchor()`.** A rectangle's second corner
   defines a *box*, not a ray: `apply_ortho` would clamp the cursor to a
   cardinal axis from corner 1, making the width or the height exactly zero, and
   `RectTool::on_pointer_down` discards degenerate rectangles. Giving RECT an
   anchor to make `@` work would make RECT **undrawable with the mouse while F8
   is on** — trading a convenience for a regression. The absolute form
   (`100,50`) covers the exact-rectangle workflow completely. Splitting
   `Tool::anchor()` into an ortho anchor and a coordinate base point is the real
   fix; it is an architecture change, so it is a follow-up demand for
   `architect`, not a patch smuggled into this one.
5. **The prompt is the tool's, pulled per frame** (ADR 0003 §C). No prompt
   string is pushed onto `App`, no `prompt()` method is added, and the status
   bar is not involved.
6. **One zoom step for the whole product.** `zoom in` typed and View → Zoom In
   clicked must never drift apart, so the 1.25 factor becomes
   `Camera::ZOOM_STEP` and both call sites use it.
7. **The tool shortcut wins.** While the command line is unfocused, the ten
   bound letters `L P R C A M E T X D` activate their tool and never reach the
   field (ADR 0003 §E1). Every acceptance criterion about focus-on-typing reads
   "any alphanumeric that is **not** a bound tool shortcut".

## Acceptance criteria

### A. The tool contract

1. `src/cmdline/mod.rs` gains, with doc comments and `derive(Debug, Clone, Copy, PartialEq)`:

   ```rust
   pub enum ToolInput {
       /// An absolute world point in mm, already resolved.
       Point(Vec2),
       /// A bare magnitude in mm. `along` is the direct-distance point when a
       /// direction existed, else None.
       Distance { value_mm: f64, along: Option<Vec2> },
   }
   impl ToolInput { pub fn as_point(self) -> Option<Vec2>; }
   ```

   `as_point()` returns `Some(p)` for `Point(p)` and `along` for `Distance`.
   Coordinates are world millimetres, Y-up.

2. `Tool::on_command_input` becomes
   `fn on_command_input(&mut self, input: ToolInput, doc: &mut Document, history: &mut History) -> bool { false }`
   — it keeps the default body, so `SelectTool`, `TrimTool`, `ExtendTool` and
   `DeleteTool` are untouched and a typed coordinate can never silently select
   or delete something. The method takes **no `&mut App`**: LCV-041's narrowing
   holds. `ToolManager::on_command_input` is re-signed identically and forwards.

3. **Five tools take the canonical body verbatim** — `LineTool`,
   `PolylineTool`, `RectTool`, `ArcTool`, `MoveTool`:

   ```rust
   fn on_command_input(&mut self, input: ToolInput, doc: &mut Document, history: &mut History) -> bool {
       match input.as_point() {
           Some(p) => { self.on_pointer_down(p, false, doc, history); true }
           None => false,
       }
   }
   ```

4. **`CircleTool` overrides the `Distance` arm**: while in
   `CircleState::WaitingRadius { center }`, `Distance { value_mm, .. }` **is**
   the radius — `on_pointer_down(center + Vec2::new(value_mm, 0.0), …)`, or an
   equivalent that produces a circle of exactly `value_mm` radius — and no
   direction is required. `value_mm <= EPSILON` is rejected: return `false`,
   commit nothing, stay in `WaitingRadius`. In `CircleState::Idle` a `Distance`
   returns `false`. This is what makes a mouse-free session work:
   `c` ⏎ `50,50` ⏎ `25` ⏎ commits a circle of radius 25 mm centred at (50, 50)
   with `App::last_cursor_world == None`.

5. **`anchor()` overrides** (decision 3), each returning the most recently fixed
   point of the in-progress state:
   - `MoveTool`: `WaitingDest { base, .. } => Some(base)`, `Idle => None`.
   - `CircleTool`: `WaitingRadius { center } => Some(center)`, `Idle => None`.
   - `ArcTool`: `WaitingEnd { start } => Some(start)`,
     `WaitingMid { end, .. } => Some(end)`, `Idle => None`.
   - `RectTool`: **no override** (decision 4). A test asserts
     `RectTool` in `WaitingSecondCorner` returns `None`, named so that deleting
     it is a deliberate act.

6. After a typed input the app **polls succession exactly as the pointer path
   does**, so `m` ⏎ `0,0` ⏎ `@10,0` ⏎ hands back to `SelectTool` like the mouse
   flow. `poll_successor` (today the private `fn poll_successor(app: &mut App)`
   at `src/app/viewport.rs:162`) becomes `pub(super)` and is called from
   `src/app/cmdline.rs`. **One copy only** — do not duplicate the body, and do
   not promote it into `src/app/mod.rs`, which is at its line budget (AC 27).

7. Mutation still flows through `history.commit(cmd, doc)` inside the tool.
   No new mutation path, so ADR 0002 §B's revision-based dirty signal keeps
   working for typed input with no extra code: after a typed commit,
   `app.dirty_since` is `Some` on the next frame.

### B. Resolution — `src/app/cmdline.rs`

8. New file `src/app/cmdline.rs` with `pub fn submit(app: &mut App, raw: &str)`,
   declared `mod cmdline;` and re-exported `pub use cmdline::submit;` from
   `src/app/mod.rs`. `src/ui/command_line.rs`'s Enter branch calls
   `crate::app::submit(app, &text)` **and nothing else** — the parse, the
   dispatch, the ring push and the feedback all live in `submit`.

9. `submit` runs, in this order:
   1. Clear `app.command_feedback`.
   2. If the active tool `wants_raw_input()` → forward and return **(added by
      LCV-112; this demand may leave the hook out entirely rather than stub it)**.
   3. `cmdline::parse(raw)`.
   4. If the parse is anything other than `Empty`, push the trimmed `raw` to
      `app.command_history` — **including `Unknown`**, because recall exists so
      a typo can be fixed. `Empty` is not pushed.
   5. Dispatch per AC 10.

10. **Dispatch table.**

    | parse result | effect |
    |---|---|
    | `Tool(kind)` | `app.tool_manager.set_tool(tools::make(kind))`; no feedback (the prompt changes, which is the feedback) |
    | `Toggle(Snap\|Grid\|Ortho)` | flips `snap_enabled` / `grid_enabled` / `ortho_enabled` — identical semantics to F3 / F7 / F8; feedback `SNAP on` / `SNAP off` (likewise `GRID`, `ORTHO`) |
    | `Zoom(In)` / `Zoom(Out)` | `app.camera.zoom_in(Camera::ZOOM_STEP)` / `zoom_out(Camera::ZOOM_STEP)`; no feedback |
    | `Zoom(Extents)` | `handle_zoom_extents(&mut app.camera, &app.document, app.camera.viewport_size_px)` — already a no-op on the zero-area first-frame sentinel; no feedback |
    | `Point(p)` | send `ToolInput::Point(p)` to the tool |
    | `Relative(d)` | anchor `Some(a)` → send `ToolInput::Point(a + d)`; anchor `None` → **no call, no mutation**, feedback `No base point for relative input.` |
    | `Distance(v)` | send `ToolInput::Distance { value_mm: v, along }` per AC 11 |
    | `Empty` | route `egui::Key::Enter` to the active tool via the existing path (AC 14) |
    | `Unknown(text)` | feedback `Unknown command: "<text>"`, nothing else |

11. **Direct-distance resolution.** `along` is `Some(anchor + v * dir)` when
    **all** of: the active tool's `anchor()` is `Some(anchor)`;
    `app.last_cursor_world` is `Some(cursor)`; `(cursor - anchor).length() >
    EPSILON`; and `dir` is the unit vector of `cursor - anchor` **after
    `apply_ortho(anchor, cursor)` when `app.ortho_enabled`**. Otherwise `along`
    is `None`. `v` may be negative, which places the point on the opposite ray —
    that is R14 behaviour and is not special-cased.

12. **Typed points bypass snap entirely** and bypass ortho except as the
    direction source in AC 11: no `resolve_snap` call anywhere in
    `src/app/cmdline.rs`, and no `apply_ortho` applied to a `Point`.

13. **`submit` never writes `app.last_cursor_world`.** The cursor belongs to the
    mouse; faking it from a typed point would corrupt the next direction. A
    test asserts `last_cursor_world` is unchanged across a typed commit.

14. **`Empty` keeps "Enter finishes the polyline" alive.** `submit` routes
    `egui::Key::Enter` to the active tool through
    `src/app/input.rs`'s existing helper, promoted to
    `pub(super) fn route_to_tool(app: &mut App, key: egui::Key)` — the
    `std::mem::take` dance is written once, not twice. This must not double-fire
    with `TOOL_ROUTED_KEYS`: when the field has focus the gate's `wants_kbd`
    early-out already suppresses the direct route, and a test asserts a single
    Enter while focused finishes a polyline exactly once.

15. **Rejection feedback.** When the tool returns `false`:
    - if the input was `Distance { along: None, .. }` → feedback
      `No direction for distance input — move the cursor or type X,Y.`
    - otherwise → feedback `<name> does not accept that input.` where `<name>`
      is `app.tool_manager.active_tool_name()` (e.g. `Select does not accept
      that input.`).
    In both cases the document, the history and the tool's phase are unchanged.

16. **`Camera::ZOOM_STEP`.** `pub const ZOOM_STEP: f64 = 1.25;` is added to
    `src/render/camera.rs`; `src/ui/menubar.rs`'s `do_zoom_in` / `do_zoom_out`
    and `src/app/cmdline.rs` all use it. No numeric zoom literal remains in
    either file. (`src/app/` must not import `crate::ui`, and does not today —
    the constant lives in `render`, which both may use.)

### C. Prompts

17. `status_text()` returns exactly these literals. Each is the R14 form: the
    command name, `Specify`, what is wanted, and a trailing colon.

    | tool / state | literal |
    |---|---|
    | `SelectTool` (all) | `Command:` |
    | `LineTool::Idle` | `LINE Specify first point:` |
    | `LineTool::WaitingSecondPoint` | `LINE Specify next point (Enter to finish):` |
    | `PolylineTool::Idle` | `PLINE Specify start point:` |
    | `PolylineTool::WaitingSecondPoint` | `PLINE Specify next point (Enter to finish):` |
    | `RectTool::Idle` | `RECT Specify first corner:` |
    | `RectTool::WaitingSecondCorner` | `RECT Specify opposite corner:` |
    | `CircleTool::Idle` | `CIRCLE Specify center point:` |
    | `CircleTool::WaitingRadius` | `CIRCLE Specify radius:` |
    | `ArcTool::Idle` | `ARC Specify start point:` |
    | `ArcTool::WaitingEnd` | `ARC Specify end point:` |
    | `ArcTool::WaitingMid` | `ARC Specify point on arc:` |
    | `MoveTool::Idle` | `MOVE Specify base point:` |
    | `MoveTool::WaitingDest` | `MOVE Specify destination point:` |

    `CircleTool` and `ArcTool` have **no `status_text` override today** and
    inherit `name()`, which is why they show no phase prompt; both gain one.
    `TrimTool`, `ExtendTool`, `DeleteTool` and `TextTool` keep their current
    literals (TEXT's are LCV-112's).

18. **The tests that assert on the old literals are updated to `assert_eq!`
    against the new ones**, not to looser substrings. Exactly these break and
    must be fixed: `src/tools/line.rs:146,153,192,213`
    (`contains("start")` / `contains("end")`), `src/tools/polyline.rs:160`
    (`starts_with("PLINE:")`), and `src/tools/manager.rs:379`
    (`active_status_text() == "Select"` → `"Command:"`). The assertions in
    `rect.rs` and `move_.rs` still pass; convert them to `assert_eq!` too so
    the prompt table has one authoritative encoding in the test suite.

19. `Tool::status_text`'s doc comment example (`src/tools/tool.rs:79`) is
    updated to a literal from the table, and `src/ui/command_line.rs`'s module
    header describes the strip as prompt + feedback + field.

### D. Focus, recall and feedback

20. **Four `App` fields**, each with a doc comment naming this demand:
    `pub command_history: CommandHistory`, `pub command_feedback: String`
    (empty = none), `pub focus_command_line: bool` (one-shot focus request),
    `pub command_line_focused: bool` (a plain mirror of the widget's
    `has_focus()`, written each frame by the widget). All default to
    empty/false in `App::default()`.

21. **Focus-on-typing** (ADR 0003 §E2). In `src/app/input.rs` the `Event::Text`
    branch — today `app.tool_manager.on_text_input(ch)` — becomes: when
    `!wants_kbd` **and** no shortcut fired this frame **and** the text is
    non-empty → append it to `app.command_line_input` and set
    `app.focus_command_line = true`. `src/ui/command_line.rs` consumes the flag
    **after** `ui.add(...)` with
    `if std::mem::take(&mut app.focus_command_line) { response.request_focus(); }`.
    The character lands **exactly once**: the widget cannot double-consume the
    same frame's text because it had no focus while it ran.

22. **The shortcut flag.** `dispatch_shortcuts` returns `bool` ("a shortcut
    consumed this key"); `process_shortcuts` folds the per-key results into one
    frame flag and passes it to `process_input(ctx, app, shortcut_fired)`. This
    is what stops `l` from both starting LINE and typing an `l` — a real
    keyboard emits `Event::Key` *and* `Event::Text` for one keystroke. No new
    keyboard reader is introduced: `dispatch_shortcuts` and `process_input`
    remain the only two (ADR 0002 §A6), `consume_key` stays banned, and
    `dispatch_shortcuts` must **not** be `#[must_use]` — `tests/lcv070.rs` and
    `tests/lcv104.rs` call it as a bare statement and must keep compiling
    unchanged.

23. **Recall.** `ArrowUp` / `ArrowDown` are read in `process_input` **before**
    the `if wants_kbd { return; }` early-out and act only when
    `app.command_line_focused` is true. Up replaces `command_line_input` with
    `command_history.older()` when it returns `Some`; Down with `newer()`,
    **clearing the field** when `newer()` returns `None` after a recall was in
    progress. No `consume_key`: egui 0.29.1's single-line `TextEdit` leaves its
    buffer alone on arrow keys.

24. **The feedback line.** `src/ui/command_line.rs` renders
    `app.command_feedback` immediately after the prompt label and before the
    `▶` separator, in a colour visually distinct from the prompt, and renders
    nothing when it is empty. It is a display string only — no control flow
    reads it. It is cleared at the top of every `submit` (AC 9) and by the
    widget's existing Escape branch, so a stale error is always dismissible.

25. **Escape**. Unchanged in behaviour and unchanged in mechanism: the gate
    routes Escape to the tool (ungated, even while typing), and the widget's
    own `lost_focus() + key_pressed(Escape)` branch clears the field. The widget
    reads a key **solely to disambiguate its own `lost_focus()`** (ADR 0003
    §E5) — never to dispatch.

26. **The gate table in `src/app/input.rs`'s module header is updated** to
    ADR 0003 §E4: the `typed characters → ToolManager::on_text_input` row is
    replaced by `typed characters → seed + focus the command line | Event::Text
    | no`, and a `command recall | ArrowUp, ArrowDown | only while the command
    line has focus` row is added. The `tool activation` row is corrected to the
    ten keys `L P R C A M E T X D` (it lists nine today, missing `D`).

### E. Structure and non-regression

27. **`src/app/mod.rs` stays ≤ 300 implementation lines.** It is at **293**
    today (the first `#[cfg(test)]` is line 294) and this demand adds ~18 (a
    `mod` line, a `pub use`, four documented fields, four initialisers). The
    sanctioned relief is to move `App::sync_dirty` and `App::mark_clean` (~36
    lines, `src/app/mod.rs:186-221`) into an `impl App` block in
    `src/app/autosave.rs` (44 implementation lines today) — they are autosave
    machinery living in the wrong file. **Do not move the five `action_*`
    wrappers**: LCV-113 moves them into `src/app/file_ops.rs` and two demands
    moving the same block is a guaranteed conflict. Do not move `App::commit`
    (LCV-112 decides its fate). Whichever of LCV-111 / LCV-113 lands second
    re-measures before choosing.

28. Every other touched file stays ≤ 300 implementation lines (first
    `#[cfg(test)]` attribute at column 0 of its own line is the boundary; inline
    tests do not count). Today: `src/tools/arc.rs` 154, `src/tools/move_.rs`
    195, `src/tools/rect.rs` 151, `src/ui/shortcuts.rs` 141,
    `src/app/viewport.rs` 190, `src/app/input.rs` 106. `src/app/cmdline.rs` is
    new and must itself stay under the cap.

29. No `unsafe`; no `unwrap()` / `expect()` in `src/`; `src/cmdline/` stays
    pure (no `egui`/`eframe`/`rfd`, and `ToolInput` adds no import to it beyond
    `crate::geometry`); `src/app/` still does not import `crate::ui`.

30. **No mouse behaviour regresses.** With ortho **off**, every tool draws by
    mouse exactly as before. With ortho **on**: LINE and PLINE are unchanged;
    MOVE, CIRCLE and ARC become ortho-constrained (decision 3, intended); RECT
    is **not** ortho-constrained and still draws a full box (decision 4).

31. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    `cargo test --all` all exit 0.

### F. The milestone check

32. **The roadmap sequence works.** With `App::default()`, `SelectTool` active,
    `ortho_enabled == false` and `last_cursor_world` seeded to `(200, 0)`
    (a hover to the right of the second point), submitting `l`, `0,0`,
    `@100,0`, `50` in that order produces **exactly two `Entity::Line`s** —
    `(0,0)→(100,0)` and `(100,0)→(150,0)` — with `history.len() == 2` and
    coordinates exact to 1e-9 mm.

33. **The no-direction case is a clean refusal.** The same sequence with
    `last_cursor_world == None` commits the first line, then `50` commits
    nothing, leaves `history.len() == 1`, leaves `LineTool` in
    `WaitingSecondPoint` (its prompt is still `LINE Specify next point (Enter to
    finish):`) and sets `command_feedback` to the AC 15 direction message.

34. **The mouse-free circle works**: `c`, `50,50`, `25` with
    `last_cursor_world == None` commits one circle, centre (50, 50), radius 25.

## Expected tests

**Quality bar (Marco 1).** Every acceptance criterion above is covered by an
automated test or tagged **[manual]** below. This demand touches input handling
and `App::update_ui`, so **every behavioural claim about focus, keys, submit or
prompts is proved by a headless test driving the real `App::update_ui(ctx)`**
through `tests/harness/mod.rs` — unit tests that call `dispatch_shortcuts` or
`on_command_input` directly prove the function is correct and prove nothing
about whether it is reachable, which is the exact blind spot ADR 0002 was
written to close and the reason ten "Done" features shipped non-functional in
the May-2026 drive. Follow ADR 0003 §F2 (the two recipes) and §F3 (the five
egui 0.29.1 traps) rather than rediscovering them — in particular **trap 5**:
Enter only submits if the field had focus in the *previous* frame, so every
command-line test is at least two frames, and a one-frame Enter tap is a silent
false negative that looks exactly like a broken parser. egui is pinned at
**0.29.1**; do not upgrade, and `egui_kittest` (needs ≥ 0.30) stays out of
scope.

- **Harness (ADR 0003 §F1)** — extend `tests/harness/mod.rs` additively with
  `text_events(s)`, `type_command(ctx, app, s)` (one frame per character) and
  `submit_command(ctx, app, s)` (`type_command` + an Enter `tap`), and add
  trap 5 to the module header beside the three rules already there. Existing
  helpers `SCREEN`, `key_events`, `raw_input`, `frame`, `tap` keep their
  signatures; `tests/lcv070.rs`, `tests/lcv104.rs` and every other existing
  test must still pass untouched.
- **Unit — `ToolInput` (AC 1)** in `src/cmdline/mod.rs`:
  `as_point_returns_the_point`, `as_point_returns_along_for_distance`,
  `as_point_is_none_without_a_direction`.
- **Unit — the tools (AC 3, 4, 5)** inline in each tool file:
  `command_point_acts_exactly_like_a_click` for each of the six (assert the
  committed entity's coordinates and the resulting state, and assert the same
  result as a literal `on_pointer_down` call);
  `circle_distance_is_the_radius` and `circle_rejects_a_non_positive_radius`;
  `circle_distance_is_ignored_while_idle`;
  `anchor_follows_the_last_fixed_point` for Move / Circle / Arc (both arc
  phases); `rect_has_no_anchor_so_ortho_cannot_flatten_it` (AC 5, named for
  decision 4); `select_trim_extend_delete_reject_command_input` asserting the
  `false` default and an unchanged document.
- **Unit — resolution (AC 10, 11, 12, 13)** inline in `src/app/cmdline.rs`,
  calling `submit` on an `App::default()` with no `Context` where the path
  allows: `absolute_point_commits_at_exact_coordinates`,
  `relative_adds_to_the_anchor`, `relative_without_an_anchor_is_refused`
  (document unchanged **and** feedback set), `distance_projects_along_the_cursor_direction`,
  `distance_uses_the_ortho_direction_when_f8_is_on`,
  `distance_without_a_cursor_is_refused`,
  `distance_with_cursor_equal_to_anchor_is_refused`,
  `typed_point_does_not_move_last_cursor_world`,
  `typed_point_ignores_snap` (enable snap, place an entity whose endpoint is
  near the typed point, assert the committed coordinate is the typed one),
  `toggles_match_f3_f7_f8`, `zoom_in_uses_the_same_step_as_the_view_menu`
  (assert `Camera::ZOOM_STEP` and equal resulting `mm_per_px` for both paths),
  `unknown_sets_feedback_and_mutates_nothing`,
  `rejected_input_is_still_pushed_to_the_ring`,
  `empty_is_not_pushed_to_the_ring`.
- **Integration — `tests/lcv111.rs`** with `mod harness;`, all through the real
  frame body:
  - `roadmap_sequence_draws_two_lines` (AC 32) — the milestone check, using the
    honest per-character path for at least one of the four commands.
  - `no_direction_is_a_clean_refusal` (AC 33) and
    `mouse_free_circle_commits` (AC 34).
  - One test per wired tool asserting the committed entity's coordinates from
    typed input: LINE, PLINE, RECT (absolute only), CIRCLE, ARC, MOVE.
  - `relative_move_hands_back_to_select` (AC 6) — `m` ⏎ `0,0` ⏎ `@10,0` ⏎ with
    a selection, asserting the entity moved **and**
    `active_tool_name() == "Select"`.
  - `typing_l_while_unfocused_starts_line_and_types_nothing` and
    `typing_l_while_focused_types_an_l_and_does_not_start_line` (AC 21, 22) —
    the double-dispatch pair; the second seeds focus with `z` first, per
    ADR 0003 §E1's `ze` example.
  - `typing_a_digit_focuses_and_seeds_the_field` — one character, asserting
    `command_line_input == "5"` exactly once (not `"55"`).
  - `enter_on_an_empty_field_finishes_the_polyline_exactly_once` (AC 14) —
    asserting the polyline committed its segments and the tool is back to
    `Idle`, with no double route.
  - `arrow_up_recalls_and_arrow_down_clears` (AC 23), driven through
    `App::update_ui` with the field focused, plus
    `arrows_do_nothing_while_the_field_is_unfocused`.
  - `escape_clears_the_field_and_the_feedback_and_cancels_the_tool` (AC 24, 25).
  - `typed_commit_marks_the_document_dirty` (AC 7) — asserting `dirty_since`
    becomes `Some` without letting the 800 ms debounce elapse (trap 2: never
    assert on disk).
  - `prompt_follows_the_active_phase` (AC 17) — drive LINE and ARC through
    their phases by typed input and `assert_eq!` the prompt literal at each
    step, which is the only test that proves the operator can see where they
    are.
- **[manual] smoke** — `cargo run`, then, in one session: (a) type `50,25` ⏎
  with SELECT active and confirm the feedback reads `Select does not accept that
  input.` and nothing is drawn; (b) run the AC 32 roadmap sequence with the
  mouse resting to the right and confirm two connected segments and that the
  prompt changes at every step; (c) press F8, draw a rectangle with the mouse,
  and confirm it is still a full rectangle and not a flat line (decision 4);
  (d) press F8 and drag a MOVE and a CIRCLE and confirm both are now
  axis-constrained (decision 3); (e) type `snap` ⏎ and confirm the status-bar
  SNAP badge flips exactly as F3 does; (f) type `ze` ⏎ and confirm zoom
  extents; (g) press ArrowUp repeatedly in the focused field and confirm the
  last commands come back in order.

## Risks

- **TEXT string entry is non-functional between this demand and LCV-112.**
  This demand repurposes the `Event::Text` gate row, so `TextTool`'s
  per-character typing stops working the moment it lands, and LCV-112 restores
  it through the command line. That is an accepted, bounded regression in an
  unreleased milestone — but it means **LCV-112 must land in the same
  milestone, and `Tool::on_text_input` must not be deleted here** (LCV-112
  deletes it, together with `TextTool::on_text_input`, once nothing needs it).
  A merge that ships LCV-111 without LCV-112 ships a broken TEXT tool.
- **`text` cannot be typed from an unfocused command line**, because `t` is
  bound to TRIM (ADR 0003 §E1: the tool shortcut wins). The operator reaches
  TEXT with the `D` key, the toolbar or the Tools menu, or by focusing the
  field first. This is the visible cost of the bare-letter carve-out and the
  strongest argument for the §E6 switch — which is why the switch gets its own
  demand rather than being smuggled in here.
- **Ortho now applies to MOVE, CIRCLE and ARC** (decision 3). Anyone who has
  used v2 with F8 on will read this as a regression; it is intended, it is
  R14-correct, and AC 30 plus the manual smoke are the record.
- **The caret after a recall** may sit at position 0 rather than at the end of
  the recalled text, because the buffer is replaced behind egui's back.
  Accepted for Marco 1: the operator's next act is almost always Enter or
  select-all. Do not add a second key reader to fix it.
- **Double dispatch is the failure mode to watch.** If `typing_l_while_unfocused…`
  and its focused twin are not both present and both green, the shortcut flag
  (AC 22) is not actually threaded and `l` will either start LINE *and* type an
  `l`, or do neither.
- **`src/app/mod.rs` and `src/ui/menubar.rs` are contended** with LCV-113 /
  LCV-114 / LCV-115 / LCV-116. AC 27 fixes exactly which block this demand may
  move.

## Open questions

*(none — demand is Ready)*

## Notes

- Binding contract: [ADR 0003](../../adr/0003-command-line-input-contract.md)
  §B (resolution, `ToolInput`, the canonical body, feedback), §C (prompts stay a
  pull through `status_text`), §E (focus, the amended gate table, §E6), §F
  (test recipes, harness additions, the five traps, F4 minimum coverage).
  ADR 0002 is **extended, not superseded**: its A1–A6 and B stand.
- Prompt wording follows AutoCAD R14's transcript (`Specify first point:`,
  `Specify next point or [Undo]:`) with the command name prefixed, because v2's
  strip shows one line and not a scrolling history. `Command:` is R14's idle
  prompt verbatim.
- `status_text()` has exactly one consumer today,
  `src/ui/command_line.rs:29` via `ToolManager::active_status_text`. The status
  bar shows coordinates, the tool name and the mode badges and is **not**
  touched by this demand.
- The v1 flow this reproduces is `../LaserCAD-R14/src/ui/command-line.ts`
  plus `../LaserCAD-R14/src/tools/*.ts`; v1 resolved `@` and distances inside
  the command-line module, which is the same single-place decision as
  `src/app/cmdline.rs`.
- `EPSILON` is `1e-9` mm (`src/geometry/`). Millimetres are canonical, angles
  in the kernel are radians, world Y is up.
- The follow-up this demand deliberately defers: **split `Tool::anchor()` into
  an ortho anchor and a coordinate base point**, so RECT can accept `@dx,dy`
  without losing mouse ortho. Route to `architect`; it is a trait change, not a
  product tweak.
