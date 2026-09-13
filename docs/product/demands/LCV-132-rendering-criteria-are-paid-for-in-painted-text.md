# LCV-132 — A rendering criterion is paid for in painted text, not in a source scan

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-125 (its rework lands the paint-list infrastructure this demand promotes into the harness; nothing to extract until then)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

Every UI demand in this repo has been written against a premise that is half
wrong, and the half that is wrong is the half that matters.

The premise: egui rendering cannot be asserted at the pinned 0.29.1, because
`egui_kittest` needs ≥ 0.30. Therefore a rendering acceptance criterion gets a
**bounded source scan** — "the panel's implementation section contains the
needle `TextStyle::Monospace`" — plus a headless frame proving the path does not
panic. That is what LCV-116, LCV-121, LCV-122 and LCV-125 all did.

The true half is that **pixels** cannot be asserted. The false half is the
conclusion. **Painted text runs can be asserted, today, at 0.29.1, with no new
dependency and no version bump.** `egui::Context::run` returns
`FullOutput { shapes, .. }`; each `ClippedShape` carries a `Shape`; and
`Shape::Text(TextShape { pos, galley, .. })` yields the exact string that was
laid out and where it was put.

### What the gap costs, measured

`reviewer-rust` wrote the probe during the LCV-125 review, ran it against the
shipped code, and used it to kill **five mutants that survived the full gate** —
`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and
all 1208 tests green:

| mutation applied to the shipped code | breaks | caught by the gate |
|---|---|---|
| wrap the transcript loop in `ui.horizontal_wrapped` — every row runs into one paragraph | LCV-125 AC 2, AC 5 | **no** |
| iterate the transcript `.rev()` — newest row first | LCV-125 AC 5 | **no** |
| `if role == "note" { continue; }` — the undo-shape note never reaches the screen | LCV-125 AC 4 | **no** |
| guard the plaintext-key warning on a non-empty key — invisible on exactly the frame an operator is about to paste a key in | LCV-125 AC 10 | **no** |
| the same guard on the step-budget explanation | LCV-125 AC 9 | **no** |

Five stated acceptance criteria, five silent breakages. The reason is one
sentence and it belongs in every future UI demand: **a source scan asserts that
a call is written; nothing asserted that the paint happened.** Those are
different claims. The gap between them is where all five live, and it is also
where the fourth one lives — a warning that vanishes at the moment it is needed
is a security-relevant regression that the scan for its sentence cannot see,
because the sentence is still right there in the source.

This matters beyond one panel. The repo has shipped **six un-failable source
scans**, each of which passed review at the time. The scan is the weakest test
form in use here by a wide margin, and the paint-shape seam is strictly stronger
at the same cost to write. The next person deciding between the two should find
that written down rather than rediscovering it.

### Verified independently, at the pin

`product-owner` re-ran the probe against `HEAD` before opening this demand. It
works, it costs **0.01 s** for two full-app frames, and the shipped plaintext
warning comes back verbatim as a single run — all 100 characters of it, which is
the shape LCV-125 AC 10 always wanted:

```
y= 421 x=  408 "The API key is stored in plain text in settings.json. Anyone who can read that file can read your key."
```

It also turned up five traps, every one of which a naive reading of the snippet
in the review walks straight into. They are why this demand exists as a
*promotion* rather than as an instruction to "write a helper":

- **Paint order is not reading order.** In the settings dialog the runs come
  back as: menu bar (`y=4`), status bar (`y=782`), command line (`y=760`),
  toolbar (`y=26..275`), the dialog's own fields (`y=353..473`), and the window
  title `"Agent Settings"` **last** (`y=318`). Asserting on raw paint order
  across surfaces is asserting on egui's internals.
- **One frame paints five surfaces and they interleave vertically.** The
  toolbar's `Polyline` lands six points from the transcript's second row.
  Sorting the whole frame by `y` braids unrelated surfaces together. The way out
  is `ClippedShape::clip_rect`: egui clips a panel's contents to the panel and a
  scroll area's contents to a rect inside it, so surface membership is rect
  containment — but it also means a **`Window` title is clipped to the whole
  screen**, so a title can never be used as the marker that identifies a
  surface.
- **Runs on one visual row do not share a `y`.** `"API Key"` paints at `y=402`
  and its own field at `y=401`; a panel heading and its `×` differ by four.
  Grouping a visual line by `y` equality is wrong on the shipped UI as it
  stands, and sorting by `(y, x)` reads a `Grid`'s value *before* its label.
  A tolerance is required, and it must be justified by measurement, not by
  taste: the closest two genuinely different lines on the two surfaces under
  test are 13 points apart.
- **Empty runs exist.** An empty `TextEdit` emits a `Shape::Text` with an empty
  galley (`y=401`, `y=192`), and hint text paints at the same `y` as the empty
  content it hints. Dropping empty runs is what lets a line assert as
  `["API Key"]` and *mean* "the key field shows nothing at all".
- **The first frame is not settled.** Sizes are allocated on the frame after a
  widget is first seen, so a run may be missing or misplaced on frame one. That
  is safe for a presence assertion — it fails loudly — and **unsafe for an
  absence assertion**, which passes for the wrong reason.

### Why this is a promotion, not an invention

LCV-125's rework is landing all of that right now, inside
`tests/lcv125_agent_panel_and_settings.rs`: a `Run` type, a recursive
`collect_text`, a `painted_runs` frame driver with a positive control, a
measured `SAME_LINE` tolerance, a `lines_on_surface_of` that scopes by clip rect
and groups into visual lines, and a `texts` that drops the coordinates so
assertions read as literal tables. It is roughly **110 lines of general test
infrastructure sitting in one demand's test file**, with three call sites and
nothing in it specific to the agent panel.

That is the whole demand: move it to `tests/harness/`, where ADR 0002 §A3 says
shared plumbing lives, before the second UI demand copy-pastes it and the two
copies drift. The design work is done and reviewed; this demand must **not**
redesign it.

## Scope

- Moving LCV-125's paint-list infrastructure into `tests/harness/mod.rs`
  unchanged in behaviour, with its doc comments intact.
- Those doc comments carrying the five traps above. They are as much the
  deliverable as the code.
- The documented assertion idiom — a literal table of visual lines — and one
  named alternative for when a test needs to prove two lines are distinct.
- Rewriting LCV-125's three call sites to use the harness copy, so it ships with
  three real callers and exactly one collector in the tree.
- One rule in `AGENTS.md` §Implementation Rules: a rendering acceptance
  criterion is not satisfied by a source scan alone.
- Deleting the scratch probe if it is still in the tree.

## Out of scope

- **Upgrading egui, or adding `egui_kittest`.** The entire point is that this
  works at the pinned **0.29.1**. "Better rendering tests" is not a licence to
  bump a version — a bump re-validates every headless test in the repo and every
  trap recorded in ADR 0002, and that is a separate demand with a separate risk
  budget. AC 8 pins `Cargo.toml` and `Cargo.lock` as unchanged so this cannot
  arrive by the side door.
- **Pixels.** No snapshot images, no image diffing, no golden files, no
  rasterising. The true half of the old premise stays true.
- **Asserting colour, font, text style, size, or any rect.** LCV-125 AC 2 says a
  `tool` row "is not the same colour as assistant prose"; that stays a source
  scan, because a colour assertion is a brittle claim about a theme. This demand
  raises the floor for *text and its order*, not for appearance.
- **Absolute coordinates.** No test asserts that anything is at `y=421`. Every
  positional claim is relative — strictly-increasing, or distinct. Absolute
  values depend on font metrics, theme spacing and screen size, and would make
  the harness a source of flakes rather than a cure for them.
- **Retrofitting the already-shipped UI.** The status bar, the shortcuts dialog,
  the toolbar, the viewport overlay and the command line keep the tests they
  have. Retrofitting is unbounded; the rule that earns its keep is that the
  *next* UI demand reaches for this. Rewriting LCV-125's three call sites is
  de-duplication of the code this demand extracts, not a retrofit.
- **Re-opening LCV-125 AC 6.** A paint-shape test could now assert that the
  dummy key appears in no painted run anywhere on screen, which is stronger than
  what AC 6 asserts. It is still out of scope here: LCV-125 is closing, and this
  demand does not reopen a criterion that passed.
- **A UI-testing framework.** No widget queries, no click-by-label, no
  accessibility tree, no `find_button("Send")`. Functions that hand back text,
  positions and visual lines; every judgement stays at the call site.
- **Making the helper reachable from `#[cfg(test)]` modules under `src/`.**
  `tests/harness/` is not compiled into the library. Paint-shape assertions live
  in `tests/`, one file per demand, per ADR 0002 §A3 — a second copy under
  `src/` would be the thing this demand exists to prevent.

