# LCV-134 — The F1 dialog has 8pt of headroom and the next tool needs 21

- **Status**: Ready
- **Phase**: 11
- **Depends on**: LCV-133 (Done — it landed the two-column layout this demand sizes). **Ordering, not a dependency**: this must land **before any demand that adds a tool to `TOOLS` or a row to `SHORTCUT_GROUPS`**. See §Notes.
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

LCV-133 fixed the fold: all eight groups of the F1 dialog are painted, inside
the clip, at 1280×800, 1024×600 and 800×600. What it did not buy is room. The
shipped content clears the body clip bottom by **8.00pt**, identically at all
three sizes, and one shortcut row costs **21.00pt** of vertical pitch. **The
next tool anybody adds to `TOOLS` puts a row back under the fold**, and the
tests LCV-133 shipped go on calling the dialog correct.

That is not a prediction. Measured at `14708d3` by adding one real `ToolEntry`
to `TOOLS` in a throwaway copy of the repo and rendering the real `App` on a
settled frame at `pixels_per_point = 1.0`:

| screen | deepest column slack | runs painted / required | what the operator sees |
|---|---|---|---|
| 1280×800 | **−13.00** | 66 / 66 | `Ctrl+Y` / `Redo` painted at `y = 653.68`, height 14.0, clip bottom 654.68 |
| 1024×600 | **−13.00** | 66 / 66 | the same row, the same way |
| 800×600 | **−13.00** | 66 / 66 | the same row, the same way |

One point of that row is on screen. The `Edit` group reads as *Undo, and
nothing else* — and the row that goes is not the row you added, which is why
nobody looks. What the suite says about it, in the same copy:

- `tests/lcv133_shortcuts_dialog_fits.rs`: **four of five green**, including all
  three of LCV-133 AC 1/AC 2's size tests and its AC 3 derived run count (66 = 66, because
  a straddling run is still a painted run).
- The fifth, `ac2_containment_check_is_not_decorative`, goes red — and it is a
  **negative control at 1280×460**, not a report of the defect. Its message
  reads *"if it is now fully contained, this screen height no longer
  demonstrates the case and must be shrunk further"*. It tells you to tune the
  probe. It does not mention the clipped row, and at the three real sizes it
  asserts nothing. It is not specific to `+1 tool` either: the same message
  fires at `+2 tools` (re-measured at `2860edd`: 3 pass, 2 fail), and on any
  growth that shifts content past its pin. **AC 8 fixes that message.**
- The four other reds are the tool-inventory counts
  (`tool_rows_match_the_toolbar_table`, `toolbar_table_is_the_v010_tool_set`,
  `tools_make_covers_all_toolbar_entries`,
  `tools_menu_entries_activate_their_tool`). Each says "you added a tool, update
  the number". None of them is about the dialog.

So the whole signal available to the person who adds the eleventh tool is
"update four counts and shrink a probe". They will. And the dialog goes back to
hiding content, which is the defect LCV-126 shipped with and LCV-133 was opened
to fix.

Growth past that, measured the same way (painted / required, where required is
one run per heading and two per row, derived from `SHORTCUT_GROUPS` and
`tool_rows()`):

| growth | left slack | right slack | painted / required | which LCV-133 criterion fails |
|---|---|---|---|---|
| shipped | +8.00 | +32.00 | 64 / 64 | nothing — correct |
| **+1 tool** | **−13.00** | +32.00 | 66 / 66 | **nothing that names the dialog** |
| +2 tools | −13.00 | +32.00 | 66 / 68 | its AC 3 — **plus the same misdiagnosing control** |
| +3 tools | +17.00 | −21.00 | 68 / 70 | its AC 2 (`Help` outside the clip) and its AC 3 |
| +1 group of 4 rows | +8.00 | −10.00 | 68 / 73 | its AC 1 (`View` painted zero times), AC 2, AC 3 |

The cap doing this is not a screen-size problem and cannot be escaped by buying
a monitor: the body clip is **426.00pt at every screen size tested**, because
`Window::new` bakes `.default_size([340.0, 420.0])` and, for a body that is a
`ScrollArea`, that is a *maximum* (ADR 0009 §Context, egui 0.29.1).

## Scope

All five parts, in one demand, because each is inert without the others. The
assertions alone land red on day one — today's content is 13pt short of the
21pt floor. The sizing call alone lands unguarded: measured, it buys +45.32pt at
600-high screens, two added tools bring that back to +20.32pt, and nothing in
the suite would say so — or would notice the call being deleted as gratuitous.

