# LCV-133 — The F1 dialog hides five of its eight groups behind its own fold

- **Status**: Done
- **Phase**: 11
- **Depends on**: LCV-126 (Done — it put the `Command line` group in the table this demand makes visible). **Ordering, not a dependency**: this demand must land **before** LCV-132. See §Notes.
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust, reviewed and approved by reviewer-rust (1237 passed, 0 failed, 2 ignored). Commits: aa36bd8, 14708d3. CI: not run (billing hold).

## Problem

LCV-126 exists because an operator who presses F1 looking for the command line
finds no mention of it and concludes there is none. It added the group, proved
by paint assertion that the group renders, and shipped. **The operator still
does not see it.** The group is one scroll below the fold, and an operator who
has just been told the dialog is "the inventory of what the keyboard does" has
no reason to scroll a dialog that looks complete.

### The mechanism

`Window::new` in egui 0.29.1 bakes `.default_size([340.0, 420.0])`
(`egui-0.29.1/src/containers/window.rs:64`), and `Resize::default_height`'s own
doc comment says: *"if the contents is a `ScrollArea` then this decides the
maximum size"* (`resize.rs:98-104`). `shortcuts_dialog` in `src/ui/dialogs.rs`
never calls `.default_height()`, so its scroll area is capped at roughly 420pt
regardless of how much room is on screen.

### What is actually hidden (measured 2026-09-13, at `768ee9e`)

The reviewer reported `Command line` and `Help` below the fold. Measurement on
the shipped code, on a settled unscrolled frame, is worse than that:

| screen | text runs painted | headings painted | headings **not** painted |
|---|---|---|---|
| 1280 × 800 | 36 | `Tools`, `File`, `Edit` | `View`, `Modes`, `Drawing`, `Command line`, `Help` |
| 1024 × 600 | 36 | same three | same five |
| 900 × 2000 | 36 | same three | same five |

**Five of the eight groups are invisible**, not two, and the clip is *identical
on a 2000pt-tall screen* — 36 runs spanning 434pt in all three cases. The
operator cannot fix this by maximising the window, by using a big monitor, or by
resizing the dialog (it is `.resizable(false)`). The eight groups plus ten tool
rows need ~836pt in one column; they get ~426.

This is a **pre-existing defect, not one LCV-126 introduced** — `Help` was below
the fold before the `Command line` group existed. Do not read this as a rework
of LCV-126. That demand is correct as written, its §Out of scope was right to
refuse to resize the dialog, and its status does not change.

### The one-line fix does not work, and this was measured before it was rejected

The obvious remedy is to give the `Window` a `.default_height()`. It was tried:

| configuration | 1280 × 800 | 1024 × 600 | 900 × 2000 |
|---|---|---|---|
| shipped (no call) | 3 of 8 | 3 of 8 | 3 of 8 |
| `.default_height(screen − 80)` | 7 of 8 — **`Help` still hidden** | 5 of 8 | 8 of 8 |
| `.default_height(9999)` | 7 of 8, and the window **overflows the screen top** (first run at `y = −0`) | 5 of 8 | 8 of 8 |

`.default_height()` is a real improvement and it is still not a fix: on 1280×800
— the harness's own screen and an entirely ordinary laptop — `Help` remains
clipped, and on 1024×600 four groups do. It cannot pass the acceptance criterion
this demand exists for, because **one column of this content does not fit on a
normal screen at any window height**. An unbounded height makes the window taller
than the display, which is worse than scrolling.

### Two columns does work, and needs no sizing call at all

Measured with the real row set, split as `Tools`/`File`/`Edit` in the left column
and `View`/`Modes`/`Drawing`/`Command line`/`Help` in the right:

| screen | text runs | headings painted | galleys wrapped | every bottom run fully inside its clip rect |
|---|---|---|---|---|
| 1280 × 800 | 65 | **8 of 8** | 0 | yes |
| 1024 × 600 | 65 | **8 of 8** | 0 | yes |
| 800 × 600 | 65 | **8 of 8** | 0 | yes |

