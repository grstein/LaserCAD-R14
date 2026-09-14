# ADR 0004 — Measuring the 300-LOC cap

- **Status**: Accepted
- **Amended (1)**: 2026-09-13 — `src/app/mod.rs` reached 292 and Consequences
  said it had no seam to pre-decide. It has one now, because the thing that grew
  is known: the agent harness. Consequences gains §"The `src/app/mod.rs` seam",
  which also corrects LCV-123 AC 20's destination for `AgentState`. Rule 4 is
  applied, not changed; nothing in rules 1..4 is reversed.
- **Amended (2)**: 2026-09-13 — `src/ui/dialogs.rs` reached **299** on LCV-133 and
  flagged `architect` as rule 4 asks. Consequences gains §"The `src/ui/dialogs.rs`
  seam". Rule 4 is applied, not changed; nothing in rules 1..4 is reversed.
- **Amended (3)**: 2026-09-14 — two additions from LCV-132, both in
  Consequences. §"What the cap binds under `tests/`" states which files under
  `tests/` rule 3's exemption 2 already covers (an integration-test binary: yes;
  `tests/harness/`: no) — stated, not widened. §"The `tests/harness/paint.rs`
  seam" records a seam at **267**, three lines below the 270 band, because
  LCV-132 AC 11 flagged it there and three queued demands will push it in. Rule
  4 is applied, not changed; nothing in rules 1..4 is reversed.
- **Date**: 2026-09-13
- **Deciders**: architect

## Context

`AGENTS.md` states the cap twice ("Hard cap: **300 LOC per `.rs`**",
"**One responsibility per file**, ≤300 LOC") and neither statement says what
counts as a LOC. ADR 0002 §"The 300-LOC cap and `src/app.rs`" already fixed the
reading — the cap applies to **implementation** LOC, an inline
`#[cfg(test)] mod tests` block does not count — but neither `AGENTS.md` line
points at it, so the amendment is invisible at the point of reading.

It has now cost a review cycle. A reviewer read the `AGENTS.md` line literally,
ran `wc -l` on `src/text/hershey.rs` (497) and `src/text/layout.rs` (380), and
raised a blocking finding demanding a file split. Under the real rule those
files are **53** and **87** implementation lines — the tests start at line 54
and line 88. The split was not ordered, but it would have damaged two files for
no reason. The reviewer asked for this amendment themselves.

The same reviewer's first attempt to measure the rule correctly also failed:
grepping for `cfg(test)` anywhere on a line matched
`src/app/mod.rs:154`, a **doc comment** that reads
`/// … Safe to call from any `#[cfg(test)]` context.`, and returned an
implementation count of 153 for a file whose real count is 287. So the rule
needs a measurement recipe, not just a definition.

Two files in the tree are also outside the rule as ADR 0002 stated it:

- `src/text/hershey_data.rs` — 990 lines, zero `fn`, a single
  `pub static GLYPHS` table of public-domain Hershey stroke data. It has no
  inline test module, so the ADR 0002 formula scores it 990 and it shipped
  anyway. It is a data table, not logic.
- `src/geometry/snap/tests.rs` — 216 lines, declared from
  `src/geometry/snap/mod.rs:188` as `#[cfg(test)] mod tests;`. It is entirely
  test code living in its own file, which is the split the cap asks for.

## Decision

**The cap is 300 implementation lines per `.rs` file, measured mechanically.**

1. **Implementation LOC = total lines, minus the top-level
   `#[cfg(test)] mod tests` block and everything after it.** The anchor is a
   **bare `#[cfg(test)]` starting at column 0**. Nothing below it counts.

2. **Measure with this command. Do not eyeball it and do not `wc -l`:**

   ```bash
   awk '/^#\[cfg\(test\)\]/{print NR-1; f=1; exit} END{if(!f) print NR}' <file>
   ```

   It prints the implementation LOC of one file, and `0` witnesses are cheap:
   `src/text/hershey.rs` → 53, `src/text/layout.rs` → 87,
   `src/app/mod.rs` → 287, `src/app/file_ops.rs` → 281.

   A pattern that matches `#[cfg(test)]` anywhere on a line is **wrong** — it
   matches doc-comment prose. That is the `src/app/mod.rs` → 153 false
   positive.

3. **Two exemptions, both narrow and both checkable by eye:**
   - A file that contains **only a `const`/`static` data table and no `fn`**
     (`src/text/hershey_data.rs`). Splitting a font table across files buys
     nothing.
   - A file that contains **only test code**, i.e. one declared from its parent
     as `#[cfg(test)] mod tests;` (`src/geometry/snap/tests.rs`). It is already
     the result of a split.

   No other exemption exists. A file with one `fn` in it is not a data table.

