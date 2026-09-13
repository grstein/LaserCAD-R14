# LCV-112 — TEXT complete: raw-input mode for the string, then the height, in exactly one undo step

- **Status**: Done
- **Phase**: 11
- **Depends on**: LCV-111 (`ToolInput`, `src/app/cmdline.rs::submit`, the focus plumbing)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: 2ef12af, 307cfad — feat(LCV-112): TEXT tool raw-input mode — string, then height, one undo step; test(LCV-112): cover the final-commit focus release; fix a weakened vehicle. Reviewed twice: first pass returned REWORK REQUIRED over a missing focus-release test; the rework added it; second pass returned APPROVED after re-running both mutations in a scratch worktree.

## Problem

The TEXT tool is half a tool, and after LCV-111 it is none of one.

Today `TextTool` builds its string one `char` at a time from the raw
`Event::Text` stream (`src/tools/text.rs:83`), handles its own Backspace, and
commits at a **hard-coded 5 mm height** that the operator cannot change from
anywhere in the UI. There is no cursor, no selection, no paste, and no way to
say "make it 10 mm". For engraving work that is the whole job: a part label
that has to fit a 12 mm boss, a fiducial marked at a legible size, an ID
etched at whatever the laser can resolve. A text tool whose size is a constant
in the source is not a text tool.

LCV-111 then repurposes the `Event::Text` gate row to seed and focus the
command line, which stops `TextTool::on_text_input` from ever being called.
Between LCV-111 and this demand, TEXT cannot type at all. **This demand is not
optional and must land in Marco 1** — it is what makes the regression LCV-111
introduces a one-demand window rather than a shipped defect.

The fix is the R14 flow, which is also the simplest one: click the insertion
point, type the string into the command line (a real text field, with a caret,
arrow keys and paste, for free), press Enter, type a height or press Enter for
5 mm, and the whole string appears as one undoable object.

## Scope

- **`Tool::wants_raw_input` and `Tool::on_raw_input`** (ADR 0003 §D), both with
  default bodies, so only `TextTool` implements them.
- **`src/app/cmdline.rs::submit` step 1**: when the active tool wants raw
  input, forward the string unparsed and return — no parse, no ring push.
- **`src/ui/command_line.rs` holds focus** every frame while the active tool
  wants raw input.
- **`TextTool` rebuilt** on the three-state machine
  `Idle → WaitingText → WaitingHeight`, with the height prompt, the 5 mm
  default, the validated range and the invalid-input retry.
- **`TextTool` commits through `history.commit`**, dropping its `App::commit`
  call.
- **Deletions**: `TextTool::on_text_input`, `TextTool`'s Backspace arm,
  `Tool::on_text_input`, and `ToolManager::on_text_input` — the trait method
  loses its last implementor and goes with it.
- **The TEXT prompts**, per the table in AC 6.

## Out of scope

- **Remembering the last height as the next default** (R14 shows
  `Specify height <10>:` after a 10 mm text). That needs an interpolated
  prompt, i.e. `status_text` returning `Cow<'static, str>`, which
  ADR 0003 §C defers to a revisit criterion. The default is always 5 mm and
  the prompt literal always says `<5>`.
- **A live per-character preview of the string.** The `TextEdit` owns the
  editing now, so the tool does not see the string until Enter. See AC 9 and
  §Risks: the preview appears at the height phase, which is still before the
  commit.
- **Text style, font choice, rotation, justification, mirroring, multi-line
  text (MTEXT), a text-properties dialog, editing an existing text object.**
  Hershey single-stroke at `spacing_factor = 1.0`, left-aligned on the
  baseline, horizontal — exactly what `layout_text` does today.
- **Changing `src/text/`.** `layout_text`, `CAP_HEIGHT_HERSHEY` and the
  Hershey tables are untouched.
- **Removing `Backspace` from `TOOL_ROUTED_KEYS`.** It looks like TextTool's
  key, but `DeleteTool::on_key` and `SelectTool::on_key` both handle it; the
  gate row stays.
- **Deleting `App::commit`.** It loses its last *tool* caller here, but
  `src/io/file_actions.rs`, `tests/lcv103.rs`, `tests/lcv070.rs` and
  `tests/app_tool.rs` use it as a setup helper. Only its doc comment changes.
- **A `text` alias reachable from an unfocused command line.** `t` is bound to
  TRIM (ADR 0003 §E1), so TEXT is entered with the `D` key, the toolbar, the
  Tools menu, or by focusing the field first. That carve-out belongs to the
  ADR 0003 §E6 demand, not to this one.