No `.default_height()`, no `.default_width()`, no `.max_size()` — three candidate
widths (620 / 680 / 760) and no width call at all produced byte-identical
results, because with `.resizable(false)` the window already sizes its width to
content. **The fix is the layout and nothing else.** Both columns together span
434pt, which fits inside the baked cap with room to spare, and the existing
`ScrollArea` stays as the safety net for a screen shorter than that.

The split matters and must not be assumed: putting only `File` in the left
column leaves `Help` clipped again (62 runs, `Help` missing). That failing
configuration is this demand's mutation.

## Scope

- `shortcuts_dialog` in `src/ui/dialogs.rs` lays its content out in two columns,
  balanced so that every heading and every row is painted without scrolling at
  1280×800, 1024×600 and 800×600.
- A new test file asserting exactly that, by paint assertion.
- The consequent rework of `tests/lcv126_command_line_group.rs`'s line grouping
  (AC 4), which a two-column layout breaks. See AC 4 — this is not optional and
  not a surprise.
- One sentence in `src/ui/dialogs.rs`'s doc comment recording why the layout is
  two columns, so the next person does not "simplify" it back.

## Out of scope

- **Upgrading egui.** Pinned at 0.29.1. Everything above was measured at the pin
  and the fix needs nothing newer.
- **Resizing the window, `.default_height()`, `.max_size()`, `.resizable(true)`,
  or removing the `ScrollArea`.** Measured not to fix it (the table above), and
  the scroll area stays as the safety net. If the implementer finds a case the
  two-column layout does not cover, **stop and report** rather than stacking a
  sizing call on top of the layout change — two half-fixes are how a dialog
  acquires a permanent mystery.
- **Search, filtering, scroll-position memory, a printable cheat sheet,
  collapsible groups, icons, colour, or any restyling beyond the split.**
  LCV-126 put these out of scope and the reasoning is unchanged: this dialog is
  a static inventory an operator reads for four seconds.
- **Changing which bindings are documented.** No row is added, removed or
  reworded. `SHORTCUT_GROUPS` and `tool_rows()` are inputs here, not targets. If
  a row looks wrong, that is a finding to report, not an edit to make.
- **Reordering the groups as they read.** The split assigns whole groups to
  columns; it does not reorder them within a column and does not interleave.
- **Touching the other three dialogs** (`confirm_dialog`, the two at
  `src/ui/dialogs.rs:58` and `:90`, `about_dialog`). They have a handful of
  lines each and no fold.
- **Generalising the split into a reusable multi-column helper.** One dialog
  needs this. A helper with no second caller is speculative API.

## Acceptance criteria

1. **Every group is painted on an unscrolled frame.** A test opens the shortcuts
   dialog through `App` and asserts that on a **settled** frame with no scroll
   input, all eight headings — `Tools`, `File`, `Edit`, `View`, `Modes`,
   `Drawing`, `Command line`, `Help` — are painted as `Shape::Text` runs, at each
   of three screen sizes: **1280×800, 1024×600 and 800×600**. Each size is its
   own assertion so a failure names the size.

2. **Painted is not the same as visible, and the test must know the
   difference.** For each of the eight headings the test also asserts the run is
   **fully inside its clip rect** — `pos.y >= clip.top()` and
   `pos.y + galley.size().y <= clip.bottom()`. An egui `ScrollArea` culls content
   far outside the view but *does* emit a shape for a row straddling the edge, so
   a presence-only assertion would pass on a heading the operator sees sliced in
   half. The same containment check holds for the last row of the last group
   (`F1` / `This dialog`), which is the true bottom of the content.

3. **Every documented row is painted too, not just the headings.** The count of
   non-empty text runs on the dialog's surface is asserted to equal the number
   the content requires, derived at runtime from `SHORTCUT_GROUPS` and
   `tool_rows()` — never a hand-typed `65`. A literal would have to be edited
   every time a tool is added, and the person adding the tool would edit it to
   whatever number made the test pass.