- **The pre-decided file split, as the first commit.** `src/ui/dialogs.rs` is at
  **299** implementation LOC (ADR 0004 rule 2's `awk` recipe) and the sizing call
  alone takes it to **300** — measured. ADR 0004 Amended (2) already decided the
  seam and the trigger; this demand executes it. See AC 1.
- **The sizing call**: `.default_height(ctx.screen_rect().height() - 80.0)` on
  the shortcuts `Window`, **on top of** the two columns and the `ScrollArea`,
  both of which stay exactly as LCV-133 landed them.
- **Three assertions** on the dialog body, at 1280×800, 1024×600 and 800×600:
  every non-empty run contained, a named minimum slack, and the window rect
  inside the screen rect.
- **Two growth mutations**, run and recorded in the handover: +3 tools, and +1
  group of four rows. The second of those is LCV-133's AC 5 probe as amended
  (ADR 0009 decision 3), run against a dialog that can pass it honestly.
- **One failure message**: the negative control in
  `tests/lcv133_shortcuts_dialog_fits.rs` currently sends a reader to weaken the
  control when the real cause is content growth, and following it manufactures a
  green suite over a sliced row. See AC 8.

## Out of scope

- **A third column.** Measured at `14708d3`, three balanced columns, no sizing
  call: the window is **1195pt wide**. At 1280×800 it paints 64/64 (window
  `[43, 1238]`). At 1024×600 the window is `[−86, 1109]` — 171pt wider than the
  screen, both outer columns hanging off the edges. At 800×600 it paints **36 of
  64 runs** (window `[−198, 997]`), the third column cut off horizontally. It
  trades a vertical clip for a horizontal one at exactly the two sizes that
  motivated LCV-133. **Rejected on measurement**, and the width argument does not
  depend on how the sections are balanced.
- **`.max_size()`, `.resizable(true)`, removing the `ScrollArea`.** Rejected by
  LCV-133 §Out of scope on its own measurements; nothing here reopens them. The
  `ScrollArea` stays as the safety net for a screen shorter than the content, and
  it is what makes a too-short screen degrade to scrolling instead of to loss.
- **Shortening rows** (dropping `shortcut_row`'s `{binding:<20}` padding). It
  buys width. The constraint is height.
- **Changing the column split rule, the group order, or which bindings are
  documented.** `SHORTCUT_GROUPS`, `tool_rows()` and `split_into_columns` are
  inputs here, not targets. No row is added, removed or reworded — the two
  synthetic growth mutations are run and reverted, never shipped, because the
  dialog documents bindings that exist.
- **Restyling, icons, colour, search, filtering, collapsible groups, scroll
  position memory, a printable sheet.** Refused twice already (LCV-126, LCV-133
  §Out of scope) and refused again: this dialog is a static inventory an operator
  reads for four seconds.
- **Upgrading egui.** Pinned at 0.29.1; every number here is a snapshot of that
  pin (ADR 0009 §Revisit criteria).
- **Touching `confirm_dialog`, `error_dialog` or `about_dialog`.** They have a
  handful of lines each and no fold. The split moves them nowhere.
- **A reusable "sized dialog" helper.** One dialog needs this. A helper with no
  second caller is speculative API.

## Why this is not LCV-133 relitigated

LCV-133 §Out of scope rejected `.default_height()` — and it was right, on its
own measurements, in the configuration it measured: **one column**. Alone, in
one column, the call leaves `Help` clipped at 1280×800 and four groups clipped
at 1024×600, so it could not pass that demand's AC 1. That finding is not
reversed and not disputed.

This is a different configuration. The call goes **on top of the two columns
that shipped**, and the two-column layout is what fixes the fold; the call only
buys headroom the columns cannot. Measured on the real `App` at `14708d3` with
the columns held exactly as landed: today's content goes from +8.00/+32.00pt of
slack to **+145.32/+169.32** at 1280×800 and **+45.32/+69.32** at both 600-high
sizes, with all 64 runs painted and contained at all three sizes and the full
suite at **1237 passed, 0 failed** — the same count as LCV-133's handover. That
is the ADR 0009 decision 5 configuration, measured, and it is what this demand
executes.

LCV-133 also warned, correctly, that stacking a sizing call on top of the layout
*after the fact and unexplained* is how a dialog acquires a permanent mystery.
The answer to that is not to skip the call; it is to land it with the assertions
that say what it is for, which is AC 4, AC 5 and AC 6 below.

## Acceptance criteria

1. **The split is the first commit, and it changes no behaviour.** ADR 0004
   Amended (2) is executed before this demand's own change:
   `ShortcutGroup`, `SHORTCUT_GROUPS`, `tool_rows`, `Section`, `item_count`,
   `sections`, `split_into_columns`, `render_column`, `shortcuts_dialog`,
   `shortcut_row` and the LCV-116 / LCV-133 half of the inline tests (including
   the private helpers `key_reader_needles`, `hits` and `implementation_source`,
   which no remaining test uses) move to a new `src/ui/shortcuts_dialog.rs` with
   a module doc. `cargo test --all --no-fail-fast` reports the **same pass count
   before and after** (1237 at `14708d3`). The post-split `awk` LOC of both files
   is reported; ADR 0004 predicts ~130 and ~180.

2. **Nothing outside `src/ui/` changes, and the three `include_str!` needles
   move with their tests.** `src/ui/mod.rs` re-exports exactly the names it
   re-exports today — `about_dialog, confirm_dialog, error_dialog,
   shortcuts_dialog, tool_rows, DialogResult, ShortcutGroup, SHORTCUT_GROUPS` —
   with the last four sourced from the new module, so `src/app/panels.rs`'s
   `crate::ui::shortcuts_dialog(ctx, &mut app.shortcuts_open)` and
   `tests/lcv133_shortcuts_dialog_fits.rs`'s `use lasercad::ui::{tool_rows,
   SHORTCUT_GROUPS}` (already on the re-export since `14708d3`) are untouched.
   `the_shortcuts_dialog_reads_no_key`,
   `the_dialog_holds_no_second_copy_of_the_tool_table` and
   `implementation_source` read `include_str!("shortcuts_dialog.rs")` after the
   move; a needle left pointing at `dialogs.rs` keeps passing and proves nothing,
   which is the self-match failure `src/lib.rs`'s
   `run_arms_native_dialogs_as_its_first_statement` already documents.
   `include_str!("shortcuts.rs")` and `include_str!("../app/input.rs")` resolve
   unchanged from `src/ui/`.

3. **The sizing call, with one sentence saying why.** `shortcuts_dialog`'s
   `Window` gains `.default_height(ctx.screen_rect().height() - 80.0)`. Both
   columns stay, the `ScrollArea` stays, no width call is added. The doc comment
   gains a sentence recording that the call exists to buy headroom above
   `Window`'s baked 420pt `default_size` — which for a `ScrollArea` body is a
   *maximum*, per `Resize::default_height`'s own doc — and that AC 5's slack floor
   is what proves it is still working. Without that sentence the next reader
   deletes the call as gratuitous, exactly as LCV-133 AC 6 anticipated for the
   columns.

4. **Every non-empty run in the dialog body is inside the body clip rect.** Not
   the eight headings, not the `F1` / `This dialog` row — **every** non-empty
   `Shape::Text` run scoped to the dialog body satisfies `pos.y >= clip.top()`
   and `pos.y + galley.size().y <= clip.bottom()`, asserted at **1280×800,
   1024×600 and 800×600**, each size its own test so a failure names the size.
   The failure message lists the offending runs' text. Rationale, measured: the
   first run to go is `Ctrl+Y` / `Redo`, which no hand-typed expectation in this
   repo mentions (ADR 0009 decision 4).

5. **A named minimum slack, and it is a row.** The deepest column's bottom
   clears the body clip bottom by at least **21.00pt**, asserted at the same
   three sizes. 21.00pt is one row's vertical pitch measured at the pin —
   consecutive rows inside a group are exactly 21.00pt apart (ADR 0009 records
   the row *widget* at 20.91pt; the pitch is what a new row actually costs). It
   is a named `const` with a doc comment citing ADR 0009 and its revisit
   criterion, not a magic number at a call site. Measured: today **+8.00** →
   fails without AC 3's sizing call, **+145.32 / +45.32** → passes with it. This
   criterion is the one that makes the failure arrive on the commit that adds the
   row instead of on the bug report that follows it.

6. **The window stays on the screen.** The dialog's window rect is fully inside
   `ctx.screen_rect()` at the three sizes: `top >= 0.0`, `left >= 0.0`,
   `right <= screen.right()`, `bottom <= screen.bottom()`. This is not
   redundant with AC 4: with the sizing call the body clip bottom lands **exactly
   on the screen edge** at 600-high screens (measured: `clip = [136.68, 600.00]`),
   so a run-only assertion cannot tell "the content fits" from "the window hangs
   off the bottom and egui clipped it at the screen edge". Measured with the
   sizing call and today's content, the window rect is `[241, 201]–[1039, 662.7]`
   at 1280×800 and `[113, 101]–[911, 562.7]` / `[1, 101]–[799, 562.7]` at the two
   600-high sizes — inside at all three. The rect is read from egui's own area
   memory, `ctx.memory(|m| m.area_rect(egui::Id::new("Keyboard shortcuts")))`,
   which is the `Area` id, **not** a paint-surface marker — LCV-133's rule about
   never scoping painted runs by the window title still holds for AC 4 and AC 5.

7. **Growth is measured at three sizes and recorded in the handover.** Two
   mutations, each applied, rendered, recorded and **reverted** — neither may
   ship, because the dialog documents bindings that exist:
   - **+3 `ToolEntry`s with shortcuts.** Every run painted and contained at all
     three sizes, count derived at runtime. Expected **70 / 70**.
   - **+1 `ShortcutGroup` with four rows** — this is LCV-133's AC 5 probe as
     amended by ADR 0009 decision 3: LCV-133 AC 1's heading check **and** its AC 3
     derived count are run **together** under the mutation, and all nine headings must be
     painted exactly once and contained. Expected **73 / 73** at all three sizes.
     A probe that re-runs only a fixed expectation is not acceptable here: at
     `14708d3` the whole-section cut that puts 19 items left and 22 right paints
     65 of 73 runs, passes the heading check, and culls **all four rows of the
     group the probe just added** (ADR 0009 §Context; re-derived for this demand).
   Report the deepest-column slack for each mutation. **Two outcomes are known,
   expected, and are not failures of this demand**: under +1 group of four rows
   at 600-high screens the slack is **+5.32pt**, below AC 5's floor, and the
   window rect bottom lands at **602.7 on a 600pt screen**, 2.68pt off the edge,
   while all 73 runs are painted and contained. AC 5 firing there is the design
   working — it goes red two commits before any content is lost (measured: +2
   tools leaves +20.32pt at 600-high with everything still painted and contained).
   AC 5 and AC 6 are asserted on the **real** content; the growth mutations are
   asserted on runs painted and contained.

8. **The negative control offers both causes, and ranks them.**
   `ac2_containment_check_is_not_decorative` in
   `tests/lcv133_shortcuts_dialog_fits.rs` fires whenever `F1` is fully
   contained at its 1280×460 pin, and today it names exactly one cause —
   that the pin is stale and "must be shrunk further". That is the one cause
   whose remedy *weakens the control*, and it is the wrong one on every growth
   case this demand exists to catch. Re-measured at `2860edd` against the real
   `App`: the same message fires verbatim at **+1 tool** and again at
   **+2 tools**, where `F1` is contained with 16.32pt to spare
   (`pos.y = 429.682`, `height = 14.0`, clip bottom `460.000`) because the
   `ScrollArea` has begun culling a *different* row — and it will fire on any
   growth that shifts content past that pin. It is a recurring trap, not a
   one-off (ADR 0009 Amendment (1), correction 3).

   **A green suite reached by obeying the current message is a false green.**
   Measured end to end by `architect`: with one tool added, moving the
   control's screen to `[1280.0, 444.0]` — exactly the shrink the message asks
   for — turns the file **green 5 of 5 while `Ctrl+Y` / `Redo` is still sliced**
   at 1280×800. The suite then certifies the defect LCV-126 and LCV-133
   exist to fix, and it does so for an implementer who did nothing but what the
   failure told them to do.

   So the message must offer both causes and rank them, in substance:

   > `F1` is fully contained at 1280×460. Either this screen height no
   > longer demonstrates the case, **or** the dialog content grew and the
   > `ScrollArea` is culling a different row — check the deepest column's slack
   > at 1280×800 before shrinking this number.

   Wording may be adapted to the assertion's formatting, but it must (a) name
   content growth as a cause, (b) send the reader to the deepest column's slack
   at 1280×800 — which after AC 5 is a test that names the real cause in the
   units of the defect — and (c) put shrinking the pin **last**. Nothing else in
   that test changes: the 1280×460 pin, its `!vertically_contained` claim and
   the positive control above it stay exactly as LCV-133 landed them. This is a
   message-only edit, and it is here because ADR 0009 declined to make it:
   `implementer-rust` owns the test, not the ADR.

9. **Caps and gates.** Post-split `awk` LOC reported for both
   `src/ui/dialogs.rs` and `src/ui/shortcuts_dialog.rs`; if either lands in
   270–300, flag `architect` rather than splitting again. All three local gates
   green, with the pass count reported: `cargo fmt --all -- --check`,
   `cargo clippy --all-targets -- -D warnings`, `cargo test --all
   --no-fail-fast`. **CI: not run (billing hold)** — the local gate is the
   acceptance gate.

## Expected tests

**Where the new tests live.** If LCV-132 has landed, a new file
`tests/lcv134_shortcuts_dialog_headroom.rs` importing the shared
`harness::` collector and adding no collector of its own. If LCV-132 has **not**
landed, add the AC 4 / AC 5 / AC 6 tests to
`tests/lcv133_shortcuts_dialog_fits.rs`, whose `Run` already carries the
untruncated `pos` and `height` these criteria need, and whose `dialog_body`
already scopes to the body by the `Command line` marker — a fifth divergent copy
of that collector is precisely what LCV-132 exists to delete. Name them
`lcv134_*` and say why in one line of the module doc.

- **Integration / AC 4 — containment, three tests, one per size.** Note that on
  today's content the containment check passes *with or without* the sizing call
  (+8.00pt is still inside), which is why AC 5 exists — so the mutation that
  proves AC 4 has to add content. Mutation: revert the sizing call **and** add
  one `ToolEntry` with a shortcut; confirm all three sizes fail naming `Ctrl+Y`
  and `Redo`, then restore the sizing call with the tool still added and confirm
  all three pass. Second mutation, in the failing configuration: weaken the
  assertion to the eight headings only and confirm it goes green with a row
  outside the clip — that weakened assertion is what the suite has today.
  Report all three messages; revert the synthetic tool.
- **Integration / AC 5 — the slack floor.** Mutation: revert the sizing call on
  today's unmodified content and confirm AC 5 fails at +8.00 at all three sizes
  **while AC 4 still passes**. That pair is the demonstration that the floor is
  the tripwire and containment is the backstop. Second mutation: set the floor to
  0.0 and confirm the pre-sizing-call layout passes it — a floor nothing can fail
  is a comment. Restore 21.00 and the sizing call (+145.32 / +45.32).
- **Integration / AC 6 — the window rect.** Mutation: assert against
  `screen_rect().expand(100.0)` instead and confirm the test still passes, then
  restore — a containment assertion against a rect that cannot bind proves
  nothing. Report the measured window rect at each of the three sizes.
- **Integration / AC 8 — the control's message.** No new test: the mutation is
  the check. With one `ToolEntry` added (AC 7's revert discipline applies), run
  the file and paste the **new** failure message verbatim in the handover; it
  must name content growth and send the reader to the slack. Then, in the same
  mutated tree, do what the **old** message asked — move the control's screen to
  `[1280.0, 444.0]` — and report the result: it must be **5 of 5 green** with
  `Ctrl+Y` / `Redo` still sliced at 1280×800, which is the false green this
  criterion exists to make unreachable. Revert the pin, then the tool. Baseline
  for comparison, measured at `2860edd` on unmutated content: the control's own
  numbers at its pin are `pos.y = 446.682`, `height = 14.0`, clip bottom
  `460.000` — a **0.682pt** straddle, which is the margin the whole control
  turns on.
- **Integration / AC 1 + AC 2 — the split is inert.** The pass count before and
  after the split commit, from the same command. Mutation: leave one
  `include_str!("dialogs.rs")` needle pointing at the old file and confirm the
  test still passes — that is the evidence for why all three must move, and it is
  reported, not left in.
- **[manual] smoke — 60 seconds, and it is the thing the demand is about.**
  1. `cargo run`. Type **nothing** into the command line: the CAD grammar knows
     only the single-letter aliases plus `text` (LCV-131), so a full word such as
     `LINE` is forwarded to the model when a key is configured (LCV-124), turning
     a smoke into a billable request. One key and the mouse.
  2. Press **F1**. Without scrolling and without touching the wheel, confirm all
     eight headings are visible and that the last row of each column is fully
     legible — in particular `Ctrl+Y` / `Redo` at the bottom of the left column,
     which is the row that goes first.
  3. Press **Esc**, resize the window to roughly 1024×600, repeat step 2.
  4. Record both window sizes and whether any row was cut off at the bottom edge
     or the window bottom ran past the screen.

## Test hygiene (mandatory)

- **At least two frames.** The dialog's first frame paints zero text runs — its
  `Area` has not been placed yet. Asserting on frame 1 proves nothing.
- **Never scope painted runs by the window title.** `Window::new("Keyboard
  shortcuts")`'s title is clipped to the whole screen, and four of this dialog's
  headings collide by exact text with a menu-bar label. Scope on a run genuinely
  inside the body, as `tests/lcv133_shortcuts_dialog_fits.rs` does via the
  `Command line` marker. AC 6's `Id::new("Keyboard shortcuts")` is egui's area id
  and is not affected by this rule.
- **Drop empty and whitespace-only runs before counting or measuring**, or the
  count and the slack are both noise.
- **Recurse into `Shape::Vec`** — the only nesting variant at 0.29.1.
- **`pixels_per_point = 1.0`**, set explicitly. Every number in this demand is at
  that setting.
- Any source scan added here is bounded at the offset of a bare `#[cfg(test)]` at
  column 0, with needles built by `concat!`. Prefer not to add one: a scan
  asserting `.default_height(` is written would pass on a dialog that paints
  nothing, which is the whole failure mode this demand is about.
- No test reaches a real endpoint.

## Open questions

None.

## Notes

- **Provenance of every number here.** Re-measured by `product-owner` at
  `14708d3`, egui 0.29.1, `pixels_per_point = 1.0`, on a settled (second) frame,
  at 1280×800, 1024×600 and 800×600. Three methods: the real `App` with
  `shortcuts_open: true`; a geometry replica of `shortcuts_dialog` that
  reproduces the shipped body byte-for-byte (clip `[236.68, 662.68]`, 64 runs,
  +8.00/+32.00 slack) and was used only for configurations the shared worktree
  must not carry; and, for the `+1 tool` and sizing-call cases, a throwaway copy
  of the repository with the real mutation applied and the real suite run.
- **ADR 0009 is amended (`2860edd`) and this demand is written against the
  amended text.** Two §Context numbers written at `4e4f3ba` did not reproduce
  when `product-owner` re-measured them for this demand — the `+3 tools` row's
  required run count (`68 / 68`, actually **68 / 70**, so AC 3 fails there as
  well as AC 2) and the claim that the first three growth rows "pass every test
  LCV-133 shipped" — and `architect` confirmed both and corrected them in place.
  Verifying them turned up a **third**, which is where this demand's AC 8 comes
  from: the control's misdiagnosis is not specific to `+1 tool`. Re-measured
  here at `2860edd`, `+2 tools` gives 3 pass / 2 fail with the control's message
  unchanged, and the ADR's false-green demonstration (pin → `[1280.0, 444.0]`,
  5 of 5 green, row still sliced) is the reason AC 8 is a criterion and not a
  note. ADR 0009 decision 5's own figures (145/169, 45/69, 70/70, 73/73, 36 of
  64 at 800×600) all reproduced exactly, and decisions 1..5 are unchanged.
