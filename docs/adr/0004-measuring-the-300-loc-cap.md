# ADR 0004 — Measuring the 300-LOC cap

- **Status**: Accepted
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