4. **LCV-126's AC 4 test is reworked in this demand, by this implementer.**
   `tests/lcv126_command_line_group.rs::the_command_line_group_is_painted_between_drawing_and_help`
   groups runs into visual lines by `y` within `SAME_LINE` and asserts a
   five-line window equals an exact `Vec<Vec<String>>`. Both columns share one
   clip rect and paint at overlapping `y` values, so after this change each of
   those lines gains the left column's runs and the assertion fails. The rework:
   **bucket runs by column before grouping by `y`** — the two columns are
   separated by a wide, empty band in `x`, and the left column's rightmost run
   ends well before the right column's leftmost begins. The test must keep
   asserting the same thing it asserts today: `Command line`, then its three
   binding/description rows in order, then `Help`, as consecutive visual lines.
   **Its scroll must be deleted**, because there is nothing left to scroll to and
   a `MouseWheel` that no longer moves anything is a comment pretending to be
   code. The demand is not done while that test is red or weakened.

5. **The split is derived, not hand-typed.** The assignment of groups to columns
   is computed from the content (for example: fill the left column until its
   accumulated item count reaches half the total, then the rest go right), so
   adding a tool or a group rebalances without an edit. A hand-typed index
   silently becomes wrong the first time the table changes, which is the failure
   mode this dialog already has.

6. **The doc comment says why.** `shortcuts_dialog`'s doc comment gains a
   sentence recording that the layout is two columns because one column of this
   content is clipped by `Window`'s baked 420pt `default_size` independently of
   screen size, and that the `ScrollArea` remains as a safety net. Without it the
   next reader deletes the split as gratuitous.

7. **Caps and gates.** `src/ui/dialogs.rs` is **246 implementation LOC today** by
   ADR 0004's `awk` recipe; the post-change number is reported, and if it lands in
   270–300 the implementer flags `architect` rather than splitting the file.
   `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` and
   `cargo test --all --no-fail-fast` all green locally. **CI: not run (billing
   hold)** — the local gate is the acceptance gate.

## Expected tests

New file `tests/lcv133_shortcuts_dialog_fits.rs`. It needs a paint collector; see
§Notes on which one.

- **Integration / AC 1 + AC 2 — the three screen sizes.** Mutation, and it is the
  one measured above: **assign only `File` to the left column** (split after one
  group instead of the balanced point). Run, confirm the 1280×800 case fails
  naming `Help` as unpainted; revert. Second mutation: **revert the layout to one
  column entirely** and confirm all three sizes fail, naming five headings each.
  Report both messages.
- **Integration / AC 2 — the containment check is not decorative.** Mutation:
  weaken the assertion to presence only, then re-run against a layout where the
  last row straddles the clip edge (reachable by shrinking the screen height in
  the test to just below the content height). Confirm the weakened version passes
  and the containment version fails. If that configuration cannot be produced at
  0.29.1, say so in the handover and keep the containment check anyway — the cost
  is two comparisons.
- **Integration / AC 3 — the run count.** Mutation: drop `Help` from
  `SHORTCUT_GROUPS`'s iteration in the render loop while leaving the table
  intact, and confirm the count assertion fails, not only the heading assertion.
  This is the mutant class LCV-125's review found survives every source scan.
- **Integration / AC 4 — LCV-126 stays honest.** Run
  `tests/lcv126_command_line_group.rs` before and after the rework. Mutation:
  swap `ArrowUp` and `ArrowDown` in the table and confirm the reworked test still
  catches the order — the column bucketing must not have turned an ordered
  assertion into a set membership one.
- **Unit / AC 5 — the split adapts.** Mutation: add a synthetic ninth group with
  four rows to `SHORTCUT_GROUPS`, run the AC 1 test unchanged, and confirm it
  still passes at all three sizes; remove it. If it fails, the split rule is
  wrong and the demand is not done.
- **Unit / AC 7** — the `awk` LOC number, in the handover.
- **[manual] smoke** — this is the criterion the whole demand is about, so it is
  worth the ninety seconds:
  1. `cargo run`. Do **not** type anything into the command line during this
     smoke — the CAD grammar knows only the single-letter aliases plus `text`
     (LCV-131), so a full word such as `LINE` or `CIRCLE` is forwarded to the
     model when a key is configured (LCV-124), turning a smoke into a billable
     request. This smoke uses one key and the mouse.
  2. Press **F1**.
  3. Read the dialog top to bottom **without scrolling and without moving the
     mouse wheel**. Confirm all eight headings are on screen — `Tools`, `File`,
     `Edit`, `View`, `Modes`, `Drawing`, `Command line`, `Help` — and that the
     `Command line` group shows its three rows in full, with no text cut off at
     the bottom edge and no horizontal truncation of a description.
  4. Press **Esc** (or the window ×) and confirm the dialog closes.
  5. Resize the application window to roughly 1024×600 and repeat steps 2–4.
  Record both window sizes used and whether any row was clipped.