4. **Name the seam at 270, split when a demand crosses 300.** A file between
   270 and 300 implementation lines is *not* refactored on sight. Instead the
   architect records where it would split and on what trigger, so that the
   demand that finally crosses the cap executes a decided seam instead of
   inventing one against a deadline. A speculative split is a worse outcome
   than a late one: it churns a file no demand asked about and it guesses the
   seam without the evidence the next demand would have supplied.

   The first application of rule 4 is `src/app/file_ops.rs` (281) — see
   Consequences.

## Consequences

**Easier**

- The rule is decidable in one command by whoever is reading it, so reviewer
  and implementer cannot disagree about a file's size.
- Inline `#[cfg(test)]` modules stay where AGENTS.md §Implementation Rules asks
  for them, with no cap pressure to move or thin them. `src/app/file_ops.rs`
  carries 371 lines of tests against 281 of implementation; that ratio is a
  feature.

**Harder / committed to**

- Doc comments **do** count against the cap. They are implementation lines by
  this definition, and `AGENTS.md` also mandates doc comments on every `pub`
  item, so a heavily-documented struct eats budget. Accepted deliberately:
  loosening the measure to "code lines only" would need a second, fuzzier
  counting rule, and the honest response to a file that is 280 lines of doc
  comments is still to split it.
- The two exemptions are a list, not a principle. Adding a third one is an ADR,
  not a judgement call in review.

**The `src/app/file_ops.rs` seam, pre-decided per rule 4.**

`src/app/file_ops.rs` is at 281 and `src/app/mod.rs` at 287. Neither is split
now: LCV-114 (bed size) and LCV-115 (export presets) touch
`src/io/file_actions.rs`, `src/ui/menubar.rs` and `src/document/`, and reach
`src/app/file_ops.rs` at most through its five one-line `action_*` wrappers.
The pressure is real but it is not this demand's.

When a demand does cross 300 in `src/app/file_ops.rs`, the seam is the **egui
boundary**, which already runs through the middle of the file:

| stays in `src/app/file_ops.rs` | moves to `src/app/discard.rs` |
|---|---|
| `PendingAction`, `App::has_unsaved_changes`, `App::mark_saved`, the four `request_*` guards, the five `action_*` wrappers | `draw_discard_dialog`, `apply_dialog_result`, `poll_close_request` |

Those three functions are the only ones in the file that take an
`&egui::Context`; everything else is the guard state machine. The split
therefore leaves `file_ops.rs` egui-free and puts all the immediate-mode dialog
plumbing in one file, which is the same phase-per-file shape ADR 0002 §"The
300-LOC cap" used for `src/app.rs`. `src/app/mod.rs` re-exports the three moved
names exactly as it does today, so no caller outside `app` changes.

For `src/app/mod.rs` there is no seam to pre-decide: 105 of its 287 lines are
the `struct App` field list and its doc comments, which cannot move without
moving `App`. If it crosses, the thing that grew is what moves out, by the
existing phase-per-file rule.

*(Amended (1), 2026-09-13. The paragraph above is kept as written and its
reasoning still holds — but its conclusion no longer does. The thing that grew
is now known, so the seam **is** pre-decided; see below.)*

**The `src/app/mod.rs` seam, pre-decided per rule 4.**

`src/app/mod.rs` is at **292** implementation lines at `5452fd6`, measured with
the rule 2 recipe. It went 265 → 274 → 289 → 292 across Marco 2, and every one
of those increments was the agent harness. Eight lines of headroom is still not
a reason to split — rule 4 forbids that, and neither demand in flight touches
the file (LCV-124 is `src/agent/classifier.rs` and `src/app/cmdline.rs`; LCV-125
is `src/agent/panel.rs` and `src/agent/settings_ui.rs`). It *is* a reason to
write the seam down now, which is the entire purpose of rule 4: whoever crosses
the cap will cross it mid-implementation, and that is the worst moment to be
inventing a seam.

**The seam is `AgentState`: the eight agent fields leave `App` as one field.**

| stays in `src/app/mod.rs` | moves to `src/app/agent_state.rs` |
|---|---|
| every other `App` field, `impl App`, the module doc, and one `pub agent: AgentState` | `agent_panel_open`, `agent_chat`, `agent_input_draft`, `agent_busy`, `agent_rx`, `agent_fence`, `agent_applied`, `agent_turn_label`, wrapped in `pub struct AgentState` |

Four notes make it executable without a second decision:

- **The destination is a new `src/app/agent_state.rs`, not `src/app/agent_turn.rs`.**
  LCV-123 AC 20 named `agent_turn.rs`, and that is now wrong: `agent_turn.rs` is
  itself at 272, and the struct with its doc comments is ~26 lines, landing it
  at ~298 with no headroom. That seam would trade one capped file for another
  and buy a single demand's worth of time. A fourth file joins ADR 0007 §D8's
  three; state with no behaviour is its own responsibility, which is what
  AGENTS.md §Implementation Rules asks a file to have.
- **`TurnFence` gains `Default` in its derive.** Its three fields are `u64`,
  `u64` and `bool`, and `TurnFence::default()` is `TurnFence::new(0)`, so
  `AgentState` can `#[derive(Default)]` and `src/app/init.rs` collapses nine
  agent initialiser lines into one.
- **The arithmetic.** `src/app/mod.rs` loses the 22-line field block and gains
  three lines for `pub agent: AgentState` plus a `mod` / `pub use` pair, landing
  near **275** — back inside the band with real headroom, rather than one line
  under the cap.
- **`agent_settings_open` does not move.** It sits with `about_open` and
  `shortcuts_open` in the dialog-visibility cluster and belongs to the settings
  dialog, not to a turn. The seam follows the turn.

The cost is call sites, and it is mechanical. The eight fields are read or
written across `src/agent/bridge.rs`, `src/agent/panel.rs`,
`src/app/agent_apply.rs`, `src/app/agent_poll.rs`, `src/app/agent_turn.rs`,
`src/app/init.rs`, `src/app/panels.rs`, `src/ui/toolbar.rs` and three
integration tests. Fields drop their prefix inside the struct
(`app.agent_busy` → `app.agent.busy`) and the compiler finds every site. Two
source scans carry field paths in their needles —
`every_repaint_request_in_src_is_conditional` and LCV-123 AC 19's `agent_busy`
guard scan — so those needles and `AGENTS.md` §Event flow → Repaint policy are
updated in the same commit, with the conditional-repaint count staying at three.

**The `src/ui/dialogs.rs` seam, pre-decided per rule 4.**

`src/ui/dialogs.rs` is at **299** implementation lines at `aa36bd8`, measured with
the rule 2 recipe: 246 before LCV-133, 299 after. One line of headroom is not
headroom, and rule 4 still forbids splitting on sight — no demand is queued
against the file, and LCV-132 touches `tests/`, not this. So the seam is written
down now, and the trigger is sharper than usual: **the next demand that adds an
implementation line to this file executes the split as its first commit**, before
its own change, because at 299 it has no other option.

**The seam is the dialog itself, not data-versus-presentation: the keyboard
shortcuts dialog leaves whole.**

| stays in `src/ui/dialogs.rs` | moves to `src/ui/shortcuts_dialog.rs` |
|---|---|
| the module doc, `DialogResult`, `confirm_dialog`, `error_dialog`, `about_dialog`, and their inline tests | `ShortcutGroup`, `SHORTCUT_GROUPS`, `tool_rows`, `Section`, `item_count`, `sections`, `split_into_columns`, `render_column`, `shortcuts_dialog`, `shortcut_row`, and the LCV-116 / LCV-133 half of the inline tests |

Five notes make it executable without a second decision:

- **Why not the data/presentation seam.** Moving only `ShortcutGroup` /
  `SHORTCUT_GROUPS` / `tool_rows` into a table file leaves the split rule, the
  renderer and the table in two files that only ever call each other — a
  two-file module nobody asked for, and both halves still change together on
  every demand that touches this dialog. The seam above leaves three generic
  modal helpers in one file and one self-contained feature in the other, which
  is what AGENTS.md §Module tree's "one responsibility per file" asks for. The
  thing that grew is what moves out, which is the same rule the `src/app/mod.rs`
  seam above applies.
- **The arithmetic.** ~160 implementation lines move plus a new module doc, so
  `src/ui/dialogs.rs` lands near **130** and `src/ui/shortcuts_dialog.rs` near
  **180**. Both have real headroom, not one line of it.
- **`src/ui/mod.rs` keeps every name it re-exports today**, sourced from the new
  module (`pub use shortcuts_dialog::{shortcuts_dialog, tool_rows, ShortcutGroup,
  SHORTCUT_GROUPS};`). The only caller outside `src/ui/` is
  `src/app/panels.rs`, which goes through `crate::ui::shortcuts_dialog` and does
  not change. `tests/lcv133_shortcuts_dialog_fits.rs` imports the deep path
  `lasercad::ui::dialogs::{…}` and must be repointed — or, better, moved to the
  `lasercad::ui::{…}` re-export while it is being touched.