## Acceptance criteria

1. **The infrastructure moves, whole, and does not change behaviour.** Six
   items move from `tests/lcv125_agent_panel_and_settings.rs` into
   `tests/harness/mod.rs` and become `pub`, keeping their names and their doc
   comments: the `Run` type (`clip`, `y`, `x`, `text`), `SAME_LINE`,
   `collect_text`, `painted_runs`, `lines_on_surface_of`, and `texts`. The move
   is a move: **no renames, no signature changes, no added parameters, no
   generalisation** beyond the `pub` visibility and whatever the module split
   forces. A behavioural change smuggled in as part of a move is a review
   blocker, because the three call sites are the only thing proving the move was
   faithful.

   `painted_runs` keeps its `runs.len() > 10` positive control. A harness helper
   that asserts is unusual and deliberate: it is what stops a test that renders
   nothing at all from reading as "the string is absent, as expected".

2. **Nesting and clip propagation are proven, not assumed.** A test hands
   `collect_text` a hand-built `Shape::Vec` containing a `Shape::Text`, nested
   two deep, and asserts the inner string comes back **and carries the clip rect
   of the `ClippedShape` it was nested inside**. `Shape::Vec` is the only shape
   variant at 0.29.1 that contains other shapes; the doc comment says so, so an
   egui bump has one named place to revisit. Nesting is currently exercised only
   incidentally by whatever the shipped widgets happen to emit — this pins it.