- **Ordering.** This must land before any demand that adds a tool to `TOOLS` or a
  row to `SHORTCUT_GROUPS`; after it, such a demand gets a red test naming the
  slack in its own commit. Against **LCV-132** there is no hard constraint —
  see §Expected tests for the two shapes, one per LCV-132 state — but the
  scheduling preference is **this demand first**: AC 8 and the growth mutations
  edit `tests/lcv133_shortcuts_dialog_fits.rs`, and editing a helper while it is
  private to one file is a local change, where the same edit after LCV-132 has
  promoted it to a shared `pub` helper is a cross-file change with innocent
  bystanders. That is LCV-133 §Notes' own reasoning, applied once more; LCV-132
  §Notes carries the full argument. If LCV-132 lands first anyway, use the
  harness and delete nothing.
- **The split is not this demand's idea.** ADR 0004 Amended (2) decided the seam,
  the destination name, the re-export list, the three `include_str!` needles and
  why `src/ui/shortcuts_dialog.rs` is safe next to `src/ui/shortcuts.rs` (which
  reads keys — the moved `the_shortcuts_dialog_reads_no_key` is what keeps the
  ADR 0002 §A6 two-key-readers invariant honest, and it must move, not be
  dropped). Read that section before the first commit; nothing in it is
  re-decided here.
- Related: ADR 0009 (the cap, the growth table, decision 5), ADR 0004 Amended (2)
  (the seam), LCV-133 (the columns, and §Known limitation), LCV-126 (the group),
  LCV-132 (the harness), LCV-131 / LCV-124 (why the smoke types nothing),
  `shortcuts_dialog` and `split_into_columns` in `src/ui/dialogs.rs`.