## Product decisions (do not re-open these)

1. **Raw means raw.** While `wants_raw_input()` is true, `submit` does not
   parse and does not consult the alias table: typing the word `line` inserts
   the word `line` as geometry, it does not switch tools. Nothing typed in raw
   mode reaches the recall ring either — the ring is a command transcript, and
   `HELLO` is not a command.
2. **Height range: 0.1 mm to 2000 mm inclusive.** The floor is one laser kerf:
   below it the Hershey strokes of adjacent glyphs merge into an unreadable
   scorch, so the operator gets a refusal instead of a ruined part. The ceiling
   is `BED_MAX_MM` (LCV-114 fixes the bed at 1..2000 mm), because text taller
   than the largest possible bed cannot be cut and would wreck zoom-extents for
   the rest of the session. ADR 0003 §D says "`parse_number` > 0"; this range
   is a subset of that and tightens it, it does not contradict it.
3. **Invalid height re-prompts; it never guesses and never clamps.** A value
   that will not parse, is non-finite, or is outside the range leaves the tool
   in `WaitingHeight` with nothing committed, the entered text intact, and a
   prompt that states the range. Clamping 5000 to 2000 would silently produce
   text the operator did not ask for.
4. **Empty height means 5 mm**, because Enter-to-accept-the-default is the R14
   reflex and the bracketed `<5>` in the prompt is the promise.
5. **Empty text cancels.** Submitting an empty (or whitespace-only) string at
   the `TEXT Enter text:` prompt returns the tool to `Idle` with nothing
   committed and no anchor kept. Committing zero glyphs would put an empty
   undo entry on the stack.
6. **Exactly one undo entry per text object.** Nothing is committed until the
   height is accepted; the acceptance commits a single `CreateEntities`
   carrying every glyph stroke of the whole string. One Ctrl+Z removes the
   entire text.
7. **One number grammar.** The height is parsed with `cmdline::parse_number`
   (LCV-110), not an ad-hoc `str::parse::<f64>`. `10,5` is therefore not a
   height — the comma is a coordinate separator everywhere in the product.

## Acceptance criteria

### The trait surface

1. `src/tools/tool.rs` gains two methods with default bodies:

   ```rust
   /// True while this tool wants the command line as a free-text field.
   fn wants_raw_input(&self) -> bool { false }
   /// The submitted text, unparsed. Returns true if consumed.
   fn on_raw_input(&mut self, _raw: &str, _doc: &mut Document, _history: &mut History) -> bool { false }
   ```

   `ToolManager` forwards both. `TextTool` is the only implementor; every other
   tool keeps the defaults and is untouched.

2. `src/app/cmdline.rs::submit` begins: if
   `app.tool_manager.wants_raw_input()` → call `on_raw_input(raw, &mut
   app.document, &mut app.history)`, poll succession exactly as the typed-point
   path does, and **return before `parse`**. The raw string is **not** pushed
   to `command_history`, and `command_feedback` is left empty.

3. `src/ui/command_line.rs` calls `response.request_focus()` every frame while
   `app.tool_manager.wants_raw_input()` is true, in the same place it consumes
   the `focus_command_line` one-shot. This is why raw mode needs **no**
   keyboard-gate exception: with the field focused, `ctx.wants_keyboard_input()`
   is true and the existing gate already suppresses the bare tool keys. The
   gate table in `src/app/input.rs` is **not** modified by this demand.

### The TEXT state machine

4. `TextToolState` becomes exactly three variants:

   ```rust
   Idle,
   WaitingText { anchor: Vec2 },
   WaitingHeight { anchor: Vec2, text: String, invalid: bool },
   ```

   `wants_raw_input()` is `false` in `Idle` and `true` in the other two.

5. **Transitions.**

   | from | event | to | commits |
   |---|---|---|---|
   | `Idle` | `on_pointer_down(p)` | `WaitingText { anchor: p }` | no |
   | `WaitingText` | `on_pointer_down` | `WaitingText` (anchor does **not** move) | no |
   | `WaitingText` | `on_raw_input(s)`, `s.trim()` non-empty | `WaitingHeight { text: s (verbatim), invalid: false }` | no |
   | `WaitingText` | `on_raw_input(s)`, `s.trim()` empty | `Idle` | no (decision 5) |
   | `WaitingHeight` | `on_raw_input("")` | `Idle` | **yes**, at 5.0 mm |
   | `WaitingHeight` | `on_raw_input(s)`, `parse_number(s) == Some(h)`, `0.1 <= h <= 2000.0` | `Idle` | **yes**, at `h` |
   | `WaitingHeight` | any other `s` | `WaitingHeight { invalid: true }`, text kept | no (decision 3) |
   | any | `on_key(Escape)` / `cancel()` | `Idle` | no |

   `on_raw_input` returns `true` in every row where the tool was in
   `WaitingText` or `WaitingHeight` — including the invalid-height row, so the
   app never reports the input as unhandled — and `false` in `Idle`.