3. **It ships with three callers and no fourth.** The three `painted_runs` call
   sites in `tests/lcv125_agent_panel_and_settings.rs` are rewritten against the
   harness copy and the private originals are deleted — **one collector in the
   tree, not two**. The converted tests assert exactly what they asserted before
   the move, and the diff removes more lines than it adds. No other test file is
   touched. If LCV-125's rework has not landed when this demand starts, **stop
   and report**: there is nothing to extract, and writing a helper against a
   hypothetical caller is how speculative APIs get in.

4. **The five mutants die, and the report says which test killed each.** This is
   the criterion that decides whether the demand worked. For each of the five
   mutations tabulated in §Problem, the implementer applies it to the shipped
   code, runs `cargo test --all --no-fail-fast`, records the **test name and the
   assertion message** that fails, and reverts. Five mutations, five named
   failures, all reported in the handover. A mutation that survives is a finding
   to report, not a criterion to quietly relax.

5. **The five traps are in the module header, in the harness's existing voice.**
   The file already opens with four numbered hard rules, each carrying its
   reason rather than just its rule, and that is why they work. This demand adds
   a fifth through ninth in the same shape — paint order is not reading order;
   surfaces interleave and membership is clip-rect containment, so a `Window`
   title can never be the marker; runs on one visual row do not share a `y`
   (the `402`/`401` label-and-field case, and why `SAME_LINE` is 6 and not a
   round number); empty runs exist and are dropped; the first frame is not
   settled, so an **absence** assertion must be made on a later one. Where the
   moved doc comments already say a thing, the header cites them rather than
   repeating them.

6. **The assertion idiom is written down, with what it catches.** In the same
   header:
   - **A literal table of visual lines** —
     `assert_eq!(texts(&lines_on_surface_of(&runs, marker)), vec![vec!["…"], …])`.
     One assertion covering presence, verbatim text, left-to-right order within
     a line, and top-to-bottom order of lines. `texts` drops the `y`, which is
     what keeps absolute coordinates out of the assertion (§Out of scope).
     This one form kills all five mutants in §Problem: a missing row shortens
     the table, a reversal reorders it, and `horizontal_wrapped` collapses seven
     lines into one.
   - **The y-carrying variant**, `lines_on_surface_of` without `texts`, only
     when a test must prove two lines are *distinct* rather than merely present
     and ordered — and then as a strict inequality, never as a value.

   The marker is a string the test injected into its own fixture content
   (LCV-125 uses `LCV125ROW-{role}`) or a shipped label the test has pinned
   elsewhere, and `lines_on_surface_of` already asserts it is painted **exactly
   once**. Never a substring: a test controls its fixture text, it does not
   control the product's copy, and a filter that matches shipped copy silently
   changes meaning the day the copy changes.

7. **The rule is recorded where the next UI demand will hit it.** `AGENTS.md`
   §Implementation Rules gains one entry, in the existing voice: a rendering
   acceptance criterion is not satisfied by a source scan alone — a scan asserts
   that a call is written, not that the paint happened — and painted text is
   assertable at the pinned egui via `tests/harness/mod.rs`. It cites this
   demand and the five surviving mutants as the witness, the way the
   path-comparison rule cites its two CI breakages.

   Two hand-offs the implementer **opens as tasks and does not action**:
   ADR 0002 §A3 enumerates the harness as "exactly five items" and it is already
   stale — eight ship today, ten after this demand — and the rule in this
   criterion may belong in ADR 0002 rather than `AGENTS.md`. Both are
   `architect`'s file and `architect`'s call.

8. **No dependency moved, and the caps hold.** `git diff` over `Cargo.toml` and
   `Cargo.lock` is **empty** — no new crate, no egui bump, no feature flag.
   `tests/harness/mod.rs` stays at or under 300 implementation LOC by ADR 0004's
   `awk` recipe, never `wc -l`, and the number is reported — **117 before, and
   roughly 110 lines are moving in**, so this lands near 230 and the headroom
   matters from here on. If it comes out above 270, flag `architect` and do not
   split the file on your own initiative (ADR 0004). The
   scratch probe `tests/zz_probe_lcv125.rs` is deleted if present; it prints and
   asserts nothing, and must never be committed. Full gate green:
   `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --all --no-fail-fast`.