## Test hygiene (mandatory)

- **At least two frames.** Measured at the pin: the dialog's **first frame paints
  zero text runs** — the window's area has not been placed yet — and every frame
  from the second on is complete and identical. Asserting on frame 1 proves
  nothing and asserting absence on frame 1 proves something false.
- **Never use the window title as a surface marker.** `Window::new("Keyboard
  shortcuts")`'s title is clipped to the whole screen, so scoping by it matches
  every surface in the frame. Scope on a run genuinely inside the body. This trap
  is already written down in `tests/lcv126_command_line_group.rs`'s module doc;
  read it before writing a line of this.
- **Empty and whitespace-only runs exist** and must be dropped before counting,
  or AC 3's count is noise.
- Recurse into `Shape::Vec` — it is the only nesting variant at 0.29.1 and a
  collector that skips it sees almost nothing.
- Any source scan added here is bounded at the offset of a bare `#[cfg(test)]` at
  column 0, with needles built by `concat!`. **But prefer not to add one at all**:
  every criterion in this demand is a paint assertion precisely because a scan
  asserting `.columns(2` is written would pass on a dialog that paints nothing.
- No test reaches a real endpoint.

## Open questions

None.

## Notes

- Found by `implementer-rust` while implementing LCV-126 and confirmed three ways
  by `reviewer-rust`, 2026-09-13. Every number in §Problem was re-measured
  independently by `product-owner` against `768ee9e` before this body was written,
  including the two rejected `.default_height()` configurations and the accepted
  two-column one.
- **Why two columns rather than the one-line sizing call**, stated once so the
  decision is not relitigated: `.default_height()` leaves `Help` clipped on a
  1280×800 screen and four groups clipped on 1024×600. It does not pass AC 1. The
  column split does, at every size tested, with no sizing call. This is not a
  preference for a nicer layout — it is the only measured configuration that
  fixes the defect.
- **Ordering against the other two demands that touch
  `tests/lcv126_command_line_group.rs`.** Three now do: LCV-126 (landed, wrote
  it), LCV-133 (this one, changes what its AC 4 test asserts), LCV-132 (moves its
  inline collector into `tests/harness/`). **Do LCV-133 first.** Changing what a
  helper asserts while it is private to one file is a local edit; doing it after
  LCV-132 has promoted it to a shared `pub` helper with three call sites turns the
  same fix into a cross-file change with two innocent bystanders. LCV-132's AC 3
  already absorbs that file "on the same terms… whatever the count was when you
  arrived", so it needs no amendment — but if LCV-132 has already landed when this
  is picked up, **stop and report** instead of editing the shared helper alone.
- **Which collector to use**: if LCV-132 has landed, import
  `harness::{painted_runs, lines_on_surface_of, texts}` and add nothing. If it has
  not, copy the collector from `tests/lcv126_command_line_group.rs` as LCV-126 did
  — **do not invent a third shape**, because LCV-132 AC 3 exists to delete exactly
  the duplicates, and a differently-shaped one is harder to delete than an
  identical one.
- Measured references at the pin:
  `egui-0.29.1/src/containers/window.rs:64` (`.default_size([340.0, 420.0])`),
  `egui-0.29.1/src/containers/resize.rs:98-104` (the `default_height` doc comment
  that explains the cap), `egui-0.29.1/src/containers/scroll_area.rs:223`
  (`auto_shrink: Vec2b::TRUE`, which is why the window will shrink to content once
  the cap stops binding).
- Related: LCV-126 (the group), LCV-132 (the harness promotion), LCV-131 (why the
  smoke types nothing), LCV-124 (why a typed word can cost money), ADR 0004 (the
  `awk` recipe), `src/ui/dialogs.rs:212` (`shortcuts_dialog`),
  `src/ui/dialogs.rs:146` (`SHORTCUT_GROUPS`).