- **Three `include_str!` needles move with the tests.**
  `the_shortcuts_dialog_reads_no_key`,
  `the_dialog_holds_no_second_copy_of_the_tool_table` and
  `implementation_source` all read `include_str!("dialogs.rs")`; after the move
  they read `include_str!("shortcuts_dialog.rs")`. A scan left pointing at the
  old file keeps passing and stops proving anything, which is the same
  self-match failure `src/lib.rs`'s
  `run_arms_native_dialogs_as_its_first_statement` already documents.
  `include_str!("shortcuts.rs")` and `include_str!("../app/input.rs")` resolve
  unchanged from `src/ui/`.
- **On the name.** `src/ui/shortcuts.rs` reads keys; `src/ui/shortcuts_dialog.rs`
  must not, and that is enforced by test, not by naming — the moved
  `the_shortcuts_dialog_reads_no_key` is exactly that enforcement, and ADR 0002
  §A6's "two key readers" invariant is unchanged by the move.

**What the cap binds under `tests/`.**
*(Amended (3), 2026-09-14. Rule 3's exemptions were written against `src/`.
Nothing below is a third exemption; it is exemption 2 stated where it already
applied.)*

Exemption 2 — *a file that contains **only** test code* — decides both cases
under `tests/`, in opposite directions:

- **An integration-test binary directly in `tests/` is exempt.** It is entirely
  test code; the cap has never bound it and does not now. Measured on
  2026-09-14 with the rule 2 recipe: `tests/lcv123_agent_turn.rs` is **895**,
  `tests/lcv129_agent_timeout_and_cancel.rs` **557**,
  `tests/lcv125_agent_panel_and_settings.rs` **552**. None of those is a
  finding, and none should be reported as one. What keeps such a file honest is
  that it holds one demand's assertions; when a second demand wants its
  helpers, they move to `tests/harness/` — and the cap picks them up there.
- **`tests/harness/` is *not* exempt.** It carries no test body of its own (its
  tests live in `tests/lcv132_harness_is_shared.rs`), it is shared
  implementation compiled into every consumer binary, and it is the code a
  reviewer reads to decide whether a painted-text assertion means anything.
  LCV-132 AC 11 already measured every file there against the cap.

**The `tests/harness/paint.rs` seam, pre-decided per rule 4.**

`tests/harness/paint.rs` is at **267** implementation lines at `dbca0a3`,
measured with the rule 2 recipe (the three harness modules are `mod.rs` **152**,
`paint.rs` **267**, `scan.rs` **106** — 525 in total). 267 is three lines
*below* the 270 band, so rule 4 does not formally bind yet and the file is not
split now. The seam is recorded anyway, for three reasons: LCV-132 AC 11 set 270
as its own ceiling and flagged `architect` at 267 as rule 4 asks; three queued
demands (LCV-139, LCV-140, LCV-141) depend on LCV-132 and each will add
painted-text assertions; and [ADR 0002](0002-headless-input-tests-and-dirty-tracking.md)
§A3's `pixels_per_point` guard alone lands the file near 272.

**The seam is production versus interpretation of a `Run`.**

| stays in `tests/harness/paint/mod.rs` | moves to `tests/harness/paint/lines.rs` |
|---|---|
| the "painted text, not a scan" preamble, traps 6, 7 and 8, `Run` with `Run::y` / `Run::x`, `collect_text`, `runs_in`, `painted_runs_at`, `painted_runs` | traps 1–5, the assertion idiom, `SAME_LINE`, `scoped_runs`, `group_into_lines`, `lines_on_surface_of`, `texts` |

Six notes make it executable without a second decision:

- **Why a directory, not a sibling `tests/harness/lines.rs`.**
  `tests/harness/paint/mod.rs` re-exports the moved names
  (`pub use lines::{group_into_lines, lines_on_surface_of, scoped_runs, texts, SAME_LINE};`),
  so every `use harness::paint::{…}` in the five consumers —
  `tests/lcv125_agent_panel_and_settings.rs`,
  `tests/lcv126_command_line_group.rs`,
  `tests/lcv129_agent_timeout_and_cancel.rs`,
  `tests/lcv132_harness_is_shared.rs` and
  `tests/lcv133_shortcuts_dialog_fits.rs` — resolves unchanged, and the split
  repoints no test. It is also what `AGENTS.md` §Module tree asks of a directory
  module. A sibling module renames the concept at every `harness::paint::` call
  site to save one directory.