6. **Prompts** (`status_text`, still `&'static str`, still pulled per frame):

   | state | literal |
   |---|---|
   | `Idle` | `TEXT Specify start point:` |
   | `WaitingText` | `TEXT Enter text:` |
   | `WaitingHeight { invalid: false }` | `TEXT Specify height <5>:` |
   | `WaitingHeight { invalid: true }` | `TEXT Height must be between 0.1 and 2000 mm. Specify height <5>:` |

   The `invalid` flag exists precisely so the retry message is a second
   `&'static str` rather than a formatted `String` — no `Cow`, no allocation,
   no `App::command_feedback` (which `on_raw_input` cannot reach anyway).

7. **The commit.** On an accepted height the tool builds
   `layout_text(&text, anchor, height_mm, 1.0)` and, when the result is
   non-empty, commits **one** `Box<dyn Command>` — `CreateEntities` with every
   stroke — via `history.commit(cmd, doc)`. It no longer calls `App::commit`,
   so `TextTool` stops needing `&mut App` to mutate anything (LCV-041
   alignment; ADR 0002 §B's revision-based dirty signal covers both paths, so
   nothing is lost). If `layout_text` returns an empty vector — e.g. a string of
   characters with no Hershey glyphs — **nothing is committed** and the state
   still returns to `Idle`.

8. **Height is applied exactly.** `TextTool::height_mm` is set from the
   accepted value before layout. For a 10 mm `H` placed at anchor `(0, 0)`, the
   committed strokes span `y ∈ [0.0, 10.0]` to within 1e-9 mm — `'H'` occupies
   Hershey rows `hy ∈ [-9, 0]` (baseline to cap) and
   `scale = height_mm / CAP_HEIGHT_HERSHEY` with `CAP_HEIGHT_HERSHEY = 9.0`.
   Do **not** use `'O'` as the oracle: it spans `hy ∈ [-9, +9]` and descends
   below the baseline.

9. **Preview.** `preview()` returns an empty vector in `Idle` and in
   `WaitingText` (the string lives in the `TextEdit`, and the tool does not see
   it until Enter), and in `WaitingHeight` returns
   `layout_text(&text, anchor, 5.0, 1.0)` — the placement preview at the
   default height, shown while the operator decides the real one.

### Deletions

10. `TextTool::on_text_input`, `TextTool`'s `egui::Key::Backspace` arm,
    `Tool::on_text_input` and `ToolManager::on_text_input` are **deleted**.
    `grep -rn "on_text_input" src/ tests/` returns nothing. The trait's method
    count goes down by one net across LCV-111 + LCV-112
    (`+wants_raw_input`, `+on_raw_input`, `−on_text_input`, and
    `on_command_input` re-signed).

11. `TextTool::on_key` keeps only its `Escape` arm (`cancel()`), and its `app:
    &mut App` parameter becomes unused — rename to `_app`; do not change the
    trait signature. `src/tools/text.rs` may keep its `use crate::app::App;`
    solely for that signature.

12. `App::commit`'s doc comment (`src/app/mod.rs:175-182`) — "TextTool is the
    only caller of this method" — is corrected: after this demand **no tool
    calls it**; its remaining callers are test and fixture code. The method
    itself stays (§Out of scope).

13. The tests that assert deleted behaviour are removed or rewritten, not
    commented out: `src/tools/text.rs`'s `status_text_both_states` (new
    literals, as `assert_eq!` against the AC 6 table), its Backspace tests
    (`text.rs:206-211`), and every test that drives the tool with
    `on_text_input`. `src/app/input.rs`'s inline `TextTool` typing test (if
    LCV-111 has not already replaced it) goes too.

### Structure and gates

14. `src/tools/text.rs` and every other touched file stay **≤ 300
    implementation lines** (lines before the first `#[cfg(test)]`); `text.rs`
    is 126 today. No `unsafe`, no `unwrap()` / `expect()` in `src/`.
    `src/cmdline/` stays pure; `src/tools/` still imports no `eframe` and no
    `rfd`.

15. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`
    and `cargo test --all` all exit 0.

### The milestone check

16. **The roadmap flow works end to end.** Activate TEXT (press `D`, or click
    the Text button), click the insertion point at `(0, 0)`, type `HELLO`,
    press Enter, type `10`, press Enter. Result: five glyphs' worth of
    `Entity::Line` strokes in the document, spanning `y ∈ [0.0, 10.0]` exactly
    (AC 8), `history.len() == 1`, and **one Ctrl+Z leaves the document empty**.

17. **The default works**: the same flow with Enter pressed at the height
    prompt commits text of height 5.0 mm — `y ∈ [0.0, 5.0]` for `H` — still in
    one undo step.

18. **The refusal works**: at the height prompt, submitting `abc`, then `0`,
    then `5000`, then `10,5` each leaves the document empty, leaves the tool in
    `WaitingHeight` with the text intact, and shows the AC 6 retry prompt; a
    following `10` ⏎ then commits normally. The operator never loses the string
    they typed.

## Expected tests

**Quality bar (Marco 1).** Every acceptance criterion above is covered by an
automated test or tagged **[manual]** below. This demand changes input routing
and the frame body, so **the whole TEXT flow is proved headlessly through the
real `App::update_ui(ctx)`** via `tests/harness/mod.rs` — including the pointer
click, which is why ADR 0003 §F3 trap 3 matters here specifically: a pointer
test needs a warm-up frame carrying `PointerMoved` alone before the frame
carrying `PointerButton`, or the widget rect is never registered, the viewport
handler never runs, and the anchor is never set. Trap 5 applies to every Enter:
the field must have had focus in the *previous* frame, so each submit is at
least two frames. Trap 1 forbids `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S`; `Ctrl+Z`
is safe and AC 16 needs it. Trap 2 forbids letting the 800 ms autosave debounce
elapse. Use `submit_command` / `type_command` from LCV-111's harness additions.
egui is pinned at **0.29.1**; do not upgrade, and `egui_kittest` (needs ≥ 0.30)
stays out of scope.

- **Unit — the state machine (AC 4, 5)** inline in `src/tools/text.rs`, driving
  `on_pointer_down` / `on_raw_input` directly:
  `idle_wants_no_raw_input_then_both_waiting_states_do`,
  `click_anchors_and_a_second_click_does_not_move_it`,
  `non_empty_text_advances_to_the_height_prompt`,
  `empty_text_cancels_and_commits_nothing`,
  `whitespace_only_text_cancels_and_commits_nothing`,
  `escape_from_either_waiting_state_returns_to_idle_uncommitted`,
  `on_raw_input_returns_false_only_when_idle`.
- **Unit — the height (AC 5, 7, 8, decisions 2–4, 7)**:
  `empty_height_commits_at_five_millimetres`,
  `valid_height_commits_at_that_height`,
  `height_below_the_floor_is_refused` (`0`, `0.05`, `-3`),
  `height_above_the_ceiling_is_refused` (`2000.1`, `1e9`),
  `height_boundaries_are_inclusive` (`0.1` and `2000` both commit),
  `unparseable_height_is_refused` (`abc`, `` `10mm` ``, `nan`, `inf`),
  `comma_is_not_a_decimal_point_in_a_height` (`10,5` refused — the locale rule),
  `a_refused_height_keeps_the_text_and_sets_the_retry_prompt`,
  `a_refused_height_then_a_valid_one_commits_normally`,
  `ten_millimetre_h_spans_exactly_ten_millimetres` (the AC 8 oracle, `'H'`,
  with a comment saying why not `'O'`),
  `commit_is_a_single_command` (`history.len() == 1` after a five-letter
  string), `layout_with_no_glyphs_commits_nothing`.
- **Unit — prompts (AC 6)**: `prompt_matches_each_state`, four `assert_eq!`
  against the literals, including the `invalid: true` retry.
- **Unit — preview (AC 9)**: `preview_is_empty_until_the_height_prompt`,
  `preview_at_the_height_prompt_uses_the_default_height`.
- **Unit — deletions (AC 10)**: the `grep` for `on_text_input` recorded in the
  commit message; the compiler proves the rest.
- **Integration — `tests/lcv112.rs`** with `mod harness;`, every test through
  the real frame body:
  - `text_flow_commits_ten_millimetre_text_in_one_undo_step` (AC 16) — activate
    TEXT by key, warm-up frame + click, `submit_command("HELLO")`,
    `submit_command("10")`, assert the y-extent and `history.len() == 1`, then
    `tap(Ctrl+Z)` and assert the document is empty.
  - `enter_at_the_height_prompt_uses_five_millimetres` (AC 17).
  - `invalid_height_keeps_the_text_and_re_prompts` (AC 18), ending with a valid
    height that commits.
  - `raw_mode_holds_focus` (AC 3) — after the anchor click, assert
    `app.command_line_focused` is true on the following frame **without** the
    operator typing anything, and that pressing `l` in raw mode types an `l`
    into the field instead of activating LINE (the gate reuse this demand
    depends on).
  - `raw_input_is_not_parsed` (decision 1) — `submit_command("line")` at the
    text prompt advances to the height prompt with
    `active_tool_name() == "TEXT"`, and the committed geometry is the word
    "line".
  - `raw_input_is_not_recalled` (decision 1) — after the flow,
    `app.command_history.len()` is unchanged by `HELLO` and `10`; ArrowUp does
    not bring back `HELLO`.
  - `escape_mid_flow_commits_nothing_and_clears_the_field`.
  - `text_commit_marks_the_document_dirty` — `dirty_since` is `Some`, asserted
    without touching the disk (trap 2).
- **[manual] smoke** — `cargo run`: press `D`, click in the viewport, type
  `HELLO`, Enter, type `10`, Enter; confirm the text is drawn at the click
  point at a plausible 10 mm, that the prompt read `TEXT Enter text:` and then
  `TEXT Specify height <5>:`, and that one Ctrl+Z removes the whole word.
  Repeat with `5000` at the height prompt and confirm the range message
  appears and the string is not lost. Then export to SVG and confirm the text
  strokes appear as `<path>`/`<line>` elements in the same document units
  (mm) — TEXT is only useful if it reaches LaserGRBL.

## Risks

- **The live per-character preview is lost.** LCV-048 previewed the string as
  it was typed; the `TextEdit` now owns the buffer, so the preview appears one
  Enter later, at the height phase (AC 9). This is inherent to ADR 0003 §D and
  is the price of getting a caret, arrow keys and paste for free. Do not try to
  restore it by reading `App::command_line_input` from the tool — `preview()`
  has no `App` and must not get one.
- **Ordering with LCV-111.** This demand assumes `src/app/cmdline.rs`,
  `ToolInput` and the focus fields already exist. Landing it first is not
  possible; landing LCV-111 without it ships a TEXT tool that cannot type.
- **`request_focus()` every frame** is a hammer. If it fights another widget
  (the agent panel's input, a modal), the symptom is a field that cannot be
  left. The `raw_mode_holds_focus` test plus the manual smoke are the guard;
  Escape is always the way out, because the gate routes it ungated.
- **`'O'` as a test oracle will silently pass a wrong scale** — it spans
  `hy ∈ [-9, +9]`, so its y-extent is twice the cap height. AC 8 names `'H'`
  for exactly this reason.
- **`layout_text` returns empty for `height_mm <= 0`**, which the AC 5 range
  already excludes; the AC 7 empty-vector guard is belt and braces for glyphless
  strings, not a second height check.

## Open questions

*(none — demand is Ready)*

## Notes

- Binding contract: [ADR 0003 §D](../../adr/0003-command-line-input-contract.md)
  (the two methods, the state table, the single-undo rule, "height parsing uses
  `cmdline::parse_number`"), §B5 step 1 (submit skips the parser), §F3 traps
  1–5, §F4 ("the full TEXT flow ending in `history.len() == 1` and one Ctrl+Z
  clearing the document").
- `layout_text(text, origin, height_mm, spacing_factor)` is
  `src/text/layout.rs`; it returns an empty `Vec` for `height_mm <= 0`, scales
  by `height_mm / CAP_HEIGHT_HERSHEY` with `CAP_HEIGHT_HERSHEY = 9.0`, and
  places glyph rows at `py = origin.y + (-hy) * scale`. `spacing_factor` stays
  `1.0`.
- `BED_MAX_MM = 2000.0` comes from LCV-114; if that demand changes the bed
  ceiling, this range follows it in the same change.
- v1 reference: `../LaserCAD-R14/src/tools/text.ts` prompted for the string and
  then the height through the same command line, with 5 mm as the default —
  this demand reproduces that flow, including the default.
- Millimetres are canonical; the height is a millimetre value and is never
  converted, rounded or clamped on its way to `layout_text`.