## Expected tests

- **Unit / AC 1** — the harness compiles and all six items are reachable from a
  consumer binary; `painted_runs` against `App::default()` with the agent
  settings dialog open returns the plaintext warning sentence as an
  exact-equality match, and `lines_on_surface_of` scoped by that sentence
  returns fewer runs than the frame painted.
- **Unit / AC 2** — the hand-built nested-`Shape::Vec` test. Prove it
  discriminates twice: flatten `collect_text` to a non-recursive `match` and
  show it goes red; hand the nested run a different clip rect and show the
  propagation assertion goes red.
- **Integration / AC 3** — the three converted LCV-125 tests still pass,
  asserting the same things they asserted before the move. A scan is not needed
  here; the diff is the evidence, and it must remove more lines than it adds.
- **Mutation / AC 4** — the five, by name, each reverted after measuring:
  (a) `ui.horizontal_wrapped` around the transcript loop;
  (b) `.rev()` on the transcript iteration;
  (c) `if role == "note" { continue; }`;
  (d) the plaintext-key warning guarded on a non-empty key;
  (e) the step-budget explanation guarded the same way.
  Report the failing test name and assertion message for each. **(d) is the one
  that matters most** — it is the only one of the five with a security
  consequence, and it is invisible to every test form the repo had before.
- **Manual / AC 5, AC 6** — the doc comment reads in the file's voice, carries
  all five traps with their reasons and both idioms with what each catches.
- **Manual / AC 7** — the `AGENTS.md` entry exists; the two tasks for
  `architect` exist and cite the 5-vs-8 drift.
- **Manual / AC 8** — `git diff --stat` over `Cargo.toml` and `Cargo.lock` is
  empty; the `awk` number is in the handover; the probe file is gone.
- **[manual] smoke** — none for product behaviour; this demand ships no
  user-visible change and must not. One timing check instead: report the wall
  time of `cargo test --all --no-fail-fast` before and after. Two full-app
  frames cost 0.01 s, so the expected delta is noise — a measurable regression
  means the helper is doing something it was not asked to do.

## Test hygiene (mandatory)

- The source scans this demand replaces are not deleted where they still assert
  something a paint test cannot see — colour, style, the absence of a `Document`
  import. Adding a paint test is not a licence to drop a scan; it is a licence to
  stop pretending a scan covers rendering.
- Any scan this demand does add is bounded to the implementation section (bare
  `#[cfg(test)]` at column 0) with needles built by `concat!`. Canonical
  example: `guard_is_runtime_not_cfg` at `src/io/dialogs.rs:179-194`.
- Rebuild compared paths from `components()` joined with `/` — never
  `Path::display()`. This has broken CI twice.
- No test asserts an absolute coordinate. See §Out of scope.
- No test reaches a real endpoint.

## Open questions

None. Both judgement calls that were open — where the rule lives, and whether
ADR 0002 §A3's stale enumeration is fixed here — are folded into AC 7 as
hand-offs to `architect` rather than left as questions, so this demand does not
wait on them. This demand is Ready when `demand-manager` flips it.

## Notes

- Origin: `reviewer-rust`, 2026-09-13, during the LCV-125 review. It did not
  report the gap in the abstract — it wrote the collector, ran it against the
  shipped panel, and killed five mutants with it. Re-run and confirmed by
  `product-owner` at `bc9ca22` before this demand was opened; the trap list in
  §Problem is from that run, not from the report.
- **The next customer is already known.** LCV-129's Cancel button (its AC 8 —
  the button renders only while a turn is running — and its AC 7, where the
  cancel note and the undo-shape note must land in that order) is the first
  rendering criterion queued behind this one. If this demand lands first, LCV-129's implementer uses the harness;
  LCV-129's expected-tests section says so conditionally, so neither demand
  blocks the other.
- Ordering: this demand cannot start before LCV-125's rework lands (AC 3), and
  is most valuable if it lands before LCV-129 is implemented. That is a
  scheduling preference for `project-manager`, not a dependency.
- Forward-looking by decision, not by omission: the status bar, shortcuts dialog
  and toolbar keep their current tests. See §Out of scope for why.
- Related: `tests/harness/mod.rs`,
  [ADR 0002 §A3/§A4](../../adr/0002-headless-input-tests-and-dirty-tracking.md),
  [ADR 0004](../../adr/0004-measuring-the-300-loc-cap.md) (the `awk` recipe),
  `AGENTS.md` §Implementation Rules, LCV-125 (AC 2, 4, 5, 9, 10 — the five
  criteria the mutants broke), LCV-129 (the next customer).