- **The seam is where `Run` changes hands, and it is where the growth is.** The
  collector is four functions over an API pinned at 0.29.1 and is finished;
  what every next demand adds is *interpretation* — a surface's width, a second
  column bucketing, a new grouping. "The thing that grew is what moves out" —
  the rule the `src/app/mod.rs` and `src/ui/dialogs.rs` seams above apply —
  points the same way. **The placement rule for the next helper follows from
  it: anything that reads a `Run` belongs in `lines.rs`; only something that
  produces one belongs in `mod.rs`.** That includes
  `tests/lcv126_command_line_group.rs`'s private column bucketing (trap 3) if a
  second surface ever needs it.
- **`Run::y` / `Run::x` stay with `Run`,** though only `lines.rs` calls them.
  Splitting an inherent impl across files to move eight lines buys nothing and
  costs a reader the type's whole surface.
- **The arithmetic.** Roughly 77 code lines plus the five traps and the
  assertion idiom (~55 header lines) move: `paint/mod.rs` lands near **140**,
  `paint/lines.rs` near **135**. Real headroom on both, which is what rule 4
  asks a seam to produce.
- **What the split must repoint** — three things, and only one of them is found
  by a compiler:
  1. **The rustdoc intra-doc links that cross the new boundary** — the bracketed
     references to `group_into_lines` (trap 1 and `Run::text`), `SAME_LINE`
     (trap 4 and `Run`), `painted_runs_at` (trap 7) and `Run::y` (trap 8) —
     become `super::` / `lines::` paths. `cargo doc` is not in the gate, so a
     broken one **fails silently**. This is the part to check by hand.
  2. `tests/harness/mod.rs`'s module header, which opens "Three modules, divided
     by kind" and gives `paint` one line.
  3. `AGENTS.md` §Implementation Rules' painted-text entry, which cites
     `tests/harness/paint.rs` and becomes `tests/harness/paint/`.

  Nothing in `tests/lcv132_harness_is_shared.rs` moves: it imports
  `collect_text`, `runs_in` and `Run`, all collection-side.
- **Trigger, and who executes it.** The first demand that would take
  `tests/harness/paint.rs` over **300** executes this split as its first commit,
  before its own change. A demand that adds an *interpretation* helper while the
  file is still under 300 should execute it first too, rather than land a helper
  in the file it is about to leave. On today's queue that is **LCV-141** (the
  agent panel within the right third — a geometry claim over a surface rect),
  with LCV-139 and LCV-140 the next candidates; whichever arrives first owns it,
  and it needs no new decision.

**The rest of the band, for the record.** `src/app/agent_apply.rs` is at 284,
`src/app/file_ops.rs` at 281, `src/app/agent_turn.rs` at 272 and
`src/io/settings.rs` at 265. Only `agent_apply.rs` needs a seam named, and it is
obvious: `apply`, `transcribe`, `plan` and `in_range` are the command half,
while `with_count` through `list_selection` are ten pure `String`-returning
formatters with no `App` and no `Command` between them. Those ten move to
`src/app/agent_prose.rs` when a demand crosses the cap. `agent_turn.rs` has no
pre-decided seam; what it must **not** absorb is `AgentState`, per the note
above.

## Alternatives considered

- **Leave `AGENTS.md` alone and expect reviewers to read ADR 0002** — already
  failed once, in exactly the way a rule that lives in two places fails.
- **Count only non-comment, non-blank code lines** — needs a tool nobody has
  wired up, changes the cap's effective strictness silently, and rescores every
  file in the tree at once.
- **Raise the cap to 400** — the cap is doing its job; the twelve files that
  exceed 300 *total* lines are exceeding it on test code, which is exactly the
  thing that should not be rationed.
- **Split `src/app/file_ops.rs` now, preemptively** — rule 4 exists to reject
  this. 19 lines of headroom with no demand queued against the file is not a
  reason to touch it.
- **Exempt doc comments** — see Consequences; a second fuzzy rule for a problem
  no file has yet.

## Revisit criteria

- A third file legitimately needs an exemption → amend the list by a new ADR,
  not by review precedent.
- A `cargo`-native LOC tool lands in the toolchain pin → replace the `awk`
  recipe, keep the definition.
- `src/app/file_ops.rs` crosses 300 → execute the seam above; no new decision
  is required.
- `src/app/mod.rs` or `src/app/agent_apply.rs` crosses 300 → execute the seam
  named for it in Consequences; no new decision is required.
- Any demand adds an implementation line to `src/ui/dialogs.rs` (at 299) →
  execute the seam named for it in Consequences first; no new decision is
  required.
- `tests/harness/paint.rs` (at 267) crosses 300, **or** a demand adds a helper
  to it that reads a `Run` rather than producing one → execute the seam named
  for it in Consequences first; no new decision is required.
